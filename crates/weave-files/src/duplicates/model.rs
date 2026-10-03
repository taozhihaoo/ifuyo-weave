//! 重复扫描的数据模型（M3 §12/§29/§43/§45）。

use serde::{Deserialize, Serialize};
use std::time::SystemTime;

use weave_core::prelude::{OperationId, WeaveError};

/// 扫描阶段（M3 §47）：UI 据此告诉用户"为什么还没结束"。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanStage {
    Scanning,
    Filtering,
    PartialHashing,
    FullHashing,
    Grouping,
    Completed,
}

/// 重复组内单个文件条目（M3 §33 紧凑结构）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateFileEntry {
    /// 组内稳定序号（按规范化路径排序后分配）。
    pub file_id: String,
    pub path: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
    /// 完整 SHA-256（小写 hex）。组内所有成员此值相同。
    pub full_hash: String,
}

/// 精确重复组（M3 §12）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateGroup {
    /// 稳定身份：`grp_{full_hash 前 16 hex}`——由内容派生，不依赖扫描顺序
    /// （M3 §43；UI 展示编号是展示层概念，不入持久化事务）。
    pub group_id: String,
    pub file_count: u64,
    /// 组内单文件大小（组内所有成员 size 相同）。
    pub file_size: u64,
    /// (fileCount - 1) × fileSize —— 理论可回收空间，非实际删除承诺（M3 §12）。
    pub wasted_size: u64,
    pub files: Vec<DuplicateFileEntry>,
}

/// 重复扫描报告（M3 §29 的结构化字段全集）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DuplicateScanReport {
    pub scan_id: OperationId,
    pub roots: Vec<String>,
    pub status: weave_core::prelude::ScanStatus,
    pub stage: ScanStage,
    pub duration_ms: u64,

    pub files_scanned: u64,
    pub directories_scanned: u64,
    pub other_entries: u64,
    /// size 分组后进入候选的文件数。
    pub candidate_files: u64,
    pub partial_hashed: u64,
    pub full_hashed: u64,

    pub duplicate_groups: u64,
    pub duplicate_files: u64,
    /// 所有重复组的 wasted_size 合计（潜在可回收，非承诺）。
    pub potential_reclaimable_size: u64,

    pub skipped: u64,
    pub failed: u64,
    /// 扫描中发生变化、被排除出可靠结果的文件数（M3 §17）。
    pub changed_during_scan: u64,

    pub warnings: Vec<String>,
    pub errors: Vec<String>,

    /// 取消时的部分结果必须明确标记（M3 §27/§50）。
    pub partial_result: bool,

    /// 重复组明细（wasted DESC 排序；GroupId 内容派生）。
    pub groups: Vec<DuplicateGroup>,
}

/// Recycle 计划构建错误 / 条目校验错误复用统一错误。
pub fn recycle_plan_error(code: &str, message: impl Into<String>) -> WeaveError {
    WeaveError::validation(code, message).with_location("weave-files::duplicates")
}
