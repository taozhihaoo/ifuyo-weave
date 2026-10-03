//! M5：Data 的 IPC 命令（open/inspect/page/filter/sort/preview/apply/export）。
//!
//! 会话句柄（§17/§84）：session 存于 AppState 内存（ephemeral，§18——
//! 不是数据库）；分页 + 每页上限防 IPC 爆炸（§84/§85）；
//! 导出/覆盖复用 M4 安全管线（TOCTOU/原子替换/事务/历史/Undo，
//! §69/§72-§75——Dest 新文件走 atomic_write + 历史记录）。

// IPC 边界与 D10 同理：Err DTO 体积不构成热路径问题（见 DECISIONS.md D10）。
#![expect(clippy::result_large_err)]

use std::collections::HashMap;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_core::prelude::{CancellationToken, OperationId, WeaveError};
use weave_data::{
    CsvDialect, DataLimits, DataSession, FilterOperator, FilterRule, HeaderDecision, SortSpec,
    TypedMode, detect_header, parse_csv, parse_jsonl, profile_table, table_to_delimited,
    table_to_json, table_to_jsonl,
};

use crate::commands::IpcError;
use crate::ops_dto::PlanDto;

fn err(code: impl AsRef<str>, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code.as_ref(), message)
        .with_location("data_service")
        .into()
}

fn limits() -> DataLimits {
    DataLimits::default()
}

/// 会话存储（§17 ephemeral；上限 8 个，超出挤掉最旧）。
#[derive(Default)]
pub struct DataSessions {
    inner: Mutex<HashMap<String, DataSession>>,
    counter: std::sync::atomic::AtomicU64,
}

const MAX_SESSIONS: usize = 8;

impl DataSessions {
    pub fn new() -> Self {
        Self::default()
    }

    fn insert(&self, session: DataSession) -> String {
        let mut map = self.inner.lock().expect("sessions");
        let n = self
            .counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
            + 1;
        let handle = format!("ds_{n}");
        map.insert(handle.clone(), session);
        if map.len() > MAX_SESSIONS
            && let Some(oldest) = map
                .keys()
                .min_by_key(|k| k.trim_start_matches("ds_").parse::<u64>().unwrap_or(0))
                .cloned()
        {
            map.remove(&oldest);
        }
        handle
    }

    fn with<R>(&self, handle: &str, f: impl FnOnce(&mut DataSession) -> R) -> Option<R> {
        self.inner.lock().expect("sessions").get_mut(handle).map(f)
    }
}

// ─── DTOs ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataColumnDto {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataDiagnosticDto {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub row: Option<f64>,
    pub column: Option<f64>,
}

