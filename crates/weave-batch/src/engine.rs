//! Batch Engine（M7 上 §20–§36）：同一 JobPlan 的执行；item 级失败隔离
//! （§25/§49）；协作取消（§32-§34）；Filter 拒绝 ⇒ Skipped（§14）。
//!
//! 诚实进度（§30）：per-item 计数；不伪造百分比。

use std::path::PathBuf;

use weave_core::prelude::CancellationToken;

use crate::item::{ItemContext, ItemPayload};
use crate::plan::{JobPlan, StageSpec};
use weave_media::ImageLimits;

/// 条目状态（§27）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ItemStatus {
    Success,
    Failed,
    Skipped,
    Cancelled,
}

/// 阶段结果（§28：哪个 stage 成功/失败/跳过）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StageResult {
    pub stage_index: usize,
    pub stage: String,
    pub ok: bool,
    pub note: String,
}

/// 条目结果（§27）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ItemResult {
    pub item_id: String,
    pub source: PathBuf,
    pub output: Option<PathBuf>,
    pub status: ItemStatus,
    pub error: Option<String>,
    pub stage_results: Vec<StageResult>,
    pub input_bytes: u64,
    pub output_bytes: u64,
}

/// Job 结果（§26 Result Model）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JobResult {
    pub items: Vec<ItemResult>,
    pub total: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub skipped: u64,
    pub cancelled: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
    /// true = Preview（§21：无副作用模拟）；false = Execute。
    pub preview: bool,
}

/// 阶段错误（§51 分类）。
#[derive(Debug, Clone)]
pub struct StageError {
    pub category: &'static str,
    pub message: String,
}

impl StageError {
    fn new(category: &'static str, message: impl Into<String>) -> Self {
        Self {
            category,
            message: message.into(),
        }
    }
}

fn stage_name(stage: &StageSpec) -> String {
    match stage {
        StageSpec::Source => "source".into(),
        StageSpec::Filter { .. } => "filter".into(),
        StageSpec::TextTransform { .. } => "text_transform".into(),
        StageSpec::ImageResize { .. } => "image_resize".into(),
        StageSpec::Encode { format, .. } => format!("encode:{format}"),
        StageSpec::Export { .. } => "export".into(),
    }
}

