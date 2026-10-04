//! Batch Engine（M7 上 §20–§36 + 下 §123/§126/§201）：同一 JobPlan 的
//! 执行；item 级失败隔离（§25/§49）；协作取消（§32-§34）；Filter 拒绝
//! ⇒ Skipped（§14）。
//!
//! 诚实进度（§30）：per-item 计数；不伪造百分比；回调只在调用线程触发。
//! 有界并发（下 §123/§211）：workers 参数（≥1）；结果按快照序还原，
//! 与 worker 数无关（确定性 §54/§124——批内输出名预claim 快照序小者胜）。
//! Pause（下 §90/§143）：安全点检查 pause 旗标——未开始条目计入
//! `pending`（非终态；Resume 走 journal 子集执行）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::mpsc;

use weave_core::prelude::CancellationToken;

use crate::item::{ItemContext, ItemPayload};
use crate::plan::{InputSnapshotEntry, JobPlan, StageSpec};
use weave_media::ImageLimits;

/// 条目状态（§27）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ItemStatus {
    Success,
    Failed,
    Skipped,
    Cancelled,
}

/// 阶段结果（§28：哪个 stage 成功/失败/跳过）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StageResult {
    pub stage_index: usize,
    pub stage: String,
    pub ok: bool,
    pub note: String,
}

/// 条目结果（§27）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ItemResult {
    pub item_id: String,
    pub source: PathBuf,
    pub output: Option<PathBuf>,
    pub status: ItemStatus,
    pub error: Option<String>,
    /// 下 §142/§237：Failed 时是否可 Retry（Validation/Unsupported ⇒ 否）。
    pub retryable: bool,
    /// 输出目标执行前已存在（覆盖写）——History 撤销策略依据。
    pub output_replaced: bool,
    pub stage_results: Vec<StageResult>,
    pub input_bytes: u64,
    pub output_bytes: u64,
}

/// Job 结果（§26 Result Model + 下 pending）。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JobResult {
    pub items: Vec<ItemResult>,
    pub total: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub skipped: u64,
    pub cancelled: u64,
    /// Pause 安全点后未开始条目（非终态；Resume 重跑，下 §143）。
    pub pending: u64,
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

    /// 下 §142/§237：非瞬态（计划/类型错误）Retry 无意义。
    pub fn is_retryable(&self) -> bool {
        !matches!(self.category, "Validation" | "Unsupported")
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
        StageSpec::DocumentInspect => "document_inspect".into(),
        StageSpec::PdfRotate { degrees, .. } => format!("pdf_rotate:{degrees}"),
    }
}

/// 静态推导条目最终扩展名（与运行时 current_ext 转移一致：Resize⇒png、
/// Encode⇒format、其余不变）。用于批内输出名预claim（下 §124 确定性）。
fn intended_ext(plan: &JobPlan, entry: &InputSnapshotEntry) -> String {
    let mut ext = entry
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("bin")
        .to_ascii_lowercase();
    for stage in &plan.pipeline.stages {
        match stage {
            StageSpec::ImageResize { .. } => ext = "png".into(),
            StageSpec::Encode { format, .. } => ext = format.clone(),
            _ => {}
        }
    }
    ext
}

/// 批内输出名预claim（下 §124）：同 Job 两条 item 目标同名时，**快照序
/// 小者胜**——预计算、与 worker 数无关（结果确定性不因并发改变）。
fn claim_outputs(plan: &JobPlan, indices: &[usize]) -> HashMap<PathBuf, usize> {
    let mut claimed: HashMap<PathBuf, usize> = HashMap::new();
    let export_dir = plan
        .pipeline
        .stages
        .iter()
        .find_map(|s| match s {
            StageSpec::Export {
                destination_dir, ..
            } => Some(destination_dir.clone()),
            _ => None,
        })
        .unwrap_or_else(|| plan.destination_dir.clone());
    for &i in indices {
        let entry = &plan.input_snapshot[i];
        let stem = entry
            .path
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_else(|| "item".into());
        let out = export_dir.join(format!("{stem}.{}", intended_ext(plan, entry)));
        claimed.entry(out).or_insert(i);
    }
    claimed
}

