//! M2.3：剪贴板捕获/恢复服务（平台适配层——M3 §23 隔离原则）。
//!
//! - **监听**（§14）：workspace `unsafe_code = forbid` ⇒ 不手写 Win32
//!   listener；采用提示词允许的 fallback——低频轮询（1s，文本为主；
//!   图像探测 5s 节流），全程可暂停/取消，进程退出即停（无服务残留 §76）。
//! - **自循环抑制**（§15）：Weave 写回前登记 expected fingerprint，
//!   捕获循环命中即跳过一次；叠加内容去重双保险（不依赖 text==lastText）。
//! - **读取**（§48）：任何一次读取失败 ⇒ 记录后继续监听，绝不崩溃。
//! - **隐私**（§52/§53）：日志/审计只进 id/kind/size，绝不进内容。
//! - **历史删除 ≠ 文件删除**（§28/§65）：本服务不提供任何触碰原文件的路径。

#![expect(clippy::result_large_err)] // D10

use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_clipboard::model::{
    ClipboardEntry, ContentKind, EntryStatus, RetentionPolicy, Sensitivity, SourceInfo,
    SourceState, search_entries,
};
use weave_clipboard::{CaptureInput, CapturePipeline, ClipboardStore};
use weave_core::prelude::WeaveError;

use crate::commands::IpcError;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("clipboard_service")
        .into()
}

/// 轮询间隔（§14 fallback：事件驱动需 unsafe FFI，安全封装不可得 ⇒ 低频轮询）。
const POLL_INTERVAL_MS: u64 = 1000;
/// 图像探测节流（每 N tick 一次——DIB 解码开销控制，§9）。
const IMAGE_PROBE_EVERY_TICKS: u64 = 5;

/// 运行时共享状态（AppState 持有）。
pub struct ClipboardRuntime {
    pub entries: Mutex<Vec<ClipboardEntry>>,
    pub store: ClipboardStore,
    pub paused: std::sync::Arc<AtomicBool>,
    pub enabled: std::sync::Arc<AtomicBool>,
    pub capture_text: AtomicBool,
    pub capture_images: AtomicBool,
    pub capture_files: AtomicBool,
    /// Weave 写回后的预期指纹（§15 自循环抑制；命中即清）。
    pub suppress: Mutex<Option<String>>,
    /// 最近一次捕获的文本指纹（快速跳过未变化内容）。
    pub last_text_hash: Mutex<Option<String>>,
    tick: AtomicU64,
    pub policy: Mutex<RetentionPolicy>,
}

impl ClipboardRuntime {
    pub fn open(
        dir: &std::path::Path,
        config: &crate::config::ClipboardConfig,
    ) -> Result<Self, WeaveError> {
        let store = ClipboardStore::open(dir)?;
        let outcome = store.load()?;
        let policy = RetentionPolicy {
            max_entries: config.max_entries as usize,
            retention_days: Some(config.retention_days as u64),
            max_storage_bytes: 200 * 1024 * 1024,
        };
        Ok(Self {
            entries: Mutex::new(outcome.entries),
            store,
            paused: std::sync::Arc::new(AtomicBool::new(false)),
            enabled: std::sync::Arc::new(AtomicBool::new(config.enabled)),
            capture_text: AtomicBool::new(config.capture_text),
            capture_images: AtomicBool::new(config.capture_images),
            capture_files: AtomicBool::new(config.capture_files),
            suppress: Mutex::new(None),
            last_text_hash: Mutex::new(None),
            tick: AtomicU64::new(0),
            policy: Mutex::new(policy),
        })
    }

    /// 共享捕获标志给 Agent ClipboardComponent（§27：单一主人，标志位同源）。
    pub fn paused_flag_clone(&self) -> std::sync::Arc<AtomicBool> {
        std::sync::Arc::clone(&self.paused)
    }

    pub fn enabled_flag_clone(&self) -> std::sync::Arc<AtomicBool> {
        std::sync::Arc::clone(&self.enabled)
    }

