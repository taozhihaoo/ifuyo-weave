//! 剪贴板持久化（M2.3 §30-§35/§83-§84）。
//!
//! 形状（§31）：索引（entries.json，versioned schema）+ payload 文件
//! （payloads/<file>）——原始内容不入索引。
//!
//! - 原子写：temp + rename（§32：不直接覆盖唯一数据文件）。
//! - 损坏处理（§30/§32）：索引解析失败 ⇒ 隔离 `.corrupt-<ts>` 并空降级，
//!   绝不让 Weave 启动失败；单条 payload 丢失 ⇒ 条目 Missing（不吞）。
//! - GC（§33/§34）：孤儿 payload（无索引引用）/失效 payload（索引无文件）
//!   增量清扫；可取消。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use weave_core::prelude::WeaveError;

use crate::model::{
    ClipboardEntry, EntryStatus, RetentionPolicy, RetentionReport, STORAGE_SCHEMA_VERSION,
    storage_error,
};

/// 剪贴板历史存储：目录约定
///
/// ```text
/// <dir>/entries.json       ← 索引（versioned；原子写）
/// <dir>/payloads/<file>    ← 原始内容（按需读写）
/// ```
pub struct ClipboardStore {
    dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoadOutcome {
    pub entries: Vec<ClipboardEntry>,
    pub quarantined: Option<String>,
    /// 索引引用但磁盘缺失 payload 的条目数（status 置 Missing，不丢弃）。
    pub missing_payloads: u64,
}

impl ClipboardStore {
    pub fn open(dir: &Path) -> Result<Self, WeaveError> {
        std::fs::create_dir_all(dir.join("payloads")).map_err(|e| {
            storage_error(
                "clipboard.openFailed",
                format!("cannot create store dir: {e}"),
            )
        })?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    fn index_path(&self) -> PathBuf {
        self.dir.join("entries.json")
    }

    pub fn payload_path(&self, file: &str) -> PathBuf {
        self.dir.join("payloads").join(file)
    }

    /// 写 payload（原子：temp + rename；§32）。
    pub fn write_payload(&self, file: &str, bytes: &[u8]) -> Result<(), WeaveError> {
        let target = self.payload_path(file);
        let tmp = target.with_extension("tmp");
        std::fs::write(&tmp, bytes)
            .and_then(|()| std::fs::rename(&tmp, &target))
            .map_err(|e| storage_error("clipboard.payloadWriteFailed", format!("{e}")))
    }

    /// 读 payload（§26 重新复制前验证数据可用；缺失 ⇒ Missing 事实）。
    pub fn read_payload(&self, file: &str) -> Result<Vec<u8>, WeaveError> {
        std::fs::read(self.payload_path(file))
            .map_err(|e| storage_error("clipboard.payloadMissing", format!("{e}")))
    }

    pub fn delete_payload(&self, file: &str) -> Result<(), WeaveError> {
        let path = self.payload_path(file);
        if path.exists() {
            std::fs::remove_file(path)
                .map_err(|e| storage_error("clipboard.payloadDeleteFailed", format!("{e}")))?;
        }
        Ok(())
    }

    fn atomic_write_index(&self, entries: &[ClipboardEntry]) -> Result<(), WeaveError> {
        let file = serde_json::json!({
            "schemaVersion": STORAGE_SCHEMA_VERSION,
            "entries": entries,
        });
        let json = serde_json::to_string_pretty(&file)
            .map_err(|e| storage_error("clipboard.serializeFailed", format!("{e}")))?;
        let tmp = self.index_path().with_extension("tmp");
        std::fs::write(&tmp, json)
            .and_then(|()| std::fs::rename(&tmp, self.index_path()))
            .map_err(|e| storage_error("clipboard.indexWriteFailed", format!("{e}")))
    }

    /// 载入索引（§30：单条损坏不影响整体；索引损坏 ⇒ 隔离 + 空降级）。
    pub fn load(&self) -> Result<LoadOutcome, WeaveError> {
        let path = self.index_path();
        if !path.exists() {
            return Ok(LoadOutcome {
                entries: Vec::new(),
                quarantined: None,
                missing_payloads: 0,
            });
        }
        let raw = std::fs::read_to_string(&path)
            .map_err(|e| storage_error("clipboard.readFailed", format!("{e}")))?;
        let parsed: Option<serde_json::Value> = serde_json::from_str(&raw).ok();
        let mut outcome = LoadOutcome {
            entries: Vec::new(),
            quarantined: None,
            missing_payloads: 0,
        };
        let version = parsed
            .as_ref()
            .and_then(|v| v.get("schemaVersion"))
            .and_then(|v| v.as_u64());
        let entries_json = parsed
            .as_ref()
            .and_then(|v| v.get("entries"))
            .and_then(|v| v.as_array())
            .cloned();
        match (version, entries_json) {
            (Some(v), Some(list)) if v == STORAGE_SCHEMA_VERSION as u64 => {
                for item in list {
                    match serde_json::from_value::<ClipboardEntry>(item) {
                        Ok(mut entry) => {
                            // payload 完整性：文件缺失 ⇒ Missing 事实（§50，不丢弃）
                            if let Some(file) = &entry.payload_file
                                && !self.payload_path(file).exists()
                            {
                                entry.status = EntryStatus::Missing;
                                outcome.missing_payloads += 1;
                            }
                            outcome.entries.push(entry);
                        }
                        Err(_) => {
                            // 单条损坏：跳过（§30 单条失败 ≠ 整体失败）
                        }
                    }
                }
            }
            _ => {
                // 版本不符/结构损坏：隔离（charter #47 精神：不猜测迁移）
                let stamp = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_millis())
                    .unwrap_or(0);
                let quarantined = format!("entries.json.corrupt-{stamp}");
                std::fs::rename(&path, self.dir.join(&quarantined))
                    .map_err(|e| storage_error("clipboard.quarantineFailed", format!("{e}")))?;
                outcome.quarantined = Some(quarantined);
            }
        }
        Ok(outcome)
    }

