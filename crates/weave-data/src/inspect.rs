//! 数据检查（M5 上 §43–§48）：告诉用户数据是什么，绝不替用户修改。
//!
//! 诚实原则（§44/§47/§48）：每项统计标注 Exact / Unavailable；exact unique
//! 有基数上限，达到 ⇒ Unavailable + 原因（不静默降级、不无限增长）。

use std::collections::HashSet;

use crate::limits::DataLimits;
use crate::table::DataTable;

/// Potential type（§6：观测而非保证）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PotentialType {
    Empty,
    String,
    Integer,
    Decimal,
    Boolean,
    /// 保守 ISO 日期（yyyy-mm-dd；§38：ambiguous 格式不猜测）。
    DateIso,
}

impl PotentialType {
    pub fn as_str(self) -> &'static str {
        match self {
            PotentialType::Empty => "empty",
            PotentialType::String => "string",
            PotentialType::Integer => "integer",
            PotentialType::Decimal => "decimal",
            PotentialType::Boolean => "boolean",
            PotentialType::DateIso => "dateIso",
        }
    }

    /// 单元格的 potential 类型观测。
    pub fn observe(cell: &str) -> Self {
        let t = cell.trim();
        if t.is_empty() {
            return PotentialType::Empty;
        }
        if t == "true" || t == "false" {
            return PotentialType::Boolean;
        }
        // Integer：无前导零（"0" 允许；§57 "00123" 不当整数——Identifier-like）
        if !t.starts_with(['+', '-'])
            && (t == "0"
                || (t.starts_with(|c: char| c.is_ascii_digit() && c != '0')
                    && t.chars().all(|c| c.is_ascii_digit())))
        {
            return PotentialType::Integer;
        }
        // 无 i128 兜底：前导零/超 i64 数值如实保持 String（§7 精度优先）
        // Decimal：标准十进制（无前导零前缀数值，如 "01.5" 不算）
        if !t.starts_with(['+', '-']) && t.contains('.') {
            let (int_part, frac) = t.split_once('.').expect("has dot");
            let int_ok = int_part.is_empty()
                || int_part == "0"
                || (int_part.starts_with(|c: char| c.is_ascii_digit() && c != '0')
                    && int_part.chars().all(|c| c.is_ascii_digit()));
            let frac_ok = frac.chars().all(|c| c.is_ascii_digit()) && !frac.is_empty();
            if int_ok && frac_ok {
                return PotentialType::Decimal;
            }
        }
        if t.parse::<f64>().is_ok() && !t.starts_with('0') && t.contains(['e', 'E']) {
            return PotentialType::Decimal; // 科学计数法
        }
        // 保守 ISO 日期 yyyy-mm-dd（§38：dd/mm 等歧义格式不猜测）
        if t.len() == 10
            && t.as_bytes()[4] == b'-'
            && t.as_bytes()[7] == b'-'
            && t.chars().enumerate().all(|(i, c)| {
                if i == 4 || i == 7 {
                    c == '-'
                } else {
                    c.is_ascii_digit()
                }
            })
        {
            return PotentialType::DateIso;
        }
        PotentialType::String
    }
}

/// 唯一值统计（§47：exact 有上限；Unavailable 如实）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UniqueStat {
    Exact {
        /// distinct 值数。
        distinct: u64,
        /// 参与统计的非空行数。
        eligible: u64,
    },
    /// 达到 exact 基数上限 ⇒ Unavailable + 原因（§47）。
    Unavailable { reason: String },
}

/// 列 profile（§95）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnProfile {
    pub column_id: String,
    pub name: String,
    /// 观测类型集合（混合 ⇒ 多项，如 ["integer","string"]）。
    pub potential_types: Vec<&'static str>,
    pub null_count: u64,
    pub total_rows: u64,
    pub unique: UniqueStat,
    /// Identifier-like 迹象：整数单元格带前导零（§6 "00123" 如实标注）。
    pub leading_zero_numeric: bool,
    /// 统计口径（§44/§48）。
    pub basis: &'static str,
}

impl ColumnProfile {
    /// Null rate（§45：null 定义 = 空串（trim 后）；"0" 绝不视为 null）。
    pub fn null_rate(&self) -> Option<f64> {
        if self.total_rows == 0 {
            None
        } else {
            Some(self.null_count as f64 / self.total_rows as f64 * 100.0)
        }
    }

    pub fn unique_rate(&self) -> Option<f64> {
        match &self.unique {
            UniqueStat::Exact { distinct, eligible } if *eligible > 0 => {
                Some(*distinct as f64 / *eligible as f64 * 100.0)
            }
            _ => None,
        }
    }
}

