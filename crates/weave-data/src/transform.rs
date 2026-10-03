//! DataTransformPlan（M5 上 §78–§80）：有序、确定性、可序列化的清洗规则。
//!
//! 规则顺序是数据语义的一部分（§79）——执行严格按序，绝不自动重排。
//! 全部纯逻辑（§81）：输入表 + 计划 ⇒ 输出表 + 诊断；文件写入在应用层。

use serde::{Deserialize, Serialize};
#[cfg(feature = "specta")]
use specta::Type;

use crate::diagnostics::{Basis, DataDiagnostic, DataSeverity};
use crate::table::DataTable;

/// 空值定义（§30/§31：哪些表示计入"空"——绝不默认把 "NULL"/"N/A"/"0"
/// 与空串混同）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(rename_all = "camelCase")]
pub struct NullPolicy {
    pub empty_string: bool,
    pub whitespace_only: bool,
    /// 显式配置的 null-like token（如 "NULL"、"N/A"）。
    pub tokens: Vec<String>,
}

impl Default for NullPolicy {
    fn default() -> Self {
        Self {
            empty_string: true,
            whitespace_only: false,
            tokens: Vec::new(),
        }
    }
}

impl NullPolicy {
    pub fn is_null(&self, cell: &str) -> bool {
        if self.empty_string && cell.is_empty() {
            return true;
        }
        if self.whitespace_only && cell.trim().is_empty() {
            return true;
        }
        self.tokens.iter().any(|t| t == cell)
    }
}

/// 清洗规则（§0.3/§28–§37/§40/§32–§34）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum DataTransformRule {
    /// Trim（§32）：left/right/both。
    Trim { column_id: String, side: String },
    /// 大小写归一（§33：lower/upper/title——确定性简单变换，D40 同源）。
    CaseNormalize { column_id: String, form: String },
    /// Fill Empty（§30）：按 NullPolicy 判空后填充。
    FillEmpty {
        column_id: String,
        value: String,
        #[serde(default)]
        null_policy: Option<NullPolicy>,
    },
    /// Find / Replace（§34：literal exact ± case；regex 复用 M4 线性引擎
    /// 纪律——escape + 限额）。
    FindReplace {
        column_id: String,
        find: String,
        replace_with: String,
        regex: bool,
        case_sensitive: bool,
    },
    /// Column Split（§28）：delimiter + max splits + 新列名；缺 delimiter
    /// ⇒ 原列保留、新列留空（§28 显式策略）。
    SplitColumn {
        column_id: String,
        delimiter: String,
        max_splits: u32,
        new_names: Vec<String>,
    },
    /// Column Merge（§29）：多列 → 新列；源列默认保留（§29 明示）。
    MergeColumns {
        column_ids: Vec<String>,
        separator: String,
        new_name: String,
    },
    /// Column Delete（§26：破坏性——Preview first）。
    DeleteColumn { column_id: String },
    /// Column Rename（§25：只改显示名）。
    RenameColumn { column_id: String, new_name: String },
    /// Deduplicate Rows（§35/§36）：列集（空 = 全列）+ Keep First/Last。
    DeduplicateRows {
        column_ids: Vec<String>,
        keep: String,
    },
    /// Date Normalization（§39）：仅无歧义 yyyy/mm/dd → ISO；歧义 ⇒
    /// 逐格 Anomaly 不猜测（§38）。
    DateNormalizeIso { column_id: String },
    /// Numeric Normalization（§40：显式 decimal/thousands 分隔符，§40 禁
    /// 全局逗号替换）。
    NumericNormalize {
        column_id: String,
        decimal_separator: String,
        thousands_separator: Option<String>,
    },
}

/// 清洗计划（§78）：有序 + 确定性 + 可序列化（面向 M10 复用设计）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(rename_all = "camelCase")]
pub struct DataTransformPlan {
    pub rules: Vec<DataTransformRule>,
}

