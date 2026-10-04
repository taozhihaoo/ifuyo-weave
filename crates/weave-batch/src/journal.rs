//! Job Journal（M7 下 §42/§92/§183/§196/§197）：崩溃恢复的持久化事实。
//!
//! 形态：每 Job 一个 append-only JSONL——首行 header（schema 版本 + 计划 +
//! 总数），其后每完成一条 item 追加一行 record，终态追加 finish 行。
//! 任何非正常终止（崩溃/断电）留下的文件 = Interrupted(Recoverable)。
//!
//! 损坏处理（§197）：尾部半行/坏行**不 panic、不静默删除**——返回
//! 完好前缀 + 损坏标记，Resume 时把原文件整体隔离为 `.corrupt`（保留
//! 原件），恢复只基于完好前缀（§185 No Magic Recovery）。

use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Journal schema 版本（§196：unsupported version ⇒ 显式拒绝）。
pub const JOURNAL_SCHEMA_VERSION: u32 = 1;

/// Journal 首行（§196：versioned schema）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalHeader {
    pub schema_version: u32,
    pub job_id: String,
    /// 创建时刻（UNIX epoch 毫秒）。
    pub created_ms: u64,
    /// 完整 JobPlan（§19 可序列化——Resume 时重建执行）。
    pub plan: crate::plan::JobPlan,
    pub total_items: u64,
}

/// 单条 item 的完成事实（追加点写，崩溃前已落盘 = 已发生）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalItemRecord {
    /// 快照序（§18 item_{n} 的 n）。
    pub item_index: usize,
    pub status: JournalItemStatus,
    pub output_path: Option<String>,
    pub output_size: Option<u64>,
    pub error: Option<String>,
}

/// Journal 侧条目状态（只记终态事实；Paused 不是 item 事实）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum JournalItemStatus {
    Success,
    Failed,
    Skipped,
    /// §14/§15：拒绝也是事实，Resume 不再重跑。
    Cancelled,
}

/// 终态行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JournalFinish {
    /// completed | failed | cancelled | paused
    pub state: String,
}

/// Journal 单行。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum JournalLine {
    Header(JournalHeader),
    Item(JournalItemRecord),
    Finish(JournalFinish),
}

/// Journal 载入结果：完好前缀 + 损坏事实（§197）。
#[derive(Debug, Clone, PartialEq)]
pub struct JournalLoad {
    pub header: Option<JournalHeader>,
    pub items: Vec<JournalItemRecord>,
    pub finish: Option<JournalFinish>,
    /// 尾部坏行数（>0 = 文件曾被截断/损坏；原件在 resume 时隔离）。
    pub corrupt_tail_lines: usize,
}

impl JournalLoad {
    /// 已成功完成的 item index 集（§126 Resume 依据）。
    pub fn completed_indices(&self) -> Vec<usize> {
        let mut out: Vec<usize> = self
            .items
            .iter()
            .filter(|r| r.status == JournalItemStatus::Success)
            .map(|r| r.item_index)
            .collect();
        out.sort_unstable();
        out.dedup();
        out
    }

    /// 已有终态事实（含 Failed/Skipped/Cancelled）的 index 集——Resume
    /// 不重跑这些（§39 副作用不重复）。
    pub fn settled_indices(&self) -> Vec<usize> {
        let mut out: Vec<usize> = self.items.iter().map(|r| r.item_index).collect();
        out.sort_unstable();
        out.dedup();
        out
    }
}

/// 目录内 journal 文件名约定。
pub fn journal_path(dir: &Path, job_id: &str) -> PathBuf {
    dir.join(format!("job-{job_id}.jsonl"))
}

