//! DataSession（M5 上 §17–§24）：ephemeral / local-only / bounded /
//! non-persistent 的会话态——不是数据库（§18：无 SQL、无索引引擎）。
//!
//! 视图（filter/sort/search）是**非破坏性**的（§20）：底表不变；
//! 行身份 row_N 跟随数据移动（§15）。

use crate::table::DataTable;

/// 过滤算子（§19）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOperator {
    Contains,
    Equals,
    NotEquals,
    StartsWith,
    EndsWith,
    Empty,
    NotEmpty,
}

impl FilterOperator {
    pub fn as_str(&self) -> &'static str {
        match self {
            FilterOperator::Contains => "contains",
            FilterOperator::Equals => "equals",
            FilterOperator::NotEquals => "notEquals",
            FilterOperator::StartsWith => "startsWith",
            FilterOperator::EndsWith => "endsWith",
            FilterOperator::Empty => "empty",
            FilterOperator::NotEmpty => "notEmpty",
        }
    }
}

/// 单条过滤规则（§21 FilterRule：ColumnId + Operator + Value；
/// 多条规则间为 AND——OR 组留待 UI 需要时扩展，语义显式不默认）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FilterRule {
    pub column_id: String,
    pub operator: FilterOperator,
    pub value: String,
    /// 大小写敏感（§19：默认敏感、显式）。
    pub case_sensitive: bool,
}

impl FilterRule {
    fn matches(&self, cell: &str) -> bool {
        let (hay, needle) = if self.case_sensitive {
            (cell.to_string(), self.value.clone())
        } else {
            (cell.to_lowercase(), self.value.to_lowercase())
        };
        match self.operator {
            FilterOperator::Contains => hay.contains(&needle),
            FilterOperator::Equals => hay == needle,
            FilterOperator::NotEquals => hay != needle,
            FilterOperator::StartsWith => hay.starts_with(&needle),
            FilterOperator::EndsWith => hay.ends_with(&needle),
            FilterOperator::Empty => cell.trim().is_empty(),
            FilterOperator::NotEmpty => !cell.trim().is_empty(),
        }
    }
}

/// 排序（§22/§23：asc/desc、字符串序；null/empty 恒排最后——一致的
/// 显式决策，D47）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SortSpec {
    pub column_id: String,
    pub descending: bool,
    /// 忽略大小写（Unicode 小写折叠，同 M4 §52）。
    pub ignore_case: bool,
}

/// 过滤+排序后的视图：**数据行索引列表**（指向底表；非破坏，§20）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataView {
    pub indices: Vec<usize>,
}

impl DataView {
    pub fn full(table: &DataTable) -> Self {
        Self {
            indices: (0..table.row_count()).collect(),
        }
    }

    pub fn apply_filters(&mut self, table: &DataTable, rules: &[FilterRule]) {
        if rules.is_empty() {
            return;
        }
        let column_indexes: Vec<Option<usize>> =
            rules.iter().map(|r| table.index_of(&r.column_id)).collect();
        self.indices.retain(|&row_idx| {
            let row = &table.rows[row_idx];
            rules.iter().zip(&column_indexes).all(|(rule, col_idx)| {
                let cell = col_idx
                    .and_then(|i| row.get(i))
                    .map(String::as_str)
                    .unwrap_or("");
                rule.matches(cell)
            })
        });
    }

    /// 稳定排序（§22：相等的行保持原相对顺序——sort_by 是稳定排序；
    /// null/empty 恒最后，§23/D47）。
    pub fn apply_sort(&mut self, table: &DataTable, sort: &SortSpec) {
        let Some(col_idx) = table.index_of(&sort.column_id) else {
            return;
        };
        let key_of = |row_idx: usize| -> (bool, String) {
            let cell = table
                .rows
                .get(row_idx)
                .and_then(|r| r.get(col_idx))
                .map(String::as_str)
                .unwrap_or("");
            let is_empty = cell.trim().is_empty();
            let key = if sort.ignore_case {
                cell.trim().to_lowercase()
            } else {
                cell.trim().to_string()
            };
            (is_empty, key)
        };
        let mut keyed: Vec<(bool, String, usize)> = self
            .indices
            .iter()
            .map(|&idx| {
                let (empty, key) = key_of(idx);
                (empty, key, idx)
            })
            .collect();
        // null/empty 恒最后（两个方向一致）；同键稳定（装饰-排序-还原保 idx）
        keyed.sort_by(|a, b| {
            let empty_cmp = a.0.cmp(&b.0);
            if empty_cmp.is_ne() {
                return empty_cmp;
            }
            if sort.descending {
                b.1.cmp(&a.1)
            } else {
                a.1.cmp(&b.1)
            }
        });
        self.indices = keyed.into_iter().map(|(_, _, idx)| idx).collect();
    }

