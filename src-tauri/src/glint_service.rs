//! M2.8 Glint 服务（IPC 边界——Application 层，领域逻辑在 glint/ 模块）。
//!
//! - 复用（§44 无第二套）：编码/缩放走 `weave-media`（§18 单一 Image
//!   Pipeline）；剪贴板写入走 M2.3 `arboard` 路径 + 指纹抑制（§10 无
//!   自循环）；存储经 `glint::storage`（有界 §26）。
//! - 隐私（§27）：日志只记文件名/尺寸等元数据，绝不记 OCR 文本、图像
//!   内容、剪贴板内容。
//! - OCR 文本（§17）：只经返回值交 UI，不落盘、不入历史。

use tauri::Manager;

use crate::commands::IpcError;
use crate::glint::storage::DEFAULT_MAX_RECENT;
use crate::glint::{self, Annotation, CaptureRecord, MonitorInfo, OcrResultDto, WindowInfo};

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    IpcError::from(weave_core::prelude::WeaveError::io(code, message).with_location("glint_service"))
}

/// 捕获保存结果（记录 + 来源描述 §4 CaptureMetadata）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct GlintCaptureSaved {
    pub record: CaptureRecord,
    /// 来源描述（如 "monitor 1"、"region 800x600"、"window: Title (app.exe)"）。
    pub source: String,
}

/// RGBA → 编码字节（复用 weave-media §18）。
fn encode_rgba(
    rgba: Vec<u8>,
    width: u32,
    height: u32,
    format: &str,
    jpeg_quality: u8,
) -> Result<Vec<u8>, IpcError> {
    let dynamic = image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(width, height, rgba)
            .ok_or_else(|| err("glint.invalidRegion", "rgba buffer size mismatch"))?,
    );
    let fmt = match format {
        "jpeg" | "jpg" => weave_media::ImageFormat::Jpeg,
        _ => weave_media::ImageFormat::Png,
    };
    weave_media::ops::encode_image(&dynamic, fmt, Some(jpeg_quality))
        .map_err(|e| err("glint.encodeFailed", e))
}

/// 保存 PNG 到 Glint 捕获目录（唯一命名，不覆盖 §8；仅记元数据日志 §27）。
fn save_png(dir: &std::path::Path, png: &[u8], width: u32, height: u32) -> Result<CaptureRecord, IpcError> {
    let path = glint::unique_output_path(dir, "png", std::time::SystemTime::now());
    std::fs::write(&path, png).map_err(|e| err("glint.saveFailed", format!("{e}")))?;
    tracing::info!(target: "glint", file = %path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), w = width, h = height, size = png.len(), "capture saved");
    Ok(CaptureRecord {
        file_name: path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        path: path.to_string_lossy().into_owned(),
        size: png.len() as f64,
        modified_ms: Some(
            std::time::SystemTime::now()
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .map(|d| d.as_millis() as f64)
                .unwrap_or(0.0),
        ),
        width,
        height,
    })
}

fn guarded_path(dir: &std::path::Path, file_name: &str) -> Result<std::path::PathBuf, IpcError> {
    if file_name.contains(['/', '\\']) || file_name.contains("..") {
        return Err(err("glint.pathTraversal", "illegal file name"));
    }
    let path = dir.join(file_name);
    if !path.starts_with(dir) || !path.is_file() {
        return Err(err("glint.notFound", format!("capture '{file_name}' not found")));
    }
    Ok(path)
}

fn load_png_bytes(path: &std::path::Path) -> Result<Vec<u8>, IpcError> {
    std::fs::read(path).map_err(|e| err("glint.readFailed", format!("{e}")))
}

fn load_rgba(path: &std::path::Path) -> Result<(Vec<u8>, u32, u32), IpcError> {
    let bytes = load_png_bytes(path)?;
    let decoded =
        image::load_from_memory(&bytes).map_err(|e| err("glint.decodeFailed", format!("{e}")))?;
    let rgba = decoded.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    Ok((rgba.into_raw(), w, h))
}

/// 预览/缩略图字节（复用 weave-media 缩放；fit 指定边长）。
fn scaled_png(path: &std::path::Path, box_px: u32) -> Result<Vec<u8>, IpcError> {
    let bytes = load_png_bytes(path)?;
    let decoded =
        image::load_from_memory(&bytes).map_err(|e| err("glint.decodeFailed", format!("{e}")))?;
    let scaled = weave_media::ops::resize_image(
        &decoded,
        &weave_media::ResizeOptions {
            mode: weave_media::FitMode::Fit,
            width: box_px,
            height: box_px,
            prevent_upscale: true,
            ..weave_media::ResizeOptions::default()
        },
    );
    weave_media::ops::encode_image(&scaled, weave_media::ImageFormat::Png, None)
        .map_err(|e| err("glint.encodeFailed", e))
}