fn diag_dtos(ds: &[weave_data::DataDiagnostic]) -> Vec<DataDiagnosticDto> {
    ds.iter()
        .map(|d| DataDiagnosticDto {
            code: d.code.clone(),
            severity: d.severity.as_str().to_string(),
            message: d.message.clone(),
            row: d.row.map(|v| v as f64),
            column: d.column.map(|v| v as f64),
        })
        .collect()
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataPageDto {
    pub session_id: String,
    pub format: String,
    pub encoding: String,
    pub columns: Vec<DataColumnDto>,
    pub rows: Vec<Vec<String>>,
    pub row_ids: Vec<String>,
    pub total_rows: f64,
    pub offset: f64,
    pub diagnostics: Vec<DataDiagnosticDto>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ColumnProfileDto {
    pub column_id: String,
    pub name: String,
    pub potential_types: Vec<String>,
    pub null_count: f64,
    pub total_rows: f64,
    /// exact:N/E 或 unavailable:<reason>（§47 如实）。
    pub unique: String,
    pub unique_rate: Option<f64>,
    pub null_rate: Option<f64>,
    pub leading_zero_numeric: bool,
}

fn profile_dtos(table: &weave_data::DataTable) -> Vec<ColumnProfileDto> {
    profile_table(table, &limits())
        .iter()
        .map(|p| ColumnProfileDto {
            column_id: p.column_id.clone(),
            name: p.name.clone(),
            potential_types: p.potential_types.iter().map(|s| s.to_string()).collect(),
            null_count: p.null_count as f64,
            total_rows: p.total_rows as f64,
            unique: match &p.unique {
                weave_data::UniqueStat::Exact { distinct, eligible } => {
                    format!("exact:{distinct}/{eligible}")
                }
                weave_data::UniqueStat::Unavailable { reason } => {
                    format!("unavailable:{reason}")
                }
            },
            unique_rate: p.unique_rate(),
            null_rate: p.null_rate(),
            leading_zero_numeric: p.leading_zero_numeric,
        })
        .collect()
}

fn page_of(
    session: &DataSession,
    offset: usize,
    format: &str,
    encoding: &str,
    diagnostics: Vec<DataDiagnosticDto>,
) -> DataPageDto {
    let page_size = limits().max_page_rows;
    let indices = &session.view.indices;
    let start = offset.min(indices.len());
    let end = (start + page_size).min(indices.len());
    let rows: Vec<Vec<String>> = indices[start..end]
        .iter()
        .map(|&i| session.table.rows.get(i).cloned().unwrap_or_default())
        .collect();
    let row_ids: Vec<String> = indices[start..end]
        .iter()
        .map(|&i| format!("row_{}", i + 1))
        .collect();
    DataPageDto {
        session_id: String::new(), // 调用处填充
        format: format.to_string(),
        encoding: encoding.to_string(),
        columns: session
            .table
            .columns
            .iter()
            .map(|c| DataColumnDto {
                id: c.id.clone(),
                name: c.name.clone(),
            })
            .collect(),
        rows,
        row_ids,
        total_rows: indices.len() as f64,
        offset: start as f64,
        diagnostics,
    }
}

fn decode_file(path: &str) -> Result<(weave_text::DecodedText, String), IpcError> {
    weave_core::prelude::validate_absolute_path(path)?;
    let meta = std::fs::metadata(path).map_err(|e| err("data.loadFailed", e.to_string()))?;
    if let Err((message, code)) = limits().check("document", meta.len()) {
        return Err(err(code, message));
    }
    let bytes = std::fs::read(path).map_err(|e| err("data.loadFailed", e.to_string()))?;
    if bytes[..bytes.len().min(8192)].contains(&0) {
        return Err(err(
            "text.binaryDetected",
            "file looks binary; data tools refuse to guess",
        ));
    }
    let decoded = weave_text::decode(&bytes).map_err(|e| err(e.code, e.message))?;
    let ext = std::path::Path::new(path)
        .extension()
        .map(|e| e.to_string_lossy().into_owned())
        .unwrap_or_default();
    let format =
        weave_data::DataFormat::from_extension(&ext).unwrap_or(weave_data::DataFormat::Csv);
    Ok((decoded, format.as_str().to_string()))
}

// ─── open ───

#[tauri::command]
#[specta::specta]
pub fn data_open(
    app: tauri::AppHandle,
    path: String,
    has_header: Option<String>,
) -> Result<DataPageDto, IpcError> {
    let (decoded, format) = decode_file(&path)?;
    let mut diagnostics: Vec<DataDiagnosticDto> = Vec::new();
    let session = match format.as_str() {
        "json" => {
            let root: serde_json::Value = serde_json::from_str(&decoded.content)
                .map_err(|e| err("data.jsonInvalid", e.to_string()))?;
            let table =
                weave_data::json_records_to_table(&root, weave_data::FlattenStrategy::DottedPaths)
                    .map_err(|ds| {
                        ds.first()
                            .map(|d| err(&d.code, &d.message))
                            .expect("non-empty")
                    })?;
            DataSession::new(table)
        }
        "jsonl" => {
            let report = parse_jsonl(
                decoded.content.as_bytes(),
                weave_data::JsonlErrorMode::CollectErrors,
                limits().max_rows_in_memory,
                limits().max_diagnostics,
            )
            .map_err(|e| err(&e.code, &e.message))?;
            diagnostics.extend(diag_dtos(&report.diagnostics));
            let root = serde_json::Value::Array(report.records.clone());
            let table =
                weave_data::json_records_to_table(&root, weave_data::FlattenStrategy::DottedPaths)
                    .unwrap_or_default();
            DataSession::new(table)
        }
        // csv / tsv（默认）
        _ => {
            let (mut dialect, note) = detect_dialect(&format, &decoded.content);
            if let Some(n) = note {
                diagnostics.push(n);
            }
            let decision = match has_header.as_deref() {
                Some("yes") | Some("no") => HeaderDecision {
                    has_header: has_header.as_deref() == Some("yes"),
                    confidence: "explicit",
                    reason: "user-specified".to_string(),
                },
                _ => {
                    let mut probe = csv::ReaderBuilder::new()
                        .delimiter(dialect.delimiter as u8)
                        .has_headers(false)
                        .from_reader(decoded.content.as_bytes());
                    let rows: Vec<Vec<String>> = probe
                        .records()
                        .take(2)
                        .flatten()
                        .map(|r| r.iter().map(String::from).collect())
                        .collect();
                    let d = detect_header(
                        rows.first().map(|r| r.as_slice()),
                        rows.get(1).map(|r| r.as_slice()),
                    );
                    diagnostics.push(DataDiagnosticDto {
                        code: "data.headerDetected".to_string(),
                        severity: "info".to_string(),
                        message: format!(
                            "header={} (confidence: {}, {})",
                            d.has_header, d.confidence, d.reason
                        ),
                        row: None,
                        column: None,
                    });
                    d
                }
            };
            dialect.has_header = decision.has_header;
            let parsed = parse_csv(decoded.content.as_bytes(), &dialect, &limits())
                .map_err(|e| err(&e.code, &e.message))?;
            diagnostics.extend(diag_dtos(&parsed.diagnostics));
            DataSession::new(weave_data::DataTable {
                columns: parsed.columns,
                rows: parsed.rows,
            })
        }
    };

    let state = app.state::<crate::state::AppState>();
    let session_id = state.data_sessions.insert(session);
    let encoding_str = crate::text_service::encoding_to_str(decoded.encoding).to_string();
    let mut page = None;
    state.data_sessions.with(&session_id, |s| {
        page = Some(page_of(s, 0, &format, &encoding_str, diagnostics.clone()));
    });
    let mut page = page.expect("session exists");
    page.session_id = session_id;
    page.encoding = encoding_str;
    Ok(page)
}

fn detect_dialect(ext: &str, sample: &str) -> (CsvDialect, Option<DataDiagnosticDto>) {
    let mut dialect = if ext.eq_ignore_ascii_case("tsv") {
        CsvDialect::tsv()
    } else {
        CsvDialect::csv()
    };
    // §67：首行候选计数 + Heuristic 证据；歧义交给用户手选
    let first_line = sample.lines().next().unwrap_or("");
    let mut best: Option<(usize, char)> = None;
    for c in [',', '\t', ';', '|'] {
        let count = first_line.matches(c).count();
        if count > 0 && best.map(|(n, _)| count > n).unwrap_or(true) {
            best = Some((count, c));
        }
    }
    let mut note = None;
    if let Some((_, c)) = best
        && dialect.delimiter != c
    {
        dialect.delimiter = c;
        note = Some(DataDiagnosticDto {
            code: "data.delimiterDetected".to_string(),
            severity: "info".to_string(),
            message: format!("delimiter auto-detected as '{c}' from header row (Heuristic)"),
            row: None,
            column: None,
        });
    }
    (dialect, note)
}

#[tauri::command]
#[specta::specta]
pub fn data_page(
    app: tauri::AppHandle,
    session_id: String,
    offset: f64,
) -> Result<DataPageDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut out = None;
    state.data_sessions.with(&session_id, |s| {
        out = Some(page_of(s, offset.max(0.0) as usize, "", "", Vec::new()));
    });
    let mut page = out.ok_or_else(|| {
        err(
            "data.unknownSession",
            format!("unknown session '{session_id}'"),
        )
    })?;
    page.session_id = session_id;
    Ok(page)
}

// ─── filter / sort ───

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FilterRuleDto {
    pub column_id: String,
    pub operator: String,
    pub value: String,
    pub case_sensitive: bool,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SortSpecDto {
    pub column_id: String,
    pub descending: bool,
    pub ignore_case: bool,
}

fn parse_operator(name: &str) -> Result<FilterOperator, IpcError> {
    Ok(match name {
        "contains" => FilterOperator::Contains,
        "equals" => FilterOperator::Equals,
        "notEquals" => FilterOperator::NotEquals,
        "startsWith" => FilterOperator::StartsWith,
        "endsWith" => FilterOperator::EndsWith,
        "empty" => FilterOperator::Empty,
        "notEmpty" => FilterOperator::NotEmpty,
        other => {
            return Err(err(
                "data.unknownOperator",
                format!("unknown operator '{other}'"),
            ));
        }
    })
}

#[tauri::command]
#[specta::specta]
pub fn data_set_view(
    app: tauri::AppHandle,
    session_id: String,
    filters: Vec<FilterRuleDto>,
    sort: Option<SortSpecDto>,
    offset: f64,
) -> Result<DataPageDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let rules: Vec<FilterRule> = filters
        .iter()
        .map(|f| {
            Ok(FilterRule {
                column_id: f.column_id.clone(),
                operator: parse_operator(&f.operator)?,
                value: f.value.clone(),
                case_sensitive: f.case_sensitive,
            })
        })
        .collect::<Result<Vec<_>, IpcError>>()?;
    let sort = sort.map(|s| SortSpec {
        column_id: s.column_id,
        descending: s.descending,
        ignore_case: s.ignore_case,
    });
    let mut out = None;
    state.data_sessions.with(&session_id, |s| {
        s.filters = rules;
        s.sort = sort;
        s.refresh_view();
        out = Some(page_of(s, offset.max(0.0) as usize, "", "", Vec::new()));
    });
    let mut page = out.ok_or_else(|| {
        err(
            "data.unknownSession",
            format!("unknown session '{session_id}'"),
        )
    })?;
    page.session_id = session_id;
    Ok(page)
}

// ─── inspect ───

#[tauri::command]
#[specta::specta]
pub fn data_inspect_profiles(
    app: tauri::AppHandle,
    session_id: String,
) -> Result<Vec<ColumnProfileDto>, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut out = None;
    state.data_sessions.with(&session_id, |s| {
        out = Some(profile_dtos(&s.table));
    });
    out.ok_or_else(|| {
        err(
            "data.unknownSession",
            format!("unknown session '{session_id}'"),
        )
    })
}

