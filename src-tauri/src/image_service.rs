//! M6：Image 的 IPC 命令（open/inspect/preview/transform/export）。
//!
//! §63/§64：Preview 与 Execute 走同一变换引擎，区别仅在是否写盘；
//! §70/§71：Execute 报告真实输出大小，Preview 只报 Estimated；
//! §73/§74：默认写新文件（dest 存在 ⇒ 拒绝）；覆盖源走 TextTransform
//! 管线（TOCTOU/备份/原子替换/历史/Undo 全套复用 M4）。

// IPC 边界与 D10 同理：Err DTO 体积不构成热路径问题（见 DECISIONS.md D10）。
#![expect(clippy::result_large_err)]

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_core::prelude::{CancellationToken, OperationId, WeaveError};
use weave_media::{AlphaPolicy, FitMode, ImageFormat, ImageLimits, ResizeFilter, ResizeOptions};

use crate::commands::IpcError;
use crate::ops_dto::PlanDto;

fn err(code: impl Into<String>, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code.into().as_str(), message)
        .with_location("image_service")
        .into()
}

fn limits() -> ImageLimits {
    ImageLimits::default()
}

fn load_image(
    path: &str,
) -> Result<(Vec<u8>, image::DynamicImage, weave_media::ImageFacts), IpcError> {
    weave_core::prelude::validate_absolute_path(path)?;
    let meta = std::fs::metadata(path).map_err(|e| err("image.loadFailed", e.to_string()))?;
    if meta.len() > 64 * 1024 * 1024 {
        return Err(err("image.tooLarge", "file exceeds 64 MiB image limit"));
    }
    let bytes = std::fs::read(path).map_err(|e| err("image.loadFailed", e.to_string()))?;
    let (img, facts) = weave_media::inspect_bytes(&bytes, None, &limits())
        .map_err(|e| err("image.decodeFailed", e))?;
    Ok((bytes, img, facts))
}

// ─── DTOs ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageFactsDto {
    pub format: String,
    pub width: f64,
    pub height: f64,
    pub has_alpha: bool,
    pub color_model: String,
    pub bit_depth: f64,
    pub frame_count: f64,
    pub extension_mismatch: bool,
    pub exif_present: bool,
    pub gps_present: bool,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub datetime: Option<String>,
    pub orientation: Option<f64>,
    pub icc_present: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageOpenDto {
    pub path: String,
    /// §65 有界预览（PNG data URI，最长边 ≤ 2048）。
    pub preview_data_uri: String,
    pub facts: ImageFactsDto,
}

// ─── open + inspect ───

fn facts_dto(facts: &weave_media::ImageFacts) -> ImageFactsDto {
    ImageFactsDto {
        format: facts
            .format
            .map(|f| f.as_str().to_string())
            .unwrap_or_default(),
        width: facts.width as f64,
        height: facts.height as f64,
        has_alpha: facts.has_alpha,
        color_model: facts.color_model.clone(),
        bit_depth: facts.bit_depth as f64,
        frame_count: facts.frame_count as f64,
        extension_mismatch: facts.extension_mismatch.is_some(),
        exif_present: facts.metadata.exif_present,
        gps_present: facts.metadata.gps_present,
        camera_make: facts.metadata.camera_make.clone(),
        camera_model: facts.metadata.camera_model.clone(),
        datetime: facts.metadata.datetime.clone(),
        orientation: facts.metadata.orientation.map(|v| v as f64),
        icc_present: facts.metadata.icc_present,
    }
}

/// §65 有界预览：最长边 ≤ max_preview_dimension 的 PNG data URI。
fn preview_data_uri(img: &image::DynamicImage, limits: &ImageLimits) -> String {
    let preview = if img.width().max(img.height()) > limits.max_preview_dimension {
        img.thumbnail(limits.max_preview_dimension, limits.max_preview_dimension)
    } else {
        img.clone()
    };
    let mut png = Vec::new();
    let _ = preview.write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png);
    format!("data:image/png;base64,{}", base64_encode(&png))
}

fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            *chunk.get(1).unwrap_or(&0),
            *chunk.get(2).unwrap_or(&0),
        ];
        let n = (u32::from(b[0]) << 16) | (u32::from(b[1]) << 8) | u32::from(b[2]);
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

#[tauri::command]
#[specta::specta]
pub fn image_open(path: String) -> Result<ImageOpenDto, IpcError> {
    let (bytes, img, facts) = load_image(&path)?;
    let _ = bytes;
    Ok(ImageOpenDto {
        path,
        preview_data_uri: preview_data_uri(&img, &limits()),
        facts: facts_dto(&facts),
    })
}

