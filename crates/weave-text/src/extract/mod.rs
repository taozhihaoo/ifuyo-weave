//! Text Extractor（M4 §56–§70）：统一 Match 模型 + 八种提取器 + 用户 Regex。
//!
//! 架构（§56）：Text → Extractor → Matcher → Range/Match → Ordering → Result。
//! 所有匹配共享 [`ExtractMatch`]；结果默认 source order（start 升序，
//! 同 offset 用 (end, value) 确定性次序，§69）。去重默认**保留出现次数**
//! （§70：occurrences 与 unique values 是两个都有价值的事实）。
//!
//! 引擎（§67/§68）：Rust `regex`（线性时间，天然 ReDoS 安全）。
//! 提取语义边界全部记录 DECISIONS D41（practical email、path 启发式、
//! Number 不含 hex/currency、JSON 取最外层、Markdown 跳过代码区）。

use crate::diagnostics::{Severity, TextDiagnostic};
use crate::offset::LineIndex;

/// 提取类型（§4 + 用户 Regex）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExtractKind {
    Url,
    Email,
    FilePath,
    Number,
    Ipv4,
    Ipv6,
    Json,
    MarkdownLink,
    Regex,
}

impl ExtractKind {
    pub fn as_str(self) -> &'static str {
        match self {
            ExtractKind::Url => "url",
            ExtractKind::Email => "email",
            ExtractKind::FilePath => "filePath",
            ExtractKind::Number => "number",
            ExtractKind::Ipv4 => "ipv4",
            ExtractKind::Ipv6 => "ipv6",
            ExtractKind::Json => "json",
            ExtractKind::MarkdownLink => "markdownLink",
            ExtractKind::Regex => "regex",
        }
    }
}

/// 统一匹配模型（§57）。offset 为字节；line/column 为 1-based / UTF-16 列。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtractMatch {
    pub kind: ExtractKind,
    /// 规范化值（URL/路径去尾标点；Markdown link 为 URL）。
    pub value: String,
    /// 原文切片（byte [start, end)）。
    pub raw_value: String,
    pub start: u64,
    pub end: u64,
    pub line: u64,
    pub column: u64,
    /// Markdown link 专有：链接文字。
    pub label: Option<String>,
}

/// 提取选项（§70：deduplicate 默认关——保留出现次数）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractOptions {
    /// 仅保留每个 (kind, value) 的首次出现（unique values 视图）。
    pub unique_values: bool,
}

fn err(code: &'static str, message: impl Into<String>) -> TextDiagnostic {
    TextDiagnostic::without_range(code, Severity::Error, message)
}

/// 排序（§69）：start 升序，同 start 按 (end, value) 确定性次序。
fn order(mut matches: Vec<ExtractMatch>) -> Vec<ExtractMatch> {
    matches.sort_by(|a, b| {
        a.start
            .cmp(&b.start)
            .then_with(|| a.end.cmp(&b.end))
            .then_with(|| a.value.cmp(&b.value))
    });
    matches
}

fn finish(
    content: &str,
    kind: ExtractKind,
    raws: Vec<(usize, usize, String, Option<String>)>,
    options: &ExtractOptions,
) -> Vec<ExtractMatch> {
    let index = LineIndex::new(content);
    let mut matches: Vec<ExtractMatch> = raws
        .into_iter()
        .map(|(start, end, value, label)| {
            let (line, column) = index.line_col(content, start as u64);
            ExtractMatch {
                kind,
                raw_value: content[start..end].to_string(),
                value,
                start: start as u64,
                end: end as u64,
                line,
                column,
                label,
            }
        })
        .collect();
    if options.unique_values {
        let mut seen = std::collections::HashSet::new();
        matches.retain(|m| seen.insert((m.kind, m.value.clone())));
    }
    order(matches)
}

/// 入口：按类型提取（用户 Regex 走 [`extract_regex`]；两者可并列调用）。
pub fn extract_matches(
    content: &str,
    kind: ExtractKind,
    options: &ExtractOptions,
) -> Vec<ExtractMatch> {
    match kind {
        ExtractKind::Regex => Vec::new(),
        _ => {
            let raws = match kind {
                ExtractKind::Url => extract_url(content),
                ExtractKind::Email => extract_email(content),
                ExtractKind::FilePath => extract_file_paths(content),
                ExtractKind::Number => extract_numbers(content),
                ExtractKind::Ipv4 => extract_ipv4(content),
                ExtractKind::Ipv6 => extract_ipv6(content),
                ExtractKind::Json => extract_json(content),
                ExtractKind::MarkdownLink => extract_markdown_links(content),
                ExtractKind::Regex => unreachable!("handled above"),
            };
            finish(content, kind, raws, options)
        }
    }
}

