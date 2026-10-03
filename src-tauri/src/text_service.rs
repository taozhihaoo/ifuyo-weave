//! M4 Text 应用服务（load/format/transform/extract/compare + 安全写回）。
//!
//! 职责（M4 §10/§93/§94）：parse/校验 → weave-text 域调用 → DTO；
//! 写回复用 M2 基础设施（Plan/Transaction/History/Undo），绝不另建一套。
//!
//! 写回设计（§89–§94）：
//! - Plan item 快照 = **加载时**的 size/mtime；执行前 Revalidate，
//!   外部改动 ⇒ `text.fileChangedSincePreview`（§90 TOCTOU）。
//! - 写前先把当前原件复制为备份（`.{name}.weave-text-bak-{op}`），
//!   原子替换（weave-files::atomic_write，§91/§92）。
//! - 事务条目 source=原文件、target=备份、original_size/modified=**写后**
//!   状态 —— M2 undo 的占用检查由此天然实现 §94："写回后用户又改过 ⇒
//!   stat 不符 ⇒ UndoConflict 拒绝覆盖"。

// IPC 边界 Err DTO 体积豁免（同 D10）。
#![expect(clippy::result_large_err)]

use std::path::PathBuf;
use std::sync::Mutex;

use weave_core::prelude::{CancellationToken, OperationId, Plan, Progress, WeaveError};
use weave_text::model::{SourceKind, TextDocument, TextFormat};

use crate::commands::IpcError;

/// 文本工具输入上限（M4 下 §104 前置；集中常量，D43）。
pub const MAX_TEXT_BYTES: u64 = 2 * 1024 * 1024;

/// 写回内容缓存：build → execute 的会话内衔接（同 PlanCache 纪律）。
#[derive(Default)]
pub struct TextWriteCache {
    entries: Mutex<std::collections::HashMap<String, TextWriteEntry>>,
}

#[derive(Clone)]
pub struct TextWriteEntry {
    pub path: PathBuf,
    pub bytes: Vec<u8>,
}

impl TextWriteCache {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&self, operation_id: &OperationId, entry: TextWriteEntry) {
        self.entries
            .lock()
            .expect("text cache")
            .insert(operation_id.to_string(), entry);
    }

    pub fn take(&self, operation_id: &str) -> Option<TextWriteEntry> {
        self.entries
            .lock()
            .expect("text cache")
            .remove(operation_id)
    }
}

fn std_fs() -> &'static weave_files::fs::StdFilesystem {
    &weave_files::fs::StdFilesystem
}

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("text_service")
        .into()
}

/// DTO 层共用的结构化校验错误构造。
pub fn err_bare(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message).into()
}

/// 加载文本文档（§14 Dropped/Opened File）：解码级联 + 二进制守卫 + 尺寸上限。
pub fn load_text_document(
    raw_path: &str,
    encoding_override: Option<&str>,
) -> Result<crate::text_dto::TextDocumentDto, IpcError> {
    weave_core::prelude::validate_absolute_path(raw_path)?;
    let meta = std::fs::metadata(raw_path).map_err(|e| err("text.loadFailed", e.to_string()))?;
    if meta.len() > MAX_TEXT_BYTES {
        return Err(err(
            "text.tooLarge",
            format!("file exceeds {MAX_TEXT_BYTES} bytes"),
        ));
    }
    let bytes = std::fs::read(raw_path).map_err(|e| err("text.loadFailed", e.to_string()))?;
    // 二进制守卫（M4 下 §103 前置）：前 8 KiB 出现 NUL ⇒ 拒绝当文本处理
    if bytes[..bytes.len().min(8192)].contains(&0) {
        return Err(err(
            "text.binaryDetected",
            "file looks binary (NUL byte in first 8 KiB); text tools refuse to guess",
        ));
    }
    let decoded = match encoding_override {
        Some(name) => {
            let encoding = parse_encoding(name)?;
            weave_text::decode_as(encoding, &bytes).map_err(|e| err(e.code, e.message))?
        }
        None => weave_text::decode(&bytes).map_err(|e| err(e.code, e.message))?,
    };
    let extension = std::path::Path::new(raw_path)
        .extension()
        .map(|e| e.to_string_lossy().into_owned());
    let format = weave_text::detect_format(extension.as_deref(), &decoded.content);
    let modified = meta
        .modified()
        .ok()
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_millis() as f64);

    let ends_with_newline = decoded.content.ends_with('\n') || decoded.content.ends_with('\r');
    Ok(crate::text_dto::TextDocumentDto {
        path: raw_path.to_string(),
        content: decoded.content,
        encoding: encoding_to_str(decoded.encoding).to_string(),
        bom: bom_to_str(decoded.bom).to_string(),
        format: format.as_str().to_string(),
        byte_size: meta.len() as f64,
        modified_ms: modified,
        ends_with_newline,
    })
}

