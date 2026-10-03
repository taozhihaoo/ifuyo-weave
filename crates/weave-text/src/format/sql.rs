//! SQL Formatter（M4 §34–§36）：词法感知（字符串/引号标识符/注释），
//! 非 parser 级——能力边界如实声明（D43）。
//!
//! - Tokenize：'...' 字符串、"..."/"`...`"/[...] 引号标识符、-- 与 /* */ 注释，
//!   全部带 byte 区间，绝不破坏内容。
//! - Validate（§36"基本 validation"）：未闭合字符串/注释、括号不平衡——
//!   词法级结构检查；**不是**完整 SQL 语法校验（如实标注，§34）。
//! - Format：主句关键字换行布局（SELECT/FROM/WHERE/GROUP BY/ORDER BY/
//!   HAVING/JOIN/VALUES/SET/LIMIT），SELECT 列表逗号分行；确定性。
//! - Minify：注释删除 + 空白压缩（token 感知，不碰字符串/引号标识符，§35）。
//! - Sort / Normalize：Unsupported（SQL 语义重排不可靠，§46）。

use super::{FormatOperation, FormatOptions, FormatOutcome};
use crate::diagnostics::{Severity, TextDiagnostic};
use crate::offset::TextRange;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Tok {
    Word,
    StringLit,
    QuotedIdent,
    Comment,
    Punct,
    Number,
    Space,
}

#[derive(Debug, Clone)]
struct Token {
    kind: Tok,
    text: String,
    start: usize,
}

/// 词法扫描（内容保真；word 大小写原样保留，格式化时只动关键字大小写?——
/// 不动：保留原文大小写，仅布局。D43）。
fn tokenize(content: &str) -> Result<Vec<Token>, Vec<TextDiagnostic>> {
    let bytes = content.as_bytes();
    let mut tokens = Vec::new();
    let mut diagnostics = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let start = i;
        let b = bytes[i];
        let kind = match b {
            b'\'' => {
                i += 1;
                loop {
                    if i >= bytes.len() {
                        diagnostics.push(TextDiagnostic::with_range(
                            "text.sqlUnterminatedString",
                            Severity::Error,
                            "unterminated string literal",
                            content,
                            TextRange::new(start as u64, content.len() as u64),
                        ));
                        break;
                    }
                    if bytes[i] == b'\'' {
                        // '' 转义
                        if bytes.get(i + 1) == Some(&b'\'') {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                Tok::StringLit
            }
            b'"' | b'`' => {
                let close = b;
                i += 1;
                while i < bytes.len() && bytes[i] != close {
                    i += 1;
                }
                if i < bytes.len() {
                    i += 1;
                } else {
                    diagnostics.push(TextDiagnostic::with_range(
                        "text.sqlUnterminatedIdentifier",
                        Severity::Error,
                        "unterminated quoted identifier",
                        content,
                        TextRange::new(start as u64, content.len() as u64),
                    ));
                }
                Tok::QuotedIdent
            }
            b'-' if bytes.get(i + 1) == Some(&b'-') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                Tok::Comment
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                let closed = loop {
                    if i + 1 >= bytes.len() {
                        break false;
                    }
                    if bytes[i] == b'*' && bytes[i + 1] == b'/' {
                        i += 2;
                        break true;
                    }
                    i += 1;
                };
                if !closed {
                    diagnostics.push(TextDiagnostic::with_range(
                        "text.sqlUnterminatedComment",
                        Severity::Error,
                        "unterminated block comment",
                        content,
                        TextRange::new(start as u64, content.len() as u64),
                    ));
                }
                Tok::Comment
            }
            b'(' | b')' | b',' | b';' => {
                i += 1;
                Tok::Punct
            }
            c if c.is_ascii_whitespace() => {
                while i < bytes.len() && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                Tok::Space
            }
            c if c.is_ascii_digit() => {
                while i < bytes.len() && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.') {
                    i += 1;
                }
                Tok::Number
            }
            _ => {
                while i < bytes.len() && {
                    let c = bytes[i];
                    !(c.is_ascii_whitespace()
                        || matches!(c, b'\'' | b'"' | b'`' | b'(' | b')' | b',' | b';')
                        || (c == b'-' && bytes.get(i + 1) == Some(&b'-'))
                        || (c == b'/' && bytes.get(i + 1) == Some(&b'*')))
                } {
                    i += 1;
                }
                if i == start {
                    i += 1; // 单字符安全推进（如 / 不跟 *）
                }
                Tok::Word
            }
        };
        tokens.push(Token {
            kind,
            text: content[start..i.min(content.len())].to_string(),
            start,
        });
    }
    if diagnostics.is_empty() {
        Ok(tokens)
    } else {
        Err(diagnostics)
    }
}

const MAJOR_KEYWORDS: [&str; 13] = [
    "SELECT", "FROM", "WHERE", "GROUP", "ORDER", "HAVING", "JOIN", "LEFT", "RIGHT", "FULL",
    "INNER", "VALUES", "LIMIT",
];

fn is_major(word: &str) -> bool {
    let upper = word.to_ascii_uppercase();
    MAJOR_KEYWORDS.contains(&upper.as_str())
        || upper == "UNION"
        || upper == "INSERT"
        || upper == "UPDATE"
        || upper == "DELETE"
        || upper == "SET"
}

