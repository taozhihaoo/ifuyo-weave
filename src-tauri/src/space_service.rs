//! M2.2：Space IPC 服务（卷概览 / 扫描 Job / 清理执行 / 快照）。
//!
//! - 扫描 Job 化（JobTracker + 取消 + 进度；结果经 SpaceResultCache 领取，
//!   与 M2.1 Compare 同一模式——不建第二套 Job）。
//! - 清理 = 复用 M3 recycle_paths + `recycle-bin:{token}` 事务目标（⇒ 既有
//!   undo_operation 可恢复）；Apply 前逐项 SizeMtime 重验（M2.0 验证契约），
//!   失配 ⇒ 该项 `space.planStale` skipped，**绝不静默删除新内容**（§36）。
//! - 释放空间双概念（§37）：ExpectedLogicalFreed（回收成功项逻辑大小合计）
//!   与 ObservedVolumeFreeDelta（卷空闲字节前后差）分开报告，不混同。

#![expect(clippy::result_large_err)] // D10：Err DTO 体积豁免

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_core::prelude::WeaveError;
use weave_files::space::{
    SnapshotStore, SpaceScanOptions, SpaceScanSession, SpaceSnapshot, growth, scan_space,
};

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("space_service")
        .into()
}

// ── 卷概览（§4.1：Windows 常规本地盘；字节内部表示，UI 转换 §47） ──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct VolumeInfoDto {
    /// 盘符（如 "C:"）。
    pub drive: String,
    pub total_bytes: f64,
    pub free_bytes: f64,
    pub used_bytes: f64,
    /// 0-100；容量 0 ⇒ None（不伪造百分比）。
    pub used_percent: Option<f64>,
}

/// 磁盘列表（sysinfo 安全 API；workspace 禁 unsafe——不手写 FFI）。
fn disk_list() -> Vec<(String, u64, u64)> {
    let disks = sysinfo::Disks::new_with_refreshed_list();
    disks
        .list()
        .iter()
        .map(|d| {
            (
                d.mount_point().to_string_lossy().into_owned(),
                d.total_space(),
                d.available_space(),
            )
        })
        .collect()
}

/// 本机可访问的本地卷（§4.1：常规本地盘；容量 0 的剔除——不伪造）。
#[tauri::command]
#[specta::specta]
pub fn space_list_volumes() -> Result<Vec<VolumeInfoDto>, IpcError> {
    let mut out = Vec::new();
    for (mount, total, free) in disk_list() {
        if total == 0 {
            continue;
        }
        let used = total.saturating_sub(free);
        // Windows 盘符根缩写（C:\）；其余挂载点（WSL/临时挂载）如实保留
        let label = if mount.len() == 3 && mount.ends_with(std::path::MAIN_SEPARATOR) {
            mount[..2].to_string()
        } else {
            mount.clone()
        };
        out.push(VolumeInfoDto {
            drive: label,
            total_bytes: total as f64,
            free_bytes: free as f64,
            used_bytes: used as f64,
            used_percent: Some(used as f64 * 100.0 / total as f64),
        });
    }
    Ok(out)
}

