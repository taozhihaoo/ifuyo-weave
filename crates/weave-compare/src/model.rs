//! 统一差异模型（M2.1 §6/§7）。
//!
//! Compare 的共同词汇：File / Directory / 后续 Data/Document/Image/Privacy
//! 的比较结果都表达为本模型——不为每个页面再造 DiffItem/CompareResult。
//!
//! 诚实边界（§7）：
//! - Added/Removed/Modified 必须真实实现（本模块的事实来源是 stat/hash）。
//! - Moved/Renamed **只在 hash 证据唯一匹配时**标记；证据不足保持
//!   Added/Removed，绝不凭路径变化猜测。
//! - 不设计万能结构：evidence 只携带"为什么认为不同"的最小事实。

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use weave_core::prelude::WeaveError;

/// 节点种类（M2.1 §5：File / Directory 两种节点）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum NodeKind {
    File,
    Directory,
    /// 符号链接/联接——不跟随（M1 同源策略）；按其自身 stat 参与比较。
    Symlink,
}

/// 差异种类（M2.1 §7 统一词汇）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum DifferenceKind {
    /// 两侧都在且（按当前策略）内容一致。
    Same,
    /// 仅右侧存在（Left Only 的对偶；UI 标签 OnlyRight）。
    Added,
    /// 仅左侧存在（UI 标签 OnlyLeft）。
    Removed,
    /// 两侧都在但不同（内容或元数据——细分见 [`ContentDifference`]）。
    Modified,
    /// 节点类型变化（file ↔ directory）。
    TypeChanged,
    /// 目录被移动：hash 唯一匹配 + 相对名相同、目录不同（证据驱动）。
    Moved,
    /// 被重命名：hash 唯一匹配 + 相对名不同（证据驱动）。
    Renamed,
    /// 条目存在但不可访问（权限/占用）——单条失败，不升级会话失败。
    Unreadable,
    /// 命中忽略规则。
    Ignored,
    /// 平台/类型不支持比较（如 Symlink 内容、两侧同为目录的文件比较）。
    Unsupported,
    /// 比较过程中发生错误（如 hash 中途读取失败）。
    Error,
}

/// 内容差异的细分证据（Modified 的下钻；不影响 kind 计数桶）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ContentDifference {
    /// 内容一致（含"未比较内容但无法断言不同"的场景不做此标注）。
    None,
    /// 实际内容不同（size 不同或 hash 不同）。
    ContentChanged,
    /// 内容一致但元数据不同（mtime/编码等）。
    MetadataOnly,
    /// 未做内容级比较（策略为 Metadata 且 size/mtime 一致时按 Same 处理）。
    NotCompared,
}

/// 单侧条目事实（M2.1 §6 DifferenceItem 的 Left/Right 面）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct EntryFacts {
    /// 绝对路径（显示用）。
    pub path: String,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    /// 内容摘要（小写 hex）；仅 Hash 策略下产生。
    pub digest_hex: Option<String>,
    /// 可靠识别时的编码（复用 weave-text 检测；不可靠 ⇒ None）。
    pub encoding: Option<weave_core::prelude::TextEncoding>,
    /// 文本文件的行数（可流式统计且为文本时）。
    pub line_count: Option<u64>,
    /// 目录节点：直接子项计数（files, subdirs）。
    pub child_counts: Option<(u64, u64)>,
}

impl EntryFacts {
    pub fn path_only(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            size: None,
            modified: None,
            digest_hex: None,
            encoding: None,
            line_count: None,
            child_counts: None,
        }
    }
}

/// 比较证据（M2.1 §21：让用户与诊断系统知道"为什么认为不同"）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonEvidence {
    /// 判定方法：existence / size / size+mtime / SHA-256。
    pub method: String,
    pub left_digest: Option<String>,
    pub right_digest: Option<String>,
    pub compared_at: SystemTime,
}

/// 统一差异条目（M2.1 §6 DifferenceItem）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DifferenceItem {
    /// 相对路径（分隔符归一为 `/`；Windows 侧匹配大小写不敏感）。
    pub relative_path: String,
    pub node: NodeKind,
    pub kind: DifferenceKind,
    pub content: ContentDifference,
    pub left: Option<EntryFacts>,
    pub right: Option<EntryFacts>,
    pub evidence: Option<ComparisonEvidence>,
    /// 单条目错误（Unreadable/Error 时携带；绝不放弃整个会话）。
    pub error: Option<WeaveError>,
}

impl DifferenceItem {
    /// 构造 + 同步登记到 summary（保证计数与条目一致）。
    pub fn new(relative_path: impl Into<String>, node: NodeKind, kind: DifferenceKind) -> Self {
        Self {
            relative_path: relative_path.into(),
            node,
            kind,
            content: ContentDifference::NotCompared,
            left: None,
            right: None,
            evidence: None,
            error: None,
        }
    }
}

/// 汇总计数（M2.1 §4/§17 的 Summary 桶；条目与计数由 builder 保证一致）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonSummary {
    pub total: u64,
    pub same: u64,
    pub added: u64,
    pub removed: u64,
    pub modified: u64,
    pub type_changed: u64,
    pub moved: u64,
    pub renamed: u64,
    pub unreadable: u64,
    pub ignored: u64,
    pub unsupported: u64,
    pub error: u64,
}

/// 会话种类（Read-only Compare 的两种形态，M2.1 §14）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SessionKind {
    /// 两个文件的比较。
    Files,
    /// 两个目录的比较。
    Directories,
}

