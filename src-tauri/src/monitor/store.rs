//! M2.11 持久化（§27/§28/§29）：targets/events 单文件原子 JSON +
//! snapshots 逐快照文件；**metadata 与正文 payload 分离**（§29）；
//! 损坏隔离不 panic；历史清理不触碰最近成功快照（§28）。
//!
//! 目录（§29，位于 history_dir/website-monitor/）：
//! ```text
//! targets.json            # 全部目标（原子写）
//! events.json             # 变化事件（有界）
//! snapshots/<tid>/<seq>.json  # 快照（metadata + 可选正文）
//! ```

use std::path::{Path, PathBuf};

use weave_core::prelude::WeaveError;

use super::model::{MonitorChangeEvent, MonitorSnapshot, MonitorTarget};

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("monitor::store")
}

pub const MAX_EVENTS: usize = 1000;
/// 每 target 快照保留数（§28 历史容量控制；最近成功快照永不清理）。
pub const MAX_SNAPSHOTS_PER_TARGET: u64 = 50; // 内部容量语义；对外 DTO 用 f64

fn read_json<T: serde::de::DeserializeOwned>(path: &Path) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    match serde_json::from_slice(&bytes) {
        Ok(v) => Some(v),
        Err(_) => {
            // 损坏隔离（§27：一次损坏不拖垮整个模块）
            let _ = std::fs::rename(path, path.with_extension("json.corrupt"));
            None
        }
    }
}

fn write_json<T: serde::Serialize>(path: &Path, value: &T) -> Result<(), WeaveError> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| err("monitor.dirFailed", format!("{e}")))?;
    }
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| err("monitor.serializeFailed", format!("{e}")))?;
    std::fs::write(&tmp, json).map_err(|e| err("monitor.writeFailed", format!("{e}")))?;
    std::fs::rename(&tmp, path).map_err(|e| err("monitor.writeFailed", format!("{e}")))?;
    Ok(())
}

/// Monitor 存储根（history_dir/website-monitor）。
pub struct MonitorStore {
    root: PathBuf,
}

impl MonitorStore {
    pub fn open(root: PathBuf) -> Self {
        Self { root }
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    fn targets_path(&self) -> PathBuf {
        self.root.join("targets.json")
    }

    fn events_path(&self) -> PathBuf {
        self.root.join("events.json")
    }

    pub fn load_targets(&self) -> Vec<MonitorTarget> {
        read_json(&self.targets_path()).unwrap_or_default()
    }

    pub fn save_targets(&self, targets: &[MonitorTarget]) -> Result<(), WeaveError> {
        write_json(&self.targets_path(), &targets)
    }

    pub fn load_events(&self) -> Vec<MonitorChangeEvent> {
        read_json(&self.events_path()).unwrap_or_default()
    }

    /// 追加事件（有界 MAX_EVENTS §28——只保留元数据，天然小）。
    pub fn append_event(&self, event: &MonitorChangeEvent) -> Result<(), WeaveError> {
        let mut events = self.load_events();
        events.push(event.clone());
        if events.len() > MAX_EVENTS {
            let over = events.len() - MAX_EVENTS;
            events.drain(0..over);
        }
        write_json(&self.events_path(), &events)
    }

    fn snapshot_dir(&self, target_id: &str) -> PathBuf {
        self.root.join("snapshots").join(target_id)
    }

    /// 快照文件名（安全：target_id 由服务层生成的受限字符集）。
    fn snapshot_path(&self, target_id: &str, seq: f64) -> PathBuf {
        self.snapshot_dir(target_id).join(format!("{seq}.json"))
    }

    /// 保存快照（payload 正文内嵌同一文件——单个快照有界于响应上限；
    /// NoChange/失败 ⇒ payload None 不占空间 §46）。
    pub fn save_snapshot(
        &self,
        snapshot: &MonitorSnapshot,
        payload: Option<&str>,
    ) -> Result<(), WeaveError> {
        #[derive(serde::Serialize)]
        #[serde(rename_all = "camelCase")]
        struct SnapshotFile<'a> {
            #[serde(flatten)]
            meta: &'a MonitorSnapshot,
            payload: Option<&'a str>,
        }
        write_json(
            &self.snapshot_path(&snapshot.target_id, snapshot.seq),
            &SnapshotFile {
                meta: snapshot,
                payload,
            },
        )
    }