// ─── transform（Cleaner）───

#[tauri::command]
#[specta::specta]
pub fn data_preview_transform(
    app: tauri::AppHandle,
    session_id: String,
    plan: weave_data::DataTransformPlan,
) -> Result<TransformPreviewDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut out = None;
    state.data_sessions.with(&session_id, |s| {
        match weave_data::apply_plan(
            &s.table,
            &plan,
            &weave_data::NullPolicy::default(),
            limits().max_diagnostics,
        ) {
            Ok((new_table, diagnostics, changed)) => {
                let sample: Vec<Vec<String>> = new_table
                    .rows
                    .iter()
                    .take(limits().preview_rows)
                    .cloned()
                    .collect();
                out = Some(TransformPreviewDto {
                    columns: new_table
                        .columns
                        .iter()
                        .map(|c| DataColumnDto {
                            id: c.id.clone(),
                            name: c.name.clone(),
                        })
                        .collect(),
                    sample_rows: sample,
                    rows_changed: changed as f64,
                    total_rows: new_table.row_count() as f64,
                    diagnostics: diag_dtos(&diagnostics),
                    ok: true,
                });
            }
            Err(errors) => {
                out = Some(TransformPreviewDto {
                    columns: Vec::new(),
                    sample_rows: Vec::new(),
                    rows_changed: 0.0,
                    total_rows: 0.0,
                    diagnostics: diag_dtos(&errors),
                    ok: false,
                });
            }
        }
    });
    out.ok_or_else(|| {
        err(
            "data.unknownSession",
            format!("unknown session '{session_id}'"),
        )
    })
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransformPreviewDto {
    pub ok: bool,
    pub columns: Vec<DataColumnDto>,
    /// Preview 采样（§77：最多 preview_rows 行，明确为 sample）。
    pub sample_rows: Vec<Vec<String>>,
    pub rows_changed: f64,
    pub total_rows: f64,
    pub diagnostics: Vec<DataDiagnosticDto>,
}

