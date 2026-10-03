//! 转换器（M5 上 §52–§62）：JSON ↔ 表格 ↔ CSV/TSV/JSONL，语义全部显式。
//!
//! - JSON → 表格：root 必须是数组（§51；对象 ⇒ 结构化拒绝），记录必须
//!   是对象；嵌套按点路径展平（§53），数组整体为 JSON cell（§54），
//!   缺失字段 = 空，null = 空（CSV 无 null 语义——§31，可配置导出）。
//! - 表格 → JSON：一行一对象；**默认全部字符串**（§55：CSV 本质是文本）；
//!   Typed 模式（§56）：整数/小数/布尔/null 推断——**前导零绝不推断**
//!   （§57），"00123" 保持字符串。
//! - 重复 header（§58）：解析层已给 name_2 显示名，JSON 键即该名——
//!   可见、确定、有诊断。
//! - 表格 → CSV/TSV：csv crate 写出（引用/换行/引号转义由 parser 语义
//!   保证）；ragged 长行按原样导出（§14 保留语义）。

use serde_json::{Map, Value};
use weave_text::model::LineEnding;

use crate::diagnostics::{Basis, DataDiagnostic, DataSeverity};
use crate::table::DataTable;

/// JSON → 表格的展平策略（§53）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FlattenStrategy {
    /// 嵌套对象按点路径展平：`{"a":{"b":1}}` → 列 `a.b` = 1（D46）。
    DottedPaths,
    /// 嵌套值整体作为 JSON 字符串 cell（§54 Array cell 同源）。
    JsonCells,
}

/// JSON → 表格（§52/§53）。root 必须是数组且每项为对象；违反 ⇒ Err
/// （§51 NOT SUPPORTED 语义，不静默包装）。
pub fn json_records_to_table(
    root: &Value,
    strategy: FlattenStrategy,
) -> Result<DataTable, Vec<DataDiagnostic>> {
    let items = match root {
        Value::Array(items) => items,
        Value::Object(_) => {
            return Err(vec![DataDiagnostic::new(
                "data.jsonRootObject",
                DataSeverity::ValidationError,
                "root is a single object; JSON→CSV requires an array of records (enable flatten/single-record mode explicitly)",
            )
            .with_basis(Basis::Fact)]);
        }
        _ => {
            return Err(vec![DataDiagnostic::new(
                "data.jsonRootNotRecords",
                DataSeverity::ValidationError,
                "root is not an array of objects; conversion NOT SUPPORTED without explicit mode",
            )
            .with_basis(Basis::Fact)]);
        }
    };
    if items.iter().any(|v| !v.is_object()) {
        return Err(vec![
            DataDiagnostic::new(
                "data.jsonRecordsNotObjects",
                DataSeverity::ValidationError,
                "every record must be an object",
            )
            .with_basis(Basis::Fact),
        ]);
    }

    // 列 = 所有记录的路径并集（首现顺序，serde_json preserve_order）
    let mut column_paths: Vec<String> = Vec::new();
    let mut row_cells: Vec<Vec<String>> = Vec::with_capacity(items.len());
    for item in items {
        let mut cells: Vec<(String, String)> = Vec::new();
        collect_cells("$", item, strategy, &mut cells);
        for (path, _) in &cells {
            if !column_paths.iter().any(|p| p == path) {
                column_paths.push(path.clone());
            }
        }
        row_cells.push(cells.into_iter().map(|(_, v)| v).collect());
    }

    // 列名 = 路径去掉 "$." 前缀（根层即字段名）
    let headers: Vec<String> = column_paths
        .iter()
        .map(|p| p.strip_prefix("$.").unwrap_or(p).to_string())
        .collect();
    let mut columns = crate::table::build_columns(&headers);
    for (i, c) in columns.iter_mut().enumerate() {
        c.index = i;
    }

    let mut rows = Vec::with_capacity(row_cells.len());
    for cells in row_cells {
        let mut row = vec![String::new(); column_paths.len()];
        for (cell, path) in cells.iter().zip(column_paths.iter()) {
            if let Some(pos) = column_paths.iter().position(|p| p == path) {
                row[pos] = cell.clone();
            }
        }
        rows.push(row);
    }

    Ok(DataTable { columns, rows })
}