    pub fn apply_config(&self, config: &crate::config::ClipboardConfig) {
        self.enabled.store(config.enabled, Ordering::SeqCst);
        self.capture_text
            .store(config.capture_text, Ordering::SeqCst);
        self.capture_images
            .store(config.capture_images, Ordering::SeqCst);
        self.capture_files
            .store(config.capture_files, Ordering::SeqCst);
        *self.policy.lock().expect("policy") = RetentionPolicy {
            max_entries: config.max_entries as usize,
            retention_days: Some(config.retention_days as u64),
            max_storage_bytes: 200 * 1024 * 1024,
        };
        if !config.enabled {
            self.paused.store(true, Ordering::SeqCst);
        } else {
            self.paused.store(false, Ordering::SeqCst);
        }
    }
}

fn hash_hex(bytes: &[u8]) -> Option<String> {
    weave_files::hash::hash_bytes(weave_core::prelude::HashAlgorithm::Sha256, bytes).ok()
}

/// 监听线程（§14 fallback 轮询；§37 生命周期：App 启动开、进程退即停）。
pub fn start_monitor(app: tauri::AppHandle) {
    std::thread::Builder::new()
        .name("clipboard-monitor".to_string())
        .spawn(move || monitor_loop(app))
        .expect("spawn clipboard monitor");
}

fn monitor_loop(app: tauri::AppHandle) {
    loop {
        std::thread::sleep(Duration::from_millis(POLL_INTERVAL_MS));
        let state = app.state::<crate::state::AppState>();
        let rt = &state.clipboard;
        if !rt.enabled.load(Ordering::SeqCst) || rt.paused.load(Ordering::SeqCst) {
            continue;
        }
        let tick = rt.tick.fetch_add(1, Ordering::SeqCst);
        // 读取失败 ⇒ 记录后继续（§48：局部失败不升级）
        if let Ok(mut clip) = arboard::Clipboard::new() {
            // ── 文件优先（§79 格式优先级：File > Image > Text）──
            if rt.capture_files.load(Ordering::SeqCst)
                && let Some(files) = read_file_list()
                && !files.is_empty()
            {
                let mut paths = files.clone();
                paths.sort_by_key(|p| p.to_lowercase());
                let fingerprint = format!("file:{}", paths.join("|").to_lowercase());
                if maybe_suppressed(rt, &fingerprint) {
                    continue;
                }
                let preview = format!("{} 个文件", files.len());
                let input = CaptureInput {
                    kind: ContentKind::File,
                    payload: Vec::new(),
                    file_paths: files,
                    preview_override: Some(preview),
                    thumbnail_png: None,
                    source: SourceInfo {
                        state: SourceState::Unknown,
                        process: None,
                    },
                };
                capture_once(&app, input, &fingerprint);
                continue;
            }
            // ── 文本 ──
            if let Ok(text) = clip.get_text()
                && !text.is_empty()
                && rt.capture_text.load(Ordering::SeqCst)
                && let Some(hash) = hash_hex(text.as_bytes())
            {
                if maybe_suppressed(rt, &hash) {
                    continue;
                }
                let unchanged = *rt.last_text_hash.lock().expect("last") == Some(hash.clone());
                if !unchanged {
                    rt.last_text_hash
                        .lock()
                        .expect("last")
                        .replace(hash.clone());
                    let sensitivity =
                        weave_clipboard::sensitivity_heuristic(ContentKind::Text, &text);
                    let _ = sensitivity;
                    let input = CaptureInput {
                        kind: ContentKind::Text,
                        payload: text.into_bytes(),
                        file_paths: vec![],
                        preview_override: None,
                        thumbnail_png: None,
                        source: SourceInfo {
                            state: SourceState::Unknown,
                            process: None,
                        },
                    };
                    capture_once(&app, input, &hash);
                }
                continue;
            }
            // ── 图像（节流探测；§8/§9：仅元数据+缩略图+原图按保留策略）──
            if rt.capture_images.load(Ordering::SeqCst)
                && tick % IMAGE_PROBE_EVERY_TICKS == 0
                && let Ok(image) = clip.get_image()
            {
                let rgba = image::RgbaImage::from_raw(
                    image.width as u32,
                    image.height as u32,
                    image.bytes.into_owned(),
                );
                if let Some(rgba) = rgba {
                    let dynamic = image::DynamicImage::ImageRgba8(rgba);
                    let (w, h) = (dynamic.width(), dynamic.height());
                    // 规范 PNG 作为指纹与原图（§13 内容指纹）
                    let encoded = weave_media::ops::encode_image(
                        &dynamic,
                        weave_media::ImageFormat::Png,
                        None,
                    );
                    if let Ok(png) = encoded
                        && let Some(hash) = hash_hex(&png)
                    {
                        if maybe_suppressed(rt, &hash) {
                            continue;
                        }
                        // 缩略图（§9：限尺寸；失败 ⇒ 降级 None）
                        let thumb = weave_media::ops::resize_image(
                            &dynamic,
                            &weave_media::ResizeOptions {
                                mode: weave_media::FitMode::Fit,
                                width: 320,
                                height: 320,
                                ..weave_media::ResizeOptions::default()
                            },
                        );
                        let thumbnail_png = weave_media::ops::encode_image(
                            &thumb,
                            weave_media::ImageFormat::Png,
                            None,
                        )
                        .ok();
                        let input = CaptureInput {
                            kind: ContentKind::Image,
                            payload: png,
                            file_paths: vec![],
                            preview_override: Some(format!("图像 {w}×{h}")),
                            thumbnail_png,
                            source: SourceInfo {
                                state: SourceState::Unknown,
                                process: None,
                            },
                        };
                        capture_once(&app, input, &hash);
                    }
                }
            }
        }
    }
}

