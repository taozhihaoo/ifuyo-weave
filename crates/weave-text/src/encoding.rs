//! 文本解码（M4 §15/§16）：Detect → Decode → Validate → Present。
//!
//! 原则（§15）：绝不"强制 utf-8 → 失败 → 静默替换字符"。解码级联：
//!
//! ```text
//! BOM 命中（UTF-8/UTF-16LE/UTF-16BE）→ 按 BOM 严格解码（错误 ⇒ 报错，不替换）
//! 无 BOM：UTF-8 严格解码成功 ⇒ Utf8（全 ASCII 时如实标 Ascii）
//!       → GB18030 严格解码成功 ⇒ Gb18030（家族保守归类，D38）
//!       → 否则 Latin-1（字节→U+00xx 无损映射，如实标注兜底）
//! ```
//!
//! 任何路径都**不产生 U+FFFD 替换符**（encoding_rs 的 had_errors 标志
//! 不通过即降级/报错）。用户可在 UI 显式指定编码覆盖自动检测（§21 同源）。

use weave_core::prelude::TextEncoding;

/// UTF-8 BOM
const BOM_UTF8: [u8; 3] = [0xEF, 0xBB, 0xBF];
const BOM_UTF16LE: [u8; 2] = [0xFF, 0xFE];
const BOM_UTF16BE: [u8; 2] = [0xFE, 0xFF];

/// BOM 事实（§16：Formatter/Transformer/Save 的策略基线，D39）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bom {
    Utf8,
    Utf16Le,
    Utf16Be,
    None,
}

/// 解码结果：内容 + 编码事实 + BOM 事实。`encoding` 为 Latin-1 兜底时
/// 调用方（UI）必须如实呈现"Latin-1 fallback"，不得伪装成 UTF-8。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedText {
    pub content: String,
    pub encoding: TextEncoding,
    pub bom: Bom,
}

/// 解码失败（当前只有 BOM 路径可能失败：截断的 UTF-16 BOM 序列等）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodeError {
    pub code: &'static str,
    pub message: String,
}