/// 用户 Regex 提取（§66/§67/§68）。零长度匹配被跳过（避免海量空匹配，
/// D41 记录）；编译错误返回结构化诊断。
pub fn extract_regex(
    content: &str,
    pattern: &str,
    options: &ExtractOptions,
) -> Result<Vec<ExtractMatch>, TextDiagnostic> {
    let re = regex::Regex::new(pattern).map_err(|e| err("text.invalidRegex", e.to_string()))?;
    let raws: Vec<(usize, usize, String, Option<String>)> = re
        .find_iter(content)
        .filter(|m| !m.is_empty())
        .map(|m| (m.start(), m.end(), m.as_str().to_string(), None))
        .collect();
    let mut matches = finish(content, ExtractKind::Regex, raws, options);
    if options.unique_values {
        let mut seen = std::collections::HashSet::new();
        matches.retain(|m| seen.insert(m.value.clone()));
    }
    Ok(matches)
}

// ── URL（§58）──────────────────────────────────────────────

fn extract_url(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let re = regex::Regex::new(r#"https?://[^\s<>"'\[\]{},]+"#).expect("static regex");
    re.find_iter(content)
        .filter_map(|m| {
            let raw = m.as_str();
            // 尾部标点裁剪（§58：https://example.com). 中的 ). 不属于 URL）
            let mut end = raw.len();
            while end > 0 {
                let c = raw[..end].chars().next_back().expect("non-empty");
                if matches!(c, '.' | ',' | ';' | ':' | '!' | '?' | '"' | '\'') {
                    end -= c.len_utf8();
                } else if c == ')'
                    && raw[..end].matches('(').count() < raw[..end].matches(')').count()
                {
                    end -= 1;
                } else {
                    break;
                }
            }
            if end < "http://x".len() {
                return None;
            }
            Some((m.start(), m.start() + end, raw[..end].to_string(), None))
        })
        .collect()
}

// ── Email（§59：practical extraction，D41 记录限制）──────────

fn extract_email(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let re = regex::Regex::new(r"[A-Za-z0-9._%+\-]+@[A-Za-z0-9.\-]+\.[A-Za-z]{2,}")
        .expect("static regex");
    re.find_iter(content)
        .map(|m| (m.start(), m.end(), m.as_str().to_string(), None))
        .collect()
}

// ── File Path（§60：启发式，D41 记录）──────────────────────

fn is_path_byte(b: u8) -> bool {
    // 排除空白与 Windows 非法字符 * ? " < > |（盘符冒号允许）
    !matches!(
        b,
        b' ' | b'\t' | b'\r' | b'\n' | b'*' | b'?' | b'"' | b'<' | b'>' | b'|'
    ) && (0x20..0x7F).contains(&b)
}

fn extract_file_paths(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        let rest = &content[i..];
        let len = if rest.starts_with("\\\\") {
            // UNC：\\server\share\...
            rest.find(char::is_whitespace).unwrap_or(rest.len())
        } else if rest.len() >= 3
            && rest.as_bytes()[0].is_ascii_alphabetic()
            && rest.as_bytes()[1] == b':'
            && rest.as_bytes()[2] == b'\\'
        {
            // Windows 盘符路径（保守：冒号后必须紧跟分隔符）
            rest.find(char::is_whitespace).unwrap_or(rest.len())
        } else if rest.starts_with('/') || rest.starts_with("./") || rest.starts_with("../") {
            // Unix 绝对 / 显式相对（词中斜杠不算：前一字节不得是字母数字，D41）
            let boundary_ok = i == 0 || !bytes[i - 1].is_ascii_alphanumeric();
            if boundary_ok {
                rest.find(char::is_whitespace).unwrap_or(rest.len())
            } else {
                0
            }
        } else {
            0
        };
        // 尾部标点裁剪：prose 列表 "path.txt, and" 的逗号等不属于路径；
        // '.' 属于版本号语义，保留（D41）
        let mut len = len;
        while len > 0 && matches!(rest.as_bytes()[len - 1], b',' | b';' | b')' | b']' | b'}') {
            len -= 1;
        }
        // 保守（D41）：裸 word/word 不当路径；必须含分隔符且全部字符合法
        if len > 3 && rest[..len].bytes().all(is_path_byte) && rest[..len].contains(['\\', '/']) {
            out.push((i, i + len, rest[..len].to_string(), None));
            i += len;
        } else {
            i += 1;
        }
    }
    out
}

// ── Number（§61，D41：不含 hex/octal/binary/currency）───────

fn extract_numbers(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let re = regex::Regex::new(r"-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?%?").expect("static regex");
    let bytes = content.as_bytes();
    re.find_iter(content)
        .filter(|m| {
            // 边界防护：前后不得是字母数字（避免 "a1" 的 1、URL 片段等）
            let before_ok = m.start() == 0 || !bytes[m.start() - 1].is_ascii_alphanumeric();
            let after_ok = m.end() >= bytes.len() || !bytes[m.end()].is_ascii_alphanumeric();
            before_ok && after_ok
        })
        .map(|m| (m.start(), m.end(), m.as_str().to_string(), None))
        .collect()
}

