//! Offset / Range 契约（M4 §18/§19）——高风险边界。
//!
//! 三层契约（§18）：
//!
//! ```text
//! Domain 内部      : byte offset（Rust String 索引一致）
//! IPC 序列化       : byte offset + line + column（column = UTF-16 code unit）
//! UI 显示          : line/column 直接展示；JS 端禁止拿 byte offset 当 str 索引
//! ```
//!
//! column 采用 UTF-16 code units（JS string index 天然一致，§18 高风险边界
//! 的消解方案，D38）；line/column 均 1-based；换行判定以 LF 为行界（CR 仅在
//! CR-only 文档中作为行界）。

/// 文本区间（byte offset，start <= end；[start, end) 半开区间）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextRange {
    pub start: u64,
    pub end: u64,
}

impl TextRange {
    pub fn new(start: u64, end: u64) -> Self {
        Self { start, end }
    }

    pub fn len(&self) -> u64 {
        self.end - self.start
    }

    pub fn is_empty(&self) -> bool {
        self.start >= self.end
    }
}

/// 行索引：构建一次，O(log n) 查询 offset → (line, column)。
/// 行界 = LF 或独立 CR（CR-only 文档兼容）。
#[derive(Debug, Clone)]
pub struct LineIndex {
    /// 每行起始 byte offset（第一行恒为 0）。
    line_starts: Vec<u64>,
    total_bytes: u64,
}

impl LineIndex {
    pub fn new(content: &str) -> Self {
        let bytes = content.as_bytes();
        let mut line_starts = vec![0u64];
        let mut i = 0;
        while i < bytes.len() {
            match bytes[i] {
                b'\n' => {
                    i += 1;
                    line_starts.push(i as u64);
                }
                b'\r' if bytes.get(i + 1) != Some(&b'\n') => {
                    i += 1;
                    line_starts.push(i as u64);
                }
                b'\r' => {
                    i += 2;
                    line_starts.push(i as u64);
                }
                _ => i += 1,
            }
        }
        Self {
            line_starts,
            total_bytes: bytes.len() as u64,
        }
    }

    pub fn line_count(&self) -> u64 {
        self.line_starts.len() as u64
    }

    /// byte offset → (line, utf16_column)，均 1-based；offset 超界时收敛到末尾。
    pub fn line_col(&self, content: &str, offset: u64) -> (u64, u64) {
        let offset = offset.min(self.total_bytes);
        let line = match self.line_starts.binary_search(&offset) {
            Ok(idx) => idx as u64 + 1,
            Err(idx) => idx as u64, // offset 在 line_starts[idx-1] 与 [idx] 之间
        };
        let line_start = self.line_starts[(line.max(1) - 1) as usize] as usize;
        let offset = offset as usize;
        let col_utf16: usize = content[line_start..offset].encode_utf16().count();
        (line.max(1), col_utf16 as u64 + 1)
    }

    /// 行号 → 该行 byte 范围（[start, end)，不含行界符）。
    pub fn line_range(&self, content: &str, line: u64) -> TextRange {
        let idx = (line.max(1) - 1) as usize;
        if idx >= self.line_starts.len() {
            return TextRange::new(self.total_bytes, self.total_bytes);
        }
        let start = self.line_starts[idx] as usize;
        let end = if idx + 1 < self.line_starts.len() {
            let mut e = self.line_starts[idx + 1] as usize;
            // 回退行界符本身
            let bytes = content.as_bytes();
            if e > start {
                if bytes[e - 1] == b'\n' {
                    e -= 1;
                    if e > start && bytes[e - 1] == b'\r' {
                        e -= 1;
                    }
                } else if bytes[e - 1] == b'\r' {
                    e -= 1;
                }
            }
            e
        } else {
            content.len()
        };
        TextRange::new(start as u64, end as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ascii_line_col_is_one_based() {
        let content = "ab\ncd\n";
        let index = LineIndex::new(content);
        assert_eq!(index.line_col(content, 0), (1, 1));
        assert_eq!(index.line_col(content, 2), (1, 3));
        assert_eq!(index.line_col(content, 3), (2, 1));
        assert_eq!(index.line_col(content, 5), (2, 3));
    }

    #[test]
    fn unicode_columns_use_utf16_units() {
        // é = 2 bytes / 1 UTF-16 unit；汉 = 3 bytes / 1 unit；😀 = 4 bytes / 2 units（surrogate）
        let content = "é汉😀\nx";
        let index = LineIndex::new(content);
        // offsets: é@0..2, 汉@2..5, 😀@5..9, \n@9
        assert_eq!(index.line_col(content, 2), (1, 2)); // 汉 前已 1 unit
        assert_eq!(index.line_col(content, 5), (1, 3)); // 😀 前已 2 units
        assert_eq!(index.line_col(content, 9), (1, 5)); // 😀 = 2 units
        assert_eq!(index.line_col(content, 10), (2, 1));
    }

    #[test]
    fn combining_characters_count_as_separate_units() {
        // e + U+0301 (combining acute) = 3 bytes, 2 UTF-16 units
        let content = "e\u{0301}\nx";
        let index = LineIndex::new(content);
        assert_eq!(index.line_col(content, 3), (1, 3));
    }

    #[test]
    fn cr_only_documents_use_cr_as_line_boundary() {
        let content = "a\rb\rc";
        let index = LineIndex::new(content);
        assert_eq!(index.line_count(), 3);
        assert_eq!(index.line_col(content, 2), (2, 1));
        let r = index.line_range(content, 2);
        assert_eq!(&content[r.start as usize..r.end as usize], "b");
    }

    #[test]
    fn line_range_excludes_line_terminator() {
        let content = "first\r\nsecond\nthird";
        let index = LineIndex::new(content);
        let r1 = index.line_range(content, 1);
        assert_eq!(&content[r1.start as usize..r1.end as usize], "first");
        let r2 = index.line_range(content, 2);
        assert_eq!(&content[r2.start as usize..r2.end as usize], "second");
        let r3 = index.line_range(content, 3);
        assert_eq!(&content[r3.start as usize..r3.end as usize], "third");
    }

    #[test]
    fn out_of_bounds_offset_clamps() {
        let content = "ab";
        let index = LineIndex::new(content);
        assert_eq!(index.line_col(content, 999), (1, 3));
    }
}