/// 单阶段执行（§13-§16）。Ok(Some(payload))=前进；Ok(None)=Filter 拒绝。
/// `dry_run`（§21 Preview）：Export 只做碰撞检查，不落盘。
fn run_stage(
    stage: &StageSpec,
    ctx: &mut ItemContext,
    dry_run: bool,
    claimed: &HashMap<PathBuf, usize>,
    item_index: usize,
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
            // 批内重名：预claim 快照序小者胜（下 §124 确定性，与 worker 数无关）
            if claimed
                .get(&out_path)
                .is_some_and(|&owner| owner != item_index)
            {
                return Err(StageError::new(
                    "Collision",
                    format!(
                        "internal duplicate output: item {} claims {}",
                        claimed[&out_path],
                        out_path.display()
                    ),
                ));
            }
            let replaced_existing = out_path.exists();
            if replaced_existing && !overwrite {
                // Preview 也报告碰撞——§22 Potential Failures
                return Err(StageError::new(
                    "Collision",
                    format!("destination already exists: {}", out_path.display()),
                ));
            }
            if !dry_run {
                ctx.output_replaced = replaced_existing;
                std::fs::write(&out_path, &bytes)
                    .map_err(|e| StageError::new("Write", e.to_string()))?;
            }
            ctx.stage_log.push((
                if dry_run { "export:preview" } else { "export" }.into(),
                out_path.to_string_lossy().into_owned(),
            ));
            Ok(Some(ItemPayload::Bytes(bytes)))
        }
        StageSpec::DocumentInspect => {
            // M8 §113：只读检查——不改负载；关键事实进 stage 日志
            let facts = weave_documents::inspect_document(
                &ctx.source_path,
                &weave_documents::DocumentResourceLimits::default(),
            );
            let pages = match facts.pages.as_ref() {
                Some(weave_documents::Field::Known(n)) => n.to_string(),
                Some(weave_documents::Field::Estimated(n)) => format!("~{n}"),
                _ => "unknown".into(),
            };
            let note = format!(
                "{} pages={pages} size={}",
                facts.format.as_str(),
                facts.size
            );
            ctx.stage_log
                .push(("document_inspect".into(), note.clone()));
            ctx.last_stage_note = note;
            Ok(Some(ctx.payload.clone()))
        }
        StageSpec::PdfRotate { degrees, pages } => {
            let bytes = match &ctx.payload {
                ItemPayload::Bytes(b) => b.clone(),
                other => {
                    return Err(StageError::new(
                        "Unsupported",
                        format!(
                            "PdfRotate requires bytes payload, got {}",
                            other.type_name()
                        ),
                    ));
                }
            };
            // 内存 rotate 走 weave-documents 域函数（§58/§218：无第二套 PDF 逻辑）
            let (out_bytes, _total, _rotated) = weave_documents::rotate_pdf_bytes(
                &bytes,
                *degrees as i64,
                pages,
                &weave_documents::DocumentResourceLimits::default(),
            )
            .map_err(|e| {
                StageError::new(
                    match e.code {
                        "pdf.badRange" | "pdf.badRotation" | "pdf.tooManyPages" => "Validation",
                        "pdf.encrypted" => "Unsupported",
                        _ => "Decode",
                    },
                    format!("{}: {}", e.code, e.message),
                )
            })?;
            ctx.current_ext = "pdf".into();
            Ok(Some(ItemPayload::Bytes(out_bytes)))
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

/// 进度回调（§30：已完成计数 + 当前条目描述；仅调用线程触发）。
pub type ProgressCallback<'a> = &'a mut dyn FnMut(u64, &str);

/// 执行参数（下 §123/§211：Concurrency = Bounded）。
pub struct JobExecOptions<'a> {
    /// worker 数（≥1；1 = 顺序执行）。
    pub workers: usize,
    /// Pause 旗标（下 §90/§143）：安全点置位 ⇒ 未开始条目计入 pending。
    pub pause: Option<&'a AtomicBool>,
    /// §21 Preview dry-run。
    pub dry_run: bool,
    /// 条目完成事实（Journal 追加点，下 §42）：worker 线程调用（Sync 约束）。
    pub on_item: Option<&'a (dyn Fn(&ItemResult) + Sync)>,
}

