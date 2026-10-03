//! 轻量文本编码识别（M1 §9）。
//!
//! 范围刻意受限：BOM 优先 → UTF-8 有效性（含 ASCII 精确标注）→ Unknown。
//! GB18030 / GBK / Latin-1 属 charter #27，**推迟到 M4**（已记录 Known
//! Limitations 与 DECISIONS.md）；对非 UTF 系文本如实返回 Unknown，不猜测。

use weave_core::prelude::TextEncoding;

/// UTF-8 BOM
const BOM_UTF8: [u8; 3] = [0xEF, 0xBB, 0xBF];
const BOM_UTF16LE: [u8; 2] = [0xFF, 0xFE];
const BOM_UTF16BE: [u8; 2] = [0xFE, 0xFF];

/// 对样本字节做编码判断。样本应来自文件开头（≤ sniff 上限）；
/// 样本为空时返回 Unknown（空文件没有可判断的编码事实）。
pub fn detect_encoding(sample: &[u8]) -> TextEncoding {
    if sample.starts_with(&BOM_UTF8) {
        return TextEncoding::Utf8Bom;
    }
    if sample.starts_with(&BOM_UTF16LE) {
        return TextEncoding::Utf16Le;
    }
    if sample.starts_with(&BOM_UTF16BE) {
        return TextEncoding::Utf16Be;
    }
    if sample.is_empty() {
        return TextEncoding::Unknown;
    }
    if sample.iter().all(|&b| b < 0x80) {
        return TextEncoding::Ascii;
    }
    // 样本可能在多字节序列中间被截断：先裁掉尾部不完整序列再验证，
    // 避免因采样边界误报。
    let trimmed = trim_truncated_utf8_tail(sample);
    if std::str::from_utf8(trimmed).is_ok() {
        TextEncoding::Utf8
    } else {
        TextEncoding::Unknown
    }
}

/// 裁掉样本尾部可能被截断的 UTF-8 序列（最多回退 3 字节）。
fn trim_truncated_utf8_tail(sample: &[u8]) -> &[u8] {
    let len = sample.len();
    let mut cut = 0usize;
    for back in 1..=3.min(len) {
        let b = sample[len - back];
        if b < 0x80 {
            break;
        }
        if b & 0xC0 == 0xC0 {
            // 找到序列首字节：判断该序列声明长度是否超出样本尾部。
            let need = if b >= 0xF0 {
                4
            } else if b >= 0xE0 {
                3
            } else {
                2
            };
            if need > back {
                cut = back;
            }
            break;
        }
    }
    &sample[..len - cut]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_boms() {
        assert_eq!(
            detect_encoding(&[0xEF, 0xBB, 0xBF, b'a']),
            TextEncoding::Utf8Bom
        );
        assert_eq!(detect_encoding(&[0xFF, 0xFE, 0x00]), TextEncoding::Utf16Le);
        assert_eq!(detect_encoding(&[0xFE, 0xFF, 0x00]), TextEncoding::Utf16Be);
    }

    #[test]
    fn distinguishes_ascii_from_utf8() {
        assert_eq!(detect_encoding(b"plain ascii"), TextEncoding::Ascii);
        assert_eq!(detect_encoding("café".as_bytes()), TextEncoding::Utf8);
        assert_eq!(detect_encoding("你好".as_bytes()), TextEncoding::Utf8);
    }

    #[test]
    fn non_utf_bytes_report_unknown_not_guesses() {
        // GB18030 编码的"中文"二字：M1 范围外，必须 Unknown（不猜测）。
        let gb18030_chinese: [u8; 4] = [0xD6, 0xD0, 0xCE, 0xC4];
        assert_eq!(detect_encoding(&gb18030_chinese), TextEncoding::Unknown);
        // Latin-1 高位字节同理。
        assert_eq!(detect_encoding(&[0xE9, 0xE8]), TextEncoding::Unknown);
    }

    #[test]
    fn truncated_multibyte_tail_does_not_break_utf8_verdict() {
        // "你" = E4 BD A0；样本截断到 2 字节时不应误判 Unknown 之外的任何猜测。
        let full = "你".as_bytes().to_vec();
        assert_eq!(detect_encoding(&full), TextEncoding::Utf8);
        let truncated = &full[..2];
        // 截断样本裁尾后为空序列 → from_utf8("") 成立 → 仍是 Utf8（采样边界不误报）。
        assert_eq!(detect_encoding(truncated), TextEncoding::Utf8);
    }

    #[test]
    fn empty_sample_is_unknown() {
        assert_eq!(detect_encoding(&[]), TextEncoding::Unknown);
    }
}
