//! M2.11 规范化与指纹（§6/§10/§55）：**Hash 基于真正比较对象**——
//! 规范化后的提取文本，不是原始 HTTP body（§55 反例：广告变了 ⇒ "正文
//! 变化"是错的）。
//!
//! - NormalizationProfile（§6）如实收窄为项目真实能力：TextOnly 提取
//!   （web::html 管线）+ 行尾统一 + 行尾空白剔除 + 空行折叠；不默认激进
//!   清洗（§6"用户必须知道比较的是什么"——UI 显示规范化说明）。
//! - Fingerprint = SHA-256(normalized_text)（weave_files 统一 hash，无
//!   第二套 §70）。

/// 规范化（§6 TrimWhitespace/NormalizeLineEndings/CollapseWhitespace）。
pub fn normalize_text(text: &str, collapse_blank_lines: bool) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_pending = false;
    let mut wrote_any = false;
    for raw_line in text.lines() {
        let line = raw_line.trim_end();
        if line.trim().is_empty() {
            if !collapse_blank_lines && wrote_any {
                // 保留语义：连续空行折叠为单个空行
                blank_pending = true;
            }
            continue;
        }
        if wrote_any {
            out.push('\n');
            if blank_pending {
                out.push('\n');
                blank_pending = false;
            }
        }
        wrote_any = true;
        out.push_str(line);
    }
    out
}

/// 指纹（§10）：规范化文本的 SHA-256 hex（统一 hash 设施 §70）。
pub fn fingerprint(normalized: &str) -> Option<String> {
    weave_files::hash::hash_bytes(
        weave_core::prelude::HashAlgorithm::Sha256,
        normalized.as_bytes(),
    )
    .ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_line_endings_and_trailing_whitespace() {
        let a = "Hello\r\nWorld   \n";
        let b = "Hello\nWorld\n";
        let (na, nb) = (normalize_text(a, true), normalize_text(b, true));
        assert_eq!(na, nb, "行尾/空白差异必须归零（假变化防线 §58）");
        assert_eq!(fingerprint(&na), fingerprint(&nb));
    }

    #[test]
    fn blank_line_collapse_is_optional() {
        let t = "a\n\n\nb\n";
        assert_eq!(normalize_text(t, true), "a\nb");
        assert_eq!(normalize_text(t, false), "a\n\nb");
    }

    #[test]
    fn real_content_change_changes_fingerprint() {
        let a = normalize_text("price: 10\nstock: yes", true);
        let b = normalize_text("price: 12\nstock: yes", true);
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }

    #[test]
    fn removal_is_a_change_not_just_addition() {
        // §3 场景 4：内容被删除必须被识别
        let a = normalize_text("para one\npara two\npara three", true);
        let b = normalize_text("para one", true);
        assert_ne!(fingerprint(&a), fingerprint(&b));
    }
}