fn maybe_suppressed(rt: &ClipboardRuntime, fingerprint: &str) -> bool {
    let mut suppress = rt.suppress.lock().expect("suppress");
    match suppress.as_ref() {
        Some(expected) if expected == fingerprint => {
            // 命中：消费抑制（仅跳过一次），不产生历史
            *suppress = None;
            true
        }
        _ => false,
    }
}

fn capture_once(app: &tauri::AppHandle, input: CaptureInput, fingerprint: &str) {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.clipboard;
    let policy = rt.policy.lock().expect("policy").clone();
    let mut entries = rt.entries.lock().expect("entries");
    let input = CaptureInput {
        source: SourceInfo {
            state: SourceState::Unknown,
            process: None,
        },
        ..input
    };
    // 捕获失败 ⇒ 结构化日志（无内容 §53）后继续监听
    if let Err(e) =
        CapturePipeline::capture(&rt.store, &mut entries, input, &policy, SystemTime::now())
    {
        tracing::warn!(code = %e.code, "clipboard capture failed");
        return;
    }
    let _ = fingerprint;
    tracing::debug!("clipboard entry captured");
}

// ── 平台读取（安全 crate 封装；§48 读取失败 ⇒ None） ──

/// CF_HDROP 文件列表读取（clipboard-win 安全封装；失败 ⇒ None 继续监听 §48）。
#[cfg(windows)]
fn read_file_list() -> Option<Vec<String>> {
    let _clip = clipboard_win::Clipboard::new().ok()?;
    clipboard_win::get(clipboard_win::formats::FileList).ok()
}

#[cfg(not(windows))]
fn read_file_list() -> Option<Vec<String>> {
    None
}

/// CF_HDROP 写回（§26：文件历史重新复制为文件剪贴板）。
#[cfg(windows)]
fn write_file_list(paths: &[String]) -> Result<(), IpcError> {
    let _clip = clipboard_win::Clipboard::new()
        .map_err(|e| err("clipboard.writeFailed", format!("open failed: {e}")))?;
    use clipboard_win::Setter;
    clipboard_win::formats::FileList
        .write_clipboard(paths)
        .map_err(|e| {
            err(
                "clipboard.writeFailed",
                format!("file list write failed: {e}"),
            )
        })
}

#[cfg(not(windows))]
fn write_file_list(_paths: &[String]) -> Result<(), IpcError> {
    Err(err(
        "clipboard.unsupported",
        "file clipboard requires windows",
    ))
}