fn parse_encoding(name: &str) -> Result<weave_core::prelude::TextEncoding, IpcError> {
    use weave_core::prelude::TextEncoding;
    Ok(match name {
        "utf8" => TextEncoding::Utf8,
        "utf8Bom" => TextEncoding::Utf8Bom,
        "utf16Le" => TextEncoding::Utf16Le,
        "utf16Be" => TextEncoding::Utf16Be,
        "ascii" => TextEncoding::Ascii,
        "gb18030" => TextEncoding::Gb18030,
        "gbk" => TextEncoding::Gbk,
        "latin1" => TextEncoding::Latin1,
        other => return Err(err("text.unknownEncoding", format!("unknown encoding '{other}'"))),
    })
}

pub fn encoding_to_str(
    encoding: weave_core::prelude::TextEncoding,
) -> &'static str {
    use weave_core::prelude::TextEncoding;
    match encoding {
        TextEncoding::Utf8 => "utf8",
        TextEncoding::Utf8Bom => "utf8Bom",
        TextEncoding::Utf16Le => "utf16Le",
        TextEncoding::Utf16Be => "utf16Be",
        TextEncoding::Ascii => "ascii",
        TextEncoding::Gb18030 => "gb18030",
        TextEncoding::Gbk => "gbk",
        TextEncoding::Latin1 => "latin1",
        TextEncoding::Unknown => "unknown",
    }
}

pub fn bom_to_str(bom: weave_text::Bom) -> &'static str {
    match bom {
        weave_text::Bom::Utf8 => "utf8",
        weave_text::Bom::Utf16Le => "utf16Le",
        weave_text::Bom::Utf16Be => "utf16Be",
        weave_text::Bom::None => "none",
    }
}

fn parse_format(name: &str) -> Result<TextFormat, IpcError> {
    Ok(match name {
        "json" => TextFormat::Json,
        "xml" => TextFormat::Xml,
        "yaml" => TextFormat::Yaml,
        "sql" => TextFormat::Sql,
        "javascript" => TextFormat::JavaScript,
        "css" => TextFormat::Css,
        "markdown" => TextFormat::Markdown,
        other => return Err(err("text.unknownFormat", format!("unknown format '{other}'"))),
    })
}

/// Format 预览（§85/§86：纯函数、无副作用）。
pub fn format_text(
    format: &str,
    operation: &str,
    content: &str,
    indent_spaces: f64,
    final_newline: bool,
) -> Result<crate::text_dto::FormatOutcomeDto, IpcError> {
    let too_big = content.len() as u64 > MAX_TEXT_BYTES;
    if too_big {
        return Err(err("text.tooLarge", "input exceeds the text size limit"));
    }
    let format = parse_format(format)?;
    let operation = parse_operation(operation)?;
    let options = weave_text::format::FormatOptions {
        indent_spaces: indent_spaces.clamp(0.0, 8.0) as u32,
        final_newline,
    };
    let outcome = weave_text::format::run_formatter(format, operation, content, &options);
    Ok(crate::text_dto::FormatOutcomeDto::from_outcome(outcome))
}