// ── 查询 ──

#[tauri::command]
#[specta::specta]
pub fn glint_list_monitors() -> Result<Vec<MonitorInfo>, IpcError> {
    glint::list_monitors().map_err(IpcError::from)
}

#[tauri::command]
#[specta::specta]
pub fn glint_list_windows() -> Vec<WindowInfo> {
    glint::list_windows()
}

#[tauri::command]
#[specta::specta]
pub fn glint_list_recent(app: tauri::AppHandle) -> Result<Vec<CaptureRecord>, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    Ok(glint::list_recent(&dir, DEFAULT_MAX_RECENT))
}

// ── 捕获（Interactive §20：单次同步调用，图像编码在 Rust 侧不阻塞 UI 线程）──

#[tauri::command]
#[specta::specta]
pub fn glint_capture_screen(app: tauri::AppHandle, monitor_index: u32) -> Result<GlintCaptureSaved, IpcError> {
    let screen = glint::capture_monitor(monitor_index as usize).map_err(IpcError::from)?;
    let png = encode_rgba(screen.rgba, screen.width, screen.height, "png", 90)?;
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let record = save_png(&dir, &png, screen.width, screen.height)?;
    Ok(GlintCaptureSaved {
        source: format!("monitor {} ({}x{} @ {},{})",
            screen.monitor.index, screen.width, screen.height, screen.monitor.x, screen.monitor.y),
        record,
    })
}

#[tauri::command]
#[specta::specta]
pub fn glint_capture_foreground_window(app: tauri::AppHandle) -> Result<GlintCaptureSaved, IpcError> {
    let (_hwnd, title, x, y, w, h, monitor_index) = glint::foreground_window_rect().map_err(IpcError::from)?;
    let screen = glint::capture_monitor(monitor_index as usize).map_err(IpcError::from)?;
    let (rgba, cw, ch) =
        glint::crop_window_from_monitor(&screen.rgba, &screen.monitor, x, y, w, h).map_err(IpcError::from)?;
    let png = encode_rgba(rgba, cw, ch, "png", 90)?;
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let record = save_png(&dir, &png, cw, ch)?;
    Ok(GlintCaptureSaved {
        source: format!("window: {title}"),
        record,
    })
}

/// 从已保存捕获裁剪区域（物理像素，图像坐标 §5）。Region Capture 流程：
/// 先全屏捕获 → UI 拖拽 → 从已捕获图像裁剪（不重拍，时刻一致）。
#[tauri::command]
#[specta::specta]
pub fn glint_crop_capture(
    app: tauri::AppHandle,
    file_name: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
) -> Result<GlintCaptureSaved, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let path = guarded_path(&dir, &file_name)?;
    let (rgba, width, height) = load_rgba(&path)?;
    let (cropped, cw, ch) = glint::crop_rgba(&rgba, width, height, x, y, w, h).map_err(IpcError::from)?;
    let png = encode_rgba(cropped, cw, ch, "png", 90)?;
    let record = save_png(&dir, &png, cw, ch)?;
    Ok(GlintCaptureSaved {
        source: format!("region {w}x{h} @ ({x},{y})"),
        record,
    })
}

// ── 标注（非破坏 §12：渲染结果写新文件，原图保留）──

#[tauri::command]
#[specta::specta]
pub fn glint_save_annotated(
    app: tauri::AppHandle,
    file_name: String,
    annotations: Vec<Annotation>,
    format: String,
    jpeg_quality: u8,
) -> Result<GlintCaptureSaved, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let path = guarded_path(&dir, &file_name)?;
    let (rgba, width, height) = load_rgba(&path)?;
    let rendered = glint::render_annotations(&rgba, width, height, &annotations).map_err(IpcError::from)?;
    let png = encode_rgba(rendered, width, height, &format, jpeg_quality)?;
    let ext = if format.eq_ignore_ascii_case("jpeg") || format.eq_ignore_ascii_case("jpg") {
        "jpg"
    } else {
        "png"
    };
    let out_path = glint::unique_output_path(&dir, ext, std::time::SystemTime::now());
    std::fs::write(&out_path, &png).map_err(|e| err("glint.saveFailed", format!("{e}")))?;
    tracing::info!(target: "glint", file = %file_name, out = %out_path.file_name().map(|n| n.to_string_lossy()).unwrap_or_default(), annotations = annotations.len(), "annotated saved");
    Ok(GlintCaptureSaved {
        record: CaptureRecord {
            file_name: out_path
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: out_path.to_string_lossy().into_owned(),
            size: png.len() as f64,
            modified_ms: Some(
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .map(|d| d.as_millis() as f64)
                    .unwrap_or(0.0),
            ),
            width,
            height,
        },
        source: format!("annotated {file_name}"),
    })
}