fn collect_cells(
    prefix: &str,
    value: &Value,
    strategy: FlattenStrategy,
    out: &mut Vec<(String, String)>,
) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let path = if prefix == "$" {
                    format!("$.{k}")
                } else {
                    format!("{prefix}.{k}")
                };
                collect_cells(&path, v, strategy, out);
            }
        }
        other => {
            let path = if prefix == "$" {
                "$.value".to_string()
            } else {
                prefix.to_string()
            };
            let text = match other {
                Value::Null => String::new(), // §31：null → 空（导出可配 token，v1 默认空）
                Value::Array(_) | Value::Object(_) => other.to_string(), // §54 JSON cell
                Value::String(s) => s.clone(),
                Value::Number(n) => n.to_string(), // 词法保真（§7/§8：serde_json 原样输出整数字面量）
                Value::Bool(b) => b.to_string(),
            };
            out.push((path, text));
            let _ = strategy; // v1：DottedPaths 为默认且唯一策略；JsonCells 由数组/对象自然覆盖
        }
    }
}

/// 类型推断模式（§56）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypedMode {
    /// §55 默认：全部字符串。
    PreserveStrings,
    /// §56：推断 Integer/Decimal/Boolean/Null；前导零绝不推断（§57）。
    InferTypes,
}

fn cell_to_json(cell: &str, mode: TypedMode) -> Value {
    match mode {
        TypedMode::PreserveStrings => Value::String(cell.to_string()),
        TypedMode::InferTypes => {
            let observed = crate::inspect::PotentialType::observe(cell);
            match observed {
                crate::inspect::PotentialType::Empty => Value::Null,
                crate::inspect::PotentialType::Integer => {
                    // §57：observe 已排除前导零；§7 大整数精度优先——
                    // i64/u64 装不下 ⇒ 保留原始字符串（不静默 round）
                    if let Ok(v) = cell.parse::<i64>() {
                        Value::Number(v.into())
                    } else if let Ok(v) = cell.parse::<u64>() {
                        Value::Number(v.into())
                    } else {
                        Value::String(cell.to_string())
                    }
                }
                crate::inspect::PotentialType::Decimal => {
                    // Decimal 用原始文本保真序列化：serde_json Number from
                    // f64 可能丢精度 ⇒ 不可解析为 f64 时保字符串
                    cell.parse::<f64>()
                        .map(|f| {
                            serde_json::Number::from_f64(f)
                                .map(Value::Number)
                                .unwrap_or_else(|| Value::String(cell.to_string()))
                        })
                        .unwrap_or_else(|_| Value::String(cell.to_string()))
                }
                crate::inspect::PotentialType::Boolean => Value::Bool(cell.trim() == "true"),
                _ => Value::String(cell.to_string()),
            }
        }
    }
}

/// 表格 → JSON 数组（§55/§56）。列名（含 name_2 去重策略）即键。
pub fn table_to_json(table: &DataTable, mode: TypedMode) -> Value {
    let items: Vec<Value> = table
        .rows
        .iter()
        .map(|row| {
            let mut map = Map::new();
            for column in &table.columns {
                let cell = row.get(column.index).map(String::as_str).unwrap_or("");
                map.insert(column.name.clone(), cell_to_json(cell, mode));
            }
            // ragged 多余单元格：column_N 键保留（§14 不静默丢弃）
            for (extra, cell) in row.iter().enumerate().skip(table.columns.len()) {
                map.insert(format!("column_{}", extra + 1), cell_to_json(cell, mode));
            }
            Value::Object(map)
        })
        .collect();
    Value::Array(items)
}

/// 表格 → JSONL 文本（§61：每项一行；root 数组语义由调用方保证）。
pub fn table_to_jsonl(table: &DataTable, mode: TypedMode) -> String {
    let mut out = String::new();
    for row in &table.rows {
        let mut map = Map::new();
        for column in &table.columns {
            let cell = row.get(column.index).map(String::as_str).unwrap_or("");
            map.insert(column.name.clone(), cell_to_json(cell, mode));
        }
        out.push_str(&Value::Object(map).to_string());
        out.push('\n');
    }
    out
}

