//! 剪贴板历史领域模型（M2.3 §4-§5/§50/§74-§75/§80-§81）。
//!
//! - Entry 元数据与 Payload 解耦（§31）：PayloadReference 指向磁盘文件，
//!   索引不装原始内容。
//! - 状态不是 bool（§50）：Captured/Persisted/Available/Missing/Corrupted/
//!   Expired/Deleted 表达生命周期与完整性。
//! - 敏感性（§18/§19）：只说 PotentiallySensitive，绝不说 Safe；
//!   有限启发式，不宣称安全检测器。
//! - Payload 原样保存（§80）：trim/大小写折叠只作用于 Preview/SearchText。

use serde::{Deserialize, Serialize};
use std::time::{Duration, SystemTime};

use weave_core::prelude::WeaveError;

pub const STORAGE_SCHEMA_VERSION: u32 = 1;

/// 内容种类（§5：按实际平台能力支持；不伪造）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentKind {
    Text,
    Image,
    File,
    /// HTML（纯文本预览 + 原始引用；列表不渲染未知 HTML §12）。
    Html,
    /// 平台上有但 Weave 不支持的格式——如实记录存在，不崩溃（§24 预览）。
    Unknown,
}

/// 敏感性（§18：三态；PotentiallySensitive ≠ Sensitive，绝不输出 Safe）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Sensitivity {
    Normal,
    PotentiallySensitive,
    Unknown,
}

/// 条目状态（§50：不用 bool IsValid 覆盖一切）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EntryStatus {
    /// 已捕获（内存事实）。
    Captured,
    /// 已持久化（索引+payload 都在盘）。
    Persisted,
    /// 可恢复（payload 校验通过）。
    Available,
    /// payload 文件缺失。
    Missing,
    /// payload 存在但校验失败。
    Corrupted,
    /// 超过保留期（等待 GC）。
    Expired,
    Deleted,
}

/// 剪贴板条目元数据（§4：不引入 AI embedding/vector/云 id 等）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntry {
    /// 去重+引用主键（内容指纹十六进制，前缀 kind）。
    pub id: String,
    pub created_at: SystemTime,
    /// 同内容再次复制 ⇒ 更新（§74：A,B,A ⇒ A 移顶 + 更新时间）。
    pub updated_at: SystemTime,
    pub kind: ContentKind,
    /// 内容指纹（hex）。去重键 = (kind, hash)。
    pub content_hash: String,
    /// 列表展示文本（截断、规范化换行；原始 Payload 不受影响 §80）。
    pub preview: String,
    /// 搜索文本（小写折叠；仅搜索用 §81）。
    pub search_text: String,
    /// payload 文件相对名（payloads/<file>）；纯路径类条目（File）可为 None。
    pub payload_file: Option<String>,
    /// 缩略图文件（Image；列表只加载缩略图 §8/§9）。
    #[serde(default)]
    pub thumbnail_file: Option<String>,
    /// 原始字节大小。
    pub size: u64,
    /// 来源应用（§16：Confirmed/Unknown/Unavailable 三态，不做正确性前提）。
    pub source: SourceInfo,
    /// File 条目：路径列表（历史删除 ≠ 文件删除 §28/§65）。
    pub file_paths: Vec<String>,
    pub is_favorite: bool,
    pub sensitivity: Sensitivity,
    pub status: EntryStatus,
}

/// 来源信息（§16：辅助信息，绝不假设前台窗口 = 复制者）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceInfo {
    pub state: SourceState,
    pub process: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceState {
    Confirmed,
    Unknown,
    Unavailable,
}

impl ClipboardEntry {
    /// 去重键（§13）：(kind, content_hash)——不同 kind 不互相去重。
    pub fn dedup_key(kind: ContentKind, content_hash: &str) -> String {
        format!("{kind:?}:{}", content_hash.to_lowercase())
    }

    pub fn dedup_id(&self) -> String {
        Self::dedup_key(self.kind, &self.content_hash)
    }
}

/// 保留策略（§20：规则明确+可测试；Favorite 不自动过期）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RetentionPolicy {
    pub max_entries: usize,
    pub retention_days: Option<u64>,
    /// 存储字节上限（§35：绝不能无限增长）。
    pub max_storage_bytes: u64,
}