#[tauri::command]
#[specta::specta]
pub fn data_apply_transform(
    app: tauri::AppHandle,
    session_id: String,
    plan: weave_data::DataTransformPlan,
) -> Result<DataPageDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut result = None;
    let mut failed = false;
    state.data_sessions.with(&session_id, |s| {
        match weave_data::apply_plan(
            &s.table,
            &plan,
            &weave_data::NullPolicy::default(),
            limits().max_diagnostics,
        ) {
            Ok((new_table, diagnostics, _)) => {
                s.table = new_table;
                s.refresh_view();
                let diags = diag_dtos(&diagnostics);
                result = Some(page_of(s, 0, "", "", diags));
            }
            Err(errors) => {
                failed = true;
                result = Some(page_of(s, 0, "", "", diag_dtos(&errors)));
            }
        }
    });
    let mut page = result.ok_or_else(|| {
        err(
            "data.unknownSession",
            format!("unknown session '{session_id}'"),
        )
    })?;
    if failed {
        return Err(err(
            "data.transformInvalid",
            "plan validation failed; see diagnostics",
        ));
    }
    page.session_id = session_id;
    Ok(page)
}

// ─── export（§69/§71/§72：Export 写新文件；覆盖源文件走 M4 管线）───

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DataExportOptionsDto {
    pub destination: String,
    /// csv | tsv | json | jsonl
    pub format: String,
    pub include_header: bool,
    /// typed: 仅 json/jsonl 生效（§56）。
    pub typed: bool,
    pub line_ending: String,
    /// §176/§179 导出范围：all = 底表全行；view = 当前过滤/排序视图。
    /// UI 必须展示 "N of M rows"（防用户误以为导出了全量）。
    pub scope: String,
}