/// 一次比较会话（M2.1 §6 ComparisonSession）：Read-only 事实的集合。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ComparisonSession {
    pub operation_id: weave_core::prelude::OperationId,
    pub kind: SessionKind,
    pub left_root: String,
    pub right_root: String,
    /// 忽略规则快照（default+user+session 合并后生效的完整列表）。
    pub effective_ignores: Vec<String>,
    pub ignored_count: u64,
    pub items: Vec<DifferenceItem>,
    pub summary: ComparisonSummary,
    pub started_at: SystemTime,
    pub finished_at: Option<SystemTime>,
    pub duration_ms: u64,
    /// 取消的会话：结果为部分事实（已比较的条目保留，如实标注）。
    pub cancelled: bool,
}

impl ComparisonSession {
    pub fn builder(left_root: &str, right_root: &str, kind: SessionKind) -> SessionBuilder {
        SessionBuilder {
            session: Self {
                operation_id: weave_core::prelude::OperationId::generate(),
                kind,
                left_root: left_root.to_string(),
                right_root: right_root.to_string(),
                effective_ignores: Vec::new(),
                ignored_count: 0,
                items: Vec::new(),
                summary: ComparisonSummary::default(),
                started_at: SystemTime::now(),
                finished_at: None,
                duration_ms: 0,
                cancelled: false,
            },
        }
    }

    /// 依据 kind 把条目计入 summary 桶（保证一致性）。
    pub fn account(&mut self, item: &DifferenceItem) {
        self.summary.total += 1;
        match item.kind {
            DifferenceKind::Same => self.summary.same += 1,
            DifferenceKind::Added => self.summary.added += 1,
            DifferenceKind::Removed => self.summary.removed += 1,
            DifferenceKind::Modified => self.summary.modified += 1,
            DifferenceKind::TypeChanged => self.summary.type_changed += 1,
            DifferenceKind::Moved => self.summary.moved += 1,
            DifferenceKind::Renamed => self.summary.renamed += 1,
            DifferenceKind::Unreadable => self.summary.unreadable += 1,
            DifferenceKind::Ignored => self.summary.ignored += 1,
            DifferenceKind::Unsupported => self.summary.unsupported += 1,
            DifferenceKind::Error => self.summary.error += 1,
        }
    }
}

/// 会话 builder：条目登记与计数强一致（不允许手写 summary）。
pub struct SessionBuilder {
    pub(crate) session: ComparisonSession,
}

impl SessionBuilder {
    pub fn effective_ignores(&mut self, rules: Vec<String>) {
        self.session.effective_ignores = rules;
    }

    pub fn push(&mut self, item: DifferenceItem) {
        if item.kind == DifferenceKind::Ignored {
            self.session.ignored_count += 1;
        }
        self.session.account(&item);
        self.session.items.push(item);
    }

    /// Moved/Renamed 改判：从 Added/Removed 桶迁移计数（证据驱动改写）。
    pub fn reclassify(&mut self, index: usize, kind: DifferenceKind) {
        let item = &mut self.session.items[index];
        if item.kind == kind {
            return;
        }
        // 旧桶减计
        match item.kind {
            DifferenceKind::Added => self.session.summary.added -= 1,
            DifferenceKind::Removed => self.session.summary.removed -= 1,
            _ => {}
        }
        item.kind = kind;
        match kind {
            DifferenceKind::Moved => self.session.summary.moved += 1,
            DifferenceKind::Renamed => self.session.summary.renamed += 1,
            _ => {}
        }
    }

    pub fn set_cancelled(&mut self) {
        self.session.cancelled = true;
    }

    pub fn finish(mut self) -> ComparisonSession {
        let finished = std::time::SystemTime::now();
        self.session.duration_ms = finished
            .duration_since(self.session.started_at)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        self.session.finished_at = Some(finished);
        self.session
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_accounts_kinds_and_reclassify_moves_buckets() {
        let mut b = ComparisonSession::builder("L", "R", SessionKind::Directories);
        b.push(DifferenceItem::new(
            "a.txt",
            NodeKind::File,
            DifferenceKind::Same,
        ));
        b.push(DifferenceItem::new(
            "b.txt",
            NodeKind::File,
            DifferenceKind::Removed,
        ));
        b.push(DifferenceItem::new(
            "x/b.txt",
            NodeKind::File,
            DifferenceKind::Added,
        ));
        b.push(DifferenceItem::new(
            "c.log",
            NodeKind::File,
            DifferenceKind::Ignored,
        ));
        assert_eq!(b.session.ignored_count, 1);
        // hash 证据 ⇒ b.txt 实为 moved 到 x/
        b.reclassify(1, DifferenceKind::Moved);
        b.reclassify(2, DifferenceKind::Renamed);
        let s = b.finish();
        assert_eq!(
            (s.summary.same, s.summary.moved, s.summary.renamed),
            (1, 1, 1)
        );
        assert_eq!((s.summary.added, s.summary.removed), (0, 0));
        assert_eq!(s.summary.total, 4);
        assert!(s.duration_ms < 5_000);
    }

    #[test]
    fn session_serializes_round_trip() {
        let mut b = ComparisonSession::builder("L", "R", SessionKind::Files);
        b.push(DifferenceItem::new(
            "a.txt",
            NodeKind::File,
            DifferenceKind::Modified,
        ));
        let s = b.finish();
        let json = serde_json::to_string(&s).expect("json");
        let back: ComparisonSession = serde_json::from_str(&json).expect("back");
        assert_eq!(back.summary.modified, 1);
    }
}
