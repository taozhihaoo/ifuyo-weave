//! M4 Text 的 IPC DTO 层（同 D4/D10 约定：f64 数值、epoch-ms 时间；
//! offset 契约 = byte + line + UTF-16 column，M4 上 §18）。

#![expect(clippy::result_large_err)]

use serde::{Deserialize, Serialize};
use specta::Type;

use crate::commands::IpcError;

// ─── 文档 ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextDocumentDto {
    pub path: String,
    pub content: String,
    pub encoding: String,
    pub bom: String,
    pub format: String,
    pub byte_size: f64,
    pub modified_ms: Option<f64>,
    pub ends_with_newline: bool,
}

// ─── Format ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FormatOutcomeDto {
    pub content: Option<String>,
    pub diagnostics: Vec<TextDiagnosticDto>,
    pub changed: bool,
}

impl FormatOutcomeDto {
    pub fn from_outcome(o: weave_text::format::FormatOutcome) -> Self {
        Self {
            content: o.content,
            diagnostics: o.diagnostics.iter().map(TextDiagnosticDto::from_diag).collect(),
            changed: o.changed,
        }
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextDiagnosticDto {
    pub code: String,
    pub severity: String,
    pub message: String,
    pub range_start: Option<f64>,
    pub range_end: Option<f64>,
    pub line: Option<f64>,
    pub column: Option<f64>,
}

impl TextDiagnosticDto {
    pub fn from_diag(d: &weave_text::TextDiagnostic) -> Self {
        Self {
            code: d.code.clone(),
            severity: d.severity.as_str().to_string(),
            message: d.message.clone(),
            range_start: d.range.map(|r| r.start as f64),
            range_end: d.range.map(|r| r.end as f64),
            line: d.line.map(|v| v as f64),
            column: d.column.map(|v| v as f64),
        }
    }
}

// ─── Transform ───

/// 变换操作（tagged enum，与 Rust TransformKind 一一对应）。
#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum TransformOpDto {
    TrimLines,
    TrimDocument,
    #[serde(rename_all = "camelCase")]
    DeduplicateLines {
        keep: String,
        blank: String,
    },
    #[serde(rename_all = "camelCase")]
    SortLines {
        descending: bool,
        case_sensitive: bool,
        blank: String,
    },
    #[serde(rename_all = "camelCase")]
    AddPrefix {
        text: String,
        skip_blank: bool,
    },
    #[serde(rename_all = "camelCase")]
    AddSuffix {
        text: String,
        skip_blank: bool,
    },
    #[serde(rename_all = "camelCase")]
    CaseConvert {
        form: String,
    },
    #[serde(rename_all = "camelCase")]
    NumberLines {
        start: f64,
        step: f64,
        separator: String,
        pad: String,
    },
    #[serde(rename_all = "camelCase")]
    FindReplace {
        find: String,
        replacement: String,
        regex: bool,
        case_insensitive: bool,
        first_only: bool,
    },
}

impl TransformOpDto {
    pub fn into_domain(self) -> Result<weave_text::TransformKind, IpcError> {
        use weave_text::{BlankPolicy, CaseForm, KeepPolicy, PadMode, TransformKind};
        fn num(v: f64, what: &str) -> Result<u64, IpcError> {
            if v < 0.0 || v != v.trunc() || v > u64::MAX as f64 {
                return Err(crate::text_service::err_bare(
                    "text.invalidNumber",
                    format!("option '{what}' = {v} is out of range"),
                ));
            }
            Ok(v as u64)
        }
        fn blank(s: &str) -> Result<BlankPolicy, IpcError> {
            Ok(match s {
                "included" => BlankPolicy::Included,
                "preserve" => BlankPolicy::Preserve,
                "first" => BlankPolicy::First,
                "last" => BlankPolicy::Last,
                other => {
                    return Err(crate::text_service::err_bare(
                        "text.invalidOption",
                        format!("unknown blank policy '{other}'"),
                    ))
                }
            })
        }
        fn keep(s: &str) -> Result<KeepPolicy, IpcError> {
            Ok(match s {
                "first" => KeepPolicy::First,
                "last" => KeepPolicy::Last,
                other => {
                    return Err(crate::text_service::err_bare(
                        "text.invalidOption",
                        format!("unknown keep policy '{other}'"),
                    ))
                }
            })
        }
        Ok(match self {
            TransformOpDto::TrimLines => TransformKind::TrimLines,
            TransformOpDto::TrimDocument => TransformKind::TrimDocument,
            TransformOpDto::DeduplicateLines {
                keep: keep_text,
                blank: blank_text,
            } => TransformKind::DeduplicateLines {
                keep: keep(&keep_text)?,
                blank: blank(&blank_text)?,
            },
            TransformOpDto::SortLines {
                descending,
                case_sensitive,
                blank: blank_text,
            } => TransformKind::SortLines {
                descending,
                case_sensitive,
                blank: blank(&blank_text)?,
            },
            TransformOpDto::AddPrefix { text, skip_blank } => {
                TransformKind::AddPrefix { text, skip_blank }
            }
            TransformOpDto::AddSuffix { text, skip_blank } => {
                TransformKind::AddSuffix { text, skip_blank }
            }
            TransformOpDto::CaseConvert { form } => TransformKind::CaseConvert {
                form: match form.as_str() {
                    "upper" => CaseForm::Upper,
                    "lower" => CaseForm::Lower,
                    "title" => CaseForm::Title,
                    "sentence" => CaseForm::Sentence,
                    other => {
                        return Err(crate::text_service::err_bare(
                            "text.invalidOption",
                            format!("unknown case form '{other}'"),
                        ))
                    }
                },
            },
            TransformOpDto::NumberLines {
                start,
                step,
                separator,
                pad,
            } => TransformKind::NumberLines {
                start: num(start, "start")?,
                step: num(step, "step")?,
                separator,
                pad: match pad.as_str() {
                    "none" => PadMode::None,
                    "zeros" => PadMode::Zeros,
                    other => {
                        return Err(crate::text_service::err_bare(
                            "text.invalidOption",
                            format!("unknown pad mode '{other}'"),
                        ))
                    }
                },
            },
            TransformOpDto::FindReplace {
                find,
                replacement,
                regex,
                case_insensitive,
                first_only,
            } => TransformKind::FindReplace {
                find,
                replacement,
                regex,
                case_insensitive,
                first_only,
            },
        })
    }
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TransformResultDto {
    pub content: String,
    pub match_count: Option<f64>,
    pub removed_lines: Option<f64>,
}

impl TransformResultDto {
    pub fn from_result(r: weave_text::TransformResult) -> Self {
        Self {
            content: r.content,
            match_count: r.match_count.map(|v| v as f64),
            removed_lines: r.removed_lines.map(|v| v as f64),
        }
    }
}

// ─── Extract ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ExtractMatchDto {
    pub kind: String,
    pub value: String,
    pub raw_value: String,
    pub start: f64,
    pub end: f64,
    pub line: f64,
    pub column: f64,
    pub label: Option<String>,
}

impl ExtractMatchDto {
    pub fn from_match(m: weave_text::ExtractMatch) -> Self {
        Self {
            kind: m.kind.as_str().to_string(),
            value: m.value,
            raw_value: m.raw_value,
            start: m.start as f64,
            end: m.end as f64,
            line: m.line as f64,
            column: m.column as f64,
            label: m.label,
        }
    }
}

// ─── Compare ───

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffLineDto {
    pub change: String,
    pub a_line: Option<f64>,
    pub b_line: Option<f64>,
    pub a_text: String,
    pub b_text: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffHunkDto {
    pub a_start: f64,
    pub a_len: f64,
    pub b_start: f64,
    pub b_len: f64,
    pub lines: Vec<DiffLineDto>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffStatsDto {
    pub added: f64,
    pub removed: f64,
    pub changed: f64,
    pub moved: f64,
    pub equal: f64,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct DiffReportDto {
    pub hunks: Vec<DiffHunkDto>,
    pub stats: DiffStatsDto,
    pub identical: bool,
    pub degraded: bool,
    pub unified: String,
}

impl DiffReportDto {
    pub fn from_report(r: weave_text::DiffReport) -> Self {
        Self {
            hunks: r
                .hunks
                .iter()
                .map(|h| DiffHunkDto {
                    a_start: h.a_start as f64,
                    a_len: h.a_len as f64,
                    b_start: h.b_start as f64,
                    b_len: h.b_len as f64,
                    lines: h
                        .lines
                        .iter()
                        .map(|l| DiffLineDto {
                            change: change_str(l.change).to_string(),
                            a_line: l.a_line.map(|v| v as f64),
                            b_line: l.b_line.map(|v| v as f64),
                            a_text: l.a_text.clone(),
                            b_text: l.b_text.clone(),
                        })
                        .collect(),
                })
                .collect(),
            stats: DiffStatsDto {
                added: r.stats.added as f64,
                removed: r.stats.removed as f64,
                changed: r.stats.changed as f64,
                moved: r.stats.moved as f64,
                equal: r.stats.equal as f64,
            },
            identical: r.identical,
            degraded: r.degraded,
            unified: weave_text::unified_diff(&r, "A", "B"),
        }
    }
}

fn change_str(change: weave_text::LineChange) -> &'static str {
    match change {
        weave_text::LineChange::Equal => "equal",
        weave_text::LineChange::Added => "added",
        weave_text::LineChange::Removed => "removed",
        weave_text::LineChange::Changed => "changed",
        weave_text::LineChange::Moved => "moved",
    }
}