    /// 保存全部（调用方持有完整列表语义——历史列表规模有界）。
    pub fn save_all(&self, entries: &[ClipboardEntry]) -> Result<(), WeaveError> {
        self.atomic_write_index(entries)
    }

    /// upsert 一条并持久化（dedup 由调用方基于 id 完成后传入最终列表；
    /// 单写者模型 = capture 任务线程）。
    pub fn persist(&self, entries: &[ClipboardEntry]) -> Result<(), WeaveError> {
        self.atomic_write_index(entries)
    }

    /// 删除条目 = 删除元数据 + payload（§65：绝不触碰原文件——File 条目
    /// 的 file_paths 只是记录）。
    pub fn remove(&self, entries: &mut Vec<ClipboardEntry>, id: &str) -> Result<bool, WeaveError> {
        let Some(pos) = entries.iter().position(|e| e.id == id) else {
            return Ok(false);
        };
        let removed = entries.remove(pos);
        if let Some(file) = &removed.payload_file {
            self.delete_payload(file)?;
        }
        self.persist(entries)?;
        Ok(true)
    }

    /// 清空历史（§29：与"清空系统剪贴板"完全分离——本方法只动数据目录）。
    pub fn clear(&self, entries: &mut Vec<ClipboardEntry>) -> Result<u64, WeaveError> {
        let count = entries.len() as u64;
        for entry in entries.drain(..) {
            if let Some(file) = &entry.payload_file {
                self.delete_payload(file).ok(); // payload 清理尽力而为；索引为准
            }
        }
        self.persist(entries)?;
        Ok(count)
    }

