//! 比较选项与忽略规则（M2.1 §8/§12）。
//!
//! 分层策略是契约（§8）：不默认对所有文件做昂贵 Hash/内容 Diff。
//! 忽略规则 = Default（极小集）+ User + Session（§12：不擅自硬编码
//! 大量开发目录；结果中如实报告 Ignored 计数）。

use serde::{Deserialize, Serialize};

/// 内容核对档位（M2.1 §8 的 Level 1/2；Level 3 Full Diff 仅文件详情按需）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentCheck {
    /// L1：existence + size + mtime（size/mtime 一致 ⇒ Same，不读内容）。
    Metadata,
    /// L2：size 不同 ⇒ Modified；size 一致 ⇒ SHA-256 流式核对（二进制
    /// 安全；M2.1 §8"二进制 Size→Hash 即可判定"）。
    Hash,
}

/// 忽略规则（按路径段名匹配；支持精确名与 `*` 通配后缀）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IgnoreRules {
    /// 默认极小集（§12：不硬编码大量项目目录；只放确定无害的两个）。
    pub defaults: Vec<String>,
    /// 用户规则（配置/会话追加；名字或 `*.ext` 模式）。
    pub user: Vec<String>,
}

impl Default for IgnoreRules {
    fn default() -> Self {
        Self {
            defaults: [".git".to_string(), "node_modules".to_string()].into(),
            user: Vec::new(),
        }
    }
}

impl IgnoreRules {
    /// 生效规则（default+user 合并，供会话快照与 UI 展示）。
    pub fn effective(&self) -> Vec<String> {
        let mut all = self.defaults.clone();
        all.extend(self.user.iter().cloned());
        all.sort();
        all.dedup();
        all
    }

    /// 相对路径是否命中忽略（任一路径段命中即忽略）。
    /// 段匹配规则：与规则完全相等，或规则形如 `*.ext` 时按后缀匹配。
    pub fn matches(&self, relative_path: &str) -> bool {
        let rules = self.effective();
        if rules.is_empty() {
            return false;
        }
        relative_path.split('/').any(|segment| {
            rules.iter().any(|rule| {
                if let Some(suffix) = rule.strip_prefix('*') {
                    segment.ends_with(suffix)
                } else {
                    segment == rule
                }
            })
        })
    }
}

/// 比较选项。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CompareOptions {
    pub content_check: ContentCheck,
    pub ignore: IgnoreRules,
    /// 文本详情比较的尺寸上限（超过 ⇒ 诚实 "too large for full preview"）。
    pub max_text_detail_bytes: u64,
    /// 单文件 Hash 的尺寸上限（None = 流式不设限；会话级保护用）。
    pub max_hash_bytes: Option<u64>,
}

impl Default for CompareOptions {
    fn default() -> Self {
        Self {
            content_check: ContentCheck::Metadata,
            ignore: IgnoreRules::default(),
            max_text_detail_bytes: 2 * 1024 * 1024,
            max_hash_bytes: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_rules_match_segments_and_suffix_patterns() {
        let rules = IgnoreRules {
            defaults: [".git".to_string(), "node_modules".to_string()].into(),
            user: ["*.log".to_string(), "dist".to_string()].into(),
        };
        assert!(rules.matches(".git/HEAD"));
        assert!(rules.matches("a/b/node_modules/pkg/index.js"));
        assert!(rules.matches("logs/app.log"));
        assert!(rules.matches("dist/main.js"));
        assert!(
            !rules.matches("src/lib/git_helper.rs"),
            "段名精确匹配，不做子串"
        );
        assert!(!rules.matches("dist.ts"), "通配按后缀匹配段名");
    }

    #[test]
    fn effective_rules_are_sorted_and_deduped() {
        let rules = IgnoreRules {
            defaults: ["node_modules".to_string(), ".git".to_string()].into(),
            user: [".git".to_string()].into(),
        };
        assert_eq!(rules.effective(), vec![".git", "node_modules"]);
    }

    #[test]
    fn options_default_is_metadata_tier_with_bounds() {
        let o = CompareOptions::default();
        assert_eq!(o.content_check, ContentCheck::Metadata);
        assert_eq!(o.max_text_detail_bytes, 2 * 1024 * 1024);
        assert_eq!(o.max_hash_bytes, None);
    }
}
