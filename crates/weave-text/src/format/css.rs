//! CSS Formatter（M4 §40–§41）：token 感知（字符串/url()/注释），
//! 结构级 Format/Minify；Validate 按 §42/§46 诚实 Unsupported。
//!
//! - Format：规则块布局（selector {\n  prop: value;\n}），@media 嵌套缩进。
//! - Minify：去注释 + 空白压缩；字符串/url()/calc() 内容原样（§41）。
//! - Sort / Normalize / Validate：Unsupported（层叠语义 / 无完整语法校验，D43）。

use super::{FormatOperation, FormatOptions, FormatOutcome};

pub fn run(operation: FormatOperation, content: &str, options: &FormatOptions) -> FormatOutcome {
    match operation {
        FormatOperation::Format => format(content, options),
        FormatOperation::Minify => minify(content, options),
        FormatOperation::Validate => FormatOutcome::unsupported(
            "full CSS grammar validation is not implemented (M4 §42/§46)",
        ),
        FormatOperation::Sort | FormatOperation::Normalize => {
            FormatOutcome::unsupported("CSS rule reordering can change cascade semantics (M4 §46)")
        }
    }
}

/// 扫描状态机：返回「结构 token」序列（保留原文）。
#[derive(Debug, Clone, PartialEq, Eq)]
enum Piece {
    /// 任意非结构性文本（selector/prop/value 片段，已 trim）。
    Text(String),
    OpenBrace,
    CloseBrace,
    Semicolon,
    Colon,
}

fn pieces(content: &str) -> Vec<Piece> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut text = String::new();
    let mut i = 0usize;
    let mut in_string: Option<u8> = None;
    let mut in_comment = false;
    let mut url_paren_depth = 0usize; // url(...) 内的空白/括号不处理
    while i < bytes.len() {
        let b = bytes[i];
        if in_comment {
            if b == b'*' && bytes.get(i + 1) == Some(&b'/') {
                in_comment = false;
                i += 2;
            } else {
                i += 1;
            }
            continue;
        }
        if let Some(q) = in_string {
            text.push(b as char);
            if b == b'\\' && i + 1 < bytes.len() {
                i += 1;
                text.push(bytes[i] as char);
            } else if b == q {
                in_string = None;
            }
            i += 1;
            continue;
        }
        match b {
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                in_comment = true;
                i += 2;
            }
            b'\'' | b'"' => {
                in_string = Some(b);
                text.push(b as char);
                i += 1;
            }
            b'(' => {
                url_paren_depth += 1;
                text.push('(');
                i += 1;
            }
            b')' => {
                url_paren_depth = url_paren_depth.saturating_sub(1);
                text.push(')');
                i += 1;
            }
            b'{' => {
                push_text(&mut out, &mut text);
                out.push(Piece::OpenBrace);
                i += 1;
            }
            b'}' => {
                push_text(&mut out, &mut text);
                out.push(Piece::CloseBrace);
                i += 1;
            }
            b';' if url_paren_depth == 0 => {
                push_text(&mut out, &mut text);
                out.push(Piece::Semicolon);
                i += 1;
            }
            b':' if url_paren_depth == 0 => {
                push_text(&mut out, &mut text);
                out.push(Piece::Colon);
                i += 1;
            }
            _ => {
                text.push(b as char);
                i += 1;
            }
        }
    }
    push_text(&mut out, &mut text);
    out
}

fn push_text(out: &mut Vec<Piece>, text: &mut String) {
    let trimmed = text.trim().to_string();
    if !trimmed.is_empty() {
        out.push(Piece::Text(trimmed));
    }
    text.clear();
}

fn format(content: &str, options: &FormatOptions) -> FormatOutcome {
    let pad = " ".repeat(options.indent_spaces as usize);
    let mut out = String::new();
    let mut depth = 0usize;
    let mut pending_open = false;
    for piece in pieces(content) {
        match piece {
            Piece::Text(t) => {
                if pending_open {
                    out.push('\n');
                    pending_open = false;
                }
                for (j, line) in t.split('\n').enumerate() {
                    if j > 0 {
                        out.push('\n');
                    }
                    out.push_str(&pad.repeat(depth));
                    out.push_str(line.trim());
                }
            }
            Piece::OpenBrace => {
                out.push_str(" {\n");
                depth += 1;
                pending_open = false;
            }
            Piece::CloseBrace => {
                depth = depth.saturating_sub(1);
                out.push('\n');
                out.push_str(&pad.repeat(depth));
                out.push('}');
                // 后续声明紧跟换行
                out.push('\n');
                out.push_str(&pad.repeat(depth));
                pending_open = false;
            }
            Piece::Semicolon => {
                out.push_str(";\n");
                out.push_str(&pad.repeat(depth));
            }
            Piece::Colon => out.push_str(": "),
        }
    }
    // 收尾：清理多余空行与缩进
    let mut cleaned = String::new();
    for line in out.lines() {
        let trimmed_end = line.trim_end();
        if trimmed_end.is_empty() && (cleaned.is_empty() || cleaned.ends_with('\n')) {
            continue;
        }
        cleaned.push_str(trimmed_end);
        cleaned.push('\n');
    }
    let mut result = cleaned.trim_end().to_string();
    let changed = result != content;
    if options.final_newline && !result.is_empty() {
        result.push('\n');
    }
    FormatOutcome::success(Some(result), changed)
}

fn minify(content: &str, options: &FormatOptions) -> FormatOutcome {
    let mut out = String::new();
    let mut pending_space = false;
    for piece in pieces(content) {
        match piece {
            Piece::Text(t) => {
                // selector/prop/value 中的内部空白压缩为单空格（字符串内容
                // 已由 pieces 原样保留——内部空格不动？pieces 不动字符串，
                // 但 Text 是聚合的；此处对含引号文本保守原样输出）
                let has_quote = t.contains('"') || t.contains('\'');
                let t = if has_quote {
                    t
                } else {
                    t.split_whitespace().collect::<Vec<_>>().join(" ")
                };
                if !t.is_empty() {
                    if pending_space {
                        out.push(' ');
                    }
                    out.push_str(&t);
                    pending_space = true;
                }
            }
            Piece::OpenBrace => {
                if out.ends_with(' ') {
                    out.pop();
                }
                out.push('{');
                pending_space = false;
            }
            Piece::CloseBrace => {
                if out.ends_with(';') {
                    out.pop();
                }
                out.push('}');
                pending_space = false;
            }
            Piece::Semicolon => {
                out.push(';');
                pending_space = false;
            }
            Piece::Colon => {
                out.push(':');
                pending_space = false;
            }
        }
    }
    let mut result = out.trim().to_string();
    let changed = result != content;
    if options.final_newline && !result.is_empty() {
        result.push('\n');
    }
    FormatOutcome::success(Some(result), changed)
}
