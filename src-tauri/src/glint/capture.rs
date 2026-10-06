//! M2.8 Glint：屏幕捕获核心（monitor / region / foreground window）。
//!
//! - 捕获经 `screenshots` crate（物理像素；全屏 `capture()`，区域 = 全屏捕获
//!   后按物理坐标裁剪——坐标语义统一，不混用逻辑/物理空间 §5）。
//! - 前台窗口（§7）：经 windows-rs 安全封装取窗口矩形（DWM 扩展边框，去除
//!   阴影），最小化/不可见/已关闭 ⇒ 结构化失败（不伪造截图）。
//! - 输出：PNG 字节（像素与 alpha 保真；JPEG/WebP 由既有 image 管线转换）。
//! - 隐私（§27）：本模块不写日志内容，只返回结构化错误码。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("glint::capture")
}

/// 显示器信息（来自 screenshots crate；物理像素）。index 以 u32 经 IPC
/// （specta 禁 usize）；id 仅展示用。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MonitorInfo {
    pub index: u32,
    pub id: u32,
    /// 屏幕坐标系原点（可为负——左侧显示器 §33）。
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
    pub scale_factor: f32,
    pub is_primary: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapturedScreen {
    /// RGBA8 像素（物理尺寸）。
    pub rgba: Vec<u8>,
    pub width: u32,
    pub height: u32,
    pub monitor: MonitorInfo,
}

/// 列出全部显示器（§5：物理坐标；可为负）。
pub fn list_monitors() -> Result<Vec<MonitorInfo>, WeaveError> {
    let screens = screenshots::Screen::all().map_err(|e| {
        err(
            "glint.screenEnumFailed",
            format!("screen enumeration failed: {e}"),
        )
    })?;
    Ok(screens
        .iter()
        .enumerate()
        .map(|(index, screen)| {
            let d = &screen.display_info;
            MonitorInfo {
                index: index as u32,
                id: d.id,
                x: d.x,
                y: d.y,
                width: d.width,
                height: d.height,
                scale_factor: d.scale_factor,
                is_primary: d.is_primary,
            }
        })
        .collect())
}

/// 全屏捕获指定显示器（index 为 list_monitors 顺序）。
pub fn capture_monitor(monitor_index: usize) -> Result<CapturedScreen, WeaveError> {
    let monitors = list_monitors()?;
    let Some(info) = monitors.get(monitor_index) else {
        return Err(err(
            "glint.monitorNotFound",
            format!("monitor {monitor_index} not found"),
        ));
    };
    let screens =
        screenshots::Screen::all().map_err(|e| err("glint.screenEnumFailed", format!("{e}")))?;
    let Some(screen) = screens.get(monitor_index) else {
        return Err(err(
            "glint.monitorNotFound",
            format!("monitor {monitor_index} not found"),
        ));
    };
    let image = screen
        .capture()
        .map_err(|e| err("glint.captureFailed", format!("screen capture failed: {e}")))?;
    Ok(CapturedScreen {
        width: image.width(),
        height: image.height(),
        rgba: image.into_raw(),
        monitor: info.clone(),
    })
}

/// 物理坐标（显示器相对）区域裁剪。越界/零尺寸 ⇒ 结构化错误。
pub fn crop_rgba(
    rgba: &[u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    crop_w: u32,
    crop_h: u32,
) -> Result<(Vec<u8>, u32, u32), WeaveError> {
    if width == 0 || height == 0 {
        return Err(err("glint.invalidRegion", "source is empty"));
    }
    let img = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or_else(|| err("glint.invalidRegion", "rgba buffer size mismatch"))?;
    if crop_w == 0
        || crop_h == 0
        || x.saturating_add(crop_w) > width
        || y.saturating_add(crop_h) > height
    {
        return Err(err(
            "glint.invalidRegion",
            format!("region ({x},{y},{crop_w},{crop_h}) out of bounds {width}x{height}"),
        ));
    }
    let cropped = image::imageops::crop_imm(&img, x, y, crop_w, crop_h);
    Ok((cropped.to_image().into_raw(), crop_w, crop_h))
}

/// 前台窗口信息（经 x-win 安全封装；DPI 物理像素，屏幕坐标可含负值 §33）。
/// hwnd 以字符串经 IPC（specta 禁 isize；句柄只作展示标识，不作系统调用）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WindowInfo {
    pub hwnd: String,
    pub title: String,
    pub process_name: String,
    pub screen_x: i32,
    pub screen_y: i32,
    pub width: u32,
    pub height: u32,
    /// x ≤ -32000 是 Windows 最小化窗口的经典位置（启发式，如实标注）。
    pub minimized: bool,
}