    /// 分页（§85：page 上限由 DataLimits 约束）。
    pub fn page(&self, offset: usize, page_size: usize) -> &[usize] {
        let start = offset.min(self.indices.len());
        let end = (start + page_size).min(self.indices.len());
        &self.indices[start..end]
    }
}

/// 数据会话（§17）：底表 + 视图。ephemeral：存于 AppState 内存，
/// 应用退出即消失；不持久化、无索引引擎（§18）。
pub struct DataSession {
    pub table: DataTable,
    pub view: DataView,
    /// 当前排序（视图重算用）。
    pub sort: Option<SortSpec>,
    /// 当前过滤规则（视图重算用）。
    pub filters: Vec<FilterRule>,
}

impl DataSession {
    pub fn new(table: DataTable) -> Self {
        let view = DataView::full(&table);
        Self {
            table,
            view,
            sort: None,
            filters: Vec::new(),
        }
    }

    /// 重算视图：filters（AND）→ sort（§79：顺序语义由调用方保证——
    /// filters/sort 是视图层，不改变底表）。
    pub fn refresh_view(&mut self) {
        let mut view = DataView::full(&self.table);
        view.apply_filters(&self.table, &self.filters);
        if let Some(sort) = &self.sort {
            view.apply_sort(&self.table, sort);
        }
        self.view = view;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::table::build_columns;

    fn session() -> DataSession {
        let columns = build_columns(&["name".into(), "age".into()]);
        let rows = vec![
            vec!["alice".into(), "30".into()],
            vec!["Bob".into(), String::new()],
            vec!["carol".into(), "25".into()],
            vec!["  ".into(), "40".into()],
        ];
        DataSession::new(DataTable { columns, rows })
    }

    #[test]
    fn filter_is_non_destructive_view() {
        // §20：过滤后底表不变
        let mut s = session();
        s.filters = vec![FilterRule {
            column_id: "col_1".into(),
            operator: FilterOperator::Contains,
            value: "a".into(),
            case_sensitive: false,
        }];
        s.refresh_view();
        assert_eq!(
            s.view.indices,
            vec![0, 2],
            "alice/carol 含 a（不区分大小写）"
        );
        assert_eq!(s.table.row_count(), 4, "底表不变");
        s.filters.clear();
        s.refresh_view();
        assert_eq!(s.view.indices.len(), 4);
    }

    #[test]
    fn multiple_filters_are_and() {
        // §21：多规则 AND（显式语义）
        let mut s = session();
        s.filters = vec![
            FilterRule {
                column_id: "col_1".into(),
                operator: FilterOperator::Contains,
                value: "a".into(),
                case_sensitive: false,
            },
            FilterRule {
                column_id: "col_2".into(),
                operator: FilterOperator::NotEmpty,
                value: String::new(),
                case_sensitive: true,
            },
        ];
        s.refresh_view();
        assert_eq!(s.view.indices, vec![0, 2]);
    }

    #[test]
    fn sort_is_stable_and_null_last_both_directions() {
        // §22/§23/D47：null/empty 恒最后；相等行保原序
        let mut s = session();
        s.sort = Some(SortSpec {
            column_id: "col_1".into(),
            descending: false,
            ignore_case: false,
        });
        s.refresh_view();
        assert_eq!(
            s.view.indices,
            vec![1, 0, 2, 3],
            "大小写敏感 asc：Bob(B) < alice(a) < carol(c)；空行恒最后"
        );
        s.sort = Some(SortSpec {
            column_id: "col_1".into(),
            descending: true,
            ignore_case: false,
        });
        s.refresh_view();
        assert_eq!(
            s.view.indices,
            vec![2, 0, 1, 3],
            "降序：carol > alice > Bob；空行仍最后"
        );
    }

    #[test]
    fn paging_slices_view() {
        let mut s = session();
        s.refresh_view();
        assert_eq!(s.view.page(0, 2), &[0, 1]);
        assert_eq!(s.view.page(2, 2), &[2, 3]);
        assert_eq!(s.view.page(99, 2), &[] as &[usize]);
    }
}
