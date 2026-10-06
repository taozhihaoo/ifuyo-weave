//! M2.10 Web 服务（IPC 边界 + 运行时）。领域逻辑在 `web/` 模块。
//!
//! 边界纪律（§1/§51/§69/§99 审计结论）：
//! - **唯一网络出口**：所有 HTTP 经 `web::fetch`（§17）。
//! - 复制走 arboard + M2.3 suppress 指纹（写前登记，消自环 §36/§51）。
//! - 导出走 M2.0 atomic_write + creation-undo（output-first 不覆盖 §69）；
//!   不建第二套写入/恢复体系。
//! - 批量走既有 JobTracker（可取消/进度/失败隔离 §41-§45），有界并发
//!   （§45），单 URL 失败不影响整体（§43）。
//! - 隐私（§53/§54）：日志/报告/批量条目 URL 一律经 sensitive masking；
//!   不保存 Cookie/Authorization（§73——每次请求全新 Agent）。
//! - 内存（§46）：单文档缓存一槽（最新）；批量不缓存原文，边取边转边写。

#![expect(clippy::result_large_err)] // D10：IpcError DTO 体积豁免

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use tauri::Manager;

use crate::commands::IpcError;
use crate::files_dto::JobHandleDto;
use crate::config::WebConfig;
use crate::jobs::JobOutcome;
use crate::web::fetch::{decode_body, fetch, FetchResponse, FetchLimits};
use crate::web::html;
use crate::web::markdown::to_markdown;
use crate::web::model::{CharsetSource, WebContentType, WebDocument, WebFetchInfo, WebLink};
use crate::web::url;
use weave_core::prelude::CancellationToken;

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    IpcError::from(weave_core::prelude::WeaveError::io(code, message).with_location("web_service"))
}

pub const MAX_BATCH_URLS: usize = 500;
pub const MAX_ANALYZE_BYTES: usize = 20 * 1024 * 1024;
const BATCH_REPORT_CACHE_CAP: usize = 5;

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

/// 导出请求（§37/§68/§69）：content 直出（UI 预览确认后的文本）或
/// job_id（缓存文档）；format 明确产物语义。
#[derive(Debug, Clone, PartialEq, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebExportRequest {
    pub job_id: Option<String>,
    /// UI 直接提交的预览内容（与 job_id 二选一；优先）。
    pub content: Option<String>,
    /// txt | md | html（html = 原始响应 HTML——Original 语义，如实标注 §37）
    pub format: String,
    pub dest_dir: String,
    pub file_name: Option<String>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebExportResult {
    pub output: String,
    pub bytes: f64,
}

/// 批量条目状态（§42；URL 经掩码 §54）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebBatchItem {
    pub masked_url: String,
    /// fetching | exported | failed | skipped | cancelled
    pub status: String,
    pub file_name: Option<String>,
    pub error_code: Option<String>,
}

/// 批量报告（§42/§72）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebBatchReport {
    pub total: f64,
    pub succeeded: f64,
    pub failed: f64,
    pub cancelled: f64,
    pub skipped: f64,
    pub items: Vec<WebBatchItem>,
}

#[derive(Debug, Clone)]
struct WebSlot {
    job_id: String,
    doc: WebDocument,
    /// 原始响应 HTML（HTML 导出 Original 语义用；单槽随最新抓取更替 §46）。
    raw_html: Option<String>,
}

/// Web 运行时（配置快照 + 单文档槽 + 批量报告缓存；无自建线程 §29 纪律——
/// 任务线程全部经既有 JobTracker）。
pub struct WebRuntime {
    pub allow_network: AtomicBool,
    config: Mutex<WebConfig>,
    latest: Mutex<Option<WebSlot>>,
    batch_reports: Mutex<HashMap<String, WebBatchReport>>,
    report_order: Mutex<Vec<String>>,
}

impl WebRuntime {
    pub fn new(config: WebConfig) -> Self {
        Self {
            allow_network: AtomicBool::new(config.allow_network),
            config: Mutex::new(config),
            latest: Mutex::new(None),
            batch_reports: Mutex::new(HashMap::new()),
            report_order: Mutex::new(Vec::new()),
        }
    }