impl DecodeError {
    fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

/// 解码字节为文本（自动检测级联，见模块文档）。
pub fn decode(bytes: &[u8]) -> Result<DecodedText, DecodeError> {
    if bytes.starts_with(&BOM_UTF8) {
        let rest = &bytes[BOM_UTF8.len()..];
        let content = std::str::from_utf8(rest).map_err(|_| {
            DecodeError::new(
                "text.encodingInvalid",
                "UTF-8 BOM present but the payload is not valid UTF-8",
            )
        })?;
        return Ok(DecodedText {
            content: content.to_string(),
            encoding: TextEncoding::Utf8Bom,
            bom: Bom::Utf8,
        });
    }
    if bytes.starts_with(&BOM_UTF16LE) || bytes.starts_with(&BOM_UTF16BE) {
        let le = bytes.starts_with(&BOM_UTF16LE);
        let enc = if le {
            encoding_rs::UTF_16LE
        } else {
            encoding_rs::UTF_16BE
        };
        let rest = &bytes[2..];
        // UTF-16 奇数长度：截断尾字节是文件截断事实 ⇒ 报错而非静默丢弃。
        if !rest.len().is_multiple_of(2) {
            return Err(DecodeError::new(
                "text.encodingInvalid",
                "UTF-16 payload has an odd trailing byte (truncated file?)",
            ));
        }
        let (content, had_errors) = enc.decode_without_bom_handling(rest);
        if had_errors {
            return Err(DecodeError::new(
                "text.encodingInvalid",
                "UTF-16 payload contains invalid surrogate sequences",
            ));
        }
        return Ok(DecodedText {
            content: content.into_owned(),
            encoding: if le {
                TextEncoding::Utf16Le
            } else {
                TextEncoding::Utf16Be
            },
            bom: if le { Bom::Utf16Le } else { Bom::Utf16Be },
        });
    }

    // 无 BOM：UTF-8 严格 → GB18030 严格 → Latin-1 兜底。
    if let Ok(content) = std::str::from_utf8(bytes) {
        let encoding = if bytes.iter().all(|&b| b < 0x80) && !bytes.is_empty() {
            TextEncoding::Ascii
        } else {
            TextEncoding::Utf8
        };
        return Ok(DecodedText {
            content: content.to_string(),
            encoding,
            bom: Bom::None,
        });
    }
    // GB18030 严格解码（had_errors ⇒ 不接受，继续降级）。
    let (content, had_errors) = encoding_rs::GB18030.decode_without_bom_handling(bytes);
    if !had_errors {
        return Ok(DecodedText {
            content: content.into_owned(),
            encoding: TextEncoding::Gb18030,
            bom: Bom::None,
        });
    }
    // Latin-1：字节 → U+00xx，无损、永不失败；兜底必须如实标注（charter #27）。
    Ok(DecodedText {
        content: bytes.iter().map(|&b| b as char).collect(),
        encoding: TextEncoding::Latin1,
        bom: Bom::None,
    })
}

/// 按显式指定编码解码（UI 手动覆盖 / Save 策略，§21）。
pub fn decode_as(encoding: TextEncoding, bytes: &[u8]) -> Result<DecodedText, DecodeError> {
    match encoding {
        TextEncoding::Latin1 => Ok(DecodedText {
            content: bytes.iter().map(|&b| b as char).collect(),
            encoding: TextEncoding::Latin1,
            bom: Bom::None,
        }),
        TextEncoding::Gb18030 | TextEncoding::Gbk => {
            let enc = if encoding == TextEncoding::Gbk {
                encoding_rs::GBK
            } else {
                encoding_rs::GB18030
            };
            let (content, had_errors) = enc.decode_without_bom_handling(bytes);
            if had_errors {
                return Err(DecodeError::new(
                    "text.encodingInvalid",
                    "payload is not valid for the selected encoding",
                ));
            }
            Ok(DecodedText {
                content: content.into_owned(),
                encoding,
                bom: Bom::None,
            })
        }
        TextEncoding::Utf8 | TextEncoding::Utf8Bom => std::str::from_utf8(bytes)
            .map(|c| DecodedText {
                content: c.to_string(),
                encoding: TextEncoding::Utf8,
                bom: Bom::None,
            })
            .map_err(|_| DecodeError::new("text.encodingInvalid", "payload is not valid UTF-8")),
        TextEncoding::Utf16Le | TextEncoding::Utf16Be => {
            if !bytes.len().is_multiple_of(2) {
                return Err(DecodeError::new(
                    "text.encodingInvalid",
                    "UTF-16 payload has an odd trailing byte (truncated file?)",
                ));
            }
            let enc = if encoding == TextEncoding::Utf16Le {
                encoding_rs::UTF_16LE
            } else {
                encoding_rs::UTF_16BE
            };
            let (content, had_errors) = enc.decode_without_bom_handling(bytes);
            if had_errors {
                return Err(DecodeError::new(
                    "text.encodingInvalid",
                    "UTF-16 payload contains invalid surrogate sequences",
                ));
            }
            Ok(DecodedText {
                content: content.into_owned(),
                encoding,
                bom: Bom::None,
            })
        }
        _ => Err(DecodeError::new(
            "text.encodingUnsupported",
            "explicit decode is not available for this encoding",
        )),
    }
}

/// 编码回写（Save/Apply 策略，§16 Preserve 默认，D39）。
/// BOM 按 `bom` 参数显式控制（Preserve/Remove/Add 由调用方决策后传入）。
pub fn encode(content: &str, encoding: TextEncoding, bom: Bom) -> Result<Vec<u8>, DecodeError> {
    let body = match encoding {
        TextEncoding::Utf16Le => {
            let mut out: Vec<u8> = content
                .encode_utf16()
                .flat_map(|u| u.to_le_bytes())
                .collect();
            if bom == Bom::Utf16Le {
                out.splice(0..0, BOM_UTF16LE);
            }
            return Ok(out);
        }
        TextEncoding::Utf16Be => {
            let mut out: Vec<u8> = content
                .encode_utf16()
                .flat_map(|u| u.to_be_bytes())
                .collect();
            if bom == Bom::Utf16Be {
                out.splice(0..0, BOM_UTF16BE);
            }
            return Ok(out);
        }
        TextEncoding::Latin1 => {
            let mut out: Vec<u8> = Vec::with_capacity(content.len());
            for ch in content.chars() {
                let c = u32::from(ch);
                if c > 0xFF {
                    return Err(DecodeError::new(
                        "text.encodingLossy",
                        "content cannot be represented in Latin-1",
                    ));
                }
                out.push(c as u8);
            }
            if bom != Bom::None {
                return Err(DecodeError::new(
                    "text.encodingUnsupported",
                    "BOM is not applicable to Latin-1",
                ));
            }
            return Ok(out);
        }
        _ => content.as_bytes().to_vec(),
    };
    let mut out = body;
    if bom == Bom::Utf8 {
        out.splice(0..0, BOM_UTF8);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use weave_core::prelude::TextEncoding;

    #[test]
    fn bom_paths_decode_strictly() {
        let mut bytes = vec![0xEF, 0xBB, 0xBF];
        bytes.extend_from_slice(b"hello");
        let d = decode(&bytes).expect("utf8 bom");
        assert_eq!(d.encoding, TextEncoding::Utf8Bom);
        assert_eq!(d.bom, Bom::Utf8);
        assert_eq!(d.content, "hello");

        let mut utf16le = vec![0xFF, 0xFE];
        utf16le.extend("a中".encode_utf16().flat_map(u16::to_le_bytes));
        let d = decode(&utf16le).expect("utf16le");
        assert_eq!(d.encoding, TextEncoding::Utf16Le);
        assert_eq!(d.content, "a中");

        // 截断的 UTF-16（奇数字节）⇒ 明确错误，不静默丢字节（§15）
        let truncated = [0xFF, 0xFE, 0x41];
        assert!(decode(&truncated).is_err());
    }

    #[test]
    fn no_bom_cascade_utf8_then_gb18030_then_latin1() {
        let d = decode("héllo".as_bytes()).expect("utf8");
        assert_eq!(d.encoding, TextEncoding::Utf8);

        // "中文" 的 GBK/GB18030 字节序列（D6 D0 CE C4）不是合法 UTF-8
        let d = decode(&[0xD6, 0xD0, 0xCE, 0xC4]).expect("gb");
        assert_eq!(d.encoding, TextEncoding::Gb18030);
        assert_eq!(d.content, "中文");

        // 无效 UTF-8 且无效 GB18030 ⇒ Latin-1 兜底（无损、如实标注）；
        // 夹具避开 FF/FE 开头（会被 BOM 检测优先命中）
        let d = decode(&[0xE9, 0x81, 0xFF]).expect("latin1");
        assert_eq!(d.encoding, TextEncoding::Latin1);
        assert_eq!(d.content.chars().count(), 3);
    }

    #[test]
    fn ascii_reported_precisely() {
        let d = decode(b"plain ascii").expect("ascii");
        assert_eq!(d.encoding, TextEncoding::Ascii);
    }

    #[test]
    fn explicit_decode_overrides_detection() {
        // 同一字节序列：按 GBK 解（成功）与按 UTF-8 解（失败）分歧
        assert!(decode_as(TextEncoding::Utf8, &[0xD6, 0xD0]).is_err());
        let d = decode_as(TextEncoding::Gbk, &[0xD6, 0xD0]).expect("gbk");
        assert_eq!(d.content, "中");
    }

    #[test]
    fn roundtrip_encode_preserves_bom_policy() {
        let content = "x中y";
        let bytes = encode(content, TextEncoding::Utf8Bom, Bom::Utf8).expect("enc");
        let d = decode(&bytes).expect("dec");
        assert_eq!(d.content, content);
        assert_eq!(d.encoding, TextEncoding::Utf8Bom);

        let bytes = encode(content, TextEncoding::Utf16Le, Bom::None).expect("enc");
        let d = decode_as(TextEncoding::Utf16Le, &bytes).expect("dec16");
        assert_eq!(d.content, content);

        // Latin-1 有损：非 Latin-1 字符必须显式报错（§15 不静默）
        assert!(encode("中", TextEncoding::Latin1, Bom::None).is_err());
        let bytes = encode("café", TextEncoding::Latin1, Bom::None).expect("latin1");
        assert_eq!(bytes, vec![b'c', b'a', b'f', 0xE9]);
    }
}