// ── IPC DTO 与命令 ──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardEntryDto {
    pub id: String,
    pub created_at_ms: Option<f64>,
    pub updated_at_ms: Option<f64>,
    pub kind: String,
    pub preview: String,
    pub size: f64,
    pub is_favorite: bool,
    pub sensitivity: String,
    pub status: String,
    pub file_count: f64,
}

fn kind_str(k: ContentKind) -> &'static str {
    match k {
        ContentKind::Text => "text",
        ContentKind::Image => "image",
        ContentKind::File => "file",
        ContentKind::Html => "html",
        ContentKind::Unknown => "unknown",
    }
}

impl ClipboardEntryDto {
    fn from_entry(e: &ClipboardEntry) -> Self {
        let to_ms = |t: SystemTime| {
            t.duration_since(std::time::UNIX_EPOCH)
                .ok()
                .map(|d| d.as_millis() as f64)
        };
        Self {
            id: e.id.clone(),
            created_at_ms: to_ms(e.created_at),
            updated_at_ms: to_ms(e.updated_at),
            kind: kind_str(e.kind).to_string(),
            preview: e.preview.clone(),
            size: e.size as f64,
            is_favorite: e.is_favorite,
            sensitivity: match e.sensitivity {
                Sensitivity::Normal => "normal".to_string(),
                Sensitivity::PotentiallySensitive => "potentiallySensitive".to_string(),
                Sensitivity::Unknown => "unknown".to_string(),
            },
            status: match e.status {
                EntryStatus::Available => "available".to_string(),
                EntryStatus::Missing => "missing".to_string(),
                EntryStatus::Corrupted => "corrupted".to_string(),
                other => format!("{other:?}").to_lowercase(),
            },
            file_count: e.file_paths.len() as f64,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardListRequestDto {
    /// all | text | image | file | favorite | sensitive
    pub filter: String,
    pub query: String,
    pub offset: f64,
    pub limit: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardListResponseDto {
    pub total: f64,
    pub items: Vec<ClipboardEntryDto>,
    pub paused: bool,
}

fn filter_kind(filter: &str) -> Option<ContentKind> {
    match filter {
        "text" => Some(ContentKind::Text),
        "image" => Some(ContentKind::Image),
        "file" => Some(ContentKind::File),
        "html" => Some(ContentKind::Html),
        _ => None,
    }
}

/// 历史列表（§44/§45：只动元数据/preview，不读 payload；分页）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_list(
    app: tauri::AppHandle,
    request: ClipboardListRequestDto,
) -> Result<ClipboardListResponseDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let entries = state.clipboard.entries.lock().expect("entries");
    let filtered: Vec<ClipboardEntry> = entries
        .iter()
        .filter(|e| match filter_kind(&request.filter) {
            Some(kind) => e.kind == kind,
            None => match request.filter.as_str() {
                "favorite" => e.is_favorite,
                "sensitive" => e.sensitivity == Sensitivity::PotentiallySensitive,
                _ => true,
            },
        })
        .cloned()
        .collect();
    let hits = search_entries(&filtered, &request.query);
    let total = hits.len() as f64;
    let offset = request.offset.max(0.0) as usize;
    let limit = request.limit.clamp(1.0, 500.0) as usize;
    let items = hits
        .into_iter()
        .skip(offset)
        .take(limit)
        .map(ClipboardEntryDto::from_entry)
        .collect();
    Ok(ClipboardListResponseDto {
        total,
        items,
        paused: state.clipboard.paused.load(Ordering::SeqCst),
    })
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardDetailDto {
    pub entry: ClipboardEntryDto,
    pub content_hash: String,
    pub file_paths: Vec<String>,
    /// File 条目：每个路径的当前状态（§10：available/missing/inaccessible）。
    pub file_states: Vec<(String, String)>,
}

/// 详情（§25：Content/时间/类型/大小/状态/hash；payload 不在列表加载）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_detail(app: tauri::AppHandle, id: String) -> Result<ClipboardDetailDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut entries = state.clipboard.entries.lock().expect("entries");
    let Some(index) = entries.iter().position(|e| e.id == id) else {
        return Err(err("clipboard.unknownEntry", format!("no entry '{id}'")));
    };
    state.clipboard.store.refresh_status(&mut entries[index]);
    let entry = &entries[index];
    let file_states = entry
        .file_paths
        .iter()
        .map(|p| {
            let st = std::fs::metadata(p);
            let status = match st {
                Ok(_) => "available",
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => "missing",
                Err(_) => "inaccessible",
            };
            (p.clone(), status.to_string())
        })
        .collect();
    Ok(ClipboardDetailDto {
        entry: ClipboardEntryDto::from_entry(entry),
        content_hash: entry.content_hash.clone(),
        file_paths: entry.file_paths.clone(),
        file_states,
    })
}

/// 重新复制（§26 P0）：读 payload → 写系统剪贴板 → 抑制自循环 → 提升条目。
#[tauri::command]
#[specta::specta]
pub fn clipboard_recopy(app: tauri::AppHandle, id: String) -> Result<(), IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.clipboard;
    let mut entries = rt.entries.lock().expect("entries");
    let Some(index) = entries.iter().position(|e| e.id == id) else {
        return Err(err("clipboard.unknownEntry", format!("no entry '{id}'")));
    };
    rt.store.refresh_status(&mut entries[index]);
    let entry = entries[index].clone();
    if entry.status == EntryStatus::Missing || entry.status == EntryStatus::Corrupted {
        return Err(err(
            "clipboard.cannotRestore",
            "payload no longer available",
        ));
    }
    let mut clip = arboard::Clipboard::new().map_err(|e| {
        err(
            "clipboard.writeFailed",
            format!("open clipboard failed: {e}"),
        )
    })?;
    match entry.kind {
        ContentKind::Text | ContentKind::Html => {
            let bytes = CapturePipeline::load_payload(&rt.store, &entry)
                .map_err(IpcError::from)?
                .ok_or_else(|| err("clipboard.cannotRestore", "no payload"))?;
            let text = String::from_utf8(bytes)
                .map_err(|_| err("clipboard.cannotRestore", "payload is not valid utf-8"))?;
            clip.set_text(text)
                .map_err(|e| err("clipboard.writeFailed", format!("set text failed: {e}")))?;
        }
        ContentKind::Image => {
            let png = CapturePipeline::load_payload(&rt.store, &entry)
                .map_err(IpcError::from)?
                .ok_or_else(|| err("clipboard.cannotRestore", "no payload"))?;
            let decoded = image::load_from_memory(&png)
                .map_err(|e| err("clipboard.cannotRestore", format!("decode failed: {e}")))?;
            let rgba = decoded.to_rgba8();
            clip.set_image(arboard::ImageData {
                width: rgba.width() as usize,
                height: rgba.height() as usize,
                bytes: std::borrow::Cow::Owned(rgba.into_raw()),
            })
            .map_err(|e| err("clipboard.writeFailed", format!("set image failed: {e}")))?;
        }
        ContentKind::File => {
            if entry.file_paths.is_empty() {
                return Err(err("clipboard.cannotRestore", "no file paths recorded"));
            }
            drop(clip); // clipboard-win 自行开关剪贴板
            write_file_list(&entry.file_paths)?;
        }
        ContentKind::Unknown => {
            return Err(err(
                "clipboard.unsupportedKind",
                "unknown format cannot be restored",
            ));
        }
    }
    // §15 自循环抑制：写回内容 = entry.content_hash
    *rt.suppress.lock().expect("suppress") = Some(entry.content_hash.to_lowercase());
    // §74 直觉：重新复制 ⇒ 提升到顶部 + 更新时间（不新建）
    let mut moved = entries.remove(index);
    moved.updated_at = SystemTime::now();
    entries.insert(0, moved);
    rt.store.persist(&entries).map_err(IpcError::from)?;
    Ok(())
}

/// 删除历史条目（§28/§65：只删元数据+payload，绝不触碰原文件）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_delete(app: tauri::AppHandle, id: String) -> Result<(), IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.clipboard;
    let mut entries = rt.entries.lock().expect("entries");
    if !rt.store.remove(&mut entries, &id).map_err(IpcError::from)? {
        return Err(err("clipboard.unknownEntry", format!("no entry '{id}'")));
    }
    Ok(())
}