    pub fn config_snapshot(&self) -> WebConfig {
        self.config.lock().expect("web config").clone()
    }

    pub fn apply_config(&self, config: &WebConfig) {
        self.allow_network.store(config.allow_network, Ordering::SeqCst);
        *self.config.lock().expect("web config") = config.clone();
    }

    fn limits(&self) -> FetchLimits {
        let c = self.config_snapshot();
        FetchLimits {
            timeout_secs: c.timeout_secs.max(1),
            max_redirects: c.max_redirects.clamp(0, 20),
            max_bytes: c.max_response_mb.max(1) as usize * 1024 * 1024,
            ..FetchLimits::default()
        }
    }

    fn store_doc(&self, slot: WebSlot) {
        *self.latest.lock().expect("web latest") = Some(slot);
    }

    pub fn store_batch_report(&self, job_id: &str, report: WebBatchReport) {
        {
            let mut order = self.report_order.lock().expect("web report order");
            order.push(job_id.to_string());
            while order.len() > BATCH_REPORT_CACHE_CAP {
                let evict = order.remove(0);
                self.batch_reports.lock().expect("web reports").remove(&evict);
            }
        }
        self.batch_reports
            .lock()
            .expect("web reports")
            .insert(job_id.to_string(), report);
    }

    pub fn batch_report(&self, job_id: &str) -> Option<WebBatchReport> {
        self.batch_reports.lock().expect("web reports").get(job_id).cloned()
    }

    fn network_enabled(&self) -> bool {
        self.allow_network.load(Ordering::SeqCst)
    }
}

/// FetchResponse + 管线 ⇒ WebDocument（远程与本地共用 §63/§65）。
pub fn build_document(
    info_base: WebFetchInfo,
    body: &[u8],
    charset_hint: Option<&str>,
) -> WebDocument {
    let (text, charset, charset_source) = decode_body(body, charset_hint);
    let dom = html::parse(&text);
    let metadata = html::extract_metadata(&dom);
    let links = html::extract_links(&dom, Some(&info_base.final_url));
    let (fragment, confidence, source) = html::extract_main_content(&dom);
    let plain = html::fragment_to_text(&fragment);
    let md = to_markdown(&html::parse(&fragment), Some(&info_base.final_url));
    let mut info = info_base;
    info.charset = charset;
    info.charset_source = charset_source;
    WebDocument {
        info,
        metadata,
        content_confidence: confidence,
        content_source: source,
        plain_text: plain,
        markdown: md,
        links,
        truncated: false,
    }
}

fn ensure_network(rt: &WebRuntime) -> Result<(), IpcError> {
    if rt.network_enabled() {
        Ok(())
    } else {
        Err(err("web.networkDisabled", "network access is disabled in settings"))
    }
}

// ── 同步命令（URL 分析/文本提取——零网络）──

#[tauri::command]
#[specta::specta]
pub fn web_clean_url(input: String, remove_tracking: bool) -> Result<crate::web::model::UrlCleanResult, IpcError> {
    url::clean(&input, remove_tracking).map_err(IpcError::from)
}

#[tauri::command]
#[specta::specta]
pub fn web_display_url(input: String) -> Result<crate::web::model::UrlDisplay, IpcError> {
    url::display(&input).map_err(IpcError::from)
}

/// 自由文本提取 URL（§10；展示文本 = URL 本身；类型 Unknown——无 base
/// 语境不硬分类 §11）。
#[tauri::command]
#[specta::specta]
pub fn web_extract_urls_text(text: String) -> Vec<WebLink> {
    url::extract_urls(&text)
        .into_iter()
        .map(|(u, _)| WebLink {
            absolute_url: u.clone(),
            display_text: u,
            link_type: crate::web::model::WebLinkType::Unknown,
            raw_href: String::new(),
        })
        .collect()
}

