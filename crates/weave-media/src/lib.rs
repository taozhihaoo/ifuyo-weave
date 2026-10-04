//! weave-media — Image resize / compress / convert / metadata（M6）。
//!
//! 职责（§105）：ImageDocument、格式检测（magic bytes）、Codec 能力矩阵、
//! 受限额守卫的解码、Resize（Fit/Fill/Exact/Scale + 防放大）、Alpha 合成、
//! 格式转换与编码（quality）、Metadata（EXIF 读取 / strip 策略）。
//! 不负责：React、Tauri、filesystem 策略、历史存储、路径校验。
//!
//! 核心纪律：
//! - §5/§7/§119：解码前用 header 尺寸做 pixel budget 守卫，防解压炸弹。
//! - §10：扩展名不是真相——格式由 magic bytes 检测并报告 mismatch。
//! - §46：Compression ≠ Resize，两者可串联但分别表达。
//! - §59：动画（GIF/animated WebP）v1 = Static Only，显式 NOT SUPPORTED。

pub mod batch;
pub mod capabilities;
pub mod detect;
pub mod inspect;
pub mod limits;
pub mod ops;

pub use batch::{
    BatchFileResult, BatchFileStatus, BatchResult, ImageBatchPlan, run_batch, snapshot_inputs,
};
pub use capabilities::{FormatCapability, capability};
pub use detect::{ExtensionMismatch, ImageFormat, detect_format};
pub use inspect::{ImageFacts, MetadataFacts, inspect_bytes};
pub use limits::ImageLimits;
pub use ops::{
    AlphaPolicy, FitMode, ResizeFilter, ResizeOptions, apply_exif_orientation,
    composite_on_background, encode_image, exif_gps_present, resize_dimensions, resize_image,
    strip_metadata_bytes,
};