/// 创建 journal（写入 header 行）。目标已存在 = 重复创建 ⇒ 拒绝（§201）。
pub fn create(dir: &Path, header: &JournalHeader) -> Result<PathBuf, String> {
    std::fs::create_dir_all(dir).map_err(|e| format!("journal dir create failed: {e}"))?;
    let path = journal_path(dir, &header.job_id);
    if path.exists() {
        return Err(format!("journal already exists: {}", path.display()));
    }
    let line = serde_json::to_string(&JournalLine::Header(header.clone()))
        .map_err(|e| format!("header serialize failed: {e}"))?;
    // 无 O_APPEND 竞争（单写者 = job 执行任务，§202 单执行者），但 flush
    // 到盘：崩溃恢复依赖已写行真实存在。
    let mut f = std::fs::File::create(&path).map_err(|e| format!("journal create failed: {e}"))?;
    writeln!(f, "{line}").map_err(|e| format!("journal write failed: {e}"))?;
    f.sync_all()
        .map_err(|e| format!("journal sync failed: {e}"))?;
    Ok(path)
}

/// 追加一行（append + flush；调用方 = 唯一写者）。
pub fn append(path: &Path, line: &JournalLine) -> Result<(), String> {
    let text = serde_json::to_string(line).map_err(|e| format!("serialize failed: {e}"))?;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(path)
        .map_err(|e| format!("journal open failed: {e}"))?;
    writeln!(f, "{text}").map_err(|e| format!("journal append failed: {e}"))?;
    f.sync_all()
        .map_err(|e| format!("journal sync failed: {e}"))?;
    Ok(())
}

/// 载入：逐行解析；坏行只计数，**不抛错、不丢完好前缀**（§197）。
pub fn load(path: &Path) -> Result<JournalLoad, String> {
    if !path.exists() {
        return Err(format!("journal missing: {}", path.display()));
    }
    let text = std::fs::read_to_string(path).map_err(|e| format!("journal read failed: {e}"))?;
    let mut out = JournalLoad {
        header: None,
        items: Vec::new(),
        finish: None,
        corrupt_tail_lines: 0,
    };
    let mut saw_corrupt = false;
    for line in text.lines() {
        if line.trim().is_empty() {
            continue;
        }
        // 损坏后不再接受后续行（保持顺序事实；全部计为 corrupt tail）
        if saw_corrupt {
            out.corrupt_tail_lines += 1;
            continue;
        }
        match serde_json::from_str::<JournalLine>(line) {
            Ok(JournalLine::Header(h)) => out.header = Some(h),
            Ok(JournalLine::Item(i)) => out.items.push(i),
            Ok(JournalLine::Finish(f)) => out.finish = Some(f),
            Err(_) => {
                saw_corrupt = true;
                out.corrupt_tail_lines += 1;
            }
        }
    }
    Ok(out)
}

/// Resume 前的损坏隔离（§197 safe quarantine）：原件复制为 `.corrupt`
/// 保留，journal 以完好前缀继续追加。返回隔离副本路径。
pub fn quarantine_corrupt(path: &Path) -> Result<PathBuf, String> {
    let copy = path.with_extension("jsonl.corrupt");
    std::fs::copy(path, &copy).map_err(|e| format!("quarantine copy failed: {e}"))?;
    Ok(copy)
}

/// 列出目录内全部 journal 及其恢复态（§183：Completed/Failed/Cancelled/
/// Interrupted(Recoverable)）。
#[derive(Debug, Clone, PartialEq)]
pub struct JournalSummary {
    pub job_id: String,
    pub path: PathBuf,
    pub header: Option<JournalHeader>,
    pub load: JournalLoad,
}

pub fn list(dir: &Path) -> Vec<JournalSummary> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().and_then(|e| e.to_str()) != Some("jsonl") {
            continue;
        }
        let Ok(load) = load(&path) else {
            continue;
        };
        let job_id = load
            .header
            .as_ref()
            .map(|h| h.job_id.clone())
            .unwrap_or_else(|| {
                path.file_stem()
                    .map(|s| s.to_string_lossy().trim_start_matches("job-").to_owned())
                    .unwrap_or_default()
            });
        out.push(JournalSummary {
            job_id,
            path,
            header: load.header.clone(),
            load,
        });
    }
    out.sort_by(|a, b| a.job_id.cmp(&b.job_id));
    out
}
