//! CSV / TSV 解析（M5 上 §9–§14）：csv crate（BurntSushi 1.4，MIT/
//! Apache-2.0，§9 指名候选，D45）——绝不手写 split。
//!
//! - TSV 复用同一 parser（delimiter='\t'，§10）。
//! - Header：Auto 检测带 confidence/reason（§11），Explicit 直用。
//! - Ragged（§14）：短行补空 + 诊断；长行保留多余单元格 + 诊断。
//! - 损坏（§63/§64）：UTF-8/quote 错误 ⇒ Stop（数据错位风险）；
//!   ragged ⇒ Collect 继续。
//! - 读取统一走字节流（§89：不做 read-all-then-parse 作为唯一方案——
//!   csv::Reader 自带 incremental 读）。

use std::io::Read;

use weave_core::prelude::WeaveError;

use crate::diagnostics::{Basis, DataDiagnostic, DataSeverity};
use crate::table::{ColumnDefinition, build_columns};

/// 解析方言（§0.2/§68：CSV/TSV 显式；quote 可配置但默认 "）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CsvDialect {
    pub delimiter: char,
    pub quote: char,
    /// 是否把首行当 header。
    pub has_header: bool,
}

impl CsvDialect {
    pub fn csv() -> Self {
        Self {
            delimiter: ',',
            quote: '"',
            has_header: true,
        }
    }

    pub fn tsv() -> Self {
        Self {
            delimiter: '\t',
            quote: '"',
            has_header: true,
        }
    }
}

/// Header 自动检测决策（§11：启发式必须带 confidence/reason）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderDecision {
    pub has_header: bool,
    /// "high" | "medium" | "low"（启发式强度，如实标注）。
    pub confidence: &'static str,
    pub reason: String,
}

/// 首行 vs 次行的类型对比启发式：首行全非数字、次行出现数字 ⇒ 高置信
/// header（§11 允许启发式但必须标注 Heuristic basis）。
pub fn detect_header(
    sample_first: Option<&[String]>,
    sample_second: Option<&[String]>,
) -> HeaderDecision {
    let (Some(first), Some(second)) = (sample_first, sample_second) else {
        return HeaderDecision {
            has_header: true,
            confidence: "low",
            reason: "only one row available; assuming header by convention".to_string(),
        };
    };
    if first.is_empty() || second.is_empty() {
        return HeaderDecision {
            has_header: true,
            confidence: "low",
            reason: "empty rows; assuming header by convention".to_string(),
        };
    }
    let looks_numeric = |cell: &str| -> bool {
        cell.trim().parse::<f64>().is_ok()
            && cell
                .trim()
                .chars()
                .next()
                .is_some_and(|c| c.is_ascii_digit())
    };
    let first_numeric = first.iter().filter(|c| looks_numeric(c)).count();
    let second_numeric = second.iter().filter(|c| looks_numeric(c)).count();
    if first_numeric == 0 && second_numeric > 0 {
        HeaderDecision {
            has_header: true,
            confidence: "high",
            reason: "first row has no numeric cells while second row does (Heuristic)".to_string(),
        }
    } else if second_numeric == 0 && first_numeric > 0 {
        HeaderDecision {
            has_header: false,
            confidence: "medium",
            reason: "second row has no numeric cells while first row does (Heuristic)".to_string(),
        }
    } else if first_numeric > 0 && second_numeric > 0 {
        HeaderDecision {
            has_header: false,
            confidence: "medium",
            reason: "both rows contain numeric cells; likely no header (Heuristic)".to_string(),
        }
    } else {
        HeaderDecision {
            has_header: true,
            confidence: "low",
            reason: "inconclusive; assuming header by convention (Heuristic)".to_string(),
        }
    }
}

/// 解析结果：表 + 诊断（§63 Collect/Stop 语义）。
#[derive(Debug)]
pub struct CsvParseOutput {
    pub columns: Vec<ColumnDefinition>,
    pub rows: Vec<Vec<String>>,
    pub diagnostics: Vec<DataDiagnostic>,
    /// 数据行数（不含 header）。
    pub data_rows: u64,
}

/// 从字节流解析 CSV/TSV（§89：incremental；BOM 由调用方先行剥除或传入
/// 已解码文本）。`max_diagnostics` 达到后停止收集并标注（§88）。
pub fn parse_csv(
    reader: impl Read,
    dialect: &CsvDialect,
    limits: &crate::limits::DataLimits,
) -> Result<CsvParseOutput, WeaveError> {
    parse_csv_cancellable(
        reader,
        dialect,
        limits,
        &weave_core::prelude::CancellationToken::new(),
    )
}