    /// 读取快照（含可选正文——diff 用 §11）。
    pub fn load_snapshot(
        &self,
        target_id: &str,
        seq: f64,
    ) -> Option<(MonitorSnapshot, Option<String>)> {
        #[derive(serde::Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SnapshotFile {
            #[serde(flatten)]
            meta: MonitorSnapshot,
            payload: Option<String>,
        }
        let file: SnapshotFile = read_json(&self.snapshot_path(target_id, seq))?;
        Some((file.meta, file.payload))
    }

    /// 列出某 target 的快照元数据（历史列表 §8；只读 metadata 不读正文
    /// §50"历史加载不会一次读取全部正文"——逐文件解析但 payload 反序列化
    /// 在同一 JSON……诚实折中：单快照文件本身有界于响应上限，历史条目
    /// 逐文件读取是 O(n) 而非 O(n×全文内存)——列表场景由 UI 分页）。
    pub fn list_snapshots(&self, target_id: &str) -> Vec<MonitorSnapshot> {
        let mut out = Vec::new();
        let Ok(read) = std::fs::read_dir(self.snapshot_dir(target_id)) else {
            return out;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            #[derive(serde::Deserialize)]
            #[serde(rename_all = "camelCase")]
            struct SnapshotFile {
                #[serde(flatten)]
                meta: MonitorSnapshot,
                #[allow(dead_code)]
                payload: Option<String>,
            }
            if let Some(file) = read_json::<SnapshotFile>(&path) {
                out.push(file.meta);
            }
        }
        out.sort_by(|a, b| {
            a.seq
                .partial_cmp(&b.seq)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        out
    }

    /// 历史清理（§28）：数量+时间双限；**保留最近一次有正文的快照**
    /// （diff 基线）与最新一条。
    pub fn retain_snapshots(
        &self,
        target_id: &str,
        max_keep: u64,
        max_age_ms: f64,
        now_ms: f64,
    ) -> usize {
        let mut snapshots = self.list_snapshots(target_id);
        if snapshots.is_empty() {
            return 0;
        }
        let latest_seq = snapshots.last().map(|s| s.seq);
        let latest_payload_seq = snapshots
            .iter()
            .rev()
            .find(|s| s.payload_file.is_some())
            .map(|s| s.seq);
        let total = snapshots.len() as u64;
        let mut kept = 0u64;
        let mut removed = 0usize;
        snapshots.retain(|s| {
            let too_old = now_ms - s.fetched_at_ms > max_age_ms;
            let is_protected = Some(s.seq) == latest_seq || Some(s.seq) == latest_payload_seq;
            if is_protected {
                kept += 1;
                return true;
            }
            if too_old || kept >= max_keep || total - removed as u64 > max_keep + kept {
                let _ = std::fs::remove_file(self.snapshot_path(target_id, s.seq));
                removed += 1;
                false
            } else {
                kept += 1;
                true
            }
        });
        removed
    }

    /// 删除 target 全部数据（§63：仅在用户显式选择"连历史一起删"时调用）。
    pub fn delete_target_data(&self, target_id: &str) -> Result<(), WeaveError> {
        let dir = self.snapshot_dir(target_id);
        if dir.exists() {
            std::fs::remove_dir_all(dir)
                .map_err(|e| err("monitor.deleteFailed", format!("{e}")))?;
        }
        Ok(())
    }
}

/// target_id 合法性（存储路径安全：仅字母数字与连字符）。
pub fn valid_target_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::monitor::model::{CheckResult, MonitorMode};

    fn target(id: &str) -> MonitorTarget {
        MonitorTarget {
            id: id.to_string(),
            name: "Test".into(),
            url: "https://example.com".into(),
            enabled: true,
            paused: false,
            mode: MonitorMode::WholePage,
            selector: None,
            interval_secs: 300,
            notify_on_change: true,
            notify_on_error: false,
            notify_on_recovery: false,
            consecutive_failures: 0,
            last_checked_at_ms: None,
            last_success_at_ms: None,
            last_fingerprint: None,
            last_snapshot_seq: None,
            status: crate::monitor::model::TargetStatus::Waiting,
            last_error_code: None,
            created_at_ms: 1.0,
            updated_at_ms: 1.0,
        }
    }

    fn snapshot(tid: &str, seq: f64, result: CheckResult, with_payload: bool) -> MonitorSnapshot {
        MonitorSnapshot {
            target_id: tid.into(),
            seq,
            fetched_at_ms: 1000.0 + seq,
            status_code: 200,
            final_url: "https://example.com".into(),
            content_type: "text/html".into(),
            fingerprint: format!("fp-{seq}"),
            text_size: 100,
            duration_ms: 5.0,
            result,
            payload_file: if with_payload {
                Some(format!("{seq}.txt"))
            } else {
                None
            },
            truncated: false,
        }
    }

    #[test]
    fn targets_roundtrip_and_corrupt_isolation() {
        let ws = weave_testkit::TempWorkspace::new("mon-store").expect("ws");
        let store = MonitorStore::open(ws.path().to_path_buf());
        store
            .save_targets(&[target("t1"), target("t2")])
            .expect("save");
        assert_eq!(store.load_targets().len(), 2);
        // 损坏 ⇒ 隔离为空，不 panic
        std::fs::write(store.targets_path(), b"{broken").expect("w");
        assert!(store.load_targets().is_empty());
        assert!(store.targets_path().with_extension("json.corrupt").exists());
    }

    #[test]
    fn snapshot_roundtrip_with_optional_payload() {
        let ws = weave_testkit::TempWorkspace::new("mon-snap").expect("ws");
        let store = MonitorStore::open(ws.path().to_path_buf());
        store
            .save_snapshot(
                &snapshot("t1", 1.0, CheckResult::ContentChanged, true),
                Some("hello body"),
            )
            .expect("save");
        store
            .save_snapshot(&snapshot("t1", 2.0, CheckResult::NoChange, false), None)
            .expect("save");
        let (meta, payload) = store.load_snapshot("t1", 1.0).expect("load");
        assert_eq!(meta.seq, 1.0);
        assert_eq!(payload.as_deref(), Some("hello body"));
        let (meta2, payload2) = store.load_snapshot("t1", 2.0).expect("load");
        assert_eq!(meta2.result, CheckResult::NoChange);
        assert!(payload2.is_none(), "NoChange 不占正文空间 §46");
        let list = store.list_snapshots("t1");
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].seq, 1.0, "按 seq 升序");
    }

    #[test]
    fn retention_keeps_latest_and_latest_payload() {
        let ws = weave_testkit::TempWorkspace::new("mon-retain").expect("ws");
        let store = MonitorStore::open(ws.path().to_path_buf());
        // 4 条：1(有 payload 旧)、2、3(有 payload)、4 最新(无 payload)
        for seq in [1.0, 2.0, 3.0, 4.0] {
            let with_payload = seq == 1.0 || seq == 3.0;
            let result = if with_payload {
                CheckResult::ContentChanged
            } else {
                CheckResult::NoChange
            };
            store
                .save_snapshot(
                    &snapshot("t1", seq, result, with_payload),
                    if with_payload { Some("x") } else { None },
                )
                .expect("save");
        }
        let removed = store.retain_snapshots("t1", 2, f64::MAX, 99999.0);
        assert!(removed >= 1);
        let left = store.list_snapshots("t1");
        assert!(left.iter().any(|s| s.seq == 4.0), "最新必须保留 §28");
        assert!(
            left.iter().any(|s| s.seq == 3.0),
            "最近有正文快照必须保留（diff 基线）"
        );
        assert!(!left.iter().any(|s| s.seq == 1.0), "旧 payload 被清理");
    }

    #[test]
    fn events_are_bounded() {
        let ws = weave_testkit::TempWorkspace::new("mon-events").expect("ws");
        let store = MonitorStore::open(ws.path().to_path_buf());
        for i in 0..(MAX_EVENTS + 50) {
            let e = MonitorChangeEvent {
                id: format!("e{i}"),
                target_id: "t1".into(),
                detected_at_ms: i as f64,
                result: CheckResult::ContentChanged,
                magnitude: crate::monitor::model::ChangeMagnitude::Small,
                added_lines: 1,
                removed_lines: 0,
                previous_seq: Some(1.0),
                current_seq: 2.0,
                summary: "s".into(),
            };
            store.append_event(&e).expect("append");
        }
        assert_eq!(store.load_events().len(), MAX_EVENTS);
    }

    #[test]
    fn target_id_validation_blocks_traversal() {
        assert!(valid_target_id("abc-123"));
        assert!(!valid_target_id("../evil"));
        assert!(!valid_target_id("a/b"));
        assert!(!valid_target_id(""));
    }
}