/// 清空历史（§29：与清空系统剪贴板完全分离——本命令不动系统剪贴板）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_clear_history(app: tauri::AppHandle) -> Result<f64, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.clipboard;
    let mut entries = rt.entries.lock().expect("entries");
    let count = rt.store.clear(&mut entries).map_err(IpcError::from)?;
    Ok(count as f64)
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ClipboardStatusDto {
    pub enabled: bool,
    pub paused: bool,
    pub total: f64,
    pub text_count: f64,
    pub image_count: f64,
    pub file_count: f64,
    pub favorite_count: f64,
    pub sensitive_count: f64,
    pub storage_bytes: f64,
}

/// 状态（§17：用户能清楚看到 ● monitoring / ○ paused）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_status(app: tauri::AppHandle) -> Result<ClipboardStatusDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.clipboard;
    let entries = rt.entries.lock().expect("entries");
    Ok(ClipboardStatusDto {
        enabled: rt.enabled.load(Ordering::SeqCst),
        paused: rt.paused.load(Ordering::SeqCst),
        total: entries.len() as f64,
        text_count: entries
            .iter()
            .filter(|e| e.kind == ContentKind::Text)
            .count() as f64,
        image_count: entries
            .iter()
            .filter(|e| e.kind == ContentKind::Image)
            .count() as f64,
        file_count: entries
            .iter()
            .filter(|e| e.kind == ContentKind::File)
            .count() as f64,
        favorite_count: entries.iter().filter(|e| e.is_favorite).count() as f64,
        sensitive_count: entries
            .iter()
            .filter(|e| e.sensitivity == Sensitivity::PotentiallySensitive)
            .count() as f64,
        storage_bytes: rt.store.storage_usage(&entries) as f64,
    })
}

