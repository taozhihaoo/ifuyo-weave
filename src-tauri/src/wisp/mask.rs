//! M2.9 Wisp 脱敏（§40/§41）：Full / Partial / Structured 三形态。
//!
//! - `mask_preview` 用于 findings/UI 展示（告知风险，不重新复制风险 §18）。
//! - `mask_text` 按偏移替换生成真实 masked 内容（保护会话用——原文由
//!   session 保留，可恢复 §11/§41）。
//! - 不宣称 Mask 对所有格式都可靠（§40）——替换是保守的字面处理。

use crate::wisp::model::SensitivityFinding;

/// 结构化预览：保留前缀（首个非字母数字段，如 `sk-`/`ghp_`）与尾部 4 位，
/// 中间以 •••• 代替；短值整体打码。
pub fn mask_preview(value: &str) -> String {
    let chars: Vec<char> = value.chars().collect();
    if chars.len() <= 8 {
        return "•".repeat(chars.len().max(3));
    }
    // 前缀：sk- / ghp_ 等样式——首 4 字符内以 '-'/'_' 结尾的段（如 sk-、ghp_）
    let mut prefix_len = 0usize;
    for (i, c) in chars.iter().take(5).enumerate() {
        if *c == '-' || *c == '_' {
            prefix_len = i + 1;
            break;
        }
    }
    let tail = 4usize;
    let body = chars.len() - prefix_len;
    if body <= tail * 2 {
        // 中段太短：只保留前缀 + 全打码
        let mut out: String = chars[..prefix_len].iter().collect();
        out.push_str(&"•".repeat(body));
        return out;
    }
    let mut out: String = chars[..prefix_len].iter().collect();
    out.push_str(&"•".repeat((body - tail).min(12)));
    out.push_str(&chars[chars.len() - tail..].iter().collect::<String>());
    out
}

/// 邮箱形态脱敏：`abcdef@gmail.com` → `a•••••f@gmail.com`（本地名首尾保留）。
pub fn mask_email(value: &str) -> Option<String> {
    let (local, domain) = value.split_once('@')?;
    let chars: Vec<char> = local.chars().collect();
    if chars.len() <= 2 {
        return Some(format!("{local}••••@{domain}"));
    }
    let mut out: String = chars[..1].iter().collect();
    out.push_str(&"•".repeat((chars.len() - 2).min(8)));
    out.push(chars[chars.len() - 1]);
    Some(format!("{out}@{domain}"))
}

/// 按发现偏移替换生成 masked 文本（重叠区间跳过后者——保守不撕内容）。
/// 输入偏移为 UTF-8 字节（rules 产出的 start/end）。
pub fn mask_text(text: &str, findings: &[SensitivityFinding]) -> String {
    let mut out = String::with_capacity(text.len());
    let mut last_end = 0usize;
    let mut ordered: Vec<&SensitivityFinding> = findings.iter().collect();
    ordered.sort_by_key(|f| (f.start_offset, f.end_offset));
    for f in ordered {
        let (s, e) = (f.start_offset as usize, f.end_offset as usize);
        if s < last_end || e > text.len() || s >= e {
            continue; // 重叠/越界/非法——跳过
        }
        out.push_str(&text[last_end..s]);
        out.push_str(&mask_preview(&text[s..e]));
        last_end = e;
    }
    out.push_str(&text[last_end..]);
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wisp::model::{Confidence, SensitivityCategory, Severity};

    fn finding(s: u32, e: u32) -> SensitivityFinding {
        SensitivityFinding {
            rule_id: "t".into(),
            category: SensitivityCategory::Credential,
            confidence: Confidence::High,
            severity: Severity::High,
            evidence: "test".into(),
            masked_preview: String::new(),
            start_offset: s,
            end_offset: e,
        }
    }

    #[test]
    fn preview_keeps_prefix_and_tail_only() {
        let m = mask_preview("ghp_AbcdefGhijKlmnopPqrstUv1234");
        assert!(m.starts_with("ghp_"));
        assert!(m.ends_with("1234"));
        assert!(m.contains("••••"));
        assert!(!m.contains("Abcdef"));
        // 短值整体打码
        assert_eq!(mask_preview("short"), "•••••");
    }

    #[test]
    fn email_masks_local_part() {
        // 本地名 a••••f（等长替换；§40 示例为示意）
        assert_eq!(
            mask_email("abcdef@gmail.com"),
            Some("a••••f@gmail.com".into())
        );
        assert_eq!(mask_email("ab@x.com"), Some("ab••••@x.com".into()));
        assert_eq!(mask_email("no-at-sign"), None);
    }

    #[test]
    fn mask_text_replaces_ranges_and_keeps_rest() {
        let text = "token: ghp_AbcdefGhijKlmnopPqrstUv1234 end";
        let s = text.find("ghp_").unwrap() as u32;
        let e = (s as usize + "ghp_AbcdefGhijKlmnopPqrstUv1234".len()) as u32;
        let masked = mask_text(text, &[finding(s, e)]);
        assert!(masked.starts_with("token: ghp_"));
        assert!(masked.ends_with(" end"));
        assert!(!masked.contains("Abcdef"));
    }

    #[test]
    fn overlapping_and_out_of_range_findings_are_skipped() {
        let text = "abcdefgh";
        // 0..4 有效；重叠 2..6 跳过；越界 6..99 跳过
        let masked = mask_text(text, &[finding(0, 4), finding(2, 6), finding(6, 99)]);
        assert_eq!(masked.chars().count(), "abcdefgh".chars().count());
        assert!(masked.starts_with(&"•".repeat(4)));
    }
}