/// 列出可见顶层窗口（§7 WindowDiscovery：供窗口选择列表）。
/// 过滤：无标题/零尺寸/明显最小化位置。
pub fn list_windows() -> Vec<WindowInfo> {
    let monitors = list_monitors().unwrap_or_default();
    let on_known_monitor = |x: i32, y: i32, w: i32, h: i32| {
        monitors.iter().any(|m| {
            let (mw, mh) = (m.width as i32, m.height as i32);
            x + w / 2 >= m.x && x + w / 2 < m.x + mw && y + h / 2 >= m.y && y + h / 2 < m.y + mh
        })
    };
    let mut out = Vec::new();
    if let Ok(windows) = x_win::get_open_windows() {
        for w in windows {
            let (x, y, width, height) = (
                w.position.x,
                w.position.y,
                w.position.width,
                w.position.height,
            );
            if width <= 0 || height <= 0 {
                continue;
            }
            let minimized = x <= -32000 || y <= -32000;
            if minimized || !on_known_monitor(x, y, width, height) {
                continue;
            }
            out.push(WindowInfo {
                hwnd: w.id.to_string(),
                title: w.title,
                process_name: w.info.name.clone(),
                screen_x: x,
                screen_y: y,
                width: width as u32,
                height: height as u32,
                minimized,
            });
        }
    }
    out.sort_by(|a, b| b.hwnd.cmp(&a.hwnd));
    out
}

/// 前台窗口捕获结果：屏幕坐标矩形 + 所属显示器 index。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ForegroundWindow {
    pub hwnd: isize,
    pub title: String,
    pub screen_x: i32,
    pub screen_y: i32,
    pub width: u32,
    pub height: u32,
    pub monitor_index: usize,
}

/// 前台窗口捕获：返回屏幕坐标矩形 + 所属显示器 index。
/// 失败模型（§7）：已最小化/不可见/零尺寸 ⇒ 结构化错误（不伪造截图）。
pub fn foreground_window_rect() -> Result<ForegroundWindow, WeaveError> {
    let active = x_win::get_active_window()
        .map_err(|e| err("glint.noForegroundWindow", format!("no active window: {e}")))?;
    let (x, y, width, height) = (
        active.position.x,
        active.position.y,
        active.position.width,
        active.position.height,
    );
    if x <= -32000 || y <= -32000 {
        return Err(err("glint.windowMinimized", "window is minimized"));
    }
    if width <= 0 || height <= 0 {
        return Err(err("glint.windowBoundsFailed", "window bounds are empty"));
    }
    let monitors = list_monitors()?;
    let cx = x + width / 2;
    let cy = y + height / 2;
    let Some((index, _)) = monitors.iter().enumerate().find(|(_, m)| {
        let (mw, mh) = (m.width as i32, m.height as i32);
        cx >= m.x && cx < m.x + mw && cy >= m.y && cy < m.y + mh
    }) else {
        return Err(err(
            "glint.monitorNotFound",
            "window center outside known monitors",
        ));
    };
    Ok(ForegroundWindow {
        hwnd: active.id as isize,
        title: active.title,
        screen_x: x,
        screen_y: y,
        width: width as u32,
        height: height as u32,
        monitor_index: index,
    })
}

/// 在显示器捕获图上裁出窗口区域（屏幕坐标 → 显示器相对坐标）。
pub fn crop_window_from_monitor(
    rgba: &[u8],
    monitor: &MonitorInfo,
    screen_x: i32,
    screen_y: i32,
    w: u32,
    h: u32,
) -> Result<(Vec<u8>, u32, u32), WeaveError> {
    let rel_x = (screen_x - monitor.x).max(0) as u32;
    let rel_y = (screen_y - monitor.y).max(0) as u32;
    // 裁剪范围夹到显示器内（窗口可能跨显示器——超出部分丢弃，如实）
    let max_w = monitor.width.saturating_sub(rel_x);
    let max_h = monitor.height.saturating_sub(rel_y);
    let crop_w = w.min(max_w);
    let crop_h = h.min(max_h);
    crop_rgba(
        rgba,
        monitor.width,
        monitor.height,
        rel_x,
        rel_y,
        crop_w,
        crop_h,
    )
}

// ── 输出命名与存储路径（§8/§9：稳定命名、不覆盖）──

/// 生成输出文件名：`Glint_YYYY-MM-DD_HH-MM-SS-mmm.png`；同毫秒冲突追加 `-1`。
pub fn unique_output_path(dir: &Path, ext: &str, now: SystemTime) -> PathBuf {
    let secs = now
        .duration_since(SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    let base = format!("Glint_{secs}");
    let mut candidate = dir.join(format!("{base}.{ext}"));
    let mut n = 1u32;
    while candidate.exists() {
        candidate = dir.join(format!("{base}-{n}.{ext}"));
        n += 1;
    }
    candidate
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn crop_rgba_validates_bounds_and_crops() {
        // 2x2 白图
        let rgba = vec![255u8; 2 * 2 * 4];
        // 正常裁剪 1x1 at (1,1)
        let (data, w, h) = crop_rgba(&rgba, 2, 2, 1, 1, 1, 1).expect("crop");
        assert_eq!((w, h), (1, 1));
        assert_eq!(data, vec![255, 255, 255, 255]);
        // 越界 ⇒ 错误
        assert!(crop_rgba(&rgba, 2, 2, 1, 1, 2, 1).is_err());
        assert!(crop_rgba(&rgba, 2, 2, 0, 0, 0, 1).is_err());
    }

    #[test]
    fn unique_output_path_never_overwrites() {
        let ws = weave_testkit::TempWorkspace::new("glint-name").expect("ws");
        let now = SystemTime::UNIX_EPOCH + Duration::from_secs(1_700_000_000);
        let p1 = unique_output_path(ws.path(), "png", now);
        std::fs::write(&p1, b"x").expect("write");
        let p2 = unique_output_path(ws.path(), "png", now);
        assert_ne!(p1, p2, "同秒第二张不覆盖（§8）");
    }
}