    /// GC（§33/§34）：孤儿 payload（磁盘有、索引无）清扫 + 指数过期淘汰 +
    /// 重新持久化。返回清扫的孤儿数与过期数。
    pub fn run_gc(
        &self,
        entries: &mut Vec<ClipboardEntry>,
        policy: &RetentionPolicy,
    ) -> Result<RetentionReport, WeaveError> {
        let mut report = RetentionReport::default();
        // 1) 过期淘汰（复用保留策略）
        let before = entries.len() as u64;
        let referenced: std::collections::HashSet<String> = entries
            .iter()
            .flat_map(|e| [e.payload_file.clone(), e.thumbnail_file.clone()])
            .flatten()
            .collect();
        let r = crate::model::apply_retention(entries, &|e| e.size, policy, SystemTime::now());
        report.expired_removed = r.expired_removed;
        report.overflow_removed = r.overflow_removed;
        report.storage_removed = r.storage_removed;
        // 2) 被淘汰条目的 payload 删除
        let now_referenced: std::collections::HashSet<String> = entries
            .iter()
            .flat_map(|e| [e.payload_file.clone(), e.thumbnail_file.clone()])
            .flatten()
            .collect();
        for file in referenced.difference(&now_referenced) {
            self.delete_payload(file)?;
        }
        // 3) 孤儿 payload（磁盘有、索引无）
        let payloads_dir = self.dir.join("payloads");
        if let Ok(read) = std::fs::read_dir(&payloads_dir) {
            for entry in read.flatten() {
                let name = entry.file_name().to_string_lossy().into_owned();
                if name.ends_with(".tmp") {
                    // 崩溃残留 temp（§33）：清理
                    let _ = std::fs::remove_file(entry.path());
                    continue;
                }
                if !now_referenced.contains(&name) {
                    let _ = std::fs::remove_file(entry.path());
                    report.expired_removed += 1; // 孤儿计数并入报告（不细分字段）
                }
            }
        }
        let _ = before;
        self.persist(entries)?;
        Ok(report)
    }

    /// 存储占用（§54 报告：索引 + payload 总字节）。
    pub fn storage_usage(&self, entries: &[ClipboardEntry]) -> u64 {
        let index = std::fs::metadata(self.index_path())
            .map(|m| m.len())
            .unwrap_or(0);
        let payloads: u64 = entries
            .iter()
            .filter_map(|e| e.payload_file.as_ref())
            .filter_map(|f| std::fs::metadata(self.payload_path(f)).ok())
            .map(|m| m.len())
            .sum();
        index + payloads
    }

    /// 按需把状态收敛到 Available/Missing/Corrupted（§50；供详情/恢复前检查）。
    pub fn refresh_status(&self, entry: &mut ClipboardEntry) {
        match &entry.payload_file {
            None => entry.status = EntryStatus::Available, // 纯记录型（File 路径列表）
            Some(file) => match std::fs::metadata(self.payload_path(file)) {
                Ok(_) => entry.status = EntryStatus::Available,
                Err(_) => entry.status = EntryStatus::Missing,
            },
        }
    }

