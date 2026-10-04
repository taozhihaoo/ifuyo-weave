//! Multi-image 批量处理（M6 下 §79–§90/§170–§175）。
//!
//! ImageBatchPlan = same operation + same options + multiple inputs（§79
//! ——不是通用 pipeline）。逐文件串行（§121 one-at-a-time）、失败隔离
//! （§84/§86：单文件 decode 失败 ⇒ Failed，其余继续）、协作取消
//! （§87-§89：已处理 N 条如实记录，未处理 = Cancelled）、输入快照
//! （§202：开始前确定文件清单，防导出文件回流）。

use std::path::{Path, PathBuf};

use weave_core::prelude::CancellationToken;

use crate::capabilities::capability;
use crate::ops::{AlphaPolicy, apply_exif_orientation, encode_image};
use crate::{ImageFormat, ImageLimits, inspect_bytes};

/// 批量操作（同操作同选项，§79）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImageBatchPlan {
    pub inputs: Vec<PathBuf>,
    pub destination_dir: PathBuf,
    pub target_format: ImageFormat,
    pub quality: Option<u8>,
    pub overwrite_existing: bool,
}

/// 单文件结果（§83）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BatchFileStatus {
    Success,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BatchFileResult {
    pub input: PathBuf,
    pub output: Option<PathBuf>,
    pub status: BatchFileStatus,
    pub error: Option<String>,
    pub input_bytes: u64,
    pub output_bytes: u64,
}

/// 批量结果（§82：Total/Succeeded/Failed/Skipped/Cancelled + 字节统计）。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct BatchResult {
    pub results: Vec<BatchFileResult>,
    pub total: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub skipped: u64,
    pub cancelled: u64,
    pub input_bytes: u64,
    pub output_bytes: u64,
}

/// 枚举输入快照（§202：开始前确定清单；仅六格式扩展名；确定性排序 §126；
/// 快照内排除目标目录中与 target 同扩展名的输出文件——§201 防自回流）。
pub fn snapshot_inputs(files: Vec<PathBuf>, target_format: ImageFormat) -> Vec<PathBuf> {
    let mut inputs: Vec<PathBuf> = files
        .into_iter()
        .filter(|p: &PathBuf| {
            p.is_file()
                && !p
                    .extension()
                    .and_then(|e| e.to_str())
                    .and_then(crate::ImageFormat::from_extension)
                    .is_some_and(|f| f == target_format)
        })
        .collect();
    inputs.sort();
    inputs
}

/// 单文件处理：decode → (resize 占位：v1 不带 resize 的批量) → orient 归一
/// → encode target → 写入 destination_dir/<stem>.<ext>。
/// 返回 Ok(输出路径, 输出字节) 或 Err(结构化原因)。
fn process_one(
    input: &Path,
    destination_dir: &Path,
    target_format: ImageFormat,
    quality: Option<u8>,
    overwrite_existing: bool,
    limits: &ImageLimits,
) -> Result<(PathBuf, u64, u64), String> {
    let bytes = std::fs::read(input).map_err(|e| format!("read failed: {e}"))?;
    let (img, facts) =
        inspect_bytes(bytes.as_slice(), None, limits).map_err(|e| format!("decode failed: {e}"))?;

    // §59：动画源在静态目标格式下显式拒绝（不静默取首帧）
    if facts.frame_count > 1 {
        return Err("animated source; static output would drop frames (NOT SUPPORTED §59)".into());
    }

    // §25/§27：物理归一 orientation（strip 语义），并检查目标 alpha 能力
    let normalized = apply_exif_orientation(&img, facts.metadata.orientation);
    let normalized = if facts.has_alpha && !capability(target_format).alpha_encode {
        crate::ops::composite_on_background(&normalized, AlphaPolicy::White)
    } else {
        normalized
    };

    let out_bytes = encode_image(&normalized, target_format, quality)
        .map_err(|e| format!("encode failed: {e}"))?;

    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "image".to_string());
    let output = destination_dir.join(format!("{stem}.{}", target_format.default_extension()));
    if output.exists() && !overwrite_existing {
        return Err(format!("destination already exists: {}", output.display()));
    }
    std::fs::write(&output, out_bytes.as_slice()).map_err(|e| format!("write failed: {e}"))?;

    Ok((output, bytes.len() as u64, out_bytes.len() as u64))
}

/// 执行批量（§84-§90）：逐文件串行 + 取消安全点 + 失败隔离。
pub fn run_batch(
    plan: &ImageBatchPlan,
    limits: &ImageLimits,
    cancel: &CancellationToken,
) -> BatchResult {
    let mut result = BatchResult {
        total: plan.inputs.len() as u64,
        ..BatchResult::default()
    };

    for input in &plan.inputs {
        // §87/§89：取消安全点——当前文件不再开始，如实记录 Cancelled
        if cancel.is_cancelled() {
            result.results.push(BatchFileResult {
                input: input.clone(),
                output: None,
                status: BatchFileStatus::Cancelled,
                error: None,
                input_bytes: 0,
                output_bytes: 0,
            });
            result.cancelled += 1;
            continue;
        }

        let input_bytes = std::fs::metadata(input).map(|m| m.len()).unwrap_or(0);
        match process_one(
            input,
            &plan.destination_dir,
            plan.target_format,
            plan.quality,
            plan.overwrite_existing,
            limits,
        ) {
            Ok((output, in_bytes, out_bytes)) => {
                result.results.push(BatchFileResult {
                    input: input.clone(),
                    output: Some(output),
                    status: BatchFileStatus::Success,
                    error: None,
                    input_bytes: in_bytes,
                    output_bytes: out_bytes,
                });
                result.succeeded += 1;
                result.input_bytes += in_bytes;
                result.output_bytes += out_bytes;
            }
            Err(reason) => {
                result.results.push(BatchFileResult {
                    input: input.clone(),
                    output: None,
                    status: BatchFileStatus::Failed,
                    error: Some(reason),
                    input_bytes,
                    output_bytes: 0,
                });
                result.failed += 1;
            }
        }
    }
    result
}

#[cfg(test)]
mod batch_tests;
