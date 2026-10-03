//! JSONL（M5 上 §59–§62）：逐行流式解析（不做 read-all 巨串），
//! 畸形行按模式处理：FailFast（Stop）/ CollectErrors（继续 + 计数）。

use std::io::{BufRead, BufReader, Read};

use weave_core::prelude::WeaveError;

use crate::diagnostics::{DataDiagnostic, DataSeverity};

/// 畸形行处理模式（§62）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum JsonlErrorMode {
    /// 首个畸形行即 Stop。
    FailFast,
    /// 收集诊断继续；valid/invalid 计数如实报告。
    CollectErrors,
}

/// JSONL 解析结果（§62）。
#[derive(Debug)]
pub struct JsonlReport {
    /// 合法记录（serde_json 值，保留键序）。
    pub records: Vec<serde_json::Value>,
    pub diagnostics: Vec<DataDiagnostic>,
    pub valid_count: u64,
    pub invalid_count: u64,
    /// 输入物理行数（含空行——空行按 skip 计入 invalid？不：空行跳过且
    /// 不计数为记录，单列 skipped_empty 事实字段）。
    pub skipped_empty: u64,
}

/// 从字节流逐行解析 JSONL（§59：BufRead 流式；max_rows 保护 §88）。
pub fn parse_jsonl(
    reader: impl Read,
    mode: JsonlErrorMode,
    max_rows: usize,
    max_diagnostics: usize,
) -> Result<JsonlReport, WeaveError> {
    let reader = BufReader::new(reader);
    let mut records = Vec::new();
    let mut diagnostics = Vec::new();
    let mut valid = 0u64;
    let mut invalid = 0u64;
    let mut skipped_empty = 0u64;

    for (index, line) in reader.lines().enumerate() {
        if records.len() >= max_rows {
            return Err(WeaveError::validation(
                "data.tooManyRows",
                format!("jsonl exceeds max rows ({max_rows})"),
            )
            .with_location("weave-data::parse_jsonl"));
        }
        let line = line.map_err(|e| {
            WeaveError::io("data.readFailed", format!("line {}: {e}", index + 1))
                .with_location("weave-data::parse_jsonl")
        })?;
        let trimmed = line.trim();
        if trimmed.is_empty() {
            skipped_empty += 1;
            continue;
        }
        match serde_json::from_str::<serde_json::Value>(trimmed) {
            Ok(value) => {
                valid += 1;
                records.push(value);
            }
            Err(e) => {
                invalid += 1;
                if diagnostics.len() < max_diagnostics {
                    diagnostics.push(
                        DataDiagnostic::new(
                            "data.jsonlInvalidLine",
                            DataSeverity::ParseError,
                            e.to_string(),
                        )
                        .at(index as u64 + 1, 1)
                        .with_basis(crate::diagnostics::Basis::Fact),
                    );
                }
                if mode == JsonlErrorMode::FailFast {
                    return Err(WeaveError::validation(
                        "data.jsonlInvalidLine",
                        format!("line {}: {e}", index + 1),
                    )
                    .with_location("weave-data::parse_jsonl"));
                }
            }
        }
    }

    Ok(JsonlReport {
        records,
        diagnostics,
        valid_count: valid,
        invalid_count: invalid,
        skipped_empty,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(src: &str, mode: JsonlErrorMode) -> Result<JsonlReport, WeaveError> {
        parse_jsonl(
            std::io::Cursor::new(src.as_bytes().to_vec()),
            mode,
            100_000,
            1_000,
        )
    }

    #[test]
    fn valid_lines_parse_in_order() {
        // §59/§60：一行一值；顺序保持
        let r = parse("{\"id\":1}\n{\"id\":2}\n", JsonlErrorMode::FailFast).expect("ok");
        assert_eq!(r.valid_count, 2);
        assert_eq!(r.records[0], serde_json::json!({"id": 1}));
        assert_eq!(r.invalid_count, 0);
    }

    #[test]
    fn fail_fast_stops_at_first_bad_line() {
        // §62：FailFast ⇒ Stop（后续合法行也不处理）
        let err = parse("{\"id\":1}\nbroken\n{\"id\":3}\n", JsonlErrorMode::FailFast)
            .expect_err("fail fast");
        assert!(err.message.contains("line 2"));
    }

    #[test]
    fn collect_errors_reports_valid_invalid_and_line_numbers() {
        // §62：Collect 模式三计数 + 行号诊断，不偷删
        let r = parse(
            "{\"id\":1}\nbroken\n{\"id\":3}\n\n",
            JsonlErrorMode::CollectErrors,
        )
        .expect("collect");
        assert_eq!(r.valid_count, 2);
        assert_eq!(r.invalid_count, 1);
        assert_eq!(r.skipped_empty, 1);
        assert_eq!(r.diagnostics[0].row, Some(2));
        assert_eq!(r.records.len(), 2, "非法行不进 records（如实）");
    }

    #[test]
    fn empty_lines_are_skipped_and_counted() {
        let r = parse("\n\n{\"a\":1}\n", JsonlErrorMode::FailFast).expect("ok");
        assert_eq!(r.skipped_empty, 2);
        assert_eq!(r.valid_count, 1);
    }
}