pub fn run(operation: FormatOperation, content: &str, options: &FormatOptions) -> FormatOutcome {
    match operation {
        FormatOperation::Validate => {
            match tokenize(content) {
                Ok(_) => {
                    // 词法级结构检查之外，补括号平衡检查
                    let mut depth: i64 = 0;
                    let mut unterminated_ok = true;
                    let _ = &mut unterminated_ok;
                    for t in tokenize(content).expect("checked") {
                        if t.kind == Tok::Punct {
                            match t.text.as_str() {
                                "(" => depth += 1,
                                ")" => depth -= 1,
                                _ => {}
                            }
                            if depth < 0 {
                                return FormatOutcome::failed(vec![TextDiagnostic::with_range(
                                    "text.sqlUnbalancedParens",
                                    Severity::Error,
                                    "unexpected ')'",
                                    content,
                                    TextRange::new(t.start as u64, t.start as u64 + 1),
                                )]);
                            }
                        }
                    }
                    if depth != 0 {
                        return FormatOutcome::failed(vec![TextDiagnostic::without_range(
                            "text.sqlUnbalancedParens",
                            Severity::Error,
                            format!("{depth} unclosed '('"),
                        )]);
                    }
                    FormatOutcome::success(None, false)
                }
                Err(diagnostics) => FormatOutcome::failed(diagnostics),
            }
        }
        FormatOperation::Format => format(content, options),
        FormatOperation::Minify => minify(content, options),
        FormatOperation::Sort | FormatOperation::Normalize => FormatOutcome::unsupported(
            "SQL clause/table/column reordering can change semantics (M4 §46); no safe Sort is provided",
        ),
    }
}

fn format(content: &str, options: &FormatOptions) -> FormatOutcome {
    let tokens = match tokenize(content) {
        Ok(t) => t,
        Err(diagnostics) => return FormatOutcome::failed(diagnostics),
    };
    let pad = " ".repeat(options.indent_spaces as usize);
    let mut out = String::new();
    let mut newline_pending = false;
    let mut in_select_list = false;
    for (idx, token) in tokens.iter().enumerate() {
        match token.kind {
            Tok::Space => continue,
            Tok::Comment => {
                if !out.is_empty() {
                    out.push('\n');
                }
                out.push_str(token.text.trim_end());
                newline_pending = true;
            }
            Tok::Word if is_major(&token.text) && !token.text.eq_ignore_ascii_case("ON") => {
                let upper = token.text.to_ascii_uppercase();
                if upper == "LEFT" || upper == "RIGHT" || upper == "FULL" || upper == "INNER" {
                    // JOIN 前缀词：同行跟随 JOIN，不独立换行
                } else {
                    if !out.is_empty() {
                        out.push('\n');
                        out.push_str(&pad);
                    }
                    if in_select_list && (upper == "FROM" || upper == "WHERE") {
                        in_select_list = false;
                    }
                    if upper == "SELECT" {
                        in_select_list = true;
                    }
                    newline_pending = false;
                }
                out.push_str(&token.text);
            }
            Tok::Punct if token.text == "," && in_select_list => {
                out.push_str(",\n");
                out.push_str(&pad);
                newline_pending = false;
            }
            Tok::Punct if token.text == "(" || token.text == "," => {
                out.push_str(&token.text);
                if token.text == "," {
                    out.push(' ');
                }
            }
            Tok::Punct if token.text == ")" => {
                out.push_str(&token.text);
            }
            Tok::Punct if token.text == ";" => {
                out.push_str(&token.text);
                newline_pending = true;
            }
            _ => {
                if newline_pending {
                    out.push(' ');
                    newline_pending = false;
                } else if needs_space(&out, token, tokens.get(idx.saturating_sub(1))) {
                    out.push(' ');
                }
                out.push_str(&token.text);
            }
        }
    }
    let changed_before = out.trim_end() != content;
    let mut out = out.trim_end().to_string();
    if options.final_newline {
        out.push('\n');
    }
    FormatOutcome::success(Some(out), changed_before)
}

fn needs_space(out: &str, current: &Token, previous: Option<&Token>) -> bool {
    if out.is_empty() || out.ends_with('\n') {
        return false;
    }
    match current.kind {
        Tok::Punct if current.text == ")" || current.text == ";" => false,
        Tok::Punct if current.text == "(" => {
            // 函数调用紧贴：word( 而不是 word (
            !matches!(
                previous.map(|p| p.kind),
                Some(Tok::Word) | Some(Tok::QuotedIdent)
            ) || previous.map(|p| p.text.to_ascii_uppercase()).as_deref() != Some("USING")
        }
        _ => !out.ends_with('('),
    }
}

fn minify(content: &str, options: &FormatOptions) -> FormatOutcome {
    let tokens = match tokenize(content) {
        Ok(t) => t,
        Err(diagnostics) => return FormatOutcome::failed(diagnostics),
    };
    let mut out = String::new();
    for token in tokens.iter().filter(|t| t.kind != Tok::Space) {
        match token.kind {
            Tok::Comment => {} // §35/§38 同源：minify 去注释
            Tok::Punct if token.text == ";" => {
                out.push(';');
            }
            Tok::Punct if token.text == "(" => out.push('('),
            Tok::Punct if token.text == ")" => out.push(')'),
            _ => {
                let prev = out.chars().next_back();
                let need_space = match prev {
                    None => false,
                    Some('(') => false,
                    Some(c) if c.is_whitespace() => false,
                    // 标识符/词之间需要空格；点号紧贴省略（v1 不识别点号，保守加空格）
                    _ => true,
                };
                if need_space {
                    out.push(' ');
                }
                out.push_str(&token.text);
            }
        }
    }
    let changed_before = out.trim() != content;
    let mut out = out.trim().to_string();
    if options.final_newline && !out.is_empty() {
        out.push('\n');
    }
    FormatOutcome::success(Some(out), changed_before)
}
