//! XML Formatter（M4 §29/§30）：quick-xml 真事件流解析。
//!
//! 能力矩阵（D43）：
//! - Validate：well-formedness（标签配对/语法；错误带 byte 位置）。
//! - Format：事件流保序重排缩进；文本/CDATA/注释/PI/属性原样保留（不改语义）。
//! - Minify：仅去掉元素间纯空白文本节点；其余原样（§41 安全子集）。
//! - Sort：Unsupported（§30：子元素顺序通常有语义，不假装能排）。

use super::{FormatOperation, FormatOptions, FormatOutcome};
use crate::diagnostics::{Severity, TextDiagnostic};
use crate::offset::TextRange;
use quick_xml::events::Event;

pub fn run(operation: FormatOperation, content: &str, options: &FormatOptions) -> FormatOutcome {
    match operation {
        FormatOperation::Validate => validate(content),
        FormatOperation::Format | FormatOperation::Normalize => format(content, options),
        FormatOperation::Minify => minify(content, options),
        FormatOperation::Sort => FormatOutcome::unsupported(
            "XML child-element order is usually semantic (M4 §30); no safe Sort capability is provided",
        ),
    }
}

fn collect(content: &str) -> Result<Vec<Event<'static>>, (String, u64)> {
    use quick_xml::Reader;
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut events = Vec::new();
    loop {
        let ev = reader
            .read_event_into(&mut buf)
            .map_err(|e| (e.to_string(), reader.buffer_position()))?;
        match &ev {
            Event::Eof => break,
            _ => events.push(ev.into_owned()),
        }
        buf.clear();
    }
    Ok(events)
}

fn diag_at(content: &str, message: String, pos: u64, code: &'static str) -> TextDiagnostic {
    TextDiagnostic::with_range(
        code,
        Severity::Error,
        message,
        content,
        TextRange::new(pos, pos),
    )
}

pub fn validate(content: &str) -> FormatOutcome {
    use quick_xml::Reader;
    let mut reader = Reader::from_str(content);
    reader.config_mut().trim_text(false);
    let mut buf = Vec::new();
    let mut stack: Vec<Vec<u8>> = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => stack.push(e.name().as_ref().to_vec()),
            Ok(Event::End(e)) => {
                let name = e.name().as_ref().to_vec();
                match stack.pop() {
                    Some(open) if open == name => {}
                    Some(open) => {
                        return FormatOutcome::failed(vec![diag_at(
                            content,
                            format!(
                                "closing </{}> does not match open <{}>",
                                String::from_utf8_lossy(&name),
                                String::from_utf8_lossy(&open)
                            ),
                            reader.buffer_position(),
                            "text.xmlMismatchedTag",
                        )]);
                    }
                    None => {
                        return FormatOutcome::failed(vec![diag_at(
                            content,
                            format!("unexpected closing </{}>", String::from_utf8_lossy(&name)),
                            reader.buffer_position(),
                            "text.xmlUnexpectedClose",
                        )]);
                    }
                }
            }
            Ok(Event::Eof) => break,
            Ok(_) => {}
            Err(e) => {
                let pos = reader.buffer_position().saturating_sub(1);
                return FormatOutcome::failed(vec![diag_at(
                    content,
                    e.to_string(),
                    pos,
                    "text.xmlInvalid",
                )]);
            }
        }
        buf.clear();
    }
    if let Some(open) = stack.last() {
        return FormatOutcome::failed(vec![TextDiagnostic::without_range(
            "text.xmlUnclosedTag",
            Severity::Error,
            format!("unclosed element <{}>", String::from_utf8_lossy(open)),
        )]);
    }
    FormatOutcome::success(None, false)
}

