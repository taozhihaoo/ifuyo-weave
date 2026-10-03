//! Formatter 引擎（M4 §22–§23/§46）：统一能力矩阵 + 诚实 Supported/Unsupported。
//!
//! 契约（§23）：same input + same options = same output。
//! 能力（§46）：任何做不到语义保持的能力必须返回 Unsupported 并给出原因——
//! 这条优先级高于"所有按钮都得亮"。各格式边界见子模块与 DECISIONS D43。

use crate::diagnostics::{Severity, TextDiagnostic};
use crate::model::TextFormat;

pub mod css;
pub mod javascript;
pub mod json;
pub mod markdown;
pub mod sql;
pub mod xml;
pub mod yaml;

/// Formatter 操作（§22）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FormatOperation {
    Validate,
    Format,
    Minify,
    Sort,
    Normalize,
}

impl FormatOperation {
    pub fn as_str(self) -> &'static str {
        match self {
            FormatOperation::Validate => "validate",
            FormatOperation::Format => "format",
            FormatOperation::Minify => "minify",
            FormatOperation::Sort => "sort",
            FormatOperation::Normalize => "normalize",
        }
    }
}

/// 统一缩进/结尾选项（§25；默认 2 空格 + final newline，D43）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormatOptions {
    pub indent_spaces: u32,
    pub final_newline: bool,
}

impl Default for FormatOptions {
    fn default() -> Self {
        Self {
            indent_spaces: 2,
            final_newline: true,
        }
    }
}

/// 单个能力的结果（§22）：内容产出（Validate 成功时 None）+ 诊断。
#[derive(Debug, Clone, PartialEq)]
pub struct FormatOutcome {
    pub content: Option<String>,
    pub diagnostics: Vec<TextDiagnostic>,
    /// 对输入有无变化（原文已符合目标形态时 false）。
    pub changed: bool,
}

impl FormatOutcome {
    pub fn unsupported(reason: &'static str) -> Self {
        Self {
            content: None,
            diagnostics: vec![TextDiagnostic::without_range(
                "text.unsupportedOperation",
                Severity::Info,
                reason,
            )],
            changed: false,
        }
    }

    pub fn success(content: Option<String>, changed: bool) -> Self {
        Self {
            content,
            diagnostics: Vec::new(),
            changed,
        }
    }

    pub fn failed(diagnostics: Vec<TextDiagnostic>) -> Self {
        Self {
            content: None,
            diagnostics,
            changed: false,
        }
    }

    /// 是否含 Error 级诊断。
    pub fn is_error(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.severity == Severity::Error)
    }
}

/// 执行一个 formatter 能力（§22 统一入口）。
pub fn run_formatter(
    format: TextFormat,
    operation: FormatOperation,
    content: &str,
    options: &FormatOptions,
) -> FormatOutcome {
    match format {
        TextFormat::Json => json::run(operation, content, options),
        TextFormat::Xml => xml::run(operation, content, options),
        TextFormat::Yaml => yaml::run(operation, content, options),
        TextFormat::Sql => sql::run(operation, content, options),
        TextFormat::JavaScript => javascript::run(operation, content, options),
        TextFormat::Css => css::run(operation, content, options),
        TextFormat::Markdown => markdown::run(operation, content, options),
        TextFormat::Unknown => FormatOutcome::unsupported(
            "format is Unknown; user must select a format explicitly (M4 §21)",
        ),
    }
}
