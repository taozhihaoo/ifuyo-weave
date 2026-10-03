//! 历史持久化（M2 §54–§56）。
//!
//! - `entries.json`：带 schema 版本的条目列表（最新在前，容量 500 有界）。
//! - `transactions/<op_id>.json`：每操作一份事务。
//! - 全部写入为 **temp + rename 原子替换**（同目录，Windows 原子语义成立）。
//! - 损坏处理（M2 §55）：解析失败 ⇒ 损坏文件隔离为 `*.corrupt` 并以空状态
//!   安全降级——绝不让 Weave 因历史损坏而无法启动；隔离事实返回给调用方
//!   供 UI 明示。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use weave_core::prelude::OperationId;
use weave_core::prelude::WeaveError;

use crate::model::{HISTORY_SCHEMA_VERSION, HistoryEntry, OperationTransaction};

/// 条目列表容量上限（最旧者被淘汰；事务文件保留——淘汰条目的 Undo 仍可用）。
pub const MAX_ENTRIES: usize = 500;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOutcome {
    pub entries: Vec<HistoryEntry>,
    /// 加载过程中被隔离的损坏文件（相对文件名），供 UI 明示（M2 §55）。
    pub quarantined: Vec<String>,
}

pub struct HistoryStore {
    dir: PathBuf,
}

impl HistoryStore {
    /// 打开（必要时创建）历史目录。
    pub fn open(dir: &Path) -> Result<Self, WeaveError> {
        fs::create_dir_all(dir).map_err(|e| {
            WeaveError::io(
                "history.openFailed",
                format!("cannot create history dir: {e}"),
            )
            .with_location("weave-history::store")
        })?;
        fs::create_dir_all(dir.join("transactions")).map_err(|e| {
            WeaveError::io(
                "history.openFailed",
                format!("cannot create transactions dir: {e}"),
            )
            .with_location("weave-history::store")
        })?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    fn entries_path(&self) -> PathBuf {
        self.dir.join("entries.json")
    }

    fn transaction_path(&self, op: &OperationId) -> PathBuf {
        self.dir.join("transactions").join(format!("{op}.json"))
    }

    /// 原子写：同目录临时文件 + rename 覆盖。
    fn atomic_write(path: &Path, content: &str) -> Result<(), WeaveError> {
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, content).map_err(|e| {
            WeaveError::io(
                "history.writeFailed",
                format!("cannot write temp file: {e}"),
            )
            .with_location("weave-history::store")
        })?;
        fs::rename(&tmp, path).map_err(|e| {
            let _ = fs::remove_file(&tmp);
            WeaveError::io(
                "history.writeFailed",
                format!("cannot replace history file: {e}"),
            )
            .with_location("weave-history::store")
        })?;
        Ok(())
    }

    fn quarantine(path: &Path) -> Option<String> {
        let stamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0);
        let file_name = path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unknown".to_string());
        let quarantined = format!("{file_name}.corrupt-{stamp}");
        let target = path.with_file_name(&quarantined);
        fs::rename(path, &target).ok().map(|_| quarantined)
    }

    /// 加载条目列表；损坏则隔离并返回空列表（安全降级，M2 §55）。
    pub fn load_entries(&self) -> Result<LoadOutcome, WeaveError> {
        let path = self.entries_path();
        if !path.exists() {
            return Ok(LoadOutcome {
                entries: Vec::new(),
                quarantined: Vec::new(),
            });
        }
        let raw = fs::read_to_string(&path).map_err(|e| {
            WeaveError::io("history.readFailed", format!("cannot read entries: {e}"))
                .with_location("weave-history::store")
        })?;
        match serde_json::from_str::<crate::model::HistoryFile>(&raw) {
            Ok(file) if file.schema_version == HISTORY_SCHEMA_VERSION => Ok(LoadOutcome {
                entries: file.entries,
                quarantined: Vec::new(),
            }),
            Ok(file) => {
                // 未知/过期 schema：如实隔离，不猜测迁移（charter #47 精神）。
                let q = Self::quarantine(&path).ok_or_else(|| {
                    WeaveError::io("history.quarantineFailed", "cannot quarantine")
                })?;
                let _ = file;
                Ok(LoadOutcome {
                    entries: Vec::new(),
                    quarantined: vec![q],
                })
            }
            Err(_) => {
                let q = Self::quarantine(&path).ok_or_else(|| {
                    WeaveError::io("history.quarantineFailed", "cannot quarantine")
                })?;
                Ok(LoadOutcome {
                    entries: Vec::new(),
                    quarantined: vec![q],
                })
            }
        }
    }

    fn write_entries(&self, entries: &[HistoryEntry]) -> Result<(), WeaveError> {
        let file = crate::model::HistoryFile {
            schema_version: HISTORY_SCHEMA_VERSION,
            entries: entries.to_vec(),
        };
        let json = serde_json::to_string_pretty(&file).map_err(|e| {
            WeaveError::internal("history.serializeFailed", format!("serialize entries: {e}"))
                .with_location("weave-history::store")
        })?;
        Self::atomic_write(&self.entries_path(), &json)
    }

    /// 事务先行落盘（执行开始前，状态 InProgress —— M2 §82/§83 crash safety）。
    pub fn save_transaction(&self, transaction: &OperationTransaction) -> Result<(), WeaveError> {
        let json = serde_json::to_string_pretty(transaction).map_err(|e| {
            WeaveError::internal(
                "history.serializeFailed",
                format!("serialize transaction: {e}"),
            )
            .with_location("weave-history::store")
        })?;
        Self::atomic_write(&self.transaction_path(&transaction.operation_id), &json)
    }

    /// 追加条目（最新在前），容量淘汰最旧；同一 operation_id 的旧条目先移除
    /// （支持"执行开始 InProgress → 结束更新终态"的两阶段写入）。
    pub fn upsert_entry(&self, entry: HistoryEntry) -> Result<(), WeaveError> {
        let mut outcome = self.load_entries()?;
        outcome
            .entries
            .retain(|e| e.operation_id != entry.operation_id);
        outcome.entries.insert(0, entry);
        outcome.entries.truncate(MAX_ENTRIES);
        self.write_entries(&outcome.entries)
    }

    /// 读取事务。损坏 ⇒ 隔离该事务文件并返回 Ok(None)（Undo 不可用但应用存活）。
    pub fn load_transaction(
        &self,
        op: &OperationId,
    ) -> Result<Option<OperationTransaction>, WeaveError> {
        let path = self.transaction_path(op);
        if !path.exists() {
            return Ok(None);
        }
        let raw = fs::read_to_string(&path).map_err(|e| {
            WeaveError::io(
                "history.readFailed",
                format!("cannot read transaction: {e}"),
            )
            .with_location("weave-history::store")
        })?;
        match serde_json::from_str::<OperationTransaction>(&raw) {
            Ok(t) => Ok(Some(t)),
            Err(_) => {
                let _ = Self::quarantine(&path);
                Ok(None)
            }
        }
    }

    /// 更新事务终态（执行结束后重写同一文件）。
    pub fn update_transaction(&self, transaction: &OperationTransaction) -> Result<(), WeaveError> {
        self.save_transaction(transaction)
    }

    /// 最近条目（已是最新在前）。
    pub fn recent(&self, limit: usize) -> Result<Vec<HistoryEntry>, WeaveError> {
        let mut entries = self.load_entries()?.entries;
        entries.truncate(limit);
        Ok(entries)
    }
}
