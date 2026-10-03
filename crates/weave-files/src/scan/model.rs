//! 目录扫描的数据模型（M1 §12）。

use std::time::SystemTime;

use serde::Serialize;
use weave_core::prelude::{OperationId, ScanStatus};

/// 扫描选项（来自调用方，均经边界校验）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanOptions {
    /// 递归深度上限；root = 0（M1 §12.5）。None = 用默认值。
    pub max_depth: Option<u32>,
    /// 已处理条目数上限（资源守卫，M1 §13）。None = 用默认值。
    pub max_entries: Option<u64>,
}

impl Default for ScanOptions {
    fn default() -> Self {
        Self {
            max_depth: Some(DEFAULT_MAX_DEPTH),
            max_entries: Some(DEFAULT_MAX_ENTRIES),
        }
    }
}

/// 默认资源边界（记录于 DECISIONS.md：depth 64 覆盖合理真实目录树；
/// entries 200k 防失控扫描；两者都可在 IPC 请求内调小/调大到硬上限内）。
pub const DEFAULT_MAX_DEPTH: u32 = 64;
pub const DEFAULT_MAX_ENTRIES: u64 = 200_000;
/// 选项硬上限：不允许请求超过此值的守卫（防 IPC 无界请求，M1 §18.1）。
pub const HARD_MAX_DEPTH: u32 = 512;
pub const HARD_MAX_ENTRIES: u64 = 5_000_000;

/// 单条文件行项（Top 榜单用；不含内容，只有事实）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FileLineItem {
    pub relative_path: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
}

/// 扫描中的单条错误记录（有界，见 [`DirectoryScanReport::errors_truncated`]）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanErrorEntry {
    pub relative_path: String,
    pub code: String,
    pub message: String,
}

/// 目录扫描报告。全部统计在 Rust 侧完成（M1 §39），UI 只展示。
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DirectoryScanReport {
    pub scan_id: OperationId,
    /// 规范化后的根路径。
    pub root: String,
    pub status: ScanStatus,
    pub started_at: SystemTime,
    pub finished_at: SystemTime,
    pub duration_ms: u64,

    pub directories_scanned: u64,
    pub files_scanned: u64,
    /// 符号链接等非普通文件/目录条目（symlink 策略：计数、不跟随）。
    pub other_entries: u64,
    pub error_count: u64,
    pub entries_processed: u64,

    /// 普通文件逻辑大小之和（symlink 不跟随、不计入；DECISIONS D-size）。
    pub total_size: u64,
    /// 观察到的最大深度（root 自身 = 0）。
    pub max_depth: u32,

    /// 空目录（相对路径）。定义：无直接普通文件且无直接子目录（DECISIONS）。
    pub empty_directories: Vec<String>,
    pub empty_directories_truncated: bool,

    /// 类型分布（按扩展名证据，扫描不做 sniff；count DESC → label ASC）。
    pub file_type_distribution: Vec<(String, u64)>,

    /// Top 20 榜单（排序规则固定：size DESC→path ASC；time ASC/DESC→path ASC）。
    pub largest_files: Vec<FileLineItem>,
    pub oldest_files: Vec<FileLineItem>,
    pub newest_files: Vec<FileLineItem>,

    pub warnings: Vec<String>,
    pub errors: Vec<ScanErrorEntry>,
    pub errors_truncated: bool,

    /// 触发资源上限时为 true，且 `limited_reason` 说明原因（M1 §13：不假装完成）。
    pub limited: bool,
    pub limited_reason: Option<String>,
}
