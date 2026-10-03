//! TextDocument 统一模型（M4 §13/§14/§17）。
//!
//! 一切文本工具（Formatter/Compare/Extractor/Transformer）共享同一输入模型，
//! 业务工具不得自行维护 `string + encoding + newline + path` 的平行结构。
//!
//! Line Ending 策略（§17，D39）：
//! - 检测 LF / CRLF / CR / Mixed（事实，不猜测主导）。
//! - 默认 Preserve：行级操作按行携带各自的原始结尾（`split_inclusive`），
//!   不无意义重排换行风格；显式选择时才做 LF/CRLF 转换。
//! - Final newline 的有无是事实，操作不得静默增删。

use weave_core::prelude::TextEncoding;

use crate::encoding::Bom;

/// 文本来源（§14）。M4 支持 Dropped/Opened File、Pasted、Typed。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceKind {
    File,
    Pasted,
    Typed,
}

/// 换行风格事实（§17）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineEnding {
    Lf,
    Crlf,
    Cr,
    /// 混合换行：如实标注；Preserve 策略下按行携带各自结尾。
    Mixed,
}

impl LineEnding {
    pub fn as_str(self) -> &'static str {
        match self {
            LineEnding::Lf => "lf",
            LineEnding::Crlf => "crlf",
            LineEnding::Cr => "cr",
            LineEnding::Mixed => "mixed",
        }
    }
}

/// 换行统计（detect 的事实基础）。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct LineEndingCounts {
    pub lf: u64,
    pub crlf: u64,
    pub cr: u64,
}

impl LineEndingCounts {
    /// 主导换行（平局时优先 CRLF > LF > CR，Windows 主平台基线）。
    pub fn dominant(&self) -> LineEnding {
        if self.crlf >= self.lf && self.crlf >= self.cr && self.crlf > 0 {
            LineEnding::Crlf
        } else if self.lf >= self.cr && self.lf > 0 {
            LineEnding::Lf
        } else if self.cr > 0 {
            LineEnding::Cr
        } else {
            LineEnding::Lf // 空文档/无换行：选 LF 作为写出基线（D39）
        }
    }

    pub fn is_mixed(&self) -> bool {
        [self.lf, self.crlf, self.cr]
            .iter()
            .filter(|c| **c > 0)
            .count()
            > 1
    }
}

/// 统计内容中的换行事实。
pub fn count_line_endings(content: &str) -> LineEndingCounts {
    let mut counts = LineEndingCounts::default();
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\r' if i + 1 < bytes.len() && bytes[i + 1] == b'\n' => {
                counts.crlf += 1;
                i += 2;
            }
            b'\r' => {
                counts.cr += 1;
                i += 1;
            }
            b'\n' => {
                counts.lf += 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    counts
}

/// 检测换行风格（事实表达：Mixed 不猜测主导——两者都给）。
pub fn detect_line_ending(content: &str) -> (LineEndingCounts, LineEnding) {
    let counts = count_line_endings(content);
    let style = if counts.is_mixed() {
        LineEnding::Mixed
    } else {
        counts.dominant()
    };
    (counts, style)
}

/// 文本格式（§21）。Auto 仅存在于 UI 选项；域内一律解析为具体格式或 Unknown。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TextFormat {
    Unknown,
    Json,
    Xml,
    Yaml,
    Sql,
    JavaScript,
    Css,
    Markdown,
}

impl TextFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            TextFormat::Unknown => "unknown",
            TextFormat::Json => "json",
            TextFormat::Xml => "xml",
            TextFormat::Yaml => "yaml",
            TextFormat::Sql => "sql",
            TextFormat::JavaScript => "javascript",
            TextFormat::Css => "css",
            TextFormat::Markdown => "markdown",
        }
    }
}

/// 统一文本输入模型（§13）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDocument {
    pub source: SourceKind,
    pub content: String,
    pub encoding: TextEncoding,
    pub bom: Bom,
    /// 换行风格事实（Mixed 表示逐行携带原样结尾）。
    pub line_ending: LineEnding,
    /// 格式事实（检测/用户指定；Unknown 时 UI 必须让用户手动选，§21）。
    pub format: TextFormat,
    /// 来源路径（Pasted/Typed 为 None）。
    pub path: Option<String>,
    /// 原始字节大小（与 content.chars 计数不同——展示与写回校验用）。
    pub byte_size: u64,
    /// 末尾是否有换行（Preserve 策略基线，§17）。
    pub ends_with_newline: bool,
}

impl TextDocument {
    pub fn from_content(source: SourceKind, content: impl Into<String>) -> Self {
        let content = content.into();
        let (_, line_ending) = detect_line_ending(&content);
        let ends_with_newline = content.ends_with('\n') || content.ends_with('\r');
        Self {
            source,
            byte_size: content.len() as u64,
            content,
            encoding: TextEncoding::Utf8,
            bom: Bom::None,
            line_ending,
            format: TextFormat::Unknown,
            path: None,
            ends_with_newline,
        }
    }

    /// 生成文档的写回策略基线视图（Preserve：编码/BOM/末尾换行原样）。
    pub fn preserve_policy(&self) -> WritePolicy {
        WritePolicy {
            encoding: self.encoding,
            bom: self.bom,
            line_ending: self.line_ending,
            ends_with_newline: self.ends_with_newline,
        }
    }
}

/// 写回策略（§16/§17/§91）：Apply/Save 前必须明确，不允许隐式默认漂移。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WritePolicy {
    pub encoding: TextEncoding,
    pub bom: Bom,
    pub line_ending: LineEnding,
    pub ends_with_newline: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_ending_detection_facts() {
        let (c, style) = detect_line_ending("a\r\nb\nc\rd");
        assert_eq!(c.crlf, 1);
        assert_eq!(c.lf, 1);
        assert_eq!(c.cr, 1);
        assert_eq!(style, LineEnding::Mixed);

        let (_, style) = detect_line_ending("a\r\nb\r\n");
        assert_eq!(style, LineEnding::Crlf);

        let (_, style) = detect_line_ending("no newlines");
        assert_eq!(style, LineEnding::Lf, "无换行时写出基线 LF（D39）");
    }

    #[test]
    fn document_records_final_newline_fact() {
        let doc = TextDocument::from_content(SourceKind::Typed, "a\nb\n");
        assert!(doc.ends_with_newline);
        let doc = TextDocument::from_content(SourceKind::Typed, "a\nb");
        assert!(!doc.ends_with_newline);
        assert_eq!(doc.byte_size, 3);
    }
}