impl Default for RetentionPolicy {
    fn default() -> Self {
        Self {
            max_entries: 500,
            retention_days: Some(30),
            max_storage_bytes: 200 * 1024 * 1024,
        }
    }
}

/// 保留/GC 结果（供报告 §54，不泄露内容）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RetentionReport {
    pub expired_removed: u64,
    pub overflow_removed: u64,
    pub storage_removed: u64,
    pub favorites_kept: u64,
}

/// 对条目列表执行保留策略（有序：最新在前）。
/// 规则（§20）：Favorite 不自动过期但仍受 max_entries/max_storage 约束
/// （溢出时先淘汰最旧的非收藏，收藏最后）。
pub fn apply_retention(
    entries: &mut Vec<ClipboardEntry>,
    payload_bytes_of: &dyn Fn(&ClipboardEntry) -> u64,
    policy: &RetentionPolicy,
    now: SystemTime,
) -> RetentionReport {
    let mut report = RetentionReport::default();
    // 1) 过期（仅非收藏；RetentionPolicy.retention_days = Some 才启用）
    if let Some(days) = policy.retention_days {
        let horizon = Duration::from_secs(days.saturating_mul(24 * 3600));
        entries.retain(|e| {
            let expired = !e.is_favorite
                && now.duration_since(e.updated_at).unwrap_or(Duration::ZERO) > horizon;
            if expired {
                report.expired_removed += 1;
            }
            !expired
        });
    }
    // 2) 条数上限：从尾部（最旧）淘汰；收藏延后——仅当无非收藏可淘汰时
    //    才从尾部淘汰收藏（§20：规则明确 + 可测试）
    if entries.len() > policy.max_entries {
        let mut overflow = entries.len() - policy.max_entries;
        // 先淘汰尾部（最旧）的非收藏
        let mut i = entries.len();
        while overflow > 0 && i > 0 {
            i -= 1;
            if !entries[i].is_favorite {
                entries.remove(i);
                overflow -= 1;
                report.overflow_removed += 1;
            }
        }
        // 仍溢出 ⇒ 剩余全是收藏：从尾部（最旧收藏）淘汰
        while overflow > 0 && !entries.is_empty() {
            entries.pop();
            overflow -= 1;
            report.overflow_removed += 1;
        }
    }
    // 3) 存储上限：从最旧开始淘汰直到 ≤ 上限
    let total: u64 = entries.iter().map(payload_bytes_of).sum();
    if total > policy.max_storage_bytes {
        let mut over = total - policy.max_storage_bytes;
        let mut kept: Vec<ClipboardEntry> = Vec::new();
        for e in entries.drain(..) {
            let size = payload_bytes_of(&e);
            if over > 0 && !e.is_favorite {
                over -= over.min(size);
                report.storage_removed += 1;
            } else {
                if over > 0 && e.is_favorite {
                    over -= over.min(size);
                    report.storage_removed += 1;
                }
                kept.push(e);
            }
        }
        *entries = kept;
    }
    report
}

/// 有限敏感启发式（§18/§19）：只标记 PotentiallySensitive；
/// 误报可接受，漏报如实（不宣称检测能力）。
pub fn sensitivity_heuristic(kind: ContentKind, text: &str) -> Sensitivity {
    if kind != ContentKind::Text && kind != ContentKind::Html {
        return Sensitivity::Unknown;
    }
    let lower = text.to_lowercase();
    // JWT 形态（eyJ 开头两段 base64url + 点）
    if text
        .split_whitespace()
        .any(|tok| tok.starts_with("eyJ") && tok.matches('.').count() >= 2)
    {
        return Sensitivity::PotentiallySensitive;
    }
    // password/token/secret/api_key 类赋值或键值形态
    for key in ["password", "passwd", "secret", "api_key", "apikey", "token"] {
        if lower.contains(&format!("{key}:")) || lower.contains(&format!("{key}=")) {
            return Sensitivity::PotentiallySensitive;
        }
    }
    // Bearer 头形态
    if lower.contains("bearer ") {
        return Sensitivity::PotentiallySensitive;
    }
    // 40+ 连续 hex（疑似密钥/哈希原文）
    let hex_run = text
        .split(|c: char| !c.is_ascii_hexdigit())
        .any(|seg| seg.len() >= 40);
    if hex_run {
        return Sensitivity::PotentiallySensitive;
    }
    Sensitivity::Normal
}