/// 本地 HTML 分析（§63：与远程同一 Parse/Extract/Transform 管线；不上网）。
#[tauri::command]
#[specta::specta]
pub fn web_analyze_html(app: tauri::AppHandle, html_text: String, base_url: Option<String>) -> Result<WebDocument, IpcError> {
    if html_text.len() > MAX_ANALYZE_BYTES {
        return Err(err(
            "web.contentTooLarge",
            format!("local html exceeds {} MB", MAX_ANALYZE_BYTES / 1024 / 1024),
        ));
    }
    let state = app.state::<crate::state::AppState>();
    let info = WebFetchInfo {
        requested_url: base_url.clone().unwrap_or_else(|| "local".to_string()),
        final_url: base_url.clone().unwrap_or_else(|| "local".to_string()),
        status: 0,
        content_type: WebContentType::Html,
        content_type_raw: "text/html (local)".to_string(),
        charset: String::new(),
        charset_source: CharsetSource::Utf8Default,
        size_bytes: html_text.len() as f64,
        duration_ms: 0.0,
        redirect_count: 0,
        redirect_chain: Vec::new(),
    };
    let doc = build_document(info, html_text.as_bytes(), None);
    state.web.store_doc(WebSlot {
        job_id: format!("local-{}", now_ms() as i64),
        doc: doc.clone(),
        raw_html: Some(html_text),
    });
    Ok(doc)
}

/// 抓取任务（§13-§16/§44：后台 Job——可取消、不阻塞 UI）。
#[tauri::command]
#[specta::specta]
pub fn web_fetch_url(app: tauri::AppHandle, url: String, allow_private: bool) -> Result<JobHandleDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    ensure_network(&state.web)?;
    let limits = state.web.limits();
    let allow_private = allow_private || state.web.config_snapshot().allow_private_targets;
    let (job_id, cancel, _sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_task = job_id.clone();
    let requested = url.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        match do_fetch_document(&requested, &limits, &cancel, allow_private) {
            Ok((doc, raw)) => {
                app_state.web.store_doc(WebSlot {
                    job_id: job_id_task.to_string(),
                    doc: doc.clone(),
                    raw_html: raw,
                });
                app_state.jobs.finish(&job_id_task, JobOutcome::WebCompleted);
            }
            Err(e) => app_state.jobs.fail(&job_id_task, e),
        }
    });
    drop(state_cell);
    Ok(JobHandleDto { job_id: job_id.to_string() })
}

fn do_fetch_document(
    requested: &str,
    limits: &FetchLimits,
    cancel: &CancellationToken,
    allow_private: bool,
) -> Result<(WebDocument, Option<String>), weave_core::prelude::WeaveError> {
    let response = fetch(requested, limits, cancel, allow_private)?;
    let charset_hint = if response.info.charset.is_empty() {
        None
    } else {
        Some(response.info.charset.clone())
    };
    let FetchResponse { info, body } = response;
    let raw_html = if info.content_type == WebContentType::Html {
        Some(String::from_utf8_lossy(&body).into_owned())
    } else {
        None
    };
    let doc = build_document(info, &body, charset_hint.as_deref());
    Ok((doc, raw_html))
}

#[tauri::command]
#[specta::specta]
pub fn web_get_result(app: tauri::AppHandle, job_id: String) -> Result<WebDocument, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let slot = state.web.latest.lock().expect("web latest");
    match slot.as_ref() {
        Some(s) if s.job_id == job_id => Ok(s.doc.clone()),
        _ => Err(err("web.noResult", format!("no web result for job '{job_id}'"))),
    }
}

/// 复制文本（§36/§51：arboard + M2.3 suppress 指纹**写前登记**）。
#[tauri::command]
#[specta::specta]
pub fn web_copy_text(app: tauri::AppHandle, text: String) -> Result<(), IpcError> {
    let state = app.state::<crate::state::AppState>();
    if let Some(fp) =
        weave_files::hash::hash_bytes(weave_core::prelude::HashAlgorithm::Sha256, text.as_bytes()).ok()
    {
        *state.clipboard.suppress.lock().expect("suppress") = Some(fp.to_lowercase());
    }
    let mut clip = arboard::Clipboard::new()
        .map_err(|e| err("web.clipboardFailed", format!("open clipboard failed: {e}")))?;
    clip.set_text(text)
        .map_err(|e| err("web.clipboardFailed", format!("set text failed: {e}")))?;
    Ok(())
}

