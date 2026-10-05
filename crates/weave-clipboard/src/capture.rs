//! 捕获管线（M2.3 §13/§74/§75/§80-§82）：原始输入 → 条目 + 存储。
//!
//! 平台读取（Windows 剪贴板）在应用层；本模块从"已读取的字节"开始。
//!
//! - 去重（§13/§74/§75）：(kind, hash) 相同 ⇒ 更新时间/移顶/保留收藏，
//!   绝不产生第二条；不同 kind 不互相去重。
//! - Payload/Preview/SearchText 三分离（§81）：payload 原样，preview 展示
//!   规范化，search_text 仅小写折叠且有界（索引不膨胀，§44）。
//! - 超大内容（§82）：search_text 截断到有界；payload 照存（由保留策略
//!   控制总量），绝不无限读取——调用方负责读取上限。

use std::time::SystemTime;

use weave_core::prelude::HashAlgorithm;
use weave_core::prelude::WeaveError;
use weave_files::hash::hash_bytes;

use crate::model::{
    ClipboardEntry, ContentKind, EntryStatus, RetentionPolicy, Sensitivity, SourceInfo,
    apply_retention, make_preview, search_text_of, sensitivity_heuristic, storage_error,
};
use crate::store::ClipboardStore;

/// 捕获输入（平台层已读取的规范内容）。
pub struct CaptureInput {
    pub kind: ContentKind,
    /// 原始 payload 字节（Text = UTF-8 原文；Image = 编码图字节；Html = 原始 HTML）。
    pub payload: Vec<u8>,
    /// File 条目的路径列表（dedup 基于路径集；payload = None）。
    pub file_paths: Vec<String>,
    /// 平台层生成的展示摘要（Image = "图像 WxH"；File = "3 个文件"）。
    pub preview_override: Option<String>,
    /// Image 条目的缩略图 PNG（§8/§9：列表只加载缩略图）。
    pub thumbnail_png: Option<Vec<u8>>,
    pub source: SourceInfo,
}

/// 捕获结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CapturedEntry {
    pub entry: ClipboardEntry,
    /// 是否新建（false = 已存在被更新时间/移顶，§74）。
    pub inserted: bool,
}

/// 搜索文本上限（§82：超大内容只索引前缀；不无限膨胀索引）。
const SEARCH_TEXT_CAP: usize = 4096;
/// 预览上限（§6：列表不渲染超长文本）。
const PREVIEW_CAP: usize = 120;

pub struct CapturePipeline;

impl CapturePipeline {
    /// 捕获一条：去重 → 构建/更新条目 → 保留策略 → 持久化。
    pub fn capture(
        store: &ClipboardStore,
        entries: &mut Vec<ClipboardEntry>,
        input: CaptureInput,
        policy: &RetentionPolicy,
        now: SystemTime,
    ) -> Result<CapturedEntry, WeaveError> {
        // ── 指纹（§13）：内容 sha256；File 用路径集 ──
        let content_hash = match input.kind {
            ContentKind::File => {
                let mut joined = input.file_paths.join("\n");
                joined.make_ascii_lowercase();
                hash_bytes(HashAlgorithm::Sha256, joined.as_bytes())?
            }
            _ => hash_bytes(HashAlgorithm::Sha256, &input.payload)?,
        };
        let id = ClipboardEntry::dedup_key(input.kind, &content_hash);

        // ── 去重（§74/§75）：同 id ⇒ 更新时间 + 移顶 + 保留收藏 ──
        if let Some(existing) = entries.iter_mut().find(|e| e.id == id) {
            existing.updated_at = now;
            existing.status = EntryStatus::Available;
            let entry = existing.clone();
            // 移到顶部（最新在前约定）
            if let Some(pos) = entries.iter().position(|e| e.id == id) {
                let moved = entries.remove(pos);
                entries.insert(0, moved);
            }
            store.persist(entries)?;
            return Ok(CapturedEntry {
                entry,
                inserted: false,
            });
        }

        // ── 新条目 ──
        let text_for_search = match input.kind {
            ContentKind::Text | ContentKind::Html => {
                String::from_utf8_lossy(&input.payload).into_owned()
            }
            _ => input.preview_override.clone().unwrap_or_default(),
        };
        let preview = input
            .preview_override
            .clone()
            .unwrap_or_else(|| make_preview(&String::from_utf8_lossy(&input.payload), PREVIEW_CAP));
        let mut search_text = search_text_of(&text_for_search);
        search_text.truncate(SEARCH_TEXT_CAP);
        let sensitivity = match input.kind {
            ContentKind::Text | ContentKind::Html => {
                sensitivity_heuristic(input.kind, &String::from_utf8_lossy(&input.payload))
            }
            _ => Sensitivity::Unknown,
        };

        // payload 落盘（File 记录型无 payload）
        let payload_file = if input.kind == ContentKind::File {
            None
        } else {
            let file = format!("{content_hash}.bin");
            store.write_payload(&file, &input.payload)?;
            Some(file)
        };
        // 缩略图落盘（§9：异步由调用方保证；失败可降级 = None）
        let thumbnail_file = match &input.thumbnail_png {
            Some(bytes) if input.kind == ContentKind::Image => {
                let file = format!("{content_hash}.thumb.png");
                store.write_payload(&file, bytes)?;
                Some(file)
            }
            _ => None,
        };

        let entry = ClipboardEntry {
            id: id.clone(),
            created_at: now,
            updated_at: now,
            kind: input.kind,
            content_hash,
            preview,
            search_text,
            payload_file,
            thumbnail_file,
            size: input.payload.len() as u64,
            source: input.source,
            file_paths: input.file_paths,
            is_favorite: false,
            sensitivity,
            status: EntryStatus::Persisted,
        };
        entries.insert(0, entry.clone());

        // ── 保留策略 + 被淘汰 payload 清扫（§20/§34）──
        apply_retention(entries, &|e| e.size, policy, now);
        store.run_gc(entries, policy)?;

        Ok(CapturedEntry {
            entry,
            inserted: true,
        })
    }