/// 截断预览（§6/§80）：换行规范化仅影响展示；原始 payload 原样。
pub fn make_preview(text: &str, max_chars: usize) -> String {
    let normalized = text.replace("\r\n", "\n");
    let first_line = normalized.lines().next().unwrap_or("");
    let mut preview: String = first_line.chars().take(max_chars).collect();
    let truncated = first_line.chars().count() > max_chars || normalized.lines().count() > 1;
    if truncated {
        preview.push('…');
    }
    preview
}

/// 搜索文本（§81）：小写折叠仅用于搜索索引。
pub fn search_text_of(text: &str) -> String {
    text.to_lowercase()
}

/// 多词大小写不敏感搜索（§7）：全部词命中才算命中；按相关性 =
/// 命中词全部出现在 preview 的排前，其余按 updated_at 降序。
pub fn search_entries<'a>(entries: &'a [ClipboardEntry], query: &str) -> Vec<&'a ClipboardEntry> {
    let words: Vec<String> = query
        .split_whitespace()
        .map(|w| w.to_lowercase())
        .filter(|w| !w.is_empty())
        .collect();
    if words.is_empty() {
        return entries.iter().collect();
    }
    let mut scored: Vec<(bool, SystemTime, &ClipboardEntry)> = entries
        .iter()
        .filter(|e| {
            let hay = &e.search_text;
            words.iter().all(|w| hay.contains(w.as_str()))
        })
        .map(|e| {
            let in_preview = words
                .iter()
                .all(|w| e.preview.to_lowercase().contains(w.as_str()));
            (in_preview, e.updated_at, e)
        })
        .collect();
    scored.sort_by(|a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    scored.into_iter().map(|(_, _, e)| e).collect()
}

