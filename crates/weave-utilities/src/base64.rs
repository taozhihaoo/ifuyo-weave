//! Base64（M9 上 §37-§43）：Standard / URL-safe、Padded / Unpadded 显式
//! 区分（§38）；Decode 严格校验（§40，lenient 为显式选项）；解码上限
//! （§43 Max decoded bytes）；编码输入按显式编码（§39，复用 M4 层）。

use weave_core::prelude::TextEncoding;

use crate::{UResult, UtilityDiagnostic, UtilityError, UtilityErrorKind, warn};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Base64Alphabet {
    /// RFC 4648 标准（+ /）。
    Standard,
    /// RFC 4648 base64url（- _；§38 不得与 Standard 混称）。
    UrlSafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Base64Padding {
    Include,
    Omit,
}

/// §43 解码安全上限（防解压式膨胀）。
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Base64DecodeLimits {
    /// 最大解码输出字节数。
    pub max_decoded_bytes: f64,
}

impl Default for Base64DecodeLimits {
    fn default() -> Self {
        Self {
            max_decoded_bytes: 64.0 * 1024.0 * 1024.0,
        }
    }
}

const STANDARD: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
const URLSAFE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789-_";

fn encode_impl(data: &[u8], alphabet: Base64Alphabet, padding: Base64Padding) -> String {
    let table: &[u8; 64] = match alphabet {
        Base64Alphabet::Standard => STANDARD,
        Base64Alphabet::UrlSafe => URLSAFE,
    };
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = *chunk.get(1).unwrap_or(&0) as u32;
        let b2 = *chunk.get(2).unwrap_or(&0) as u32;
        let n = (b0 << 16) | (b1 << 8) | b2;
        out.push(table[(n >> 18) as usize & 63] as char);
        out.push(table[(n >> 12) as usize & 63] as char);
        if chunk.len() > 1 {
            out.push(table[(n >> 6) as usize & 63] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(table[n as usize & 63] as char);
        } else {
            out.push('=');
        }
    }
    if padding == Base64Padding::Omit {
        while out.ends_with('=') {
            out.pop();
        }
    }
    out
}

/// 编码（§39：输入按显式编码转字节——非 UTF-8 编码经 encoding_rs 统一层）。
pub fn base64_encode(
    text: &str,
    encoding: TextEncoding,
    alphabet: Base64Alphabet,
    padding: Base64Padding,
) -> UResult<String> {
    let bytes = encode_input_bytes(text, encoding)?;
    Ok(encode_impl(&bytes, alphabet, padding))
}

fn encode_input_bytes(text: &str, encoding: TextEncoding) -> UResult<Vec<u8>> {
    match encoding {
        TextEncoding::Utf8 | TextEncoding::Utf8Bom | TextEncoding::Ascii => {
            Ok(text.as_bytes().to_vec())
        }
        TextEncoding::Utf16Le => Ok(text.encode_utf16().flat_map(|u| u.to_le_bytes()).collect()),
        TextEncoding::Utf16Be => Ok(text.encode_utf16().flat_map(|u| u.to_be_bytes()).collect()),
        TextEncoding::Latin1 => Ok(text.chars().map(|c| c as u8).collect()),
        TextEncoding::Gb18030 | TextEncoding::Gbk => {
            // M4 统一层负责 GB 编码（charter #27）；utilities 侧仅 UTF 系
            Err(UtilityError::new(
                UtilityErrorKind::UnsupportedEncoding,
                "base64.encodingUnsupportedHere",
                "GB-family encoding for base64 input: use the Text tool pipeline (M4)",
            ))
        }
        TextEncoding::Unknown => Err(UtilityError::new(
            UtilityErrorKind::UnsupportedEncoding,
            "base64.encodingUnknown",
            "encoding must be explicit (§39)",
        )),
    }
}

/// 解码（§40 严格；§43 上限；lenient = 忽略空白，仍校验字母表/长度）。
pub fn base64_decode(
    input: &str,
    alphabet: Base64Alphabet,
    lenient: bool,
    limits: &Base64DecodeLimits,
) -> UResult<(Vec<u8>, Vec<UtilityDiagnostic>)> {
    let table: &[u8; 64] = match alphabet {
        Base64Alphabet::Standard => STANDARD,
        Base64Alphabet::UrlSafe => URLSAFE,
    };
    let cleaned: String = if lenient {
        input.chars().filter(|c| !c.is_whitespace()).collect()
    } else {
        input.to_owned()
    };
    if cleaned.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    // §40 padding/长度校验
    let trimmed = cleaned.trim_end_matches('=');
    let pad_count = cleaned.len() - trimmed.len();
    if pad_count > 2 {
        return Err(UtilityError::invalid_input(
            "base64.invalidPadding",
            "too many '=' padding characters",
        ));
    }
    if !lenient && !cleaned.len().is_multiple_of(4) {
        return Err(UtilityError::invalid_input(
            "base64.invalidLength",
            format!(
                "length {} is not a multiple of 4 (strict mode; lenient omits padding)",
                cleaned.len()
            ),
        ));
    }
    let mut acc: u32 = 0;
    let mut bits = 0u32;
    let mut out: Vec<u8> = Vec::with_capacity(cleaned.len() / 4 * 3 + 3);
    for (pos, ch) in trimmed.char_indices() {
        let v = match table.iter().position(|t| *t as char == ch) {
            Some(v) => v as u32,
            None => {
                return Err(UtilityError::invalid_input(
                    "base64.invalidCharacter",
                    format!(
                        "invalid character '{ch}' at {pos} for {:?} alphabet (§40: no silent repair)",
                        alphabet
                    ),
                ));
            }
        };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
        if out.len() as f64 > limits.max_decoded_bytes {
            return Err(UtilityError::new(
                UtilityErrorKind::ResourceLimitExceeded,
                "base64.decodedTooLarge",
                format!("decoded output exceeds limit {}", limits.max_decoded_bytes),
            ));
        }
    }
    // 末尾残余位必须为 0（严格 canonical）
    if !lenient && bits > 0 && (acc & ((1 << bits) - 1)) != 0 {
        return Err(UtilityError::invalid_input(
            "base64.nonCanonicalTail",
            "trailing bits are non-zero (non-canonical encoding)",
        ));
    }
    let mut warnings = Vec::new();
    if lenient && pad_count == 0 && !cleaned.len().is_multiple_of(4) {
        warnings.push(warn(
            "base64.lenientUsed",
            "lenient decode accepted non-padded input",
        ));
    }
    Ok((out, warnings))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn golden_rfc4648_vectors() {
        // RFC 4648 test vectors（Standard + padded）
        for (input, expected) in [
            ("", ""),
            ("f", "Zg=="),
            ("fo", "Zm8="),
            ("foo", "Zm9v"),
            ("foob", "Zm9vYg=="),
            ("fooba", "Zm9vYmE="),
            ("foobar", "Zm9vYmFy"),
        ] {
            assert_eq!(
                base64_encode(
                    input,
                    TextEncoding::Utf8,
                    Base64Alphabet::Standard,
                    Base64Padding::Include
                )
                .expect("ok"),
                expected,
                "{input}"
            );
        }
    }

    #[test]
    fn urlsafe_differs_and_unpadded_omits() {
        // §38：base64url 与标准不得混称——Latin1 "ÿþ" = [255,254]，
        // 首个 6bit=63 ⇒ Standard '/' vs UrlSafe '_'
        let std = base64_encode(
            "ÿþ",
            TextEncoding::Latin1,
            Base64Alphabet::Standard,
            Base64Padding::Omit,
        )
        .expect("ok");
        let url = base64_encode(
            "ÿþ",
            TextEncoding::Latin1,
            Base64Alphabet::UrlSafe,
            Base64Padding::Omit,
        )
        .expect("ok");
        assert_ne!(std, url);
        assert!(std.contains('/'), "{std}");
        assert!(url.contains('_'), "{url}");
        let padded = base64_encode(
            "f",
            TextEncoding::Utf8,
            Base64Alphabet::Standard,
            Base64Padding::Include,
        )
        .expect("ok");
        let unpadded = base64_encode(
            "f",
            TextEncoding::Utf8,
            Base64Alphabet::Standard,
            Base64Padding::Omit,
        )
        .expect("ok");
        assert_eq!(padded, "Zg==");
        assert_eq!(unpadded, "Zg");
    }

    #[test]
    fn utf16_encoding_changes_bytes_and_result() {
        // §39：显式编码——"hi" UTF-16LE = 4 字节
        let r = base64_encode(
            "hi",
            TextEncoding::Utf16Le,
            Base64Alphabet::Standard,
            Base64Padding::Include,
        )
        .expect("ok");
        // LE("hi") = [0x68,0x00,0x69,0x00]
        assert_eq!(r, "aABpAA==");
    }

    #[test]
    fn strict_decode_roundtrip_and_errors() {
        let limits = Base64DecodeLimits::default();
        let (bytes, _) =
            base64_decode("Zm9vYmFy", Base64Alphabet::Standard, false, &limits).expect("ok");
        assert_eq!(bytes, b"foobar");
        // §40：非法字符结构化错误（长度 4 合法 ⇒ 字符校验触发）
        let e = base64_decode("Zm9!", Base64Alphabet::Standard, false, &limits).expect_err("bad");
        assert_eq!(e.code, "base64.invalidCharacter");
        // 非法字母表字符（- 不属于 Standard）
        let e = base64_decode("A-B=", Base64Alphabet::Standard, false, &limits).expect_err("bad");
        assert_eq!(e.code, "base64.invalidCharacter");
        // 长度非 4 倍数（strict）
        let e = base64_decode("Zg", Base64Alphabet::Standard, false, &limits).expect_err("len");
        assert_eq!(e.code, "base64.invalidLength");
        // lenient 接受无 padding
        let (bytes, _) = base64_decode("Zg", Base64Alphabet::Standard, true, &limits).expect("ok");
        assert_eq!(bytes, b"f");
        // §43 解码上限
        let tight = Base64DecodeLimits {
            max_decoded_bytes: 2.0,
        };
        let e =
            base64_decode("Zm9vYmFy", Base64Alphabet::Standard, false, &tight).expect_err("limit");
        assert_eq!(e.kind, UtilityErrorKind::ResourceLimitExceeded);
    }
}
