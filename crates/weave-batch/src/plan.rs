//! 输入快照与流水线计划（M7 上 §10/§11/§19/§56-§58）。
//!
//! - 输入快照：Execute 前记录 path/size/mtime，用于 ChangedSincePreview
//!   重校验（§10）。
//! - Pipeline = Source + 0..N Filter + 0..N Transform + 1 Export（§11
//!   Linear Only；DAG/分支/循环显式禁止，§12）。
//! - 计划可序列化（serde）——可审计、可复现（§19）。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 输入快照条目（§10/§203：Path + Size + Mtime）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InputSnapshotEntry {
    pub path: PathBuf,
    pub size: u64,
    /// UNIX epoch 毫秒。
    pub modified_ms: u64,
}

/// 快照输入清单（§55/§202：collect → sort 确定性枚举）。
pub fn snapshot_inputs(files: Vec<PathBuf>) -> Result<Vec<InputSnapshotEntry>, String> {
    let mut out = Vec::with_capacity(files.len());
    for path in files {
        let meta = std::fs::metadata(&path)
            .map_err(|e| format!("input snapshot failed for {}: {e}", path.display()))?;
        let modified_ms = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);
        out.push(InputSnapshotEntry {
            path,
            size: meta.len(),
            modified_ms,
        });
    }
    // §55：确定性排序（路径字典序，不依赖 OS 枚举顺序）
    out.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(out)
}

/// Execute 前重校验快照（§10/§204：ChangedSincePreview / FileMissing）。
pub fn revalidate_snapshot(snap: &[InputSnapshotEntry]) -> Result<(), String> {
    for entry in snap {
        match std::fs::metadata(&entry.path) {
            Ok(meta) => {
                let modified_ms = meta
                    .modified()
                    .ok()
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                if meta.len() != entry.size || modified_ms != entry.modified_ms {
                    return Err(format!(
                        "ChangedSincePreview: {} (size {}→{}, mtime {}→{})",
                        entry.path.display(),
                        entry.size,
                        meta.len(),
                        entry.modified_ms,
                        modified_ms
                    ));
                }
            }
            Err(e) => {
                return Err(format!("FileMissing: {} ({e})", entry.path.display()));
            }
        }
    }
    Ok(())
}

/// 批量文本变换算子（映射 weave_text TransformKind；可序列化计划 §19）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TextOpSpec {
    TrimLines,
    Lowercase,
    Uppercase,
    Replace {
        find: String,
        replace_with: String,
        case_sensitive: bool,
    },
}

/// Pipeline 阶段种类（§11/§13：Source/Filter/Transform/Export；DAG 禁止 §12）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StageSpec {
    /// 从输入快照读取文件字节。
    Source,
    /// §14：决定条目是否继续（Rejected ≠ Failed）。
    Filter {
        /// 扩展名白名单（小写，不含点）。
        extensions_in: Vec<String>,
        /// 可选：最大字节（超出拒绝）。
        max_bytes: Option<u64>,
    },
    /// §15：将字节解码为文本并按序应用文本变换。
    TextTransform { operations: Vec<TextOpSpec> },
    /// 解码为图像并应用 resize（M6）。
    ImageResize {
        width: u32,
        height: u32,
        mode: String,
        prevent_upscale: bool,
    },
    /// 编码当前负载为目标格式（§45-§54：quality 按格式语义）。
    Encode {
        /// png | jpeg | webp | txt
        format: String,
        quality: Option<u8>,
    },
    /// §16 Export：写入输出目录（same stem + new ext；§76 命名）。
    Export {
        destination_dir: PathBuf,
        overwrite: bool,
    },
    /// M8（上 §113）：只读文档事实——不改负载，结果进 stage 日志。
    DocumentInspect,
    /// M8（上 §25/§58）：PDF 页旋转（/Rotate 元数据语义）。degrees ∈
    /// {90,180,270}；pages = 页范围串（空 = 全部页）。
    PdfRotate { degrees: f64, pages: String },
}

/// 线性流水线（§11：0..N Filter + 0..N Transform + 1 Export）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Pipeline {
    pub stages: Vec<StageSpec>,
}