/// Batch Engine 执行（§20-§36）：
/// - item 级失败隔离（§25/§49）：单条失败不放弃其余；
/// - §33 取消：未开始 ⇒ Cancelled；流水线中途 ⇒ 该条 Cancelled；
/// - Filter 拒绝 ⇒ Skipped（§14 Rejected ≠ Failed）；
/// - Export 产物路径记入 ItemResult.output（§70 实际产物）。
pub fn execute_plan(
    plan: &JobPlan,
    cancel: &CancellationToken,
    on_progress: Option<ProgressCallback<'_>>,
) -> JobResult {
    let indices: Vec<usize> = (0..plan.input_snapshot.len()).collect();
    execute_subset(
        plan,
        &indices,
        JobExecOptions {
            workers: 1,
            pause: None,
            dry_run: false,
            on_item: None,
        },
        cancel,
        on_progress,
    )
}

/// Preview（§20-§23）：与 Execute 同一引擎、同一 JobPlan，仅 Export 不落盘
/// （§21 No destructive mutation）；碰撞仍作为潜在失败上报（§22）。
pub fn preview_plan(plan: &JobPlan) -> JobResult {
    let indices: Vec<usize> = (0..plan.input_snapshot.len()).collect();
    let mut result = execute_subset(
        plan,
        &indices,
        JobExecOptions {
            workers: 1,
            pause: None,
            dry_run: true,
            on_item: None,
        },
        &CancellationToken::new(),
        None,
    );
    result.preview = true;
    result
}