// ── IPv4（§62：octet 0-255 实校验）────────────────────────

fn extract_ipv4(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let re = regex::Regex::new(r"\b\d{1,3}(?:\.\d{1,3}){3}\b").expect("static regex");
    re.find_iter(content)
        .filter(|m| m.as_str().split('.').all(|o| o.parse::<u8>().is_ok()))
        .map(|m| (m.start(), m.end(), m.as_str().to_string(), None))
        .collect()
}

// ── IPv6（§62：std::net 真解析器，非 regex 判定）───────────

fn extract_ipv6(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i].is_ascii_hexdigit() || bytes[i] == b':' {
            let start = i;
            while i < bytes.len()
                && (bytes[i].is_ascii_hexdigit() || bytes[i] == b':' || bytes[i] == b'.')
            {
                // '.' 仅在候选已含 ':'（v4 嵌入尾）时跟随，避免吞并普通句子
                if bytes[i] == b'.' && !content[start..i].contains(':') {
                    break;
                }
                i += 1;
            }
            let candidate = &content[start..i];
            let before_ok = start == 0 || !bytes[start - 1].is_ascii_alphanumeric();
            let after_ok = i >= bytes.len() || !bytes[i].is_ascii_alphanumeric();
            if candidate.contains(':')
                && candidate.parse::<std::net::Ipv6Addr>().is_ok()
                && before_ok
                && after_ok
            {
                out.push((start, i, candidate.to_string(), None));
            }
        } else {
            i += 1;
        }
    }
    out
}

// ── JSON（§63/§64：平衡结构扫描 + serde_json 终验）─────────

fn extract_json(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let bytes = content.as_bytes();
    let mut out = Vec::new();
    let mut i = 0usize;
    while i < bytes.len() {
        match bytes[i] {
            b'{' | b'[' => {
                let open = bytes[i];
                let close = if open == b'{' { b'}' } else { b']' };
                let (close_pos, closed) = {
                    let mut depth = 0usize;
                    let mut in_string = false;
                    let mut escaped = false;
                    let mut j = i;
                    while j < bytes.len() {
                        let b = bytes[j];
                        if in_string {
                            if escaped {
                                escaped = false;
                            } else if b == b'\\' {
                                escaped = true;
                            } else if b == b'"' {
                                in_string = false;
                            }
                        } else if b == b'"' {
                            in_string = true;
                        } else if b == open {
                            depth += 1;
                        } else if b == close {
                            depth -= 1;
                            if depth == 0 {
                                break;
                            }
                        }
                        j += 1;
                    }
                    (j, j < bytes.len())
                };
                // 语法终验（§64）：平衡 ≠ 合法 JSON；字符串内的引号/转义由
                // in_string/escaped 状态机处理
                if closed {
                    let candidate = &content[i..=close_pos];
                    if serde_json::from_str::<serde_json::Value>(candidate).is_ok() {
                        out.push((i, close_pos + 1, candidate.to_string(), None));
                        i = close_pos + 1;
                        continue;
                    }
                }
                i += 1;
            }
            _ => i += 1,
        }
    }
    out
}

// ── Markdown Link（§65：跳过围栏代码块与行内代码，D41）──────

/// 内容中「处于代码上下文」的 byte 区间集合（``` 围栏 + 行内 `...`）。
fn code_spans(content: &str) -> Vec<(usize, usize)> {
    let mut spans = Vec::new();
    let mut in_fence = false;
    let mut pos = 0usize;
    for line in crate::transform::split_lines_keep(content) {
        let start = pos;
        pos += line.len();
        let trimmed = line.trim_start();
        if trimmed.starts_with("```") {
            spans.push((start, start + line.len()));
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            spans.push((start, start + line.len()));
        }
    }
    // 行内代码：成对反引号（简单规则，D41）
    let bytes = content.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            if let Some(close) = content[i + 1..].find('`') {
                spans.push((i, i + close + 2));
                i += close + 2;
            } else {
                i += 1;
            }
        } else {
            i += 1;
        }
    }
    spans
}

fn extract_markdown_links(content: &str) -> Vec<(usize, usize, String, Option<String>)> {
    let re = regex::Regex::new(r#"\[((?:[^\[\]\\]|\\.)*)\]\(([^()\s]+)(?:\s+"[^"]*")?\)"#)
        .expect("static regex");
    let spans = code_spans(content);
    let in_code = |pos: usize| spans.iter().any(|(s, e)| pos >= *s && pos < *e);
    re.captures_iter(content)
        .filter_map(|caps| {
            let whole = caps.get(0)?;
            if in_code(whole.start()) {
                return None;
            }
            let label = caps.get(1)?.as_str().to_string();
            let url = caps.get(2)?.as_str().to_string();
            Some((whole.start(), whole.end(), url, Some(label)))
        })
        .collect()
}

#[cfg(test)]
mod tests;
