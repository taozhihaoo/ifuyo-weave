//! DataTable（M5 上 §12–§15）：稳定列身份 + 稳定行身份 + 事实诊断。
//!
//! - ColumnId = `col_{n}`（1-based 解析序），rename/delete/reorder 后不变
//!   （§12/§27：内部引用绝不依赖显示名或 index）。
//! - 行身份 = `row_{n}`（1-based 数据行序，不含 header）；排序/过滤是视图
//!   操作，行 id 跟随数据移动（§15）。
//! - Ragged rows（§14）：短行补空 + 诊断；长行保留多余单元格 + 诊断
//!   （绝不静默截断/移位）。

use crate::Cell;

/// 列定义（§4 ColumnDefinition）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnDefinition {
    /// 稳定内部身份，如 "col_3"。
    pub id: String,
    /// 显示名（header 文本；空 header 时 fallback "Column N"，§13）。
    pub name: String,
    /// 解析序 index（0-based）。
    pub index: usize,
}

/// 数据表。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DataTable {
    pub columns: Vec<ColumnDefinition>,
    /// 每行 = 与 columns 同序的单元格；行向量可能长于 columns（ragged 保留）。
    pub rows: Vec<Vec<Cell>>,
}

impl DataTable {
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn column_id(&self, index: usize) -> Option<&str> {
        self.columns.get(index).map(|c| c.id.as_str())
    }

    /// 按 ColumnId 查 index（§27：内部引用走 id）。
    pub fn index_of(&self, column_id: &str) -> Option<usize> {
        self.columns.iter().position(|c| c.id == column_id)
    }

    /// 行身份：1-based 数据行号（row_{n} 的 n）。
    pub fn row_id(&self, row_index: usize) -> String {
        format!("row_{}", row_index + 1)
    }
}

/// 构建 header 列定义（§12/§13）：
/// 重复名 → `name_2`/`name_3`（可见、确定性）；空名 → `Column N` fallback。
pub fn build_columns(headers: &[String]) -> Vec<ColumnDefinition> {
    let mut used: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    headers
        .iter()
        .enumerate()
        .map(|(index, raw)| {
            let base = if raw.trim().is_empty() {
                format!("Column {}", index + 1)
            } else {
                raw.clone()
            };
            let count = used.entry(base.clone()).or_insert(0);
            *count += 1;
            let name = if !raw.trim().is_empty() && *count > 1 {
                format!("{base}_{count}")
            } else {
                base.clone()
            };
            ColumnDefinition {
                id: format!("col_{}", index + 1),
                name,
                index,
            }
        })
        .collect()
}