/// 对表做列级 profile（§43）。null 定义 = trim 后为空（§45 默认；
/// "0"/"NULL" 等标记值不默认计入——§31 要求用户显式配置才计入）。
pub fn profile_table(table: &DataTable, limits: &DataLimits) -> Vec<ColumnProfile> {
    table
        .columns
        .iter()
        .map(|column| {
            let index = column.index;
            let mut types: Vec<&'static str> = Vec::new();
            let mut null_count = 0u64;
            let mut seen: HashSet<String> = HashSet::new();
            let mut eligible = 0u64;
            let mut overflow = false;
            let mut leading_zero = false;

            for row in &table.rows {
                let cell = row.get(index).map(String::as_str).unwrap_or("");
                let t = PotentialType::observe(cell);
                if t == PotentialType::Empty {
                    null_count += 1;
                } else {
                    eligible += 1;
                    // 前导零整数迹象（§6）
                    let trimmed = cell.trim();
                    if trimmed.len() > 1
                        && trimmed.starts_with('0')
                        && trimmed.chars().all(|c| c.is_ascii_digit())
                    {
                        leading_zero = true;
                    }
                    if !overflow
                        && seen.insert(trimmed.to_string())
                        && seen.len() > limits.max_exact_unique
                    {
                        overflow = true;
                        seen.clear();
                    }
                }
                let label = t.as_str();
                if !types.contains(&label) {
                    types.push(label);
                }
            }

            let unique = if overflow {
                UniqueStat::Unavailable {
                    reason: format!(
                        "exact cardinality limit reached (>{})",
                        limits.max_exact_unique
                    ),
                }
            } else {
                UniqueStat::Exact {
                    distinct: seen.len() as u64,
                    eligible,
                }
            };

            ColumnProfile {
                column_id: column.id.clone(),
                name: column.name.clone(),
                potential_types: types,
                null_count,
                total_rows: table.row_count() as u64,
                unique,
                leading_zero_numeric: leading_zero,
                basis: "exact",
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::build_columns;

    fn table_of<const N: usize>(headers: [&str; N], rows: Vec<[&str; N]>) -> DataTable {
        DataTable {
            columns: build_columns(&headers.map(str::to_string)),
            rows: rows
                .into_iter()
                .map(|r| r.map(str::to_string).to_vec())
                .collect(),
        }
    }

    #[test]
    fn potential_types_are_observed_not_forced() {
        // §5/§6/§57："00123" 是 string（Identifier-like），"1.5" 是 decimal
        assert_eq!(PotentialType::observe("00123"), PotentialType::String);
        assert_eq!(PotentialType::observe("123"), PotentialType::Integer);
        assert_eq!(PotentialType::observe("0"), PotentialType::Integer);
        assert_eq!(PotentialType::observe("1.5"), PotentialType::Decimal);
        assert_eq!(PotentialType::observe("true"), PotentialType::Boolean);
        assert_eq!(PotentialType::observe("2026-10-03"), PotentialType::DateIso);
        assert_eq!(
            PotentialType::observe("03/10/2026"),
            PotentialType::String,
            "歧义日期不猜测（§38）"
        );
        assert_eq!(PotentialType::observe(""), PotentialType::Empty);
    }

    #[test]
    fn null_rate_excludes_zero_and_counts_empty_only() {
        // §45："0" 不是 null
        let t = table_of(["a", "b"], vec![["1", ""], ["0", "  "], ["", "x"]]);
        let profiles = profile_table(&t, &DataLimits::default());
        let a = &profiles[0];
        assert_eq!(a.null_count, 1, "\"0\" 与空串不同");
        let rate = a.null_rate().expect("rate");
        assert!((rate - 100.0 / 3.0).abs() < 1e-9);
    }

    #[test]
    fn unique_exact_and_mixed_types() {
        let t = table_of(["a", "b"], vec![["1", "x"], ["1", "y"], ["abc", ""]]);
        let profiles = profile_table(&t, &DataLimits::default());
        let a = &profiles[0];
        assert_eq!(
            a.unique,
            UniqueStat::Exact {
                distinct: 2,
                eligible: 3
            }
        );
        assert_eq!(a.unique_rate(), Some(2.0 / 3.0 * 100.0));
        assert!(a.potential_types.contains(&"integer"));
        assert!(a.potential_types.contains(&"string"), "混合类型如实报告");
    }

    #[test]
    fn unique_overflow_is_unavailable_not_approximate() {
        // §47：超限 ⇒ Unavailable + 原因（不静默近似、不无限增长）
        let limits = DataLimits {
            max_exact_unique: 3,
            ..DataLimits::default()
        };
        let t = table_of(["a", "b"], vec![["1", ""], ["2", ""], ["3", ""], ["4", ""]]);
        let profiles = profile_table(&t, &limits);
        assert!(matches!(
            &profiles[0].unique,
            UniqueStat::Unavailable { .. }
        ));
        assert_eq!(profiles[0].unique_rate(), None);
    }

    #[test]
    fn leading_zero_flag() {
        let t = table_of(["id"], vec![["00123"], ["00124"]]);
        let profiles = profile_table(&t, &DataLimits::default());
        assert!(profiles[0].leading_zero_numeric, "§6 Identifier-like 标注");
        assert_eq!(profiles[0].potential_types, vec!["string"]);
    }
}