impl Pipeline {
    /// §56/§57：结构校验——必须恰有一个 Source、至多一个 Export 且在末尾、
    /// 相邻阶段类型兼容（Text↔Text、Image↔Image、Bytes 为中转）。
    pub fn validate(&self) -> Result<(), String> {
        if self.stages.is_empty() {
            return Err("pipeline requires at least a Source stage".into());
        }
        if !matches!(self.stages[0], StageSpec::Source) {
            return Err("pipeline must start with a Source stage".into());
        }
        let export_count = self
            .stages
            .iter()
            .filter(|s| matches!(s, StageSpec::Export { .. }))
            .count();
        if export_count > 1 {
            return Err("pipeline must have at most one Export stage".into());
        }
        match self.stages.last() {
            Some(StageSpec::Export { .. }) | None => {}
            Some(_) => return Err("pipeline must end with an Export stage".into()),
        }

        // 类型跟踪（§57）：Bytes/Text/Image
        let mut current = PayloadType::Bytes;
        for (i, stage) in self.stages.iter().enumerate() {
            let next = match stage {
                StageSpec::Source => PayloadType::Bytes,
                StageSpec::Filter { .. } => current,
                StageSpec::TextTransform { .. } => {
                    if current != PayloadType::Text && current != PayloadType::Bytes {
                        return Err(format!(
                            "stage {i}: TextTransform requires Text input, got {current:?}"
                        ));
                    }
                    if current == PayloadType::Bytes {
                        // Source Bytes → decode_text 隐含（TextTransform 前自动解码）
                        PayloadType::Text
                    } else {
                        current
                    }
                }
                StageSpec::Encode { format, .. } => {
                    if format == "txt" {
                        if current != PayloadType::Text {
                            return Err(format!(
                                "stage {i}: txt encode requires Text payload, got {current:?}"
                            ));
                        }
                        PayloadType::Bytes
                    } else {
                        if current != PayloadType::Bytes && current != PayloadType::Image {
                            return Err(format!(
                                "stage {i}: {format} encode requires Image/Bytes payload, got {current:?}"
                            ));
                        }
                        PayloadType::Bytes
                    }
                }
                StageSpec::ImageResize { .. } => {
                    if current != PayloadType::Image && current != PayloadType::Bytes {
                        return Err(format!(
                            "stage {i}: ImageResize requires Image payload, got {current:?}"
                        ));
                    }
                    PayloadType::Image
                }
                StageSpec::Export { .. } => current,
                // M8：inspect 任意负载（只读旁路）；rotate 要求 Bytes ⇒ Bytes
                StageSpec::DocumentInspect => current,
                StageSpec::PdfRotate { .. } => {
                    if current != PayloadType::Bytes {
                        return Err(format!(
                            "stage {i}: PdfRotate requires Bytes payload, got {current:?}"
                        ));
                    }
                    PayloadType::Bytes
                }
            };
            current = next;
        }
        Ok(())
    }
}

/// 负载类型（§57 Input/OutputType 精简集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PayloadType {
    Bytes,
    Text,
    Image,
}

/// 完整 Job 计划（§19 JobPlan）：可序列化、可审计、可复现。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobPlan {
    pub input_snapshot: Vec<InputSnapshotEntry>,
    pub pipeline: Pipeline,
    /// 输出目录（Export stage 使用）。
    pub destination_dir: PathBuf,
    /// §24：默认 Continue（item 失败不放弃其余）。
    pub continue_on_error: bool,
    /// §41/§74：Export 覆盖已存在输出需要显式开启。
    pub overwrite_existing: bool,
}

/// 从路径清单构建快照 + 校验 pipeline（§19/§56）。
pub fn build_job_plan(
    inputs: Vec<PathBuf>,
    pipeline: Pipeline,
    destination_dir: PathBuf,
    continue_on_error: bool,
    overwrite_existing: bool,
) -> Result<JobPlan, String> {
    if inputs.is_empty() {
        return Err("job requires at least one input".into());
    }
    let input_snapshot = snapshot_inputs(inputs)?;
    if !destination_dir.is_dir() {
        return Err(format!(
            "destination dir missing: {}",
            destination_dir.display()
        ));
    }
    let plan = JobPlan {
        input_snapshot,
        pipeline,
        destination_dir,
        continue_on_error,
        overwrite_existing,
    };
    plan.pipeline.validate()?;
    Ok(plan)
}