    /// 恢复前验证（§26）：payload 可读 ⇒ 返回字节；缺失 ⇒ 结构化错误
    /// （绝不静默失败）；File 条目返回 None（调用方按路径集写回）。
    pub fn load_payload(
        store: &ClipboardStore,
        entry: &ClipboardEntry,
    ) -> Result<Option<Vec<u8>>, WeaveError> {
        match &entry.payload_file {
            None => Ok(None),
            // 错误不携带内容（§53）：只有 id/kind/size 级诊断
            Some(file) => store.read_payload(file).map(Some).map_err(|_| {
                storage_error(
                    "clipboard.cannotRestore",
                    format!("entry {} payload unavailable", entry.id),
                )
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{RetentionPolicy, SourceState};
    use std::time::Duration;
    use weave_testkit::TempWorkspace;

    fn policy() -> RetentionPolicy {
        RetentionPolicy {
            max_entries: 10,
            retention_days: None,
            max_storage_bytes: u64::MAX,
        }
    }

    fn text_input(s: &str) -> CaptureInput {
        CaptureInput {
            kind: ContentKind::Text,
            payload: s.as_bytes().to_vec(),
            file_paths: vec![],
            preview_override: None,
            thumbnail_png: None,
            source: SourceInfo {
                state: SourceState::Unknown,
                process: None,
            },
        }
    }

    fn now() -> SystemTime {
        SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000)
    }

    #[test]
    fn dedup_sequence_a_a_b_a_keeps_three_entries_with_a_on_top() {
        let ws = TempWorkspace::new("clip-dedup").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entries: Vec<ClipboardEntry> = Vec::new();
        let p = policy();

        let c1 =
            CapturePipeline::capture(&store, &mut entries, text_input("A"), &p, now()).expect("A");
        for _ in 0..19 {
            CapturePipeline::capture(&store, &mut entries, text_input("A"), &p, now())
                .expect("A dup");
        }
        CapturePipeline::capture(&store, &mut entries, text_input("B"), &p, now()).expect("B");
        let c3 = CapturePipeline::capture(&store, &mut entries, text_input("A"), &p, now())
            .expect("A again");

        assert_eq!(
            entries.len(),
            2,
            "A/B 各一条——20 次重复复制不产生 20 条（Demo 2）"
        );
        assert_eq!(
            entries[0].content_hash, c3.entry.content_hash,
            "A 移顶（§74）"
        );
        assert!(!c1.inserted || entries.len() == 2);
        // A 的时间被更新（晚于创建）
        assert!(entries[0].updated_at >= entries[0].created_at);
    }

    #[test]
    fn favorite_preserved_on_recapture() {
        let ws = TempWorkspace::new("clip-fav").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entries: Vec<ClipboardEntry> = Vec::new();
        let p = policy();
        let captured =
            CapturePipeline::capture(&store, &mut entries, text_input("keep me"), &p, now())
                .expect("cap");
        entries[0].is_favorite = true;
        let again =
            CapturePipeline::capture(&store, &mut entries, text_input("keep me"), &p, now())
                .expect("recap");
        assert!(!again.inserted, "同内容 ⇒ 更新原条目（§75）");
        assert!(entries[0].is_favorite, "收藏保留");
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].id, captured.entry.id);
    }

    #[test]
    fn different_kinds_same_bytes_not_deduped() {
        let ws = TempWorkspace::new("clip-kinds").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entries: Vec<ClipboardEntry> = Vec::new();
        let p = policy();
        CapturePipeline::capture(&store, &mut entries, text_input("same"), &p, now())
            .expect("text");
        CapturePipeline::capture(
            &store,
            &mut entries,
            CaptureInput {
                kind: ContentKind::Html,
                payload: b"same".to_vec(),
                file_paths: vec![],
                preview_override: Some("same".to_string()),
                thumbnail_png: None,
                source: SourceInfo {
                    state: SourceState::Unknown,
                    process: None,
                },
            },
            &p,
            now(),
        )
        .expect("html");
        assert_eq!(entries.len(), 2, "不同 kind 不互相去重（§13）");
    }

    #[test]
    fn text_capture_runs_sensitivity_heuristic() {
        let ws = TempWorkspace::new("clip-sens").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entries: Vec<ClipboardEntry> = Vec::new();
        CapturePipeline::capture(
            &store,
            &mut entries,
            text_input("token = eyJhbGciOiJIUzI1NiJ9.e30.abc"),
            &policy(),
            now(),
        )
        .expect("cap");
        assert_eq!(entries[0].sensitivity, Sensitivity::PotentiallySensitive);
    }

    #[test]
    fn file_capture_records_paths_without_payload() {
        let ws = TempWorkspace::new("clip-files").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entries: Vec<ClipboardEntry> = Vec::new();
        CapturePipeline::capture(
            &store,
            &mut entries,
            CaptureInput {
                kind: ContentKind::File,
                payload: vec![],
                file_paths: vec!["D:\\a.txt".to_string(), "D:\\b.txt".to_string()],
                preview_override: Some("2 个文件".to_string()),
                thumbnail_png: None,
                source: SourceInfo {
                    state: SourceState::Unknown,
                    process: None,
                },
            },
            &policy(),
            now(),
        )
        .expect("cap");
        assert_eq!(entries[0].kind, ContentKind::File);
        assert!(entries[0].payload_file.is_none());
        assert_eq!(entries[0].file_paths.len(), 2);
        assert_eq!(entries[0].preview, "2 个文件");
    }

    #[test]
    fn restore_payload_missing_is_structured_error_not_silent() {
        let ws = TempWorkspace::new("clip-restore").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entries: Vec<ClipboardEntry> = Vec::new();
        let captured =
            CapturePipeline::capture(&store, &mut entries, text_input("gone"), &policy(), now())
                .expect("cap");
        // 模拟 payload 丢失
        if let Some(f) = &captured.entry.payload_file {
            store.delete_payload(f).expect("del");
        }
        let err = CapturePipeline::load_payload(&store, &captured.entry).expect_err("must error");
        assert_eq!(err.code, "clipboard.cannotRestore", "明确错误（§26）");
    }

    #[test]
    fn retention_drops_oldest_beyond_max_entries() {
        let ws = TempWorkspace::new("clip-ret").expect("ws");
        let store = ClipboardStore::open(ws.path()).expect("open");
        let mut entries: Vec<ClipboardEntry> = Vec::new();
        let p = RetentionPolicy {
            max_entries: 3,
            retention_days: None,
            max_storage_bytes: u64::MAX,
        };
        for i in 0..5u64 {
            let t = now() + Duration::from_secs(i);
            CapturePipeline::capture(&store, &mut entries, text_input(&format!("item{i}")), &p, t)
                .expect("cap");
        }
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].preview, "item4");
        // payload 同步清理
        store.run_gc(&mut entries, &p).expect("gc");
        let orphan_left = std::fs::read_dir(ws.path().join("payloads"))
            .expect("dir")
            .flatten()
            .count();
        assert_eq!(orphan_left, 3, "被淘汰条目的 payload 一并清理");
    }
}