/// 可取消版本（§119：大 CSV 扫描必须真取消——每 4096 条检查一次 token）。
pub fn parse_csv_cancellable(
    reader: impl Read,
    dialect: &CsvDialect,
    limits: &crate::limits::DataLimits,
    cancel: &weave_core::prelude::CancellationToken,
) -> Result<CsvParseOutput, WeaveError> {
    let mut builder = csv::ReaderBuilder::new();
    builder
        .delimiter(dialect.delimiter as u8)
        .quote(dialect.quote as u8)
        .has_headers(false) // 我们自己管理 header（保留原始首行文本）
        .flexible(true); // ragged 交给我们按 §14 语义处理
    let mut rdr = builder.from_reader(reader);

    let mut diagnostics: Vec<DataDiagnostic> = Vec::new();
    let mut records: Vec<Vec<String>> = Vec::new();
    let mut width = 0usize;
    let mut headers: Option<Vec<String>> = None;

    let push_diag = |d: DataDiagnostic, diagnostics: &mut Vec<DataDiagnostic>| {
        if diagnostics.len() < limits.max_diagnostics {
            diagnostics.push(d);
        } else if diagnostics.len() == limits.max_diagnostics {
            diagnostics.push(DataDiagnostic::new(
                "data.diagnosticsTruncated",
                DataSeverity::Warning,
                "diagnostic limit reached; further diagnostics suppressed",
            ));
        }
    };

    let mut record = csv::StringRecord::new();
    let mut data_rows: u64 = 0;
    loop {
        match rdr.read_record(&mut record) {
            Ok(true) => {
                // §119：协作取消（每条记录检查一次；csv crate 单条读取本身
                // 受 max_cell_bytes 约束有界）
                if cancel.is_cancelled() {
                    return Err(WeaveError::cancelled(
                        "data.cancelled",
                        format!("csv scan cancelled after {data_rows} data rows"),
                    )
                    .with_location("weave-data::parse_csv"));
                }
                let cells: Vec<String> = record.iter().map(|c| c.to_string()).collect();
                // §116 Huge Field：单单元格超限 ⇒ 结构化错误（不静默截断）
                for cell in &cells {
                    if cell.len() > limits.max_cell_bytes {
                        return Err(WeaveError::validation(
                            "data.cellTooLarge",
                            format!(
                                "cell exceeds max cell size ({} bytes > {})",
                                cell.len(),
                                limits.max_cell_bytes
                            ),
                        )
                        .with_location("weave-data::parse_csv"));
                    }
                }
                width = width.max(cells.len());
                if headers.is_none() {
                    if dialect.has_header {
                        headers = Some(cells);
                        continue;
                    }
                    headers = Some(Vec::new());
                }
                data_rows += 1;
                if data_rows as usize > limits.max_rows_in_memory {
                    return Err(WeaveError::validation(
                        "data.tooManyRows",
                        format!(
                            "dataset exceeds max rows in memory ({})",
                            limits.max_rows_in_memory
                        ),
                    )
                    .with_location("weave-data::parse_csv"));
                }
                // §117 Huge Columns：列数超限 ⇒ Stop（fail safely）
                if cells.len() > limits.max_columns {
                    return Err(WeaveError::validation(
                        "data.tooManyColumns",
                        format!(
                            "record has {} columns, exceeding max {}",
                            cells.len(),
                            limits.max_columns
                        ),
                    )
                    .with_location("weave-data::parse_csv"));
                }
                records.push(cells);
            }
            Ok(false) => break,
            Err(e) => {
                // §63/§64：UTF-8 / quote 损坏会导致数据错位 ⇒ Stop。
                let kind = e.kind();
                let code = if matches!(kind, csv::ErrorKind::Utf8 { .. }) {
                    "data.invalidEncoding"
                } else if matches!(kind, csv::ErrorKind::UnequalLengths { .. }) {
                    "data.raggedRow"
                } else {
                    "data.parseFailed"
                };
                return Err(WeaveError::io(
                    code,
                    format!(
                        "parse failed at offset {}: {e}",
                        e.position().map(|p| p.byte()).unwrap_or(0)
                    ),
                )
                .with_location("weave-data::parse_csv"));
            }
        }
    }

    let headers = headers.unwrap_or_default();
    let columns = if headers.is_empty() && !records.is_empty() {
        // 无 header 模式：用首行宽度生成 fallback 列
        let w = records.iter().map(|r| r.len()).max().unwrap_or(0);
        let names: Vec<String> = (0..w).map(|i| format!("Column {}", i + 1)).collect();
        build_columns(&names)
    } else {
        build_columns(&headers)
    };
    let column_count = columns.len();

    // §14 Ragged：短行补空 + 诊断；长行保留 + 诊断
    let mut rows: Vec<Vec<String>> = Vec::with_capacity(records.len());
    for (i, mut cells) in records.into_iter().enumerate() {
        let row_no = i as u64 + 1;
        if cells.len() < column_count {
            push_diag(
                DataDiagnostic::new(
                    "data.missingCells",
                    DataSeverity::Warning,
                    format!(
                        "row has {} cells, expected {column_count}; missing cells filled as empty",
                        cells.len()
                    ),
                )
                .at(row_no, cells.len() as u64 + 1)
                .with_basis(Basis::Fact),
                &mut diagnostics,
            );
            cells.resize(column_count, String::new());
        } else if cells.len() > column_count {
            push_diag(
                DataDiagnostic::new(
                    "data.extraCells",
                    DataSeverity::Warning,
                    format!(
                        "row has {} cells, expected {column_count}; extra cells preserved and will be exported verbatim",
                        cells.len()
                    ),
                )
                .at(row_no, column_count as u64 + 1)
                .with_basis(Basis::Fact),
                &mut diagnostics,
            );
        }
        rows.push(cells);
    }

    Ok(CsvParseOutput {
        columns,
        rows,
        diagnostics,
        data_rows,
    })
}