/// 单阶段执行（§13-§16）。Ok(Some(payload))=前进；Ok(None)=Filter 拒绝。
/// `dry_run`（§21 Preview）：Export 只做碰撞检查，不落盘。
fn run_stage(
    stage: &StageSpec,
    ctx: &mut ItemContext,
    dry_run: bool,
) -> Result<Option<ItemPayload>, StageError> {
    match stage {
        StageSpec::Source => {
            let bytes = std::fs::read(&ctx.source_path)
                .map_err(|e| StageError::new("Read", e.to_string()))?;
            ctx.current_ext = ctx
                .source_path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("bin")
                .to_ascii_lowercase();
            Ok(Some(ItemPayload::Bytes(bytes)))
        }
        StageSpec::Filter {
            extensions_in,
            max_bytes,
        } => {
            let ext = ctx.current_ext.as_str();
            let ext_ok = extensions_in.iter().any(|x| x.eq_ignore_ascii_case(ext));
            let size = ctx.source_path.metadata().map(|m| m.len()).unwrap_or(0);
            let over = max_bytes.is_some_and(|max| size > max);
            if !ext_ok || over {
                Ok(None) // §14 Rejected ⇒ Skipped（非 Failed）
            } else {
                Ok(Some(ctx.payload.clone()))
            }
        }
        StageSpec::TextTransform { operations } => {
            let text = match &ctx.payload {
                ItemPayload::Text(t) => t.clone(),
                ItemPayload::Bytes(b) => String::from_utf8(b.clone()).map_err(|_| {
                    StageError::new("Decode", "payload is not valid UTF-8".to_string())
                })?,
                other => {
                    return Err(StageError::new(
                        "Unsupported",
                        format!(
                            "TextTransform requires text payload, got {}",
                            other.type_name()
                        ),
                    ));
                }
            };
            // §78：按序应用批量文本算子（batch 计划侧映射到 TransformKind）
            let mut current = text;
            for op in operations {
                let kind =
                    batch_text_op_to_kind(op).map_err(|e| StageError::new("Validation", e))?;
                let out = weave_text::apply_transform(&current, &kind).map_err(|e| {
                    StageError::new("Transform", format!("{}: {}", e.code, e.message))
                })?;
                current = out.content;
            }
            Ok(Some(ItemPayload::Text(current)))
        }
        StageSpec::ImageResize {
            width,
            height,
            mode,
            prevent_upscale,
        } => {
            let bytes = match &ctx.payload {
                ItemPayload::Bytes(b) => b.clone(),
                other => {
                    return Err(StageError::new(
                        "Unsupported",
                        format!(
                            "ImageResize requires bytes payload, got {}",
                            other.type_name()
                        ),
                    ));
                }
            };
            let (img, _) = weave_media::inspect_bytes(&bytes, None, &ImageLimits::default())
                .map_err(|e| StageError::new("Decode", e))?;
            let mode = match mode.as_str() {
                "fit" => weave_media::FitMode::Fit,
                "fill" => weave_media::FitMode::Fill,
                "exact" => weave_media::FitMode::Exact,
                "scale" => weave_media::FitMode::Scale,
                other => {
                    return Err(StageError::new(
                        "Validation",
                        format!("unknown resize mode '{other}'"),
                    ));
                }
            };
            let out = weave_media::resize_image(
                &img,
                &weave_media::ResizeOptions {
                    mode,
                    width: *width,
                    height: *height,
                    scale_percent: 100,
                    prevent_upscale: *prevent_upscale,
                    filter: weave_media::ResizeFilter::Lanczos3,
                },
            );
            // PNG 无损中转；最终格式由 Encode 阶段决定
            let mut png = Vec::new();
            image::DynamicImage::write_to(
                &out,
                &mut std::io::Cursor::new(&mut png),
                image::ImageFormat::Png,
            )
            .map_err(|e| StageError::new("Encode", e.to_string()))?;
            ctx.current_ext = "png".into();
            Ok(Some(ItemPayload::Bytes(png)))
        }
        StageSpec::Encode { format, quality } => {
            let bytes = match &ctx.payload {
                ItemPayload::Bytes(b) => b.clone(),
                ItemPayload::Text(t) => t.clone().into_bytes(),
                ItemPayload::Image(i) => {
                    let fmt =
                        weave_media::ImageFormat::from_extension(format).ok_or_else(|| {
                            StageError::new("Unsupported", format!("unknown format '{format}'"))
                        })?;
                    weave_media::encode_image(i, fmt, *quality)
                        .map_err(|e| StageError::new("Encode", e))?
                }
            };
            ctx.current_ext = format.clone();
            Ok(Some(ItemPayload::Bytes(bytes)))
        }
        StageSpec::Export {
            destination_dir,
            overwrite,
        } => {
            let bytes = match &ctx.payload {
                ItemPayload::Bytes(b) => b.clone(),
                ItemPayload::Text(t) => t.clone().into_bytes(),
                ItemPayload::Image(i) => {
                    let fmt = weave_media::ImageFormat::from_extension(&ctx.current_ext)
                        .unwrap_or(weave_media::ImageFormat::Png);
                    // Export 中转编码用格式默认 quality（显式 quality 属 Encode 阶段）
                    weave_media::encode_image(i, fmt, None)
                        .map_err(|e| StageError::new("Encode", e))?
                }
            };
            let stem = ctx
                .source_path
                .file_stem()
                .map(|s| s.to_string_lossy().into_owned())
                .unwrap_or_else(|| "item".to_string());
            let out_path = destination_dir.join(format!("{stem}.{}", ctx.current_ext));
            if out_path.exists() && !overwrite {
                // Preview 也报告碰撞——§22 Potential Failures
                return Err(StageError::new(
                    "Collision",
                    format!("destination already exists: {}", out_path.display()),
                ));
            }
            if !dry_run {
                std::fs::write(&out_path, &bytes)
                    .map_err(|e| StageError::new("Write", e.to_string()))?;
            }
            ctx.stage_log.push((
                if dry_run { "export:preview" } else { "export" }.into(),
                out_path.to_string_lossy().into_owned(),
            ));
            Ok(Some(ItemPayload::Bytes(bytes)))
        }
    }
}

/// 批量文本算子 → weave_text TransformKind（§19 可序列化计划的执行侧）。
fn batch_text_op_to_kind(
    op: &crate::plan::TextOpSpec,
) -> Result<weave_text::TransformKind, String> {
    use weave_text::TransformKind as K;
    Ok(match op {
        crate::plan::TextOpSpec::TrimLines => K::TrimLines,
        crate::plan::TextOpSpec::Lowercase => K::CaseConvert {
            form: weave_text::CaseForm::Lower,
        },
        crate::plan::TextOpSpec::Uppercase => K::CaseConvert {
            form: weave_text::CaseForm::Upper,
        },
        crate::plan::TextOpSpec::Replace {
            find,
            replace_with,
            case_sensitive,
        } => K::FindReplace {
            find: find.clone(),
            replacement: replace_with.clone(),
            regex: false,
            case_insensitive: !*case_sensitive,
            first_only: false,
        },
    })
}

/// Batch Engine 执行（§20-§36）：
/// - item 级失败隔离（§25/§49）：单条失败不放弃其余；
/// - §33 取消：未开始 ⇒ Cancelled；流水线中途 ⇒ 该条 Cancelled；
/// - Filter 拒绝 ⇒ Skipped（§14 Rejected ≠ Failed）；
/// - Export 产物路径记入 ItemResult.output（§70 实际产物）。
pub fn execute_plan(
    plan: &JobPlan,
    cancel: &CancellationToken,
    on_progress: Option<&mut dyn FnMut(u64)>,
) -> JobResult {
    run_job(plan, cancel, false, on_progress)
}

