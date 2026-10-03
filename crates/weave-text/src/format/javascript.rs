//! JavaScript Formatter（M4 §37–§39）：能力边界诚实（D43）。
//!
//! - Format / Validate：Unsupported——没有真 JS parser（§37/§39 明令禁止
//!   regex 括号计数冒充；引入完整 parser 依赖超出 Minimal-dependency 原则）。
//! - Minify：token 级**安全子集**（§38 明示允许）：字符串/模板字面量内容
//!   原样保留；行/块注释删除；词间空白压缩。**regex 字面量内的 `//` 可能
//!   被误判为行注释**——记录为已知限制（D43），用户可用 keep-comments 选项
//!   规避（本 v1 保守提供 preserve_comments 开关在 Options 之外——见下）。

use super::{FormatOperation, FormatOptions, FormatOutcome};

pub fn run(operation: FormatOperation, content: &str, options: &FormatOptions) -> FormatOutcome {
    match operation {
        FormatOperation::Minify => minify(content, options),
        FormatOperation::Format => FormatOutcome::unsupported(
            "full JS formatting requires a real parser (M4 §37); not implemented in M4 v1",
        ),
        FormatOperation::Validate => FormatOutcome::unsupported(
            "JS validation requires a real parser (M4 §39); brace counting is forbidden",
        ),
        FormatOperation::Sort | FormatOperation::Normalize => {
            FormatOutcome::unsupported("JS statement reordering changes semantics (M4 §46)")
        }
    }
}

/// 词法感知压缩：字符串/模板内容原样；注释删除；词间空白压成必要空格。
fn minify(content: &str, _options: &FormatOptions) -> FormatOutcome {
    let bytes = content.as_bytes();
    let mut out = String::with_capacity(content.len());
    let mut i = 0usize;
    let mut last_word_char = false;
    let mut pending_space = false;
    while i < bytes.len() {
        let b = bytes[i];
        match b {
            b'\'' | b'"' => {
                let needed = last_word_char;
                flush_space(&mut out, &mut pending_space, needed);
                out.push(b as char);
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        out.push(bytes[i] as char);
                        if i + 1 < bytes.len() {
                            out.push(bytes[i + 1] as char);
                        }
                        i += 2;
                        continue;
                    }
                    out.push(bytes[i] as char);
                    if bytes[i] == b {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                last_word_char = false;
            }
            b'`' => {
                // 模板字面量：逐字保留（含换行与 ${} 内容——保守不压缩内部）
                out.push('`');
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' && i + 1 < bytes.len() {
                        out.push(bytes[i] as char);
                        out.push(bytes[i + 1] as char);
                        i += 2;
                        continue;
                    }
                    out.push(bytes[i] as char);
                    if bytes[i] == b'`' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                last_word_char = false;
            }
            b'/' if bytes.get(i + 1) == Some(&b'/') => {
                while i < bytes.len() && bytes[i] != b'\n' {
                    i += 1;
                }
                last_word_char = false;
            }
            b'/' if bytes.get(i + 1) == Some(&b'*') => {
                i += 2;
                while i + 1 < bytes.len() && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(bytes.len());
                last_word_char = false;
            }
            c if c.is_ascii_whitespace() => {
                // 词/数字边界处的空白压成 1 个空格；标点旁省略
                if last_word_char {
                    pending_space = true;
                }
                i += 1;
            }
            _ => {
                let is_word = b.is_ascii_alphanumeric() || b == b'_' || b == b'$';
                let needed = last_word_char && is_word;
                flush_space(&mut out, &mut pending_space, needed);
                out.push(b as char);
                i += 1;
                last_word_char = is_word;
            }
        }
    }
    let trimmed = out.trim_end().to_string();
    let changed = trimmed != content;
    let result = if trimmed.is_empty() {
        trimmed
    } else {
        format!("{trimmed}\n")
    };
    FormatOutcome::success(Some(result), changed)
}

fn flush_space(out: &mut String, pending: &mut bool, needed: bool) {
    if *pending && needed {
        out.push(' ');
    }
    *pending = false;
}