/// 校验（§80）：列存在、参数合法。失败 ⇒ 结构化错误集合。
pub fn validate_plan(plan: &DataTransformPlan, table: &DataTable) -> Vec<DataDiagnostic> {
    let mut errors = Vec::new();
    for (i, rule) in plan.rules.iter().enumerate() {
        let check_column = |id: &str, errors: &mut Vec<DataDiagnostic>| {
            if table.index_of(id).is_none() {
                errors.push(
                    DataDiagnostic::new(
                        "data.ruleColumnMissing",
                        DataSeverity::ValidationError,
                        format!("rule {}: column '{id}' does not exist", i + 1),
                    )
                    .with_basis(Basis::Rule),
                );
            }
        };
        match rule {
            DataTransformRule::Trim { column_id, side } => {
                check_column(column_id, &mut errors);
                if !matches!(side.as_str(), "left" | "right" | "both") {
                    errors.push(DataDiagnostic::new(
                        "data.ruleInvalidSide",
                        DataSeverity::ValidationError,
                        format!("rule {}: unknown trim side '{side}'", i + 1),
                    ));
                }
            }
            DataTransformRule::CaseNormalize { column_id, form } => {
                check_column(column_id, &mut errors);
                if !matches!(form.as_str(), "lower" | "upper" | "title") {
                    errors.push(DataDiagnostic::new(
                        "data.ruleInvalidCase",
                        DataSeverity::ValidationError,
                        format!("rule {}: unknown case form '{form}'", i + 1),
                    ));
                }
            }
            DataTransformRule::FillEmpty { column_id, .. } => {
                check_column(column_id, &mut errors);
            }
            DataTransformRule::FindReplace {
                column_id,
                find,
                regex,
                ..
            } => {
                check_column(column_id, &mut errors);
                if *regex {
                    if let Err(e) = regex::Regex::new(find) {
                        errors.push(DataDiagnostic::new(
                            "text.invalidRegex",
                            DataSeverity::ValidationError,
                            format!("rule {}: {e}", i + 1),
                        ));
                    }
                } else if find.is_empty() {
                    errors.push(DataDiagnostic::new(
                        "text.emptyFind",
                        DataSeverity::ValidationError,
                        format!("rule {}: find must not be empty", i + 1),
                    ));
                }
            }
            DataTransformRule::SplitColumn {
                column_id,
                new_names,
                ..
            } => {
                check_column(column_id, &mut errors);
                if new_names.is_empty() {
                    errors.push(DataDiagnostic::new(
                        "data.ruleInvalidSplit",
                        DataSeverity::ValidationError,
                        format!(
                            "rule {}: split requires at least one new column name",
                            i + 1
                        ),
                    ));
                }
            }
            DataTransformRule::MergeColumns {
                column_ids,
                new_name,
                ..
            } => {
                for id in column_ids {
                    check_column(id, &mut errors);
                }
                if new_name.trim().is_empty() {
                    errors.push(DataDiagnostic::new(
                        "data.ruleInvalidMerge",
                        DataSeverity::ValidationError,
                        format!("rule {}: merge requires a new column name", i + 1),
                    ));
                }
            }
            DataTransformRule::DeleteColumn { column_id } => {
                check_column(column_id, &mut errors);
            }
            DataTransformRule::RenameColumn {
                column_id,
                new_name,
            } => {
                check_column(column_id, &mut errors);
                if new_name.trim().is_empty() {
                    errors.push(DataDiagnostic::new(
                        "data.ruleInvalidRename",
                        DataSeverity::ValidationError,
                        format!("rule {}: rename requires a non-empty name", i + 1),
                    ));
                }
            }
            DataTransformRule::DeduplicateRows { column_ids, keep } => {
                for id in column_ids {
                    check_column(id, &mut errors);
                }
                if !matches!(keep.as_str(), "first" | "last") {
                    errors.push(DataDiagnostic::new(
                        "data.ruleInvalidKeep",
                        DataSeverity::ValidationError,
                        format!("rule {}: keep must be first|last", i + 1),
                    ));
                }
            }
            DataTransformRule::DateNormalizeIso { column_id } => {
                check_column(column_id, &mut errors);
            }
            DataTransformRule::NumericNormalize {
                column_id,
                decimal_separator,
                ..
            } => {
                check_column(column_id, &mut errors);
                if decimal_separator.chars().count() != 1 {
                    errors.push(DataDiagnostic::new(
                        "data.ruleInvalidSeparator",
                        DataSeverity::ValidationError,
                        format!("rule {}: decimal separator must be one character", i + 1),
                    ));
                }
            }
        }
    }
    errors
}

