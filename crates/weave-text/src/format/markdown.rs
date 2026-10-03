//! Markdown Formatter（M4 §43–§45）：行结构感知；代码围栏内容不可侵犯。
//!
//! - Normalize（§44）：标题前后空行、连续空行折叠、行尾空白（围栏外）、
//!   final newline；**不改**列表标记/链接目标/raw HTML/行内代码/围栏内容。
//! - Sort（§45 opt-in）：仅按 H1/H2 标题切分的顶层 section 按标题文本排序
//!   （preamble 恒在首位；围栏内的 `#` 不是标题）。
//! - Format / Minify / Validate：Unsupported（Markdown 没有单一规范形态，
//!   §46 诚实声明）。

use super::{FormatOperation, FormatOptions, FormatOutcome};
use crate::transform::split_lines_keep;

pub fn run(operation: FormatOperation, content: &str, options: &FormatOptions) -> FormatOutcome {
    match operation {
        FormatOperation::Normalize => normalize(content, options),
        FormatOperation::Sort => sort(content, options),
        FormatOperation::Format => FormatOutcome::unsupported(
            "Markdown has no single canonical formatting; use Normalize (M4 §46)",
        ),
        FormatOperation::Minify => FormatOutcome::unsupported(
            "minifying Markdown would destroy document structure (M4 §43/§46)",
        ),
        FormatOperation::Validate => {
            FormatOutcome::unsupported("Markdown has no strict grammar to validate (M4 §46)")
        }
    }
}

fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

fn is_heading(line: &str, level: u32) -> bool {
    let hashes = "#".repeat(level as usize);
    match line.strip_prefix(&hashes) {
        Some(rest) => rest.starts_with(' ') || rest.is_empty(),
        None => false,
    }
}

fn normalize(content: &str, options: &FormatOptions) -> FormatOutcome {
    let lines = split_lines_keep(content);
    let mut out: Vec<String> = Vec::new();
    let mut in_fence = false;
    for raw in lines {
        let stripped_lf = raw.strip_suffix('\n').unwrap_or(raw);
        let body = stripped_lf.strip_suffix('\r').unwrap_or(stripped_lf);
        if is_fence(body) {
            out.push(body.to_string());
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            out.push(body.to_string());
            continue;
        }
        let line = body.trim_end().to_string();
        // 标题前后空行策略（§44）
        let heading = is_heading(&line, 1) || is_heading(&line, 2);
        if heading {
            while out.last().is_some_and(|l| l.trim().is_empty()) {
                out.pop();
            }
            if let Some(last) = out.last()
                && !last.trim().is_empty()
            {
                out.push(String::new());
            }
            out.push(line);
            out.push(String::new());
            continue;
        }
        // 连续空行折叠为一行（§44 blank line policy）
        if line.trim().is_empty() {
            if out.last().is_some_and(|l| l.trim().is_empty()) {
                continue;
            }
            out.push(String::new());
        } else {
            out.push(line);
        }
    }
    // 去掉文档开头的空行与末尾多余空行
    while out.first().is_some_and(|l| l.trim().is_empty()) {
        out.remove(0);
    }
    let mut result = out.join("\n");
    result = result.trim_end().to_string();
    let changed = result != content;
    if options.final_newline && !result.is_empty() {
        result.push('\n');
    }
    FormatOutcome::success(Some(result), changed)
}

fn sort(content: &str, options: &FormatOptions) -> FormatOutcome {
    let lines = split_lines_keep(content);
    #[derive(Debug)]
    struct Section {
        heading: String,
        lines: Vec<String>,
    }
    let mut preamble: Vec<String> = Vec::new();
    let mut sections: Vec<Section> = Vec::new();
    let mut in_fence = false;
    let mut current: Option<Section> = None;
    for raw in lines {
        let stripped_lf = raw.strip_suffix('\n').unwrap_or(raw);
        let body = stripped_lf.strip_suffix('\r').unwrap_or(stripped_lf);
        if is_fence(body) {
            in_fence = !in_fence;
        }
        let is_top_heading = !in_fence && (is_heading(body, 1) || is_heading(body, 2));
        if is_top_heading {
            if let Some(done) = current.take() {
                sections.push(done);
            }
            current = Some(Section {
                heading: body.trim().to_string(),
                lines: vec![body.to_string()],
            });
        } else {
            match &mut current {
                Some(section) => section.lines.push(body.to_string()),
                None => preamble.push(body.to_string()),
            }
        }
    }
    if let Some(last) = current.take() {
        sections.push(last);
    }
    sections.sort_by_key(|s| s.heading.to_lowercase());
    let mut result: Vec<String> = preamble;
    for section in sections {
        if !result.is_empty() {
            result.push(String::new());
        }
        result.extend(section.lines);
    }
    let mut text = result.join("\n");
    text = text.trim_end().to_string();
    let changed = text != content;
    if options.final_newline && !text.is_empty() {
        text.push('\n');
    }
    FormatOutcome::success(Some(text), changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_heading_spacing_and_blank_collapse() {
        let src = "# Title\ntext\n\n\n\n## Sub\ntail";
        let out = run(FormatOperation::Normalize, src, &FormatOptions::default());
        assert_eq!(
            out.content.expect("content"),
            "# Title\n\ntext\n\n## Sub\n\ntail\n"
        );
    }

    #[test]
    fn fence_content_is_untouchable() {
        // §43：围栏内部绝不被普通 whitespace 规则修改
        let src = "# T\n\n```\n  keep   this\n\n\n  exactly\n```\n\n\n\nafter";
        let out = run(FormatOperation::Normalize, src, &FormatOptions::default());
        let text = out.content.expect("content");
        assert!(text.contains("  keep   this\n\n\n  exactly"), "{text}");
    }

    #[test]
    fn sort_moves_h1_h2_sections_only() {
        let src = "preamble\n\n# B\nb-body\n\n## Z\nz\n\n# A\na-body\n";
        let out = run(FormatOperation::Sort, src, &FormatOptions::default());
        let text = out.content.expect("content");
        assert!(text.starts_with("preamble"));
        assert!(text.find("# A").expect("A") < text.find("# B").expect("B"));
    }
}