// ── 扫描（Job 化） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpaceScanRequestDto {
    pub root: String,
    /// Top-N（默认 100；§9）。
    pub top_n: Option<f64>,
    /// 候选阈值覆盖（None = 默认规则）。
    pub large_file_bytes: Option<f64>,
    pub old_file_days: Option<f64>,
    pub candidates: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DirNodeDto {
    pub rel: String,
    pub parent: Option<String>,
    pub logical_size: f64,
    pub file_count: f64,
    pub dir_count: f64,
    pub unreadable: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct LargestFileDto {
    pub path: String,
    pub name: String,
    pub size: f64,
    pub extension: Option<String>,
    pub category: String,
    pub modified_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TypeStatDto {
    pub extension: String,
    pub category: String,
    pub count: f64,
    pub bytes: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupCandidateDto {
    pub path: String,
    pub kind: String,
    pub size: f64,
    pub modified_ms: Option<f64>,
    pub reasons: Vec<String>,
    pub risk: String,
    pub confidence: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SpaceReportDto {
    pub operation_id: String,
    pub root: String,
    pub duration_ms: f64,
    pub cancelled: bool,
    pub consistency: String,
    pub logical_bytes: f64,
    pub file_count: f64,
    pub dir_count: f64,
    pub unreadable_dirs: f64,
    pub unreadable_files: f64,
    pub disappeared: f64,
    pub directories: Vec<DirNodeDto>,
    pub largest_files: Vec<LargestFileDto>,
    pub type_stats: Vec<TypeStatDto>,
    pub candidates: Vec<CleanupCandidateDto>,
    /// 与上一条同根快照的增长（§27；无先前快照 ⇒ None）。
    pub growth_delta_bytes: Option<f64>,
    pub growth_percent: Option<f64>,
}

impl SpaceReportDto {
    fn from_session(s: &SpaceScanSession, prev: Option<&SpaceSnapshot>) -> Self {
        let g = prev.map(|p| growth(&current_snapshot_of(s), p));
        Self {
            operation_id: s.operation_id.to_string(),
            root: s.root.clone(),
            duration_ms: s.duration_ms as f64,
            cancelled: s.cancelled,
            consistency: s.consistency.to_string(),
            logical_bytes: s.logical_bytes as f64,
            file_count: s.file_count as f64,
            dir_count: s.dir_count as f64,
            unreadable_dirs: s.unreadable_dirs as f64,
            unreadable_files: s.unreadable_files as f64,
            disappeared: s.disappeared as f64,
            directories: s
                .directories
                .iter()
                .map(|d| DirNodeDto {
                    rel: d.rel.clone(),
                    parent: d.parent.clone(),
                    logical_size: d.logical_size as f64,
                    file_count: d.file_count as f64,
                    dir_count: d.dir_count as f64,
                    unreadable: d.unreadable as f64,
                })
                .collect(),
            largest_files: s
                .largest_files
                .iter()
                .map(|f| LargestFileDto {
                    path: f.path.clone(),
                    name: f.name.clone(),
                    size: f.size as f64,
                    extension: f.extension.clone(),
                    category: f.category.as_str().to_string(),
                    modified_ms: f
                        .modified
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as f64),
                })
                .collect(),
            type_stats: s
                .type_stats
                .iter()
                .map(|t| TypeStatDto {
                    extension: t.extension.clone(),
                    category: t.category.as_str().to_string(),
                    count: t.count as f64,
                    bytes: t.bytes as f64,
                })
                .collect(),
            candidates: s
                .candidates
                .iter()
                .map(|c| CleanupCandidateDto {
                    path: c.path.clone(),
                    kind: format!("{:?}", c.kind).to_lowercase(),
                    size: c.size as f64,
                    modified_ms: c
                        .modified
                        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                        .map(|d| d.as_millis() as f64),
                    reasons: c.reasons.clone(),
                    risk: format!("{:?}", c.risk).to_lowercase(),
                    confidence: format!("{:?}", c.confidence).to_lowercase(),
                })
                .collect(),
            growth_delta_bytes: g.as_ref().map(|g| g.delta_bytes as f64),
            growth_percent: g.as_ref().and_then(|g| g.growth_pct),
        }
    }
}

fn current_snapshot_of(s: &SpaceScanSession) -> SpaceSnapshot {
    SpaceSnapshot {
        timestamp: std::time::SystemTime::now(),
        root: s.root.clone(),
        logical_bytes: s.logical_bytes,
        file_count: s.file_count,
    }
}

/// 快照存储位置：history_dir/space-snapshots.json（复用既有数据目录约定）。
fn snapshot_store(app: &tauri::AppHandle) -> Result<SnapshotStore, IpcError> {
    let dir = crate::rename_service::resolve_history_dir(app)?;
    Ok(SnapshotStore::at(dir.join("space-snapshots.json")))
}

#[derive(Default)]
pub struct SpaceResultCache {
    inner: std::sync::Mutex<std::collections::HashMap<String, SpaceReportDto>>,
}

impl SpaceResultCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, job_id: String, report: SpaceReportDto) {
        self.inner
            .lock()
            .expect("space cache")
            .insert(job_id, report);
    }

    pub fn get(&self, job_id: &str) -> Option<SpaceReportDto> {
        self.inner.lock().expect("space cache").get(job_id).cloned()
    }
}

/// 空间扫描（Job 化：进度 + 取消；完成后自动落一条快照供增长对比）。
#[tauri::command]
#[specta::specta]
pub fn space_scan(
    app: tauri::AppHandle,
    request: SpaceScanRequestDto,
) -> Result<JobHandleDto, IpcError> {
    let root = std::path::PathBuf::from(&request.root);
    if !root.is_dir() {
        return Err(err(
            "space.rootMissing",
            format!("'{}' is not a directory", request.root),
        ));
    }
    let options = SpaceScanOptions {
        top_n: request.top_n.unwrap_or(100.0).clamp(10.0, 1000.0) as usize,
        candidates: request.candidates.unwrap_or(true),
        rules: weave_files::space::CandidateRules {
            large_file_bytes: request.large_file_bytes.unwrap_or(1024.0 * 1024.0 * 1024.0) as u64,
            old_file_age: std::time::Duration::from_secs(
                (request.old_file_days.unwrap_or(180.0).clamp(1.0, 36500.0) * 86400.0) as u64,
            ),
            ..weave_files::space::CandidateRules::default()
        },
    };
    let store = snapshot_store(&app)?;
    let (job_id, cancel, sink, _) = {
        let state = app.state::<crate::state::AppState>();
        state.jobs.register()
    };
    let handle = app.clone();
    let job_id_for_task = job_id.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let report = scan_space(
            &weave_files::fs::StdFilesystem,
            &root,
            &options,
            &cancel,
            &mut |done, total| {
                sink.report(&weave_core::prelude::Progress::running(
                    weave_core::prelude::OperationId::generate(),
                    done,
                    total,
                ));
            },
        );
        // 快照（取消的会话也落——bestEffort 事实，标注 cancelled 由 UI 呈现）
        let prev = store.latest_for(&report.root, report.started_at);
        let _ = store.append(&current_snapshot_of(&report));
        let dto = SpaceReportDto::from_session(&report, prev.as_ref());
        let app_state = handle.state::<crate::state::AppState>();
        app_state
            .space_results
            .insert(job_id_for_task.to_string(), dto);
        app_state
            .jobs
            .finish(&job_id_for_task, crate::jobs::JobOutcome::SpaceCompleted);
    });

    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 领取扫描结果（未完成 ⇒ space.notReady）。