// ── 预览 / 缩略图 ──

#[tauri::command]
#[specta::specta]
pub fn glint_preview(app: tauri::AppHandle, file_name: String) -> Result<Vec<u8>, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let path = guarded_path(&dir, &file_name)?;
    scaled_png(&path, 1600)
}

#[tauri::command]
#[specta::specta]
pub fn glint_thumbnail(app: tauri::AppHandle, file_name: String) -> Result<Vec<u8>, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let path = guarded_path(&dir, &file_name)?;
    scaled_png(&path, 320)
}

// ── 删除 ──

#[tauri::command]
#[specta::specta]
pub fn glint_delete_capture(app: tauri::AppHandle, file_name: String) -> Result<bool, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let deleted = glint::delete_capture(&dir, &file_name).map_err(IpcError::from)?;
    if deleted {
        tracing::info!(target: "glint", file = %file_name, "capture deleted");
    }
    Ok(deleted)
}

// ── Clipboard（§10：复用 M2.3 写入路径 + 自循环抑制）──

#[tauri::command]
#[specta::specta]
pub fn glint_copy_capture_to_clipboard(app: tauri::AppHandle, file_name: String) -> Result<(), IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let path = guarded_path(&dir, &file_name)?;
    let png = load_png_bytes(&path)?;
    // 单次解码（§34 不重复 Bitmap Copy）；arboard 需要所有权 ⇒ 仅像素缓冲克隆一次
    let decoded =
        image::load_from_memory(&png).map_err(|e| err("glint.decodeFailed", format!("{e}")))?;
    let rgba = decoded.to_rgba8();
    let (w, h) = (rgba.width(), rgba.height());
    {
        let mut clip = arboard::Clipboard::new()
            .map_err(|e| err("glint.clipboardFailed", format!("open clipboard failed: {e}")))?;
        clip.set_image(arboard::ImageData {
            width: w as usize,
            height: h as usize,
            bytes: std::borrow::Cow::Owned(rgba.as_raw().clone()),
        })
        .map_err(|e| err("glint.clipboardFailed", format!("set image failed: {e}")))?;
    }
    // §15 自循环抑制：指纹 = 规范 PNG（RGBA→PNG）哈希——与 M2.3 监听同构
    let dynamic = image::DynamicImage::ImageRgba8(rgba);
    let canonical = weave_media::ops::encode_image(&dynamic, weave_media::ImageFormat::Png, None)
        .map_err(|e| err("glint.encodeFailed", e))?;
    if let Some(fp) =
        weave_files::hash::hash_bytes(weave_core::prelude::HashAlgorithm::Sha256, &canonical).ok()
    {
        let state = app.state::<crate::state::AppState>();
        *state.clipboard.suppress.lock().expect("suppress") = Some(fp.to_lowercase());
    }
    tracing::info!(target: "glint", file = %file_name, "capture copied to clipboard");
    Ok(())
}

// ── OCR（§15-§17：结构化失败；文本只经返回值）──

#[tauri::command]
#[specta::specta]
pub fn glint_ocr_capture(
    app: tauri::AppHandle,
    file_name: String,
    language: Option<String>,
) -> Result<OcrResultDto, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let path = guarded_path(&dir, &file_name)?;
    let png = load_png_bytes(&path)?;
    glint::ocr::ocr_png(&png, language.as_deref()).map_err(IpcError::from)
}

#[tauri::command]
#[specta::specta]
pub fn glint_ocr_region(
    app: tauri::AppHandle,
    file_name: String,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    language: Option<String>,
) -> Result<OcrResultDto, IpcError> {
    let dir = glint::captures_dir(&app).map_err(IpcError::from)?;
    let path = guarded_path(&dir, &file_name)?;
    let (rgba, width, height) = load_rgba(&path)?;
    let (cropped, _cw, _ch) =
        glint::crop_rgba(&rgba, width, height, x, y, w, h).map_err(IpcError::from)?;
    let dynamic = image::DynamicImage::ImageRgba8(
        image::RgbaImage::from_raw(w, h, cropped)
            .ok_or_else(|| err("glint.invalidRegion", "rgba buffer size mismatch"))?,
    );
    let png = weave_media::ops::encode_image(&dynamic, weave_media::ImageFormat::Png, None)
        .map_err(|e| err("glint.encodeFailed", e))?;
    glint::ocr::ocr_png(&png, language.as_deref()).map_err(IpcError::from)
}