/// 导出（§37/§68/§69）：output-first + atomic_write + creation-undo。
#[tauri::command]
#[specta::specta]
pub fn web_export_document(app: tauri::AppHandle, request: WebExportRequest) -> Result<WebExportResult, IpcError> {
    use weave_files::atomic_write;
    let state = app.state::<crate::state::AppState>();
    let rt = &state.web;
    let (content, source_url) = if let Some(content) = &request.content {
        (content.clone(), None)
    } else {
        let job_id = request
            .job_id
            .clone()
            .ok_or_else(|| err("web.exportNoSource", "no content and no job_id"))?;
        let slot = rt.latest.lock().expect("web latest");
        let slot = slot
            .as_ref()
            .filter(|s| s.job_id == job_id)
            .ok_or_else(|| err("web.noResult", format!("no web result for job '{job_id}'")))?;
        let content = match request.format.as_str() {
            "md" => slot.doc.markdown.clone(),
            "html" => slot
                .raw_html
                .clone()
                .ok_or_else(|| err("web.exportNoSource", "no raw html in cached document"))?,
            _ => slot.doc.plain_text.clone(),
        };
        (content, Some(slot.doc.info.final_url.clone()))
    };
    if content.trim().is_empty() {
        return Err(err("web.exportEmpty", "nothing to export"));
    }
    let file_base = match &request.file_name {
        Some(name) if !name.trim().is_empty() => name.trim().to_string(),
        _ => source_url
            .as_deref()
            .and_then(|u| url::filename_from_url(u, 100).ok())
            .unwrap_or_else(|| "web-page".to_string()),
    };
    let ext = match request.format.as_str() {
        "md" => "md",
        "html" => "html",
        _ => "txt",
    };
    let dest_dir = std::path::PathBuf::from(request.dest_dir.trim());
    if !dest_dir.is_dir() {
        return Err(err(
            "web.exportDestMissing",
            format!("destination dir not found: {}", dest_dir.display()),
        ));
    }
    let output = dest_dir.join(format!("{file_base}.{ext}"));
    // §69：不覆盖——已存在 ⇒ 拒绝（统一 Export 语义；改名职责在 UI）
    if output.exists() {
        return Err(err(
            "document.destinationExists",
            format!("output already exists: {} (output-first)", output.display()),
        ));
    }
    atomic_write(&weave_files::fs::StdFilesystem, &output, content.as_bytes())
        .map_err(|e| err("web.exportWriteFailed", format!("{}: {}", output.display(), e.message)))?;
    if let Ok(hdir) = crate::rename_service::resolve_history_dir(&app) {
        let masked_source = source_url.as_deref().map(url::mask_sensitive).unwrap_or_default();
        if let Err(e) = crate::batch_service::record_creation_transaction(
            &hdir,
            &[],
            std::slice::from_ref(&output),
            &format!("Web export · {} · {masked_source}", request.format),
        ) {
            tracing::warn!(error = %e, "web export history failed");
        }
    }
    tracing::info!(target: "web", out = %output.display(), bytes = content.len(), "web content exported");
    Ok(WebExportResult { output: output.to_string_lossy().into_owned(), bytes: content.len() as f64 })
}