/// 子集执行（下 §39 Retry / §126 Resume 的共同机制）：只跑 `indices`
/// 列出的快照条目，结果按快照序还原（与 workers 无关，§54/§123）。
/// 进度回调（§30）仅在调用线程触发（worker 事件经 channel 汇聚）。
pub fn execute_subset(
    plan: &JobPlan,
    indices: &[usize],
    opts: JobExecOptions<'_>,
    cancel: &CancellationToken,
    mut on_progress: Option<ProgressCallback<'_>>,
) -> JobResult {
    let workers = opts.workers.max(1);
    let claimed = claim_outputs(plan, indices);
    let next = AtomicU64::new(0);
    let done = AtomicU64::new(0);
    let count = indices.len();
    let spawned = workers.min(count);
    let slots: Vec<Mutex<Option<ItemResult>>> = indices.iter().map(|_| Mutex::new(None)).collect();
    let (tx, rx) = mpsc::channel::<(u64, String)>();
    // 共享状态打成引用元组：worker 闭包 move 一个引用拷贝（Send），其余
    // 共享数据保持借用；tx 每 worker 一个 clone，原始 tx 在 drain 前 drop
    // ——channel 关闭 = workers 全部退出（否则 rx 循环永不结束）。
    let shared = (&plan, &slots, &indices, &claimed, cancel, &next, &done);

    std::thread::scope(|scope| {
        for _ in 0..spawned {
            let tx_worker = tx.clone();
            scope.spawn(move || {
                let (plan, slots, indices, claimed, cancel, next, done) = shared;
                loop {
                    // §33/§34 安全点：取消/暂停 ⇒ 停止取新条目
                    if cancel.is_cancelled() {
                        return;
                    }
                    if opts.pause.is_some_and(|p| p.load(Ordering::SeqCst)) {
                        return;
                    }
                    let n = next.fetch_add(1, Ordering::SeqCst) as usize;
                    if n >= count {
                        return;
                    }
                    let idx = indices[n];
                    let result = run_one_item(plan, idx, opts.dry_run, claimed, cancel);
                    if let Some(cb) = opts.on_item {
                        cb(&result);
                    }
                    *slots[n].lock().expect("slot") = Some(result);
                    let d = done.fetch_add(1, Ordering::SeqCst) + 1;
                    let desc = plan
                        .input_snapshot
                        .get(idx)
                        .and_then(|e| e.path.file_name().map(|s| s.to_string_lossy().into_owned()))
                        .unwrap_or_default();
                    // 接收端消失（理论不可达）不算 item 失败——进度是尽力而为
                    let _ = tx_worker.send((d, desc));
                }
            });
        }
        drop(tx);
        // 调用线程汇聚进度事件（§30 计数；回调不出调用线程）
        for (d, desc) in rx {
            if let Some(cb) = on_progress.as_mut() {
                cb(d, &desc);
            }
        }
    });

    let mut result = JobResult {
        total: plan.input_snapshot.len() as u64,
        ..JobResult::default()
    };
    let paused = opts.pause.is_some_and(|p| p.load(Ordering::SeqCst));
    let mut pending: u64 = 0;
    for (pos, slot) in slots.into_iter().enumerate() {
        match slot.into_inner().expect("no poison") {
            Some(mut item) => {
                match item.status {
                    ItemStatus::Success => result.succeeded += 1,
                    ItemStatus::Failed => result.failed += 1,
                    ItemStatus::Skipped => result.skipped += 1,
                    ItemStatus::Cancelled => result.cancelled += 1,
                }
                result.input_bytes += item.input_bytes;
                result.output_bytes += item.output_bytes;
                item.item_id = format!("item_{}", indices[pos]);
                result.items.push(item);
            }
            None => {
                if paused && !cancel.is_cancelled() {
                    // 下 §143：Pause ⇒ 未开始条目 = pending（非终态）
                    pending += 1;
                } else {
                    // §33：取消 ⇒ 未开始条目终态 Cancelled
                    let entry = &plan.input_snapshot[indices[pos]];
                    result.cancelled += 1;
                    result.items.push(ItemResult {
                        item_id: format!("item_{}", indices[pos]),
                        source: entry.path.clone(),
                        output: None,
                        status: ItemStatus::Cancelled,
                        error: None,
                        retryable: false,
                        output_replaced: false,
                        stage_results: Vec::new(),
                        input_bytes: entry.size,
                        output_bytes: 0,
                    });
                }
            }
        }
    }
    result.pending = pending;
    // slots 已按快照序排列；items 顺序即快照序（§54 确定性）
    result
}

/// 单条目全管线（失败隔离边界 = 一条 item）。
fn run_one_item(
    plan: &JobPlan,
    idx: usize,
    dry_run: bool,
    claimed: &HashMap<PathBuf, usize>,
    cancel: &CancellationToken,
) -> ItemResult {
    let entry = &plan.input_snapshot[idx];
    let mut ctx = ItemContext {
        item_id: format!("item_{idx}"),
        source_path: entry.path.clone(),
        payload: ItemPayload::Bytes(Vec::new()),
        current_ext: String::new(),
        output_replaced: false,
        last_stage_note: String::new(),
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
        match run_stage(stage, &mut ctx, dry_run, claimed, idx) {
            Ok(Some(payload)) => {
                ctx.payload = payload;
                stage_results.push(StageResult {
                    stage_index: i,
                    stage: stage_name(stage),
                    ok: true,
                    note: std::mem::take(&mut ctx.last_stage_note),
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

    let retryable = failed.as_ref().is_none_or(|e| e.is_retryable());
    ItemResult {
        item_id: format!("item_{idx}"),
        source: entry.path.clone(),
        output: output_path,
        status,
        error: failed.map(|e| format!("{}: {}", e.category, e.message)),
        retryable,
        output_replaced: ctx.output_replaced,
        stage_results,
        input_bytes,
        output_bytes,
    }
}