#[tauri::command]
#[specta::specta]
pub fn space_get_result(app: tauri::AppHandle, job_id: String) -> Result<SpaceReportDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    state.space_results.get(&job_id).ok_or_else(|| {
        err(
            "space.notReady",
            format!("space result not ready for '{job_id}'"),
        )
    })
}

// ── 清理执行（候选 → Recycle → Audit → 可恢复） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupItemDto {
    pub path: String,
    /// 扫描时的源快照（stale 检测依据，§36）。
    pub size: f64,
    pub modified_ms: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CleanupResultDto {
    pub operation_id: String,
    pub recycled: f64,
    /// stale（扫描后已变化/消失）被拒条目数——绝不静默删除新内容。
    pub stale_skipped: f64,
    pub failed: f64,
    /// 回收成功项逻辑大小合计（§37 Expected）。
    pub expected_logical_freed: f64,
    /// 卷空闲字节前后差（§37 Observed；同卷可测时才有，否则 None）。
    pub observed_free_delta_bytes: Option<f64>,
}

/// 清理选中候选：Stale 重验 → recycle（Quarantine）→ 事务/历史 → freed 双报告。
#[tauri::command]
#[specta::specta]
pub fn space_cleanup(
    app: tauri::AppHandle,
    items: Vec<CleanupItemDto>,
) -> Result<CleanupResultDto, IpcError> {
    use weave_core::prelude::VerificationPolicy;
    use weave_files::fs::StdFilesystem;
    use weave_files::recycle::{RECYCLE_TARGET_PREFIX, RecycleOutcome, recycle_paths};
    use weave_files::verify::verify_fingerprint;
    use weave_history::{
        HistoryEntry, OperationStatus, OperationTransaction, Reversibility, TransactionItem,
        TransactionItemStatus,
    };

    if items.is_empty() {
        return Err(err("space.emptySelection", "no candidates selected"));
    }
    let fs = StdFilesystem;
    let now = std::time::SystemTime::now();

    // 1) Apply 前验证（§36）：Path 存在 + size/mtime 与扫描快照一致
    let mut stale = 0u64;
    let mut to_recycle: Vec<(std::path::PathBuf, u64)> = Vec::new();
    for item in &items {
        let path = std::path::PathBuf::from(&item.path);
        let modified = item.modified_ms.and_then(|ms| {
            std::time::UNIX_EPOCH.checked_add(std::time::Duration::from_millis(ms as u64))
        });
        let fingerprint = weave_core::prelude::FileFingerprint {
            path: item.path.clone(),
            size: Some(item.size as u64),
            modified,
            digest_hex: None,
        };
        match verify_fingerprint(&fs, &fingerprint, VerificationPolicy::SizeMtime, "space") {
            Ok(()) => to_recycle.push((path, item.size as u64)),
            Err(_) => stale += 1, // PlanStale：跳过并如实报告，不删除
        }
    }

    // 观察卷空闲差（同卷可测；根卷 = 第一项所在卷）
    let free_before = items.first().and_then(|i| free_bytes_of(&i.path));

    // 2) Quarantine（复用 M3 回收站；禁止永久删除，§34）
    let paths: Vec<std::path::PathBuf> = to_recycle.iter().map(|(p, _)| p.clone()).collect();
    let results = recycle_paths(&paths);
    let mut tx_items: Vec<TransactionItem> = Vec::with_capacity(results.len());
    let (mut recycled, mut failed) = (0u64, 0u64);
    let mut recycled_bytes = 0u64;
    for ((path, size), (_, outcome)) in to_recycle.iter().zip(results.iter()) {
        match outcome {
            RecycleOutcome::Success(receipt) => {
                recycled += 1;
                recycled_bytes += size;
                tx_items.push(TransactionItem {
                    item_id: format!("item_{:04}", tx_items.len()),
                    source_path: receipt.original_path.to_string_lossy().into_owned(),
                    target_path: match &receipt.token {
                        Some(t) => format!("{RECYCLE_TARGET_PREFIX}{t}"),
                        None => RECYCLE_TARGET_PREFIX.to_string(),
                    },
                    status: TransactionItemStatus::Executed,
                    timestamp: Some(receipt.deleted_at),
                    original_size: Some(*size),
                    original_modified: None,
                    original_created: None,
                });
            }
            _ => {
                failed += 1;
                tx_items.push(TransactionItem {
                    item_id: format!("item_{:04}", tx_items.len()),
                    source_path: path.to_string_lossy().into_owned(),
                    target_path: String::new(),
                    status: TransactionItemStatus::NotExecuted,
                    timestamp: Some(now),
                    original_size: Some(*size),
                    original_modified: None,
                    original_created: None,
                });
            }
        }
    }

    // 3) 事务/历史（undo 走既有 undo_operation；M2.0 Audit/中断检测直接适用）
    let operation_id = weave_core::prelude::OperationId::generate();
    let reversible = if recycled == 0 {
        Reversibility::None
    } else if failed == 0 && stale == 0 {
        Reversibility::Full
    } else {
        Reversibility::Partial
    };
    let tx = OperationTransaction {
        operation_id: operation_id.clone(),
        kind: weave_core::prelude::OperationKind::DuplicateRecycle,
        status: OperationStatus::Completed,
        timestamp: now,
        reversible,
        items: tx_items,
    };
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    if let Ok(store) = weave_history::HistoryStore::open(&history_dir) {
        let entry = HistoryEntry {
            operation_id: tx.operation_id.clone(),
            kind: tx.kind,
            timestamp: tx.timestamp,
            summary: format!("Space cleanup: recycle {recycled} items"),
            item_count: tx.items.len() as u64,
            success_count: recycled,
            failed_count: failed,
            skipped_count: stale,
            undoable: recycled > 0,
            status: OperationStatus::Completed,
            input_root: None,
            rule_summary: Some("space cleanup candidates".to_string()),
        };
        let _ = store
            .save_transaction(&tx)
            .and_then(|()| store.upsert_entry(entry));
    }

    let free_after = items.first().and_then(|i| free_bytes_of(&i.path));
    Ok(CleanupResultDto {
        operation_id: operation_id.to_string(),
        recycled: recycled as f64,
        stale_skipped: stale as f64,
        failed: failed as f64,
        expected_logical_freed: recycled_bytes as f64,
        observed_free_delta_bytes: match (free_before, free_after) {
            (Some(b), Some(a)) => Some((a as i64 - b as i64) as f64),
            _ => None,
        },
    })
}

fn free_bytes_of(path: &str) -> Option<u64> {
    // 同卷空闲字节：路径前缀最长匹配挂载点（sysinfo 安全 API）
    disk_list()
        .into_iter()
        .filter(|(mount, _, _)| path.starts_with(mount.as_str()))
        .max_by_key(|(mount, _, _)| mount.len())
        .map(|(_, _, free)| free)
}