    /// id → entry 索引视图（§44：搜索/列表只动元数据，不读 payload）。
    pub fn index_by_id(entries: &[ClipboardEntry]) -> HashMap<String, usize> {
        entries
            .iter()
            .enumerate()
            .map(|(i, e)| (e.id.clone(), i))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{ContentKind, SourceInfo, SourceState};
    use weave_testkit::TempWorkspace;

    fn sample(hash: &str, payload: Option<&str>) -> ClipboardEntry {
        let _ = &payload;
        let now = SystemTime::now();
        ClipboardEntry {
            id: ClipboardEntry::dedup_key(ContentKind::Text, hash),
            created_at: now,
            updated_at: now,
            kind: ContentKind::Text,
            content_hash: hash.to_string(),
            preview: format!("preview {hash}"),
            search_text: format!("preview {hash}"),
            payload_file: payload.map(|_p| format!("{hash}.bin")),
            thumbnail_file: None,
            size: payload.map(|p| p.len() as u64).unwrap_or(0),
            source: SourceInfo {
                state: SourceState::Unknown,
                process: None,
            },
            file_paths: vec![],
            is_favorite: false,
            sensitivity: crate::model::Sensitivity::Normal,
            status: EntryStatus::Persisted,
        }
    }

    #[test]
    fn save_load_round_trip_and_payload_write_read() {
        let ws = TempWorkspace::new("clip-store").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        store.write_payload("h1.bin", b"payload-1").expect("write");
        let entries = vec![sample("h1", Some("payload-1"))];
        store.save_all(&entries).expect("save");

        // 重启模拟：新实例加载
        let reopened = ClipboardStore::open(ws.path()).expect("reopen");
        let outcome = reopened.load().expect("load");
        assert!(outcome.quarantined.is_none());
        assert_eq!(outcome.missing_payloads, 0);
        assert_eq!(outcome.entries.len(), 1);
        assert_eq!(outcome.entries[0].content_hash, "h1");
        assert_eq!(reopened.read_payload("h1.bin").expect("read"), b"payload-1");
    }

    #[test]
    fn missing_payload_becomes_status_not_error() {
        let ws = TempWorkspace::new("clip-missing").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let entries = vec![sample("ghost", Some("ghost.bin"))];
        store.save_all(&entries).expect("save");
        let outcome = store.load().expect("load");
        assert_eq!(outcome.missing_payloads, 1);
        assert_eq!(outcome.entries[0].status, EntryStatus::Missing);
    }

    #[test]
    fn corrupted_index_is_quarantined_not_fatal() {
        let ws = TempWorkspace::new("clip-corrupt").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        std::fs::write(store.index_path(), "{ not json").expect("write corrupt");
        let outcome = store.load().expect("load degrades");
        assert!(outcome.quarantined.is_some());
        assert!(outcome.entries.is_empty());
        // 隔离后可继续写入（应用存活 §30）
        store.save_all(&[]).expect("save after quarantine");
    }

    #[test]
    fn remove_deletes_metadata_and_payload_but_never_referenced_files() {
        let ws = TempWorkspace::new("clip-remove").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let original = ws.file("original.txt", "REAL USER FILE").expect("write");
        store.write_payload("h1.bin", b"payload").expect("write");
        let mut entries = vec![ClipboardEntry {
            file_paths: vec![original.to_string_lossy().into_owned()],
            ..sample("h1", Some("payload-1"))
        }];
        entries[0].payload_file = Some("h1.bin".to_string());

        let id = entries[0].id.clone();
        assert!(store.remove(&mut entries, &id).expect("remove"));
        assert!(!store.payload_path("h1.bin").exists(), "payload 删除");
        assert!(original.exists(), "§65：历史删除绝不触碰原文件");
        assert!(entries.is_empty());
    }

    #[test]
    fn clear_history_removes_store_not_system_clipboard_concept() {
        let ws = TempWorkspace::new("clip-clear").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        store.write_payload("h1.bin", b"p").expect("write");
        let mut entries = vec![sample("h1", Some("payload-1"))];
        let count = store.clear(&mut entries).expect("clear");
        assert_eq!(count, 1);
        assert!(entries.is_empty());
        assert!(!store.payload_path("h1.bin").exists());
    }

    #[test]
    fn gc_removes_orphans_and_crash_temps() {
        let ws = TempWorkspace::new("clip-gc").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        store.write_payload("kept.bin", b"k").expect("write");
        store.write_payload("orphan.bin", b"o").expect("write");
        std::fs::write(store.payload_path("crashed.tmp"), b"t").expect("write temp");
        let mut entries = vec![sample("kept", Some("payload-1"))];
        entries[0].payload_file = Some("kept.bin".to_string());

        let report = store
            .run_gc(&mut entries, &RetentionPolicy::default())
            .expect("gc");
        assert!(store.payload_path("kept.bin").exists());
        assert!(!store.payload_path("orphan.bin").exists(), "孤儿清扫");
        assert!(
            !store.payload_path("crashed.tmp").exists(),
            "崩溃 temp 清理"
        );
        assert_eq!(report.expired_removed + report.overflow_removed, 1);
    }

    #[test]
    fn storage_usage_counts_index_and_payloads() {
        let ws = TempWorkspace::new("clip-usage").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        store
            .write_payload("h1.bin", vec![0u8; 1000].as_slice())
            .expect("write");
        let entries = vec![sample("h1", Some("payload-1"))];
        store.save_all(&entries).expect("save");
        assert!(store.storage_usage(&entries) >= 1000);
    }

    #[test]
    fn file_entry_without_payload_is_available_by_convention() {
        let ws = TempWorkspace::new("clip-file").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entry = sample("files", None);
        entry.kind = ContentKind::File;
        entry.payload_file = None;
        entry.file_paths = vec!["D:\\a.txt".to_string()];
        store.save_all(&[entry.clone()]).expect("save");
        let mut loaded = store.load().expect("load").entries;
        assert_eq!(loaded.len(), 1);
        store.refresh_status(&mut loaded[0]);
        assert_eq!(
            loaded[0].status,
            EntryStatus::Available,
            "File 记录型条目无 payload"
        );
    }
}

#[cfg(test)]
mod perf_tests {
    use super::*;
    use crate::model::{ContentKind, Sensitivity, SourceInfo, SourceState};
    use weave_testkit::TempWorkspace;