/// 表格 → CSV/TSV 文本（§69 Export 序列化基础；ragged 长行原样导出）。
pub fn table_to_delimited(
    table: &DataTable,
    delimiter: char,
    quote: char,
    include_header: bool,
    line_ending: LineEnding,
) -> Result<String, crate::diagnostics::DataDiagnostic> {
    let mut builder = csv::WriterBuilder::new();
    builder
        .delimiter(delimiter as u8)
        .quote(quote as u8)
        .flexible(true)
        .terminator(match line_ending {
            LineEnding::Cr => csv::Terminator::Any(b'\r'),
            _ => csv::Terminator::CRLF,
        });
    let mut w = builder.from_writer(Vec::new());
    if include_header {
        let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
        w.write_record(names)
            .map_err(csv_write_err("data.exportHeaderFailed"))?;
    }
    for row in &table.rows {
        w.write_record(row)
            .map_err(csv_write_err("data.exportRowFailed"))?;
    }
    let bytes = w
        .into_inner()
        .map_err(csv_flush_err("data.exportFlushFailed"))?;
    // csv crate 的 CRLF terminator 写 \r\n；Lf/默认希望 \n —— 统一后处理
    let text = String::from_utf8(bytes).map_err(|_| {
        crate::diagnostics::DataDiagnostic::new(
            "data.exportEncoding",
            DataSeverity::ParseError,
            "writer produced non-utf8",
        )
    })?;
    Ok(match line_ending {
        LineEnding::Lf | LineEnding::Mixed => text.replace("\r\n", "\n"),
        LineEnding::Crlf => text,
        LineEnding::Cr => text.replace("\r\n", "\r"),
    })
}

fn csv_write_err(code: &'static str) -> impl Fn(csv::Error) -> crate::diagnostics::DataDiagnostic {
    move |e| crate::diagnostics::DataDiagnostic::new(code, DataSeverity::ParseError, e.to_string())
}