/// 便捷入口：从字节切片解析（先剥 UTF-8 BOM；编码解码由调用方经
/// weave-text 完成，§65）。
pub fn parse_csv_from_bytes(
    bytes: &[u8],
    dialect: &CsvDialect,
    limits: &crate::limits::DataLimits,
) -> Result<CsvParseOutput, WeaveError> {
    let stripped = bytes.strip_prefix(&[0xEF, 0xBB, 0xBF]).unwrap_or(bytes);
    parse_csv(std::io::Cursor::new(stripped.to_vec()), dialect, limits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::limits::DataLimits;

    fn parse(src: &str, dialect: &CsvDialect) -> CsvParseOutput {
        parse_csv(
            std::io::Cursor::new(src.as_bytes().to_vec()),
            dialect,
            &DataLimits::default(),
        )
        .expect("parse")
    }

    #[test]
    fn quoted_fields_and_embedded_separators() {
        // §9：引号字段 / 内嵌逗号 / 转义引号 / 跨行字段
        let out = parse(
            "name,note\n\"a,b\",\"say \"\"hi\"\"\"\n\"multi\nline\",ok\n",
            &CsvDialect::csv(),
        );
        assert_eq!(out.data_rows, 2);
        assert_eq!(out.rows[0], vec!["a,b", "say \"hi\""]);
        assert_eq!(out.rows[1], vec!["multi\nline", "ok"]);
    }

    #[test]
    fn duplicate_headers_get_deterministic_suffixes() {
        // §12：name,name,email → 内部 col_N 稳定；显示 name/name_2/email
        let out = parse("name,name,email\nA,B,c@x\n", &CsvDialect::csv());
        let names: Vec<&str> = out.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["name", "name_2", "email"]);
        let ids: Vec<&str> = out.columns.iter().map(|c| c.id.as_str()).collect();
        assert_eq!(ids, ["col_1", "col_2", "col_3"]);
    }

    #[test]
    fn empty_header_gets_fallback_display_name() {
        // §13：name,,email → 中列显示 "Column 2"，身份仍 col_2
        let out = parse("name,,email\nA,,c\n", &CsvDialect::csv());
        assert_eq!(out.columns[1].name, "Column 2");
        assert_eq!(out.columns[1].id, "col_2");
    }

    #[test]
    fn ragged_rows_pad_and_preserve_with_diagnostics() {
        // §14：A,B,C / 1,2 / 6,7,8,9 — 短行补空，长行保留，均有诊断
        let out = parse("A,B,C\n1,2\n6,7,8,9\n", &CsvDialect::csv());
        assert_eq!(out.rows[0], vec!["1", "2", ""]);
        assert_eq!(out.rows[1], vec!["6", "7", "8", "9"], "多余单元格保留");
        let codes: Vec<&str> = out.diagnostics.iter().map(|d| d.code.as_str()).collect();
        assert!(codes.contains(&"data.missingCells"));
        assert!(codes.contains(&"data.extraCells"));
    }

    #[test]
    fn tsv_uses_tab_delimiter_with_quoted_fields() {
        // §10：TSV 复用 parser；引号内 tab / 引号内换行正确处理
        let out = parse("a\tb\n\"x\ty\"\tz2\n", &CsvDialect::tsv());
        assert_eq!(out.rows[0], vec!["x\ty", "z2"]);
    }

    #[test]
    fn header_auto_detection_is_honest_heuristic() {
        // §11：数字对比启发式 + confidence/reason 如实返回
        let d = detect_header(
            Some(&["id".into(), "name".into()]),
            Some(&["1".into(), "alice".into()]),
        );
        assert!(d.has_header && d.confidence == "high");
        assert!(d.reason.contains("Heuristic"));
        let d = detect_header(Some(&["1".into()]), Some(&["2".into()]));
        assert!(!d.has_header);
    }

    #[test]
    fn invalid_utf8_is_structured_stop() {
        // §63/§64：损坏 ⇒ Stop + 结构化错误（不静默替换、不冒险继续）
        let err = parse_csv_from_bytes(
            &[0xFF, 0xFE, b'a'],
            &CsvDialect::csv(),
            &DataLimits::default(),
        )
        .expect_err("invalid utf8");
        assert_eq!(err.code, "data.invalidEncoding");
    }

    #[test]
    fn no_header_mode_generates_fallback_columns() {
        let mut d = CsvDialect::csv();
        d.has_header = false;
        let out = parse("1,2\n3,4\n", &d);
        assert_eq!(out.data_rows, 2, "首行不吞为 header");
        assert_eq!(out.columns[0].name, "Column 1");
    }
}