/// 批量抓取+导出（§41-§45/§72：有界并发/取消/失败隔离/汇总）。
#[tauri::command]
#[specta::specta]
pub fn web_batch_fetch(
    app: tauri::AppHandle,
    urls: Vec<String>,
    dest_dir: String,
    format: String,
    allow_private: bool,
) -> Result<JobHandleDto, IpcError> {
    if urls.is_empty() {
        return Err(err("web.batchEmpty", "no URLs given"));
    }
    if urls.len() > MAX_BATCH_URLS {
        return Err(err("web.batchTooLarge", format!("batch limited to {MAX_BATCH_URLS} URLs")));
    }
    let dest = std::path::PathBuf::from(dest_dir.trim());
    if !dest.is_dir() {
        return Err(err(
            "web.exportDestMissing",
            format!("destination dir not found: {}", dest.display()),
        ));
    }
    if !matches!(format.as_str(), "txt" | "md" | "html") {
        return Err(err("web.unsupportedFormat", format!("format '{format}' not supported (txt/md/html)")));
    }
    let state = app.state::<crate::state::AppState>();
    ensure_network(&state.web)?;
    let limits = state.web.limits();
    let allow_private = allow_private || state.web.config_snapshot().allow_private_targets;
    let concurrency = state.web.config_snapshot().max_concurrent.clamp(1, 8) as usize;
    let masked_urls: Vec<String> = urls.iter().map(|u| url::mask_sensitive(u)).collect();
    let (job_id, cancel, sink, state_cell) = state.jobs.register();
    let handle = app.clone();
    let job_id_task = job_id.clone();
    let total = urls.len();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let _progress_op_base = weave_core::prelude::OperationId::generate();
        let items: Arc<Mutex<Vec<WebBatchItem>>> = Arc::new(Mutex::new(vec![
            WebBatchItem {
                masked_url: String::new(),
                status: "queued".to_string(),
                file_name: None,
                error_code: None,
            };
            total
        ]));
        let outputs: Arc<Mutex<Vec<std::path::PathBuf>>> = Arc::new(Mutex::new(Vec::new()));
        let next_index: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let done: Arc<Mutex<usize>> = Arc::new(Mutex::new(0));
        let sink = Arc::new(sink);
        let mut workers = Vec::new();
        for _ in 0..concurrency {
            let next = next_index.clone();
            let items_w = items.clone();
            let outs = outputs.clone();
            let done_w = done.clone();
            let cancel_w = cancel.clone();
            let sink_w = sink.clone();
            let urls_w = urls.clone();
            let masked_w = masked_urls.clone();
            let dest_w = dest.clone();
            let format_w = format.clone();
            let limits_w = limits.clone();
            let progress_op_w = _progress_op_base.clone();
            workers.push(std::thread::spawn(move || {
                loop {
                    if cancel_w.is_cancelled() {
                        return;
                    }
                    let idx = {
                        let mut guard = next.lock().expect("web batch next");
                        if *guard >= urls_w.len() {
                            return;
                        }
                        let i = *guard;
                        *guard += 1;
                        i
                    };
                    let item = do_batch_item(
                        &urls_w[idx],
                        &masked_w[idx],
                        &dest_w,
                        &format_w,
                        &limits_w,
                        &cancel_w,
                        allow_private,
                    );
                    if item.status == "exported" {
                        if let Some(fname) = &item.file_name {
                            outs.lock().expect("web batch outputs").push(dest_w.join(fname));
                        }
                    }
                    items_w.lock().expect("web batch items")[idx] = item;
                    let d = {
                        let mut guard = done_w.lock().expect("web batch done");
                        *guard += 1;
                        *guard
                    };
                    sink_w.report(&weave_core::prelude::Progress::running(
                        progress_op_w.clone(),
                        d as u64,
                        Some(total as u64),
                    ));
                }
            }));
        }
        for w in workers {
            let _ = w.join();
        }
        // 收尾：取消/部分失败也如实汇总（§43/§44）
        let mut items_guard = items.lock().expect("web batch items");
        let cancelled = cancel.is_cancelled();
        for (i, item) in items_guard.iter_mut().enumerate() {
            if item.status == "queued" {
                *item = WebBatchItem {
                    masked_url: masked_urls[i].clone(),
                    status: if cancelled { "cancelled".to_string() } else { "failed".to_string() },
                    file_name: None,
                    error_code: if cancelled { None } else { Some("web.batchNotProcessed".to_string()) },
                };
            }
        }
        let report = WebBatchReport {
            total: total as f64,
            succeeded: items_guard.iter().filter(|i| i.status == "exported").count() as f64,
            failed: items_guard.iter().filter(|i| i.status == "failed").count() as f64,
            cancelled: items_guard.iter().filter(|i| i.status == "cancelled").count() as f64,
            skipped: items_guard.iter().filter(|i| i.status == "skipped").count() as f64,
            items: items_guard.clone(),
        };
        drop(items_guard);
        // 创建型事务：已写产物登记（可撤销 §69）——部分成功也如实登记
        let written = outputs.lock().expect("web batch outputs").clone();
        if !written.is_empty() {
            if let Ok(hdir) = crate::rename_service::resolve_history_dir(&handle) {
                let _ = crate::batch_service::record_creation_transaction(
                    &hdir,
                    &[],
                    &written,
                    &format!("Web batch export · {} items · format {format}", written.len()),
                );
            }
        }
        app_state.web.store_batch_report(&job_id_task.to_string(), report);
        app_state.jobs.finish(&job_id_task, JobOutcome::WebCompleted);
    });
    drop(state_cell);
    Ok(JobHandleDto { job_id: job_id.to_string() })
}