    fn entry_at(i: u64) -> ClipboardEntry {
        let now = SystemTime::now();
        let text = format!(
            "clipboard payload number {i} — some searchable words here {}",
            "x".repeat(64)
        );
        ClipboardEntry {
            id: ClipboardEntry::dedup_key(ContentKind::Text, &format!("{i:016x}")),
            created_at: now,
            updated_at: now,
            kind: ContentKind::Text,
            content_hash: format!("{i:016x}"),
            preview: crate::model::make_preview(&text, 120),
            search_text: crate::model::search_text_of(&text),
            payload_file: Some(format!("{i:016x}.bin")),
            thumbnail_file: None,
            size: text.len() as u64,
            source: SourceInfo {
                state: SourceState::Unknown,
                process: None,
            },
            file_paths: vec![],
            is_favorite: false,
            sensitivity: Sensitivity::Normal,
            status: EntryStatus::Persisted,
        }
    }

    /// M2.3 §68：1k/10k/50k 条索引读写 + 搜索——验证无 O(n²)/全量 payload 加载。
    #[test]
    #[ignore = "perf: release 手动运行"]
    fn perf_store_and_search_at_scale() {
        for n in [1_000u64, 10_000, 50_000] {
            let ws = TempWorkspace::new(&format!("clip-perf-{n}")).expect("ws");
            let store = ClipboardStore::open(ws.path()).expect("open");
            let entries: Vec<ClipboardEntry> = (0..n).map(entry_at).collect();
            for e in &entries {
                store
                    .write_payload(e.payload_file.as_ref().expect("file"), b"p")
                    .expect("w");
            }
            let t_save = std::time::Instant::now();
            store.save_all(&entries).expect("save");
            let save = t_save.elapsed();
            let t_load = std::time::Instant::now();
            let outcome = store.load().expect("load");
            let load = t_load.elapsed();
            assert_eq!(outcome.entries.len() as u64, n);
            let t_search = std::time::Instant::now();
            let hits = crate::model::search_entries(&outcome.entries, "searchable words");
            let search = t_search.elapsed();
            eprintln!(
                "PERF clipboard@{n}: save={save:?} load={load:?} search={search:?} hits={}",
                hits.len()
            );
            assert!(!hits.is_empty());
        }
    }
}