fn parse_operation(
    name: &str,
) -> Result<weave_text::format::FormatOperation, IpcError> {
    Ok(match name {
        "validate" => weave_text::format::FormatOperation::Validate,
        "format" => weave_text::format::FormatOperation::Format,
        "minify" => weave_text::format::FormatOperation::Minify,
        "sort" => weave_text::format::FormatOperation::Sort,
        "normalize" => weave_text::format::FormatOperation::Normalize,
        other => return Err(err("text.unknownOperation", format!("unknown operation '{other}'"))),
    })
}

/// Transform 预览（纯函数）。
pub fn transform_text(
    content: &str,
    op: crate::text_dto::TransformOpDto,
) -> Result<crate::text_dto::TransformResultDto, IpcError> {
    if content.len() as u64 > MAX_TEXT_BYTES {
        return Err(err("text.tooLarge", "input exceeds the text size limit"));
    }
    let kind = op.into_domain()?;
    let result = weave_text::apply_transform(content, &kind)
        .map_err(|e| err("text.transformInvalid", format!("{}: {}", e.code, e.message)))?;
    Ok(crate::text_dto::TransformResultDto::from_result(result))
}

/// Extract（纯函数）。
pub fn extract_text(
    content: &str,
    kind: &str,
    regex: Option<String>,
    unique_values: bool,
) -> Result<Vec<crate::text_dto::ExtractMatchDto>, IpcError> {
    if content.len() as u64 > MAX_TEXT_BYTES {
        return Err(err("text.tooLarge", "input exceeds the text size limit"));
    }
    let options = weave_text::ExtractOptions { unique_values };
    let matches = if kind == "regex" {
        let pattern = regex
            .as_deref()
            .ok_or_else(|| err("text.emptyFind", "regex extractor requires a pattern"))?;
        weave_text::extract_regex(content, pattern, &options)
            .map_err(|e| err("text.invalidRegex", e.message))?
    } else {
        let domain_kind = parse_extract_kind(kind)?;
        weave_text::extract_matches(content, domain_kind, &options)
    };
    Ok(matches
        .into_iter()
        .map(crate::text_dto::ExtractMatchDto::from_match)
        .collect())
}

fn parse_extract_kind(name: &str) -> Result<weave_text::ExtractKind, IpcError> {
    use weave_text::ExtractKind;
    Ok(match name {
        "url" => ExtractKind::Url,
        "email" => ExtractKind::Email,
        "filePath" => ExtractKind::FilePath,
        "number" => ExtractKind::Number,
        "ipv4" => ExtractKind::Ipv4,
        "ipv6" => ExtractKind::Ipv6,
        "json" => ExtractKind::Json,
        "markdownLink" => ExtractKind::MarkdownLink,
        other => return Err(err("text.unknownExtractor", format!("unknown extractor '{other}'"))),
    })
}

/// Compare（纯函数；限额由域内 CompareLimits 表达，§55）。
pub fn compare_text(
    a: &str,
    b: &str,
    whitespace: &str,
    ignore_case: bool,
) -> Result<crate::text_dto::DiffReportDto, IpcError> {
    if a.len() as u64 > MAX_TEXT_BYTES || b.len() as u64 > MAX_TEXT_BYTES {
        return Err(err("text.tooLarge", "input exceeds the text size limit"));
    }
    let options = weave_text::CompareOptions {
        whitespace: match whitespace {
            "trailing" => weave_text::WhitespaceMode::Trailing,
            "all" => weave_text::WhitespaceMode::All,
            _ => weave_text::WhitespaceMode::None,
        },
        ignore_case,
    };
    match weave_text::compare_texts(a, b, &options, &weave_text::CompareLimits::default()) {
        Ok(report) => Ok(crate::text_dto::DiffReportDto::from_report(report)),
        Err(weave_text::CompareError::TooLarge { code, message }) => {
            Err(err(code, message))
        }
    }
}

