//! M2.8 Glint：屏幕捕获 / 预览 / 输出 / Clipboard 集成。
//!
//! - 捕获经 `screenshots` crate（物理像素），区域 = 全屏捕获 + 裁剪（坐标
//!   语义统一 §5）；前台窗口经 windows-rs 安全封装（最小化/不可见 ⇒ 结构化
//!   失败，不伪造截图 §7）。
//! - 复用：weave-media PNG 编码（无第二套 Image Pipeline §18）、M2.3
//!   clipboard（image 写入 §10——历史无重复）、路径安全沿用统一检查（§37）。
//! - 隐私（§27）：本模块不记录 OCR 文本/图像内容/剪贴板内容。

pub mod annotation;
pub mod capture;
pub mod ocr;
pub mod storage;

pub use annotation::{Annotation, AnnotationShape, AnnotationStyle, render as render_annotations};
pub use capture::{
    CapturedScreen, ForegroundWindow, MonitorInfo, WindowInfo, capture_monitor, crop_rgba,
    crop_window_from_monitor, foreground_window_rect, list_monitors, list_windows,
    unique_output_path,
};
pub use ocr::{OcrLineDto, OcrResultDto};
pub use storage::{CaptureRecord, captures_dir, delete_capture, list_recent};