/// 存储错误（不携带内容——§53 日志/审计只进 id/kind/size）。
pub fn storage_error(code: &str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("weave-clipboard")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(kind: ContentKind, hash: &str, age_secs: u64, favorite: bool) -> ClipboardEntry {
        let now = SystemTime::now();
        ClipboardEntry {
            id: ClipboardEntry::dedup_key(kind, hash),
            created_at: now - Duration::from_secs(age_secs),
            updated_at: now - Duration::from_secs(age_secs),
            kind,
            content_hash: hash.to_string(),
            preview: format!("preview {hash}"),
            search_text: format!("preview {hash}"),
            payload_file: Some(format!("{hash}.bin")),
            thumbnail_file: None,
            size: 10,
            source: SourceInfo {
                state: SourceState::Unknown,
                process: None,
            },
            file_paths: vec![],
            is_favorite: favorite,
            sensitivity: Sensitivity::Normal,
            status: EntryStatus::Persisted,
        }
    }

    #[test]
    fn dedup_key_is_kind_scoped() {
        let text_key = ClipboardEntry::dedup_key(ContentKind::Text, "AB");
        let image_key = ClipboardEntry::dedup_key(ContentKind::Image, "AB");
        assert_ne!(text_key, image_key, "不同 kind 不互相去重（§13）");
    }

    #[test]
    fn retention_expires_old_but_keeps_favorites() {
        let now = SystemTime::now();
        let mut entries = vec![
            entry(ContentKind::Text, "old", 40 * 24 * 3600, false),
            entry(ContentKind::Text, "old-fav", 40 * 24 * 3600, true),
            entry(ContentKind::Text, "fresh", 60, false),
        ];
        let policy = RetentionPolicy {
            retention_days: Some(30),
            ..RetentionPolicy::default()
        };
        let report = apply_retention(&mut entries, &|_| 10, &policy, now);
        assert_eq!(report.expired_removed, 1);
        assert_eq!(entries.len(), 2, "favorite 不自动过期（§20）");
        assert!(entries.iter().all(|e| e.content_hash != "old"));
    }

    #[test]
    fn retention_overflow_evicts_oldest_non_favorite_first() {
        let now = SystemTime::now();
        // 3 条，上限 2：最旧的是收藏 ⇒ 淘汰次旧的普通条目，收藏保留
        // 最新在前（index 0 = newest）
        let mut entries = vec![
            entry(ContentKind::Text, "newest", 10, false),
            entry(ContentKind::Text, "middle", 20, false),
            entry(ContentKind::Text, "oldest-fav", 30, true),
        ];
        let policy = RetentionPolicy {
            max_entries: 2,
            retention_days: None,
            max_storage_bytes: u64::MAX,
        };
        let report = apply_retention(&mut entries, &|_| 10, &policy, now);
        assert_eq!(report.overflow_removed, 1);
        assert_eq!(entries.len(), 2);
        assert!(
            entries.iter().any(|e| e.content_hash == "oldest-fav"),
            "收藏最后淘汰"
        );
        assert!(entries.iter().any(|e| e.content_hash == "newest"));
    }

    #[test]
    fn storage_limit_evicts_until_under_cap() {
        let now = SystemTime::now();
        let mut entries = vec![
            entry(ContentKind::Text, "a", 10, false),
            entry(ContentKind::Text, "b", 20, false),
            entry(ContentKind::Text, "c", 30, false),
        ];
        entries.reverse();
        let policy = RetentionPolicy {
            max_entries: usize::MAX,
            retention_days: None,
            max_storage_bytes: 25,
        };
        let report = apply_retention(&mut entries, &|_| 10, &policy, now);
        assert_eq!(report.storage_removed, 1, "30 > 25 ⇒ 淘汰最旧一条");
        let total: u64 = entries.iter().map(|e| e.size).sum();
        assert!(total <= 25);
    }

    #[test]
    fn sensitivity_detects_limited_patterns_and_never_claims_safe() {
        assert_eq!(
            sensitivity_heuristic(
                ContentKind::Text,
                "auth = eyJhbGciOiJIUzI1NiJ9.eyJzdWIiOiIxIn0.abc"
            ),
            Sensitivity::PotentiallySensitive
        );
        assert_eq!(
            sensitivity_heuristic(ContentKind::Text, "password: hunter2"),
            Sensitivity::PotentiallySensitive
        );
        assert_eq!(
            sensitivity_heuristic(ContentKind::Text, "Authorization: Bearer abc.def"),
            Sensitivity::PotentiallySensitive
        );
        let hex = format!("key = {}", "a".repeat(48));
        assert_eq!(
            sensitivity_heuristic(ContentKind::Text, &hex),
            Sensitivity::PotentiallySensitive
        );
        // 误报边界：普通文本/短 hex 不敏感
        assert_eq!(
            sensitivity_heuristic(ContentKind::Text, "hello world 0xdeadbeef"),
            Sensitivity::Normal
        );
        // 图片/文件不做文本启发式 ⇒ Unknown
        assert_eq!(
            sensitivity_heuristic(ContentKind::Image, "anything"),
            Sensitivity::Unknown
        );
    }

    #[test]
    fn preview_normalizes_display_without_touching_payload() {
        let raw = "first\r\nsecond\r\n";
        let preview = make_preview(raw, 100);
        assert!(preview.starts_with("first"), "{preview}");
        assert!(!preview.contains('\r'), "展示规范化 CRLF");
        // 原始 payload 调用方持有——本函数不返回也不修改它
        assert_eq!(raw, "first\r\nsecond\r\n");
        let long = "x".repeat(300);
        assert_eq!(
            make_preview(&long, 100).chars().count(),
            101,
            "截断 + 省略号"
        );
    }

    #[test]
    fn search_is_case_insensitive_multiword_and_preview_boosted() {
        let mut a = entry(ContentKind::Text, "a", 10, false);
        a.preview = "Network config".to_string();
        a.search_text = "network config payload-a".to_string();
        let mut b = entry(ContentKind::Text, "b", 5, false);
        b.preview = "zzz".to_string();
        b.search_text = "network payload-b".to_string();
        let entries = vec![a, b];
        let hits = search_entries(&entries, "NETWORK");
        assert_eq!(hits.len(), 2);
        // 双词
        assert_eq!(search_entries(&entries, "network config").len(), 1);
        // 相关性：preview 命中者排前（即使更旧）
        assert_eq!(search_entries(&entries, "network")[0].content_hash, "a");
        assert_eq!(search_entries(&entries, "nomatch").len(), 0);
    }

    #[test]
    fn empty_query_returns_all_in_order() {
        let entries = vec![entry(ContentKind::Text, "a", 10, false)];
        assert_eq!(search_entries(&entries, "   ").len(), 1);
    }
}