fn title_case(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at_word_start = true;
    for ch in s.chars() {
        if ch.is_whitespace() {
            at_word_start = true;
            out.push(ch);
        } else if at_word_start {
            out.extend(ch.to_uppercase());
            at_word_start = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

/// 应用计划（§78/§81：纯逻辑；严格按序；产出新表 + Anomaly/Warning + 变更计数）。
pub fn apply_plan(
    table: &DataTable,
    plan: &DataTransformPlan,
    null_policy: &NullPolicy,
    max_diagnostics: usize,
) -> Result<(DataTable, Vec<DataDiagnostic>, u64), Vec<DataDiagnostic>> {
    // §80：参数合法性先行；列存在性对"演化后的表"逐规则检查——
    // 前序规则（Split/Merge）会合法地创建后序规则引用的列
    let validation: Vec<DataDiagnostic> = validate_plan(plan, table)
        .into_iter()
        .filter(|d| d.code != "data.ruleColumnMissing")
        .collect();
    if !validation.is_empty() {
        return Err(validation);
    }
    let mut late_errors: Vec<DataDiagnostic> = Vec::new();

    let mut diagnostics: Vec<DataDiagnostic> = Vec::new();
    let mut columns = table.columns.clone();
    let mut rows = table.rows.clone();
    let mut rows_changed = 0u64;
    macro_rules! require_column {
        ($columns:expr, $rule_no:expr, $id:expr) => {
            match $columns.iter().position(|c| c.id == *$id) {
                Some(idx) => idx,
                None => {
                    late_errors.push(
                        DataDiagnostic::new(
                            "data.ruleColumnMissing",
                            DataSeverity::ValidationError,
                            format!("rule {}: column '{}' does not exist", $rule_no, $id),
                        )
                        .with_basis(Basis::Rule),
                    );
                    return Err(std::mem::take(&mut late_errors));
                }
            }
        };
    }

    macro_rules! diag {
        ($d:expr) => {{
            if diagnostics.len() < max_diagnostics {
                diagnostics.push($d);
            }
        }};
    }

    for (rule_i, rule) in plan.rules.iter().enumerate() {
        let rule_no = rule_i + 1;
        match rule {
            DataTransformRule::Trim { column_id, side } => {
                let idx = require_column!(columns, rule_no, column_id);
                {
                    for row in &mut rows {
                        let cell = row.get_mut(idx).expect("column exists");
                        let trimmed = match side.as_str() {
                            "left" => cell.trim_start(),
                            "right" => cell.trim_end(),
                            _ => cell.trim(),
                        };
                        if *cell != trimmed {
                            *cell = trimmed.to_string();
                            rows_changed += 1;
                        }
                    }
                }
            }
            DataTransformRule::CaseNormalize { column_id, form } => {
                let idx = require_column!(columns, rule_no, column_id);
                {
                    for row in &mut rows {
                        let cell = row.get_mut(idx).expect("column exists");
                        let converted = match form.as_str() {
                            "lower" => cell.to_lowercase(),
                            "upper" => cell.to_uppercase(),
                            _ => title_case(cell),
                        };
                        if *cell != converted {
                            *cell = converted;
                            rows_changed += 1;
                        }
                    }
                }
            }
            DataTransformRule::FillEmpty {
                column_id,
                value,
                null_policy: rule_policy,
            } => {
                let idx = require_column!(columns, rule_no, column_id);
                {
                    let policy = rule_policy.as_ref().unwrap_or(null_policy);
                    for row in &mut rows {
                        let cell = row.get_mut(idx).expect("column exists");
                        if policy.is_null(cell) && cell != value {
                            *cell = value.clone();
                            rows_changed += 1;
                        }
                    }
                }
            }
            DataTransformRule::FindReplace {
                column_id,
                find,
                replace_with,
                regex,
                case_sensitive,
            } => {
                let idx = require_column!(columns, rule_no, column_id);
                {
                    for row in &mut rows {
                        let cell = row.get_mut(idx).expect("column exists");
                        let new_value = if *regex {
                            match regex::Regex::new(find) {
                                Ok(re) => re.replace_all(cell, replace_with.as_str()).into_owned(),
                                Err(_) => cell.clone(),
                            }
                        } else if *case_sensitive {
                            cell.replace(find.as_str(), replace_with)
                        } else {
                            match regex::Regex::new(&format!("(?i){}", regex::escape(find))) {
                                Ok(re) => re
                                    .replace_all(cell, regex::NoExpand(replace_with.as_str()))
                                    .into_owned(),
                                Err(_) => cell.clone(),
                            }
                        };
                        if *cell != new_value {
                            *cell = new_value;
                            rows_changed += 1;
                        }
                    }
                }
            }
            DataTransformRule::SplitColumn {
                column_id,
                delimiter,
                max_splits,
                new_names,
            } => {
                let src_idx = require_column!(columns, rule_no, column_id);
                let insert_at = src_idx + 1;
                let n = new_names.len();
                // 新列 id 取现有最大序号之后，避免碰撞（§12 稳定身份）
                let max_id: usize = columns
                    .iter()
                    .filter_map(|c| {
                        c.id.strip_prefix("col_")
                            .and_then(|s| s.parse::<usize>().ok())
                    })
                    .max()
                    .unwrap_or(0);
                for (k, name) in new_names.iter().enumerate() {
                    columns.insert(
                        insert_at + k,
                        crate::table::ColumnDefinition {
                            id: format!("col_{}", max_id + 1 + k),
                            name: name.clone(),
                            index: insert_at + k,
                        },
                    );
                }
                for (i, c) in columns.iter_mut().enumerate() {
                    c.index = i;
                }
                for (ri, row) in rows.iter_mut().enumerate() {
                    let original = row.get(src_idx).cloned().unwrap_or_default();
                    let parts: Vec<&str> = if *max_splits == 0 {
                        original.split(delimiter.as_str()).collect()
                    } else {
                        original
                            .splitn(*max_splits as usize + 1, delimiter.as_str())
                            .collect()
                    };
                    // §28：缺 delimiter ⇒ 新列留空、原列保留（显式策略）
                    if parts.len() < 2 {
                        diag!(
                            DataDiagnostic::new(
                                "data.splitNoDelimiter",
                                DataSeverity::Anomaly,
                                "delimiter not found; original kept, generated columns empty",
                            )
                            .at(ri as u64 + 1, src_idx as u64 + 1)
                            .with_basis(Basis::Fact)
                        );
                    }
                    let mut generated: Vec<String> = vec![String::new(); n];
                    for (k, part) in parts.iter().take(n).enumerate() {
                        generated[k] = part.to_string();
                    }
                    let end = (src_idx + 1 + n).min(row.len());
                    row.splice(src_idx + 1..end.max(src_idx + 1), generated);
                }
                rows_changed += 1;
            }
            DataTransformRule::MergeColumns {
                column_ids,
                separator,
                new_name,
            } => {
                for id in column_ids {
                    require_column!(columns, rule_no, id);
                }
                let idxs: Vec<usize> = column_ids
                    .iter()
                    .filter_map(|id| columns.iter().position(|c| c.id == *id))
                    .collect();
                if idxs.is_empty() {
                    continue;
                }
                let insert_at = idxs[0];
                for row in &mut rows {
                    let joined = idxs
                        .iter()
                        .filter_map(|i| row.get(*i))
                        .map(String::as_str)
                        .collect::<Vec<_>>()
                        .join(separator.as_str());
                    row.insert(insert_at, joined);
                }
                columns.insert(
                    insert_at,
                    crate::table::ColumnDefinition {
                        id: format!("col_{}", columns.len() + 1),
                        name: new_name.clone(),
                        index: insert_at,
                    },
                );
                for (i, c) in columns.iter_mut().enumerate() {
                    c.index = i;
                }
                rows_changed += 1;
            }
            DataTransformRule::DeleteColumn { column_id } => {
                let idx = require_column!(columns, rule_no, column_id);
                columns.remove(idx);
                for row in &mut rows {
                    if idx < row.len() {
                        row.remove(idx);
                    }
                }
                for (i, c) in columns.iter_mut().enumerate() {
                    c.index = i;
                }
                rows_changed += 1;
            }
            DataTransformRule::RenameColumn {
                column_id,
                new_name,
            } => {
                require_column!(columns, rule_no, column_id);
                let col = columns
                    .iter_mut()
                    .find(|c| c.id == *column_id)
                    .expect("checked above");
                col.name = new_name.clone();
                rows_changed += 1;
            }
            DataTransformRule::DeduplicateRows { column_ids, keep } => {
                for id in column_ids {
                    require_column!(columns, rule_no, id);
                }
                let idxs: Vec<usize> = if column_ids.is_empty() {
                    (0..columns.len()).collect()
                } else {
                    column_ids
                        .iter()
                        .filter_map(|id| columns.iter().position(|c| c.id == *id))
                        .collect()
                };
                let key_of = |row: &Vec<String>| -> Vec<String> {
                    idxs.iter()
                        .map(|i| row.get(*i).cloned().unwrap_or_default())
                        .collect()
                };
                // §36：确定性 Keep First/Last（Last 取最后出现位置、顺序保持）
                let mut seen: std::collections::HashSet<Vec<String>> =
                    std::collections::HashSet::new();
                let mut keep_flags = vec![false; rows.len()];
                match keep.as_str() {
                    "last" => {
                        for (i, row) in rows.iter().enumerate().rev() {
                            if seen.insert(key_of(row)) {
                                keep_flags[i] = true;
                            }
                        }
                    }
                    _ => {
                        for (i, row) in rows.iter().enumerate() {
                            if seen.insert(key_of(row)) {
                                keep_flags[i] = true;
                            }
                        }
                    }
                }
                let before = rows.len();
                let mut kept_rows = Vec::new();
                for (i, row) in rows.drain(..).enumerate() {
                    if keep_flags[i] {
                        kept_rows.push(row);
                    }
                }
                rows = kept_rows;
                rows_changed += (before - rows.len()) as u64;
            }
            DataTransformRule::DateNormalizeIso { column_id } => {
                let idx = require_column!(columns, rule_no, column_id);
                {
                    for (ri, row) in rows.iter_mut().enumerate() {
                        let cell = row.get_mut(idx).expect("column exists");
                        if cell.trim().is_empty() {
                            continue;
                        }
                        let t = cell.trim();
                        let all_digits_dashes = t.chars().enumerate().all(|(i, c)| {
                            if i == 4 || i == 7 {
                                c == '-' || c == '/'
                            } else {
                                c.is_ascii_digit()
                            }
                        });
                        // §39：yyyy/mm/dd → ISO（无歧义）；其余 ⇒ Anomaly 不猜测
                        if t.len() == 10 && all_digits_dashes {
                            *cell = t.replace('/', "-");
                            rows_changed += 1;
                        } else {
                            diag!(DataDiagnostic::new(
                                "data.dateNotNormalizable",
                                DataSeverity::Anomaly,
                                format!("'{t}' is not an unambiguous ISO-compatible date; left unchanged"),
                            )
                            .at(ri as u64 + 1, idx as u64 + 1)
                            .with_basis(Basis::Fact));
                        }
                    }
                }
            }
            DataTransformRule::NumericNormalize {
                column_id,
                decimal_separator,
                thousands_separator,
            } => {
                let idx = require_column!(columns, rule_no, column_id);
                {
                    for (ri, row) in rows.iter_mut().enumerate() {
                        let cell = row.get_mut(idx).expect("column exists");
                        if cell.trim().is_empty() {
                            continue;
                        }
                        let mut normalized = cell.clone();
                        if let Some(thousands) = thousands_separator
                            && !thousands.is_empty()
                        {
                            normalized = normalized.replace(thousands.as_str(), "");
                        }
                        normalized = normalized.replace(decimal_separator.as_str(), ".");
                        if normalized != *cell {
                            if normalized.parse::<f64>().is_ok() {
                                *cell = normalized;
                                rows_changed += 1;
                            } else {
                                diag!(
                                    DataDiagnostic::new(
                                        "data.numericNotNormalizable",
                                        DataSeverity::Anomaly,
                                        format!(
                                            "'{cell}' is not a normalizable number; left unchanged"
                                        ),
                                    )
                                    .at(ri as u64 + 1, idx as u64 + 1)
                                    .with_basis(Basis::Fact)
                                );
                            }
                        }
                    }
                }
            }
        }
    }

    Ok((DataTable { columns, rows }, diagnostics, rows_changed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::build_columns;

    fn plan(rules: Vec<DataTransformRule>) -> DataTransformPlan {
        DataTransformPlan { rules }
    }

    fn sample() -> DataTable {
        DataTable {
            columns: build_columns(&["email".to_string(), "name".to_string()]),
            rows: vec![
                vec!["  A@X.com  ".to_string(), "  alice ".to_string()],
                vec!["b@y.com".to_string(), "bob".to_string()],
                vec!["A@X.com".to_string(), "dup".to_string()],
            ],
        }
    }

    #[test]
    fn rule_order_changes_semantics() {
        // §79：Dedupe 前置（未 trim ⇒ 三行互异）与后置（trim 后剩 2 行）
        // 产生不同结果——规则顺序是语义
        let t = sample();
        let dedupe_first = apply_plan(
            &t,
            &plan(vec![
                DataTransformRule::DeduplicateRows {
                    column_ids: vec!["col_1".into()],
                    keep: "first".into(),
                },
                DataTransformRule::Trim {
                    column_id: "col_1".into(),
                    side: "both".into(),
                },
            ]),
            &NullPolicy::default(),
            100,
        )
        .expect("ok");
        let trim_first = apply_plan(
            &t,
            &plan(vec![
                DataTransformRule::Trim {
                    column_id: "col_1".into(),
                    side: "both".into(),
                },
                DataTransformRule::DeduplicateRows {
                    column_ids: vec!["col_1".into()],
                    keep: "first".into(),
                },
            ]),
            &NullPolicy::default(),
            100,
        )
        .expect("ok");
        assert_ne!(dedupe_first.0.rows.len(), trim_first.0.rows.len());
    }

    #[test]
    fn trim_case_dedupe_pipeline() {
        let t = sample();
        let (out, _, _) = apply_plan(
            &t,
            &plan(vec![
                DataTransformRule::Trim {
                    column_id: "col_1".into(),
                    side: "both".into(),
                },
                DataTransformRule::CaseNormalize {
                    column_id: "col_1".into(),
                    form: "lower".into(),
                },
                DataTransformRule::DeduplicateRows {
                    column_ids: vec!["col_1".into()],
                    keep: "first".into(),
                },
            ]),
            &NullPolicy::default(),
            100,
        )
        .expect("ok");
        assert_eq!(out.rows.len(), 2);
        assert_eq!(out.rows[0][0], "a@x.com");
    }

    #[test]
    fn fill_empty_uses_null_policy() {
        // §30："N/A" 只有显式配置才算空
        let t = DataTable {
            columns: build_columns(&["c".to_string()]),
            rows: vec![
                vec![String::new()],
                vec!["N/A".to_string()],
                vec!["x".to_string()],
            ],
        };
        let policy = NullPolicy {
            empty_string: true,
            whitespace_only: false,
            tokens: vec!["N/A".to_string()],
        };
        let (out, _, _) = apply_plan(
            &t,
            &plan(vec![DataTransformRule::FillEmpty {
                column_id: "col_1".into(),
                value: "CN".into(),
                null_policy: Some(policy),
            }]),
            &NullPolicy::default(),
            100,
        )
        .expect("ok");
        assert_eq!(out.rows[0][0], "CN");
        assert_eq!(out.rows[1][0], "CN");
        assert_eq!(out.rows[2][0], "x");
    }

    #[test]
    fn split_merge_rename_delete_chain() {
        let t = DataTable {
            columns: build_columns(&["full".to_string(), "age".to_string()]),
            rows: vec![vec!["John|Doe".to_string(), "30".to_string()]],
        };
        let (out, _, _) = apply_plan(
            &t,
            &plan(vec![
                DataTransformRule::SplitColumn {
                    column_id: "col_1".into(),
                    delimiter: "|".into(),
                    max_splits: 1,
                    new_names: vec!["first".into(), "last".into()],
                },
                DataTransformRule::RenameColumn {
                    column_id: "col_4".into(),
                    new_name: "years".into(),
                },
                DataTransformRule::MergeColumns {
                    column_ids: vec!["col_3".into(), "col_4".into()],
                    separator: " ".into(),
                    new_name: "name_years".into(),
                },
                DataTransformRule::DeleteColumn {
                    column_id: "col_2".into(),
                },
            ]),
            &NullPolicy::default(),
            100,
        )
        .expect("ok");
        assert_eq!(out.rows[0][0], "John|Doe"); // split 保留原列（§28/§29）
        assert!(
            out.rows[0].contains(&"John Doe".to_string()),
            "{:?}",
            out.rows[0]
        );
        assert!(
            !out.rows[0].contains(&"30".to_string()),
            "age 列已被 delete 移除"
        );
    }

    #[test]
    fn date_normalize_iso_and_anomaly() {
        let t = DataTable {
            columns: build_columns(&["d".to_string()]),
            rows: vec![
                vec!["2026/10/03".to_string()],
                vec!["03/10/2026".to_string()],
            ],
        };
        let (out, diagnostics, _) = apply_plan(
            &t,
            &plan(vec![DataTransformRule::DateNormalizeIso {
                column_id: "col_1".into(),
            }]),
            &NullPolicy::default(),
            100,
        )
        .expect("ok");
        assert_eq!(out.rows[0][0], "2026-10-03");
        // §38/§42：歧义日期 ⇒ Anomaly（非 Error）且不猜测
        assert_eq!(out.rows[1][0], "03/10/2026");
        assert_eq!(diagnostics[0].severity, DataSeverity::Anomaly);
        assert_eq!(diagnostics[0].code, "data.dateNotNormalizable");
    }

    #[test]
    fn numeric_normalize_explicit_separators() {
        // §40：1.234,56（欧式）显式配置，绝不全局逗号替换
        let t = DataTable {
            columns: build_columns(&["v".to_string()]),
            rows: vec![vec!["1.234,56".to_string()]],
        };
        let (out, _, _) = apply_plan(
            &t,
            &plan(vec![DataTransformRule::NumericNormalize {
                column_id: "col_1".into(),
                decimal_separator: ",".into(),
                thousands_separator: Some(".".into()),
            }]),
            &NullPolicy::default(),
            100,
        )
        .expect("ok");
        assert_eq!(out.rows[0][0], "1234.56");
    }

    #[test]
    fn validation_catches_missing_column_and_bad_regex() {
        let t = sample();
        // 列存在性在 apply 时对"演化后的表"检查（§80）：col_9 不存在 ⇒ Err
        let missing = apply_plan(
            &t,
            &plan(vec![DataTransformRule::Trim {
                column_id: "col_9".into(),
                side: "both".into(),
            }]),
            &NullPolicy::default(),
            100,
        )
        .expect_err("missing column");
        assert_eq!(missing[0].code, "data.ruleColumnMissing");

        // 参数合法性（regex 编译失败）在前置校验即报
        let errs = apply_plan(
            &t,
            &plan(vec![DataTransformRule::FindReplace {
                column_id: "col_1".into(),
                find: "([".into(),
                replace_with: String::new(),
                regex: true,
                case_sensitive: true,
            }]),
            &NullPolicy::default(),
            100,
        )
        .expect_err("bad regex");
        assert!(errs.iter().any(|e| e.code == "text.invalidRegex"));
    }
}