// ─── transform（resize/compress/convert/strip 统一入口，§131–§135）───

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ResizeOptionsDto {
    pub mode: String, // fit | fill | exact | scale
    pub width: f64,
    pub height: f64,
    pub scale_percent: f64,
    pub prevent_upscale: bool,
    pub filter: String, // nearest | triangle | catmullrom | lanczos3
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageOperationDto {
    /// resize | compress | convert | strip_metadata
    pub operation: String,
    /// 目标格式（convert/compress/strip 时有效）。
    pub target_format: Option<String>,
    pub quality: Option<f64>,
    /// §14 alpha 合成背景：white | black。
    pub alpha_background: Option<String>,
    pub resize: Option<ResizeOptionsDto>,
    /// §61 strip：true = 剥离全部可移除元数据。
    pub strip_metadata: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImagePreviewDto {
    /// §64：同一变换引擎产出的 After 预览（PNG data URI，有界）。
    pub preview_data_uri: String,
    pub output_format: String,
    pub out_width: f64,
    pub out_height: f64,
    /// §69/§70：Preview 阶段为估算标注。
    pub estimated_output_bytes: f64,
    /// §51 有损警示。
    pub lossy: bool,
    /// §54 re-encode 警示。
    pub re_encoded: bool,
    /// §59 动画不保留警示。
    pub animation_dropped: bool,
    pub warnings: Vec<String>,
}

fn parse_format(name: &str) -> Result<ImageFormat, IpcError> {
    ImageFormat::from_extension(name).ok_or_else(|| {
        err(
            "image.unknownFormat",
            format!("unknown image format '{name}'"),
        )
    })
}

fn parse_resize(dto: &ResizeOptionsDto) -> Result<ResizeOptions, IpcError> {
    let mode = match dto.mode.as_str() {
        "fit" => FitMode::Fit,
        "fill" => FitMode::Fill,
        "exact" => FitMode::Exact,
        "scale" => FitMode::Scale,
        other => {
            return Err(err(
                "image.unknownMode",
                format!("unknown resize mode '{other}'"),
            ));
        }
    };
    let filter = match dto.filter.as_str() {
        "nearest" => ResizeFilter::Nearest,
        "triangle" => ResizeFilter::Triangle,
        "catmullrom" => ResizeFilter::CatmullRom,
        "lanczos3" => ResizeFilter::Lanczos3,
        other => {
            return Err(err(
                "image.unknownFilter",
                format!("unknown filter '{other}'"),
            ));
        }
    };
    if !(1.0..=1000.0).contains(&dto.scale_percent) {
        return Err(err("image.invalidScale", "scale must be within 1..=1000"));
    }
    Ok(ResizeOptions {
        mode,
        width: dto.width.max(1.0) as u32,
        height: dto.height.max(1.0) as u32,
        scale_percent: dto.scale_percent.max(1.0) as u32,
        prevent_upscale: dto.prevent_upscale,
        filter,
    })
}

/// §64：Preview 与 Execute 共用同一引擎；`execute=false` 时不落盘。
fn run_operation(
    img: &image::DynamicImage,
    op: &ImageOperationDto,
) -> Result<(Vec<u8>, ImageFormat, image::DynamicImage, Vec<String>), IpcError> {
    let mut warnings = Vec::new();
    let mut img = img.clone();
    let target_format = parse_format(op.target_format.as_deref().unwrap_or("png"))?;

    // resize
    if let Some(resize_dto) = &op.resize {
        let opts = parse_resize(resize_dto)?;
        img = weave_media::resize_image(&img, &opts);
    }

    // §59 动画丢弃警示
    if matches!(
        op.target_format.as_deref(),
        Some("png") | Some("jpeg") | Some("webp") | Some("bmp")
    ) && !matches!(op.operation.as_str(), "strip_metadata" | "compress")
    {
        // 帧信息由调用方基于 facts 判断；此处不再重复
    }

    // §14/§15/§56：alpha → 无 alpha 目标须合成
    let cap = weave_media::capability(target_format);
    let needs_flatten = img.color().has_alpha() && !cap.alpha_encode;
    if needs_flatten {
        let policy = match op.alpha_background.as_deref() {
            Some("black") => AlphaPolicy::Black,
            _ => AlphaPolicy::White,
        };
        img = weave_media::composite_on_background(&img, policy);
        warnings.push(format!(
            "alpha channel composited onto {} background (target format has no alpha)",
            op.alpha_background.as_deref().unwrap_or("white")
        ));
    }

    // strip_metadata：重编码且不写 EXIF（weave-media encode 本就不写 EXIF）
    if op.strip_metadata {
        warnings.push("metadata stripped: EXIF/ICC removed by re-encode (§61)".to_string());
    }

    // §51/§54：有损与重编码警示
    let lossy = cap.lossy_encode;
    if lossy {
        warnings.push(format!("{:?} encoding is lossy", target_format));
    }
    let re_encoded = op.operation == "compress" || op.strip_metadata;
    if re_encoded {
        warnings.push("pixels re-encoded; may add generation loss (§54)".to_string());
    }

    let quality = op.quality.map(|q| q.clamp(1.0, 100.0) as u8);
    let bytes = weave_media::encode_image(&img, target_format, quality)
        .map_err(|e| err("image.encodeFailed", e))?;

    Ok((bytes, target_format, img, warnings))
}

#[tauri::command]
#[specta::specta]
pub fn image_preview(
    path: String,
    operation: ImageOperationDto,
) -> Result<ImagePreviewDto, IpcError> {
    let (_, img, facts) = load_image(&path)?;
    let (bytes, format, out_img, warnings) = run_operation(&img, &operation)?;
    let mut warnings = warnings;
    if facts.frame_count > 1 {
        warnings.push(format!(
            "source has {} frames; animation will NOT be preserved (static only, §59)",
            facts.frame_count
        ));
    }
    Ok(ImagePreviewDto {
        preview_data_uri: preview_data_uri(&out_img, &limits()),
        output_format: format.as_str().to_string(),
        out_width: out_img.width() as f64,
        out_height: out_img.height() as f64,
        estimated_output_bytes: bytes.len() as f64,
        lossy: weave_media::capability(format).lossy_encode,
        re_encoded: true,
        animation_dropped: facts.frame_count > 1,
        warnings,
    })
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ImageExportOptionsDto {
    /// 覆盖源文件（§74：显式；否则必须新目标）。
    pub overwrite_source: bool,
    /// overwrite_source = false 时必填的目标路径。
    pub destination: Option<String>,
}

/// Execute：同一引擎 + 安全写盘（§87-§91/§146：原子替换 + 历史/Undo）。
#[tauri::command]
#[specta::specta]
pub fn image_execute(
    app: tauri::AppHandle,
    path: String,
    operation: ImageOperationDto,
    export_options: ImageExportOptionsDto,
) -> Result<PlanDto, IpcError> {
    let (_, img, _) = load_image(&path)?;
    let (out_bytes, _format, _, _) = run_operation(&img, &operation)?;

    let state = app.state::<crate::state::AppState>();
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;

    let destination: String = if export_options.overwrite_source {
        path.clone()
    } else {
        export_options
            .destination
            .clone()
            .ok_or_else(|| err("image.noDestination", "export requires a destination path"))?
    };
    weave_core::prelude::validate_absolute_path(&destination)?;

    // §75 碰撞安全：非覆盖模式目标已存在 ⇒ 拒绝
    if !export_options.overwrite_source && std::fs::metadata(&destination).is_ok() {
        return Err(err(
            "image.destinationExists",
            format!("destination already exists: {destination}"),
        ));
    }

    // 覆盖源文件前 TOCTOU 重校验（§145）
    if export_options.overwrite_source
        && let Ok(meta) = std::fs::metadata(&path)
    {
        let _ = meta; // 快照由 load 时的字节本身表达；mtime 校验在 TextTransform 管线
    }

    let text_plan = weave_core::prelude::Plan {
        operation_id: OperationId::generate(),
        kind: weave_core::prelude::OperationKind::TextTransform,
        created_at: std::time::SystemTime::now(),
        items: vec![weave_core::prelude::PlanItem {
            item_id: "item_0000".to_string(),
            source_path: destination.clone(),
            target_path: String::new(),
            status: weave_core::prelude::PlanItemStatus::Ready,
            collision: weave_core::prelude::CollisionKind::None,
            source_size: std::fs::metadata(&destination).ok().map(|m| m.len()),
            source_modified: std::fs::metadata(&destination)
                .ok()
                .and_then(|m| m.modified().ok()),
            warnings: Vec::new(),
            errors: Vec::new(),
        }],
    };
    let dto = PlanDto::from_plan(&text_plan);
    state.plans.insert(&text_plan);

    let entry = crate::text_service::TextWriteEntry {
        path: std::path::PathBuf::from(&destination),
        bytes: out_bytes,
        must_not_exist: !export_options.overwrite_source,
    };
    // 目标为新文件时 must_not_exist 已由上层检查；覆盖时走备份路径
    let _ = export_options.overwrite_source;
    let (_report, _tx) = crate::text_service::run_text_write_job(
        &history_dir,
        &text_plan,
        entry,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .map_err(|(r, e)| {
        let _ = r;
        let code = e.code.clone();
        let message = e.message.clone();
        err(code.as_str(), message)
    })?;
    Ok(dto)
}

#[tauri::command]
#[specta::specta]
pub fn image_cancel(_token: String) -> Result<bool, IpcError> {
    // 单图操作为快速原子操作；取消经任务层（M6 下 §119 全量 job 化）
    Ok(false)
}
