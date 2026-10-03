//! Text Transformer（M4 §71–§83）：纯逻辑、无文件副作用、确定性。
//!
//! 原则：
//! - 行级操作按行携带各自原始换行结尾（Preserve by default，§17/D39），
//!   绝不无意义重排换行风格。
//! - Find/Replace 的 regex 引擎 = Rust `regex`（线性时间，ReDoS 安全，§68）；
//!   替换语法 contract = Rust regex replacement（`$1`/`${name}`，§82，D40）。
//! - Title/Sentence case 采用明确定义的简单模式（§78 允许，D40 记录语义）。

use crate::diagnostics::{Severity, TextDiagnostic};

/// 行拆分：按行拆分并**保留每行的换行结尾**（LF / CRLF / 独立 CR）。
pub fn split_lines_keep(content: &str) -> Vec<&str> {
    let bytes = content.as_bytes();
    let mut lines = Vec::new();
    let mut start = 0usize;
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'\n' => {
                lines.push(&content[start..=i]);
                start = i + 1;
                i += 1;
            }
            b'\r' if bytes.get(i + 1) == Some(&b'\n') => i += 1,
            b'\r' => {
                lines.push(&content[start..=i]);
                start = i + 1;
                i += 1;
            }
            _ => i += 1,
        }
    }
    if start < content.len() {
        lines.push(&content[start..]);
    }
    lines
}

/// 行内容（不含换行结尾）。
fn line_body(line: &str) -> &str {
    let trimmed = line.strip_suffix('\n').unwrap_or(line);
    trimmed.strip_suffix('\r').unwrap_or(trimmed)
}

/// 行是否为空行（去掉结尾与空白后为空，§74）。
fn is_blank(line: &str) -> bool {
    line_body(line).trim().is_empty()
}

/// Deduplicate 的保留策略（§73）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeepPolicy {
    First,
    Last,
}

/// 空行策略（§74/§76，D40：默认全量参与，可选忽略/保位/排首/排尾）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlankPolicy {
    /// 空行与普通行同等参与（去重/排序）。
    Included,
    /// 空行不参与（去重：原样保留；排序：保持原位）。
    Preserve,
    /// 排序专用：空行排最前。
    First,
    /// 排序专用：空行排最后。
    Last,
}

/// 大小写转换形态（§78，D40 记录简单模式语义）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseForm {
    Upper,
    Lower,
    /// 每个**空白分隔词**的首个字母大写、其余小写。
    Title,
    /// 每行首个字母大写、其余字母小写。
    Sentence,
}

/// 行号填充（§79）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PadMode {
    None,
    /// 按"最后一个行号的位数"补零。
    Zeros,
}

/// 变换操作全集（§72–§82）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransformKind {
    TrimLines,
    TrimDocument,
    DeduplicateLines {
        keep: KeepPolicy,
        blank: BlankPolicy,
    },
    SortLines {
        descending: bool,
        case_sensitive: bool,
        blank: BlankPolicy,
    },
    AddPrefix {
        text: String,
        skip_blank: bool,
    },
    AddSuffix {
        text: String,
        skip_blank: bool,
    },
    CaseConvert {
        form: CaseForm,
    },
    NumberLines {
        start: u64,
        step: u64,
        separator: String,
        pad: PadMode,
    },
    FindReplace {
        find: String,
        replacement: String,
        regex: bool,
        case_insensitive: bool,
        first_only: bool,
    },
}

/// 变换结果：新内容 + 事实计数（§80 match count 等；不适用的操作为 None）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransformResult {
    pub content: String,
    /// Find/Replace 的匹配数（§80 Preview 必须明确 "12 matches"）。
    pub match_count: Option<u64>,
    /// Deduplicate 移除的行数。
    pub removed_lines: Option<u64>,
}

fn invalid_regex(message: String) -> TextDiagnostic {
    TextDiagnostic::without_range("text.invalidRegex", Severity::Error, message)
}

fn result(content: String, matches: Option<u64>, removed: Option<u64>) -> TransformResult {
    TransformResult {
        content,
        match_count: matches,
        removed_lines: removed,
    }
}

