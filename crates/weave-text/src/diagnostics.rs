//! Text Diagnostics（M4 §20）：结构化诊断，绝不只返回 "Invalid JSON"。
//!
//! 至少提供 code / message / severity / range / line / column；解析器没有
//! 提供的信息（如 expected token）绝不伪造。

use crate::offset::{LineIndex, TextRange};

/// 诊断严重级（对齐 charter 的 severity 语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    pub fn as_str(self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Info => "info",
        }
    }
}

/// 结构化文本诊断（§20）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextDiagnostic {
    /// 机器可读码，如 "text.jsonInvalid"。
    pub code: String,
    pub severity: Severity,
    /// 开发者可读消息（非 UI 文案；i18n 由 UI 层按 code 处理）。
    pub message: String,
    /// 可定位时给出字节区间；parser 未提供位置时为 None（不伪造）。
    pub range: Option<TextRange>,
    /// 1-based；range 缺失时为 None。
    pub line: Option<u64>,
    /// 1-based，UTF-16 code units（§18 契约）。
    pub column: Option<u64>,
}

impl TextDiagnostic {
    /// 从字节区间构建（自动换算 line/column）。
    pub fn with_range(
        code: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
        content: &str,
        range: TextRange,
    ) -> Self {
        let index = LineIndex::new(content);
        let (line, column) = index.line_col(content, range.start);
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            range: Some(range),
            line: Some(line),
            column: Some(column),
        }
    }

    /// 无位置诊断（parser 没给位置 ⇒ 如实为 None，§20）。
    pub fn without_range(
        code: impl Into<String>,
        severity: Severity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            range: None,
            line: None,
            column: None,
        }
    }
}