/// 构建写回计划（§87/§88/§89：Apply/Overwrite 的 Preview 契约）。
/// TOCTOU 快照（§90）随 PlanItem 下发；内容进服务端缓存。
#[allow(clippy::too_many_arguments)]
pub fn build_text_write_plan(
    cache: &TextWriteCache,
    plans: &crate::rename_service::PlanCache,
    path: &str,
    content: &str,
    encoding: &str,
    bom: &str,
    snapshot_size: f64,
    snapshot_modified_ms: Option<f64>,
) -> Result<crate::ops_dto::PlanDto, IpcError> {
    weave_core::prelude::validate_absolute_path(path)?;
    if content.len() as u64 > MAX_TEXT_BYTES {
        return Err(err("text.tooLarge", "content exceeds the text size limit"));
    }
    let encoding = parse_encoding(encoding)?;
    let bom = parse_bom(bom)?;
    let bytes = weave_text::encode(content, encoding, bom).map_err(|e| err(e.code, e.message))?;
    let plan = weave_core::prelude::Plan {
        operation_id: OperationId::generate(),
        kind: weave_core::prelude::OperationKind::TextTransform,
        created_at: std::time::SystemTime::now(),
        items: vec![weave_core::prelude::PlanItem {
            item_id: "item_0000".to_string(),
            source_path: path.to_string(),
            target_path: String::new(), // 备份路径执行期决定
            status: weave_core::prelude::PlanItemStatus::Ready,
            collision: weave_core::prelude::CollisionKind::None,
            source_size: Some(snapshot_size as u64),
            source_modified: snapshot_modified_ms
                .map(|ms| std::time::UNIX_EPOCH + std::time::Duration::from_millis(ms as u64)),
            warnings: Vec::new(),
            errors: Vec::new(),
        }],
    };
    let dto = crate::ops_dto::PlanDto::from_plan(&plan);
    plans.insert(&plan);
    cache.insert(
        &plan.operation_id,
        TextWriteEntry {
            path: PathBuf::from(path),
            bytes,
        },
    );
    Ok(dto)
}

fn parse_bom(name: &str) -> Result<weave_text::Bom, IpcError> {
    Ok(match name {
        "utf8" => weave_text::Bom::Utf8,
        "utf16Le" => weave_text::Bom::Utf16Le,
        "utf16Be" => weave_text::Bom::Utf16Be,
        "none" => weave_text::Bom::None,
        other => return Err(err("text.unknownBom", format!("unknown BOM policy '{other}'"))),
    })
}

/// 任务内执行写回：Revalidate → 备份 → 原子写 → 事务/历史落盘。
pub struct TextWriteReport {
    pub operation_id: OperationId,
    pub bytes_written: u64,
    pub backup: Option<PathBuf>,
    pub failed: bool,
    pub duration_ms: u64,
}