/// 稳定排序：同键元素保持输入相对顺序（确定性，§75）；大小写不敏感键 =
/// 全词小写（Unicode 稳定规则，§52 同源）。
fn sort_strs(lines: &mut [&str], descending: bool, case_sensitive: bool) {
    if case_sensitive {
        if descending {
            lines.sort_by(|a, b| b.cmp(a));
        } else {
            lines.sort();
        }
    } else if descending {
        lines.sort_by_key(|l| std::cmp::Reverse(l.to_lowercase()));
    } else {
        lines.sort_by_key(|l| l.to_lowercase());
    }
}

/// 应用一个变换（纯函数；同输入同选项必同输出，§71）。
pub fn apply_transform(
    content: &str,
    kind: &TransformKind,
) -> Result<TransformResult, TextDiagnostic> {
    match kind {
        TransformKind::TrimLines => {
            let lines: Vec<String> = split_lines_keep(content)
                .iter()
                .map(|line| {
                    let body = line_body(line);
                    let ending = &line[body.len()..];
                    format!("{}{ending}", body.trim())
                })
                .collect();
            Ok(result(lines.concat(), None, None))
        }
        TransformKind::TrimDocument => Ok(result(content.trim().to_string(), None, None)),
        TransformKind::DeduplicateLines { keep, blank } => {
            let lines = split_lines_keep(content);
            let key = |line: &str| line_body(line).to_string();
            let skipped = |line: &str| *blank == BlankPolicy::Preserve && is_blank(line);
            let mut removed = 0u64;
            let out: Vec<String> = match keep {
                KeepPolicy::First => {
                    let mut seen = std::collections::HashSet::new();
                    lines
                        .iter()
                        .filter(|line| {
                            skipped(line) || seen.insert(key(line)) || {
                                removed += 1;
                                false
                            }
                        })
                        .map(|l| l.to_string())
                        .collect()
                }
                KeepPolicy::Last => {
                    // 逆序标记最后一次出现；正向过滤 ⇒ 顺序保持、位置取最后出现处
                    let mut seen = std::collections::HashSet::new();
                    let mut keep_flags = vec![false; lines.len()];
                    for (i, line) in lines.iter().enumerate().rev() {
                        if skipped(line) || seen.insert(key(line)) {
                            keep_flags[i] = true;
                        }
                    }
                    removed = lines.len() as u64 - keep_flags.iter().filter(|f| **f).count() as u64;
                    lines
                        .iter()
                        .zip(&keep_flags)
                        .filter(|(_, keep)| **keep)
                        .map(|(l, _)| l.to_string())
                        .collect()
                }
            };
            Ok(result(out.concat(), None, Some(removed)))
        }
        TransformKind::SortLines {
            descending,
            case_sensitive,
            blank,
        } => {
            let lines = split_lines_keep(content);
            if *blank == BlankPolicy::Preserve {
                // 空行保持原位，非空行排序后按序回填非空槽位
                let mut non_blank: Vec<&str> =
                    lines.iter().filter(|l| !is_blank(l)).copied().collect();
                sort_strs(&mut non_blank, *descending, *case_sensitive);
                let mut it = non_blank.into_iter();
                let out: Vec<String> = lines
                    .iter()
                    .map(|l| {
                        if is_blank(l) {
                            l.to_string()
                        } else {
                            it.next().unwrap_or(l).to_string()
                        }
                    })
                    .collect();
                return Ok(result(out.concat(), None, None));
            }
            let mut all: Vec<&str> = lines.to_vec();
            match blank {
                BlankPolicy::First => {
                    all.sort_by_key(|l| !is_blank(l));
                    let rest_start = all.partition_point(|l| is_blank(l));
                    sort_strs(&mut all[rest_start..], *descending, *case_sensitive);
                }
                BlankPolicy::Last => {
                    all.sort_by_key(|l| is_blank(l));
                    let rest_end = all.partition_point(|l| !is_blank(l));
                    sort_strs(&mut all[..rest_end], *descending, *case_sensitive);
                }
                BlankPolicy::Included => {
                    sort_strs(&mut all, *descending, *case_sensitive);
                }
                BlankPolicy::Preserve => unreachable!("handled above"),
            }
            Ok(result(
                all.into_iter().collect::<Vec<_>>().concat(),
                None,
                None,
            ))
        }
        TransformKind::AddPrefix { text, skip_blank } => {
            let lines: Vec<String> = split_lines_keep(content)
                .iter()
                .map(|line| {
                    if *skip_blank && is_blank(line) {
                        line.to_string()
                    } else {
                        format!("{text}{line}")
                    }
                })
                .collect();
            Ok(result(lines.concat(), None, None))
        }
        TransformKind::AddSuffix { text, skip_blank } => {
            let lines: Vec<String> = split_lines_keep(content)
                .iter()
                .map(|line| {
                    if *skip_blank && is_blank(line) {
                        line.to_string()
                    } else {
                        let body = line_body(line);
                        let ending = &line[body.len()..];
                        format!("{body}{text}{ending}")
                    }
                })
                .collect();
            Ok(result(lines.concat(), None, None))
        }
        TransformKind::CaseConvert { form } => {
            let lines: Vec<String> = split_lines_keep(content)
                .iter()
                .map(|line| {
                    let body = line_body(line);
                    let ending = &line[body.len()..];
                    let converted = match form {
                        CaseForm::Upper => body.to_uppercase(),
                        CaseForm::Lower => body.to_lowercase(),
                        CaseForm::Title => title_case(body),
                        CaseForm::Sentence => sentence_case(body),
                    };
                    format!("{converted}{ending}")
                })
                .collect();
            Ok(result(lines.concat(), None, None))
        }
        TransformKind::NumberLines {
            start,
            step,
            separator,
            pad,
        } => {
            let lines = split_lines_keep(content);
            let last_number = start + step.saturating_mul(lines.len().saturating_sub(1) as u64);
            let width = match pad {
                PadMode::None => 0,
                PadMode::Zeros => last_number.to_string().len(),
            };
            let out: Vec<String> = lines
                .iter()
                .enumerate()
                .map(|(i, line)| {
                    let n = start + step * i as u64;
                    format!("{n:0width$}{separator}{line}")
                })
                .collect();
            Ok(result(out.concat(), None, None))
        }
        TransformKind::FindReplace {
            find,
            replacement,
            regex,
            case_insensitive,
            first_only,
        } => {
            if find.is_empty() {
                return Err(TextDiagnostic::without_range(
                    "text.emptyFind",
                    Severity::Error,
                    "find pattern must not be empty",
                ));
            }
            let pattern = if *regex {
                find.clone()
            } else {
                regex::escape(find)
            };
            let re = regex::RegexBuilder::new(&pattern)
                .case_insensitive(*case_insensitive)
                .build()
                .map_err(|e| invalid_regex(e.to_string()))?;
            let result_content = if *first_only {
                re.replacen(content, 1, replacement.as_str())
            } else {
                re.replace_all(content, replacement.as_str())
            }
            .into_owned();
            let count = if *first_only {
                Some(u64::from(re.find(content).is_some()))
            } else {
                Some(re.find_iter(content).count() as u64)
            };
            Ok(TransformResult {
                content: result_content,
                match_count: count,
                removed_lines: None,
            })
        }
    }
}

/// Title case：按空白切词，每词首个字母大写、其余小写（D40）。
fn title_case(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut at_word_start = true;
    for ch in body.chars() {
        if ch.is_whitespace() {
            at_word_start = true;
            out.push(ch);
        } else if at_word_start {
            out.extend(ch.to_uppercase());
            at_word_start = false;
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

/// Sentence case：每行首个字母大写、其余字母小写（D40 简单模式）。
fn sentence_case(body: &str) -> String {
    let mut out = String::with_capacity(body.len());
    let mut first_done = false;
    for ch in body.chars() {
        if !first_done {
            if ch.is_alphabetic() {
                out.extend(ch.to_uppercase());
                first_done = true;
            } else {
                out.push(ch);
            }
        } else {
            out.extend(ch.to_lowercase());
        }
    }
    out
}

#[cfg(test)]
mod tests;