#[tauri::command]
#[specta::specta]
pub fn data_export(
    app: tauri::AppHandle,
    session_id: String,
    options: DataExportOptionsDto,
) -> Result<PlanDto, IpcError> {
    let state = app.state::<crate::state::AppState>();
    let mut serialized: Vec<u8> = Vec::new();
    let mut exported_rows: u64 = 0;
    let mut total_rows: u64 = 0;
    state.data_sessions.with(&session_id, |s| {
        let mode = if options.typed {
            TypedMode::InferTypes
        } else {
            TypedMode::PreserveStrings
        };
        let le = match options.line_ending.as_str() {
            "crlf" => weave_data::LineEnding::Crlf,
            "cr" => weave_data::LineEnding::Cr,
            _ => weave_data::LineEnding::Lf,
        };
        // §176/§179：scope = view 时仅导出当前过滤/排序视图的行（保持视图
        // 排序——导出即所见）；scope = all 导出底表全行。
        let view_rows: Vec<Vec<String>> = if options.scope == "view" {
            s.view
                .indices
                .iter()
                .filter_map(|&i| s.table.rows.get(i).cloned())
                .collect()
        } else {
            s.table.rows.clone()
        };
        exported_rows = view_rows.len() as u64;
        total_rows = s.table.row_count() as u64;
        let scoped = weave_data::DataTable {
            columns: s.table.columns.clone(),
            rows: view_rows,
        };
        let text = match options.format.as_str() {
            "tsv" => table_to_delimited(&scoped, '\t', '"', options.include_header, le),
            "json" => {
                Ok(serde_json::to_string_pretty(&table_to_json(&scoped, mode)).unwrap_or_default())
            }
            "jsonl" => Ok(table_to_jsonl(&scoped, mode)),
            _ => table_to_delimited(&scoped, ',', '"', options.include_header, le),
        };
        if let Ok(t) = text {
            serialized = t.into_bytes();
        }
    });
    if serialized.is_empty() {
        return Err(err(
            "data.unknownSession",
            format!("unknown session '{session_id}'"),
        ));
    }
    weave_core::prelude::validate_absolute_path(&options.destination)?;

    // §70/§71/§176：Export 写新文件（scope 决定 all|view 行集）。
    // 目标已存在 ⇒ 拒绝（防静默覆盖）。
    if std::fs::metadata(&options.destination).is_ok() {
        return Err(err(
            "data.destinationExists",
            format!("destination already exists: {}", options.destination),
        ));
    }

    // 用 TextTransformPlan 管线落盘（复用 M4 Plan/TOCTOU/原子写/历史/Undo）
    let content = String::from_utf8(serialized)
        .map_err(|_| err("data.exportEncoding", "export produced non-utf8"))?;
    let plan = weave_data::DataTransformPlan { rules: Vec::new() };
    let _ = plan;
    let session = app.state::<crate::state::AppState>();
    let history_dir = crate::rename_service::resolve_history_dir(&app)?;
    let text_plan = weave_core::prelude::Plan {
        operation_id: OperationId::generate(),
        kind: weave_core::prelude::OperationKind::TextTransform,
        created_at: std::time::SystemTime::now(),
        items: vec![weave_core::prelude::PlanItem {
            item_id: "item_0000".to_string(),
            source_path: options.destination.clone(),
            target_path: String::new(),
            status: weave_core::prelude::PlanItemStatus::Ready,
            collision: weave_core::prelude::CollisionKind::None,
            source_size: None,
            source_modified: None,
            warnings: Vec::new(),
            errors: Vec::new(),
        }],
    };
    let dto = PlanDto::from_plan(&text_plan);
    session.plans.insert(&text_plan);
    let entry = crate::text_service::TextWriteEntry {
        path: std::path::PathBuf::from(&options.destination),
        bytes: content.into_bytes(),
        must_not_exist: true,
    };
    // tx 已落历史（undo 经既有 undo_operation，§75）；report 计数进 DTO 摘要
    let (report, _tx) = crate::text_service::run_text_write_job(
        &history_dir,
        &text_plan,
        entry,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .map_err(|(r, e)| {
        let _ = r;
        err(&e.code, e.message.clone())
    })?;
    let _ = report.bytes_written;
    Ok(dto)
}