/// Preview（§20-§23）：与 Execute 同一引擎、同一 JobPlan，仅 Export 不落盘
/// （§21 No destructive mutation）；碰撞仍作为潜在失败上报（§22）。
pub fn preview_plan(plan: &JobPlan) -> JobResult {
    let mut result = run_job(plan, &CancellationToken::new(), true, None);
    result.preview = true;
    result
}

fn run_job(
    plan: &JobPlan,
    cancel: &CancellationToken,
    dry_run: bool,
    mut on_progress: Option<&mut dyn FnMut(u64)>,
) -> JobResult {
    let mut result = JobResult {
        total: plan.input_snapshot.len() as u64,
        ..JobResult::default()
    };

    for (n, entry) in plan.input_snapshot.iter().enumerate() {
        // §33 取消安全点：未开始 ⇒ Cancelled
        if cancel.is_cancelled() {
            result.cancelled += 1;
            result.items.push(ItemResult {
                item_id: format!("item_{n}"),
                source: entry.path.clone(),
                output: None,
                status: ItemStatus::Cancelled,
                error: None,
                stage_results: Vec::new(),
                input_bytes: entry.size,
                output_bytes: 0,
            });
            continue;
        }

        let mut ctx = ItemContext {
            item_id: format!("item_{n}"),
            source_path: entry.path.clone(),
            payload: ItemPayload::Bytes(Vec::new()),
            current_ext: "bin".into(),
            stage_log: Vec::new(),
        };
        let input_bytes = entry.size;
        let mut stage_results: Vec<StageResult> = Vec::new();
        let mut failed: Option<StageError> = None;
        let mut rejected = false;
        let mut cancelled_mid = false;

        for (i, stage) in plan.pipeline.stages.iter().enumerate() {
            // §34 取消安全点：阶段间即安全点（与 Safe Write 结合）
            if cancel.is_cancelled() {
                cancelled_mid = true;
                stage_results.push(StageResult {
                    stage_index: i,
                    stage: stage_name(stage),
                    ok: false,
                    note: "cancelled".into(),
                });
                break;
            }
            match run_stage(stage, &mut ctx, dry_run) {
                Ok(Some(payload)) => {
                    ctx.payload = payload;
                    stage_results.push(StageResult {
                        stage_index: i,
                        stage: stage_name(stage),
                        ok: true,
                        note: String::new(),
                    });
                }
                Ok(None) => {
                    rejected = true;
                    stage_results.push(StageResult {
                        stage_index: i,
                        stage: stage_name(stage),
                        ok: false,
                        note: "rejected by filter".into(),
                    });
                    break;
                }
                Err(e) => {
                    stage_results.push(StageResult {
                        stage_index: i,
                        stage: stage_name(stage),
                        ok: false,
                        note: e.message.clone(),
                    });
                    failed = Some(e);
                    break;
                }
            }
        }

        let status = if cancelled_mid {
            ItemStatus::Cancelled
        } else if rejected {
            ItemStatus::Skipped
        } else if failed.is_some() {
            ItemStatus::Failed
        } else {
            ItemStatus::Success
        };

        // §70：Export 产物路径（成功时才有）
        let output_path = if status == ItemStatus::Success {
            plan.pipeline.stages.iter().rev().find_map(|s| match s {
                StageSpec::Export {
                    destination_dir, ..
                } => {
                    let stem = entry
                        .path
                        .file_stem()
                        .map(|s| s.to_string_lossy().into_owned())
                        .unwrap_or_else(|| "item".into());
                    Some(destination_dir.join(format!("{stem}.{}", ctx.current_ext)))
                }
                _ => None,
            })
        } else {
            None
        };

        // §70：产物字节——Execute 读落盘文件；Preview（未落盘）取内存负载长度
        let output_bytes = if dry_run {
            ctx.payload.as_bytes().map_or(0, |b| b.len() as u64)
        } else {
            output_path
                .as_ref()
                .and_then(|p| std::fs::metadata(p).ok())
                .map(|m| m.len())
                .unwrap_or(0)
        };

        match status {
            ItemStatus::Success => result.succeeded += 1,
            ItemStatus::Failed => result.failed += 1,
            ItemStatus::Skipped => result.skipped += 1,
            ItemStatus::Cancelled => result.cancelled += 1,
        }
        result.items.push(ItemResult {
            item_id: format!("item_{n}"),
            source: entry.path.clone(),
            output: output_path,
            status,
            error: failed.map(|e| format!("{}: {}", e.category, e.message)),
            stage_results,
            input_bytes,
            output_bytes,
        });
        // §30 诚实进度：per-item 已完成计数，不伪造百分比
        if let Some(cb) = on_progress.as_mut() {
            cb(result.items.len() as u64);
        }
    }
    result
}