pub fn run_text_write_job(
    history_dir: &std::path::Path,
    plan: &Plan,
    entry: TextWriteEntry,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(Progress),
) -> Result<(TextWriteReport, weave_history::OperationTransaction), (TextWriteReport, WeaveError)>
{
    let started = std::time::Instant::now();
    let _ = on_progress;
    let item = plan
        .items
        .first()
        .ok_or_else(|| WeaveError::validation("text.emptyPlan", "text write plan has no items"))
        .map_err(|e| (
            TextWriteReport {
                operation_id: plan.operation_id.clone(),
                bytes_written: 0,
                backup: None,
                failed: true,
                duration_ms: started.elapsed().as_millis() as u64,
            },
            e,
        ))?;

    // §90 TOCTOU Revalidate：外部改动 ⇒ 拒绝写回
    let current = std::fs::metadata(&entry.path);
    if let Ok(meta) = &current
        && let Some(expected) = item.source_size
        && meta.len() != expected
    {
        return Err((
            TextWriteReport {
                operation_id: plan.operation_id.clone(),
                bytes_written: 0,
                backup: None,
                failed: true,
                duration_ms: started.elapsed().as_millis() as u64,
            },
            WeaveError::conflict(
                "text.fileChangedSincePreview",
                format!(
                    "file changed since preview (size {} != {}); reload and re-preview",
                    meta.len(),
                    expected
                ),
            )
            .with_location("text_service::run_text_write_job"),
        ));
    }

    // 备份当前原件（Save As 到不存在路径时跳过）
    let backup = parent_dir(&entry.path).join(format!(
        ".{}.weave-text-bak-{}",
        entry
            .path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
        plan.operation_id
    ));
    let backup = if current.is_ok() {
        std::fs::copy(&entry.path, &backup).map_err(|e| {
            (
                TextWriteReport {
                    operation_id: plan.operation_id.clone(),
                    bytes_written: 0,
                    backup: None,
                    failed: true,
                    duration_ms: started.elapsed().as_millis() as u64,
                },
                WeaveError::io(
                    "text.backupFailed",
                    format!("cannot back up original: {e}"),
                )
                .with_location("text_service::run_text_write_job"),
            )
        })?;
        Some(backup)
    } else {
        None
    };

    if cancel.is_cancelled() {
        return Err((
            TextWriteReport {
                operation_id: plan.operation_id.clone(),
                bytes_written: 0,
                backup,
                failed: true,
                duration_ms: started.elapsed().as_millis() as u64,
            },
            WeaveError::cancelled("text.cancelled", "text write cancelled before write")
                .with_location("text_service::run_text_write_job"),
        ));
    }

    // 原子替换（§91/§92）
    if let Err(e) = weave_files::atomic_write(std_fs(), &entry.path, &entry.bytes) {
        return Err((
            TextWriteReport {
                operation_id: plan.operation_id.clone(),
                bytes_written: 0,
                backup,
                failed: true,
                duration_ms: started.elapsed().as_millis() as u64,
            },
            e.with_location("text_service::run_text_write_job"),
        ));
    }

    // 写后状态进事务（M2 undo 占用检查据此实现 §94）
    let post = std::fs::metadata(&entry.path).ok();
    let post_size = post.as_ref().map(|m| m.len());
    let post_modified = post.and_then(|m| m.modified().ok());
    let tx = weave_history::OperationTransaction {
        operation_id: plan.operation_id.clone(),
        kind: plan.kind,
        status: weave_history::OperationStatus::Completed,
        timestamp: std::time::SystemTime::now(),
        reversible: weave_history::Reversibility::Full,
        items: vec![weave_history::TransactionItem {
            item_id: item.item_id.clone(),
            source_path: entry.path.to_string_lossy().into_owned(),
            target_path: backup
                .as_ref()
                .map(|b| b.to_string_lossy().into_owned())
                .unwrap_or_default(),
            status: weave_history::TransactionItemStatus::Executed,
            timestamp: post_modified,
            original_size: post_size,
            original_modified: post_modified,
            original_created: None,
        }],
    };
    let entry_record = weave_history::HistoryEntry {
        operation_id: tx.operation_id.clone(),
        kind: tx.kind,
        timestamp: tx.timestamp,
        summary: format!("Text write {} bytes", entry.bytes.len()),
        item_count: 1,
        success_count: 1,
        failed_count: 0,
        skipped_count: 0,
        undoable: true,
        status: weave_history::OperationStatus::Completed,
        input_root: None,
        rule_summary: None,
    };
    let history_ok = weave_history::HistoryStore::open(history_dir)
        .and_then(|s| {
            s.save_transaction(&tx)
                .and_then(|_| s.upsert_entry(entry_record))
        })
        .is_ok();
    let report = TextWriteReport {
        operation_id: plan.operation_id.clone(),
        bytes_written: entry.bytes.len() as u64,
        backup,
        failed: !history_ok,
        duration_ms: started.elapsed().as_millis() as u64,
    };
    if history_ok {
        Ok((report, tx))
    } else {
        Err((
            report,
            WeaveError::io("history.unavailable", "history persistence failed")
                .with_location("text_service::run_text_write_job"),
        ))
    }
}

fn parent_dir(path: &std::path::Path) -> PathBuf {
    path.parent().map(|p| p.to_path_buf()).unwrap_or_default()
}

/// 纯函数入口共用的小文档构造（UI Paste/Typed 来源）。
pub fn pasted_document(content: String) -> TextDocument {
    TextDocument::from_content(SourceKind::Pasted, content)
}