#[allow(clippy::too_many_arguments)]
fn do_batch_item(
    requested: &str,
    masked: &str,
    dest_dir: &std::path::Path,
    format: &str,
    limits: &FetchLimits,
    cancel: &CancellationToken,
    allow_private: bool,
) -> WebBatchItem {
    let mut item = WebBatchItem {
        masked_url: masked.to_string(),
        status: "fetching".to_string(),
        file_name: None,
        error_code: None,
    };
    let response = match fetch(requested, limits, cancel, allow_private) {
        Ok(r) => r,
        Err(e) => {
            item.status = "failed".to_string();
            item.error_code = Some(e.code);
            return item;
        }
    };
    if cancel.is_cancelled() {
        item.status = "cancelled".to_string();
        return item;
    }
    // 转换（与单文档同一管线 §63）
    let charset_hint = if response.info.charset.is_empty() { None } else { Some(response.info.charset.clone()) };
    let FetchResponse { info, body } = response;
    let doc = build_document(info, &body, charset_hint.as_deref());
    let content = match format {
        "md" => doc.markdown,
        "html" => String::from_utf8_lossy(&body).into_owned(),
        _ => doc.plain_text,
    };
    if content.trim().is_empty() {
        item.status = "skipped".to_string();
        item.error_code = Some("web.exportEmpty".to_string());
        return item;
    }
    // 文件名：host_path slug + 冲突 -N（§38/§41）
    let base = url::filename_from_url(requested, 100).unwrap_or_else(|_| "web-page".to_string());
    let ext = match format {
        "md" => "md",
        "html" => "html",
        _ => "txt",
    };
    let mut output = dest_dir.join(format!("{base}.{ext}"));
    let mut n = 1u32;
    while output.exists() {
        output = dest_dir.join(format!("{base}-{n}.{ext}"));
        n += 1;
    }
    match weave_files::atomic_write(&weave_files::fs::StdFilesystem, &output, content.as_bytes()) {
        Ok(()) => {
            item.status = "exported".to_string();
            item.file_name = Some(
                output
                    .file_name()
                    .map(|f| f.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            );
            if let Ok(parent_dir) = dest_dir
                .canonicalize()
                .map(|p| p.to_string_lossy().into_owned())
            {
                let _ = parent_dir; // 输出目录事实已验证（is_dir 前置）
            }
            item
        }
        Err(e) => {
            item.status = "failed".to_string();
            item.error_code = Some(format!("web.exportWriteFailed: {}", e.message));
            item
        }
    }
}

#[tauri::command]
#[specta::specta]
pub fn web_batch_result(app: tauri::AppHandle, job_id: String) -> Result<WebBatchReport, IpcError> {
    let state = app.state::<crate::state::AppState>();
    state
        .web
        .batch_report(&job_id)
        .ok_or_else(|| err("web.noResult", format!("no batch report for job '{job_id}'")))
}

/// 设置立即生效（§84；持久化走既有 set_app_config 全量 round-trip）。
#[tauri::command]
#[specta::specta]
pub fn web_apply_config(app: tauri::AppHandle, config: WebConfig) {
    let state = app.state::<crate::state::AppState>();
    state.web.apply_config(&config);
}

#[tauri::command]
#[specta::specta]
pub fn web_get_config(app: tauri::AppHandle) -> WebConfig {
    let state = app.state::<crate::state::AppState>();
    state.web.config_snapshot()
}