pub fn format(content: &str, options: &FormatOptions) -> FormatOutcome {
    let events = match collect(content) {
        Ok(e) => e,
        Err((message, pos)) => {
            return FormatOutcome::failed(vec![diag_at(content, message, pos, "text.xmlInvalid")]);
        }
    };
    let indent = " ".repeat(options.indent_spaces as usize);
    let mut out = String::new();
    let mut depth = 0usize;
    // 上一段是否为"行内文本"（决定 End 是否换行缩进）
    let mut inline_text = false;
    for ev in events {
        match ev {
            Event::Start(e) => {
                newline_indent(&mut out, &indent, depth, inline_text);
                out.push_str(&String::from_utf8_lossy(e.as_ref()));
                depth += 1;
                inline_text = false;
            }
            Event::End(e) => {
                depth = depth.saturating_sub(1);
                if !inline_text {
                    newline_indent(&mut out, &indent, depth, false);
                }
                out.push_str("</");
                out.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                out.push('>');
                inline_text = false;
            }
            Event::Text(t) => {
                // 原样保留转义实体；仅纯空白文本节点折叠（元素间缩进噪声）
                let raw = String::from_utf8_lossy(t.as_ref()).into_owned();
                if !raw.trim().is_empty() {
                    out.push_str(raw.trim());
                    inline_text = true;
                }
            }
            Event::CData(t) => {
                out.push_str("<![CDATA[");
                out.push_str(&String::from_utf8_lossy(t.as_ref()));
                out.push_str("]]>");
                inline_text = true;
            }
            Event::Comment(t) => {
                newline_indent(&mut out, &indent, depth, false);
                out.push_str("<!--");
                out.push_str(&String::from_utf8_lossy(t.as_ref()));
                out.push_str("-->");
                inline_text = false;
            }
            Event::PI(t) => {
                newline_indent(&mut out, &indent, depth, false);
                out.push_str("<?");
                out.push_str(&String::from_utf8_lossy(t.as_ref()));
                out.push_str("?>");
                inline_text = false;
            }
            Event::Decl(d) => {
                out.push_str("<?xml");
                out.push_str(&String::from_utf8_lossy(d.as_ref()));
                out.push_str("?>\n");
                inline_text = false;
            }
            Event::DocType(d) => {
                out.push_str("<!DOCTYPE");
                out.push_str(&String::from_utf8_lossy(d.as_ref()));
                out.push('>');
                inline_text = false;
            }
            Event::Empty(e) => {
                newline_indent(&mut out, &indent, depth, inline_text);
                out.push_str(&String::from_utf8_lossy(e.as_ref()));
                inline_text = false;
            }
            Event::GeneralRef(r) => {
                out.push_str(&String::from_utf8_lossy(r.as_ref()));
                inline_text = true;
            }
            Event::Eof => break,
        }
    }
    let changed = out != content;
    if options.final_newline && !out.ends_with('\n') {
        out.push('\n');
    }
    FormatOutcome::success(Some(out), changed)
}

fn newline_indent(out: &mut String, indent: &str, depth: usize, inline: bool) {
    if !out.is_empty() && !inline && !out.ends_with('\n') {
        out.push('\n');
    }
    out.push_str(&indent.repeat(depth));
}

pub fn minify(content: &str, options: &FormatOptions) -> FormatOutcome {
    let events = match collect(content) {
        Ok(e) => e,
        Err((message, pos)) => {
            return FormatOutcome::failed(vec![diag_at(content, message, pos, "text.xmlInvalid")]);
        }
    };
    let mut out = String::new();
    for ev in events {
        match ev {
            Event::Text(t) => {
                let raw = String::from_utf8_lossy(t.as_ref()).into_owned();
                if !raw.trim().is_empty() {
                    out.push_str(raw.trim());
                }
            }
            Event::CData(t) => {
                out.push_str("<![CDATA[");
                out.push_str(&String::from_utf8_lossy(t.as_ref()));
                out.push_str("]]>");
            }
            Event::Comment(t) => {
                out.push_str("<!--");
                out.push_str(&String::from_utf8_lossy(t.as_ref()));
                out.push_str("-->");
            }
            Event::Decl(_) => out.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"),
            Event::End(e) => {
                out.push_str("</");
                out.push_str(&String::from_utf8_lossy(e.name().as_ref()));
                out.push('>');
            }
            Event::GeneralRef(r) => out.push_str(&String::from_utf8_lossy(r.as_ref())),
            other => out.push_str(&String::from_utf8_lossy(other.as_ref())),
        }
    }
    let changed = out != content;
    let out = if options.final_newline {
        format!("{out}\n")
    } else {
        out
    };
    FormatOutcome::success(Some(out), changed)
}