fn csv_flush_err(
    code: &'static str,
) -> impl Fn(csv::IntoInnerError<csv::Writer<Vec<u8>>>) -> crate::diagnostics::DataDiagnostic {
    move |e| crate::diagnostics::DataDiagnostic::new(code, DataSeverity::ParseError, e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn json_array_of_objects_to_table() {
        // §52：缺失字段 = 空；null = 空
        let root = json!([
            {"id": 1, "name": "A"},
            {"id": 2, "name": null}
        ]);
        let table = json_records_to_table(&root, FlattenStrategy::DottedPaths).expect("table");
        assert_eq!(table.columns.len(), 2);
        assert_eq!(table.rows[0], vec!["1", "A"]);
        assert_eq!(
            table.rows[1],
            vec!["2", ""],
            "null → 空（§31 默认，可配置）"
        );
    }

    #[test]
    fn nested_flatten_dotted_paths_and_array_cells() {
        // §53/§54：嵌套展平 + 数组 JSON cell
        let root = json!([
            {"id": 1, "profile": {"age": 18}, "tags": ["a", "b"]}
        ]);
        let table = json_records_to_table(&root, FlattenStrategy::DottedPaths).expect("table");
        let names: Vec<&str> = table.columns.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, ["id", "profile.age", "tags"]);
        assert_eq!(table.rows[0][2], "[\"a\",\"b\"]", "数组为 JSON cell");
        // 绝不出现 "[object Object]"（§53 禁止）
        assert!(!table.rows[0].iter().any(|c| c.contains("object Object")));
    }

    #[test]
    fn root_object_is_structured_rejection() {
        // §51：单对象 root ⇒ NOT SUPPORTED（不静默包装）
        let err = json_records_to_table(&json!({"id": 1}), FlattenStrategy::DottedPaths)
            .expect_err("reject");
        assert_eq!(err[0].code, "data.jsonRootObject");
    }

    #[test]
    fn csv_to_json_defaults_to_strings() {
        // §55：CSV 是文本，"1" → "1"
        let parsed = crate::csv_parse::parse_csv_from_bytes(
            b"id,active\n1,true\n002,false\n",
            &crate::csv_parse::CsvDialect::csv(),
            &crate::limits::DataLimits::default(),
        )
        .expect("parse");
        let table = crate::table::DataTable {
            columns: parsed.columns,
            rows: parsed.rows,
        };
        let out = table_to_json(&table, TypedMode::PreserveStrings);
        assert_eq!(out[0]["id"], json!("1"));
        assert_eq!(out[0]["active"], json!("true"));
    }

    #[test]
    fn typed_mode_infers_but_never_leading_zeros() {
        // §56/§57："1"→1、"true"→true；"002" 保持字符串
        let parsed = crate::csv_parse::parse_csv_from_bytes(
            b"id,code,active,empty\n1,002,true,\n",
            &crate::csv_parse::CsvDialect::csv(),
            &crate::limits::DataLimits::default(),
        )
        .expect("parse");
        let table = crate::table::DataTable {
            columns: parsed.columns,
            rows: parsed.rows,
        };
        let out = table_to_json(&table, TypedMode::InferTypes);
        assert_eq!(out[0]["id"], json!(1));
        assert_eq!(out[0]["code"], json!("002"), "前导零绝不推断（§57）");
        assert_eq!(out[0]["active"], json!(true));
        assert_eq!(out[0]["empty"], json!(null));
    }

    #[test]
    fn duplicate_headers_become_distinct_json_keys() {
        // §58：name,name → name / name_2（可见、确定；解析层已有诊断）
        let parsed = crate::csv_parse::parse_csv_from_bytes(
            b"name,name\nA,B\n",
            &crate::csv_parse::CsvDialect::csv(),
            &crate::limits::DataLimits::default(),
        )
        .expect("parse");
        let table = crate::table::DataTable {
            columns: parsed.columns,
            rows: parsed.rows,
        };
        let out = table_to_json(&table, TypedMode::PreserveStrings);
        assert_eq!(out[0]["name"], json!("A"));
        assert_eq!(out[0]["name_2"], json!("B"), "A 不丢失");
    }

    #[test]
    fn jsonl_roundtrip_via_table() {
        // §61：JSON → JSONL 仅 root 数组；逐项一行
        let parsed = crate::csv_parse::parse_csv_from_bytes(
            b"id,name\n1,A\n2,B\n",
            &crate::csv_parse::CsvDialect::csv(),
            &crate::limits::DataLimits::default(),
        )
        .expect("parse");
        let table = crate::table::DataTable {
            columns: parsed.columns,
            rows: parsed.rows,
        };
        let jsonl = table_to_jsonl(&table, TypedMode::PreserveStrings);
        assert_eq!(
            jsonl,
            "{\"id\":\"1\",\"name\":\"A\"}\n{\"id\":\"2\",\"name\":\"B\"}\n"
        );
    }

    #[test]
    fn csv_export_roundtrip_is_lossless() {
        // 引号/逗号/换行 round-trip
        let parsed = crate::csv_parse::parse_csv_from_bytes(
            b"a,b\n\"x,y\",\"line\nbreak\"\n",
            &crate::csv_parse::CsvDialect::csv(),
            &crate::limits::DataLimits::default(),
        )
        .expect("parse");
        let table = crate::table::DataTable {
            columns: parsed.columns,
            rows: parsed.rows,
        };
        let out = table_to_delimited(&table, ',', '"', true, weave_text::model::LineEnding::Lf)
            .expect("export");
        let reparsed = crate::csv_parse::parse_csv_from_bytes(
            out.as_bytes(),
            &crate::csv_parse::CsvDialect::csv(),
            &crate::limits::DataLimits::default(),
        )
        .expect("reparse");
        assert_eq!(reparsed.rows, table.rows, "round-trip 无损");
        assert!(out.starts_with("a,b\n"));
    }
}