/// 暂停/恢复监听（§17 隐私 P0：用户必须能停）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_set_paused(app: tauri::AppHandle, paused: bool) -> Result<(), IpcError> {
    let state = app.state::<crate::state::AppState>();
    state.clipboard.paused.store(paused, Ordering::SeqCst);
    tracing::info!(paused, "clipboard monitoring toggled");
    Ok(())
}

/// 收藏切换（§21）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_set_favorite(
    app: tauri::AppHandle,
    id: String,
    favorite: bool,
) -> Result<(), IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rt = &state.clipboard;
    let mut entries = rt.entries.lock().expect("entries");
    match entries.iter_mut().find(|e| e.id == id) {
        Some(entry) => {
            entry.is_favorite = favorite;
            rt.store.persist(&entries).map_err(IpcError::from)?;
            Ok(())
        }
        None => Err(err("clipboard.unknownEntry", format!("no entry '{id}'"))),
    }
}

/// 读缩略图（§8：列表只加载缩略图；按需取用）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_thumbnail(app: tauri::AppHandle, id: String) -> Result<Vec<u8>, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let entries = state.clipboard.entries.lock().expect("entries");
    let entry = entries
        .iter()
        .find(|e| e.id == id)
        .ok_or_else(|| err("clipboard.unknownEntry", format!("no entry '{id}'")))?;
    match &entry.thumbnail_file {
        Some(file) => state
            .clipboard
            .store
            .read_payload(file)
            .map_err(IpcError::from),
        None => Err(err("clipboard.noThumbnail", "entry has no thumbnail")),
    }
}

/// 监控设置立即生效（§42；设置持久化走既有 set_app_config）。
#[tauri::command]
#[specta::specta]
pub fn clipboard_apply_config(app: tauri::AppHandle, config: crate::config::ClipboardConfig) {
    let state = app.state::<crate::state::AppState>();
    state.clipboard.apply_config(&config);
}
