//! Rename 规则模型与纯函数应用管线（M2 §12–§18）。
//!
//! 全部函数 deterministic、无副作用：输入 (base, ext, context) → 输出名。
//! 规则按用户列表顺序应用；模板（若提供）最后重组。

use chrono::{DateTime, Local};
use weave_core::prelude::WeaveError;

/// 日期字段（M2 §15；默认 Modified 由调用方选择）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateField {
    Created,
    Modified,
    Accessed,
}

/// 日期格式（M2 §15 至少三种；结构化 formatter，非 UI 手拼）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DateFormat {
    YmdDash,    // YYYY-MM-DD
    YmdCompact, // YYYYMMDD
    YmDash,     // YYYY-MM
}

impl DateFormat {
    pub fn format(self, time: std::time::SystemTime) -> String {
        let dt: DateTime<Local> = time.into();
        match self {
            DateFormat::YmdDash => dt.format("%Y-%m-%d").to_string(),
            DateFormat::YmdCompact => dt.format("%Y%m%d").to_string(),
            DateFormat::YmDash => dt.format("%Y-%m").to_string(),
        }
    }
}

/// 大小写形式（M2 §16；Unicode 感知——`str::to_lowercase/to_uppercase`，
/// Title Case 采用简单词首大写语义并记录于 docs/rename-rules.md）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaseForm {
    Lower,
    Upper,
    Title,
}

/// 单条重命名规则（M2 §13–§18 全部十类）。
#[derive(Debug, Clone, PartialEq)]
pub enum RenameRule {
    Prefix {
        text: String,
    },
    Suffix {
        text: String,
    },
    Replace {
        find: String,
        replace_with: String,
    },
    RegexReplace {
        pattern: String,
        replacement: String,
    },
    Counter {
        start: u64,
        step: u64,
        width: u32,
    },
    Date {
        field: DateField,
        format: DateFormat,
    },
    Case {
        form: CaseForm,
    },
    Extension {
        new_extension: String,
    },
    Template {
        template: String,
    },
}

/// 计划阶段的规则编译产物：预校验 + 可执行形态。
#[derive(Debug, Clone)]
pub enum CompiledRule {
    Prefix {
        text: String,
    },
    Suffix {
        text: String,
    },
    Replace {
        find: String,
        replace_with: String,
    },
    RegexReplace {
        regex: regex::Regex,
        replacement: String,
    },
    Counter {
        start: u64,
        step: u64,
        width: u32,
    },
    Date {
        field: DateField,
        format: DateFormat,
    },
    Case {
        form: CaseForm,
    },
    Extension {
        new_extension: String,
    },
    Template {
        segments: Vec<TemplateSegment>,
    },
}

#[derive(Debug, Clone)]
pub enum TemplateSegment {
    Literal(String),
    Name,
    Ext,
    Counter,
    Date,
    Original,
}

/// 单条目的规则应用上下文（M2 §12：name + metadata + sequence context）。
#[derive(Debug, Clone, Copy)]
pub struct RenameContext<'a> {
    /// 确定性排序后的序号（0-based）。
    pub index: u64,
    /// 该文件的规则基准时间（默认 Modified；不可用则 None）。
    pub date: Option<&'a std::time::SystemTime>,
}

/// 模板解析错误：未知占位符必须成为 Plan Error 而非静默原文（M2 §18）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemplateError {
    pub placeholder: String,
}

fn parse_template(template: &str) -> Result<Vec<TemplateSegment>, TemplateError> {
    let mut segments = Vec::new();
    let mut literal = String::new();
    let mut chars = template.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '{' {
            let mut name = String::new();
            let mut closed = false;
            for n in chars.by_ref() {
                if n == '}' {
                    closed = true;
                    break;
                }
                name.push(n);
            }
            if !closed {
                return Err(TemplateError {
                    placeholder: format!("unclosed {{{name}"),
                });
            }
            match name.as_str() {
                "name" => {
                    if !literal.is_empty() {
                        segments.push(TemplateSegment::Literal(std::mem::take(&mut literal)));
                    }
                    segments.push(TemplateSegment::Name);
                }
                "ext" => {
                    if !literal.is_empty() {
                        segments.push(TemplateSegment::Literal(std::mem::take(&mut literal)));
                    }
                    segments.push(TemplateSegment::Ext);
                }
                "counter" => {
                    if !literal.is_empty() {
                        segments.push(TemplateSegment::Literal(std::mem::take(&mut literal)));
                    }
                    segments.push(TemplateSegment::Counter);
                }
                "date" => {
                    if !literal.is_empty() {
                        segments.push(TemplateSegment::Literal(std::mem::take(&mut literal)));
                    }
                    segments.push(TemplateSegment::Date);
                }
                "original" => {
                    if !literal.is_empty() {
                        segments.push(TemplateSegment::Literal(std::mem::take(&mut literal)));
                    }
                    segments.push(TemplateSegment::Original);
                }
                other => {
                    return Err(TemplateError {
                        placeholder: other.to_string(),
                    });
                }
            }
        } else {
            literal.push(c);
        }
    }
    if !literal.is_empty() {
        segments.push(TemplateSegment::Literal(literal));
    }
    Ok(segments)
}

/// 规则预校验 + 编译（正则/模板错误在此成为结构化错误，禁止带病进 Plan）。
pub fn compile_rules(rules: &[RenameRule]) -> Result<Vec<CompiledRule>, WeaveError> {
    rules
        .iter()
        .map(|rule| {
            Ok(match rule {
                RenameRule::Prefix { text } => CompiledRule::Prefix { text: text.clone() },
                RenameRule::Suffix { text } => CompiledRule::Suffix { text: text.clone() },
                RenameRule::Replace { find, replace_with } => {
                    if find.is_empty() {
                        return Err(WeaveError::validation(
                            "rename.emptyFind",
                            "Replace rule requires a non-empty find text",
                        )
                        .with_location("weave-files::rename"));
                    }
                    CompiledRule::Replace {
                        find: find.clone(),
                        replace_with: replace_with.clone(),
                    }
                }
                RenameRule::RegexReplace {
                    pattern,
                    replacement,
                } => {
                    let regex = regex::Regex::new(pattern).map_err(|e| {
                        WeaveError::validation(
                            "rename.invalidRegex",
                            format!("invalid regex '{pattern}': {e}"),
                        )
                        .with_location("weave-files::rename")
                        .with_suggestion("check the pattern syntax before executing")
                    })?;
                    CompiledRule::RegexReplace {
                        regex,
                        replacement: replacement.clone(),
                    }
                }
                RenameRule::Counter { start, step, width } => CompiledRule::Counter {
                    start: *start,
                    step: *step,
                    width: *width,
                },
                RenameRule::Date { field, format } => CompiledRule::Date {
                    field: *field,
                    format: *format,
                },
                RenameRule::Case { form } => CompiledRule::Case { form: *form },
                RenameRule::Extension { new_extension } => {
                    let normalized = new_extension.trim_start_matches('.').to_string();
                    CompiledRule::Extension {
                        new_extension: normalized,
                    }
                }
                RenameRule::Template { template } => {
                    let segments = parse_template(template).map_err(|e| {
                        WeaveError::validation(
                            "rename.invalidTemplate",
                            format!(
                                "unknown or malformed template placeholder '{{{}}}'",
                                e.placeholder
                            ),
                        )
                        .with_location("weave-files::rename")
                        .with_suggestion("allowed: {name} {ext} {counter} {date} {original}")
                    })?;
                    CompiledRule::Template { segments }
                }
            })
        })
        .collect()
}

/// Base/Ext 拆分（M2 §17：photo.final.jpg → base=photo.final, ext=jpg）。
pub fn split_name(full_name: &str) -> (String, Option<String>) {
    match std::path::Path::new(full_name).extension() {
        Some(ext) => {
            let stem_len = full_name.len() - ext.len() - 1;
            (
                full_name[..stem_len].to_string(),
                Some(ext.to_string_lossy().into_owned()),
            )
        }
        None => (full_name.to_string(), None),
    }
}

fn apply_case(form: CaseForm, base: &str) -> String {
    match form {
        CaseForm::Lower => base.to_lowercase(),
        CaseForm::Upper => base.to_uppercase(),
        CaseForm::Title => {
            // 简单词首大写：每个"词"（空白后首字母）Unicode 大写，其余小写。
            let mut out = String::with_capacity(base.len());
            let mut at_word_start = true;
            for c in base.chars() {
                if c.is_whitespace() {
                    at_word_start = true;
                    out.push(c);
                } else if at_word_start {
                    out.extend(c.to_uppercase());
                    at_word_start = false;
                } else {
                    out.extend(c.to_lowercase());
                }
            }
            out
        }
    }
}

/// 把有序编译规则应用到 (base, ext) 上。counter 与 date 由上下文提供。
pub fn apply_rules(
    compiled: &[CompiledRule],
    base: &str,
    ext: Option<&str>,
    context: &RenameContext<'_>,
) -> (String, Option<String>) {
    let mut base = base.to_string();
    let mut ext = ext.map(str::to_string);
    let date_string: Option<String> = context.date.map(|t| DateFormat::YmdDash.format(*t));

    for rule in compiled {
        match rule {
            CompiledRule::Prefix { text } => base = format!("{text}{base}"),
            CompiledRule::Suffix { text } => base = format!("{base}{text}"),
            CompiledRule::Replace { find, replace_with } => {
                base = base.replace(find.as_str(), replace_with);
            }
            CompiledRule::RegexReplace { regex, replacement } => {
                base = regex.replace_all(&base, replacement.as_str()).into_owned();
            }
            CompiledRule::Counter { start, step, width } => {
                let value = start.saturating_add(context.index.saturating_mul(*step));
                base = format!("{value:0width$}", width = *width as usize);
            }
            CompiledRule::Date { .. } => {
                base = date_string.clone().unwrap_or_default();
            }
            CompiledRule::Case { form } => base = apply_case(*form, &base),
            CompiledRule::Extension { new_extension } => {
                ext = if new_extension.is_empty() {
                    None
                } else {
                    Some(new_extension.clone())
                };
            }
            CompiledRule::Template { segments } => {
                // 模板（若提供）是最后一步：组合出**完整最终名**（含扩展名），
                // 原 ext 无条件消费——"new-{counter}.txt" 与 "vacation-{counter}.{ext}"
                // 都产出单扩展名的正确结果（§17/§18）。
                let mut composed = String::new();
                for segment in segments {
                    match segment {
                        TemplateSegment::Literal(l) => composed.push_str(l),
                        TemplateSegment::Name => composed.push_str(&base),
                        TemplateSegment::Ext => {
                            if let Some(e) = &ext {
                                composed.push_str(e);
                            }
                        }
                        TemplateSegment::Counter => {
                            composed.push_str(&format!("{:03}", context.index + 1));
                        }
                        TemplateSegment::Date => {
                            composed.push_str(date_string.as_deref().unwrap_or(""));
                        }
                        TemplateSegment::Original => composed.push_str(&base),
                    }
                }
                base = composed;
                ext = None;
            }
        }
    }
    (base, ext)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, SystemTime};

    fn ctx(index: u64) -> RenameContext<'static> {
        RenameContext { index, date: None }
    }

    fn compile(rules: &[RenameRule]) -> Vec<CompiledRule> {
        compile_rules(rules).expect("valid rules")
    }

    #[test]
    fn prefix_suffix_replace_on_base_only() {
        let compiled = compile(&[
            RenameRule::Prefix {
                text: "vacation-".to_string(),
            },
            RenameRule::Suffix {
                text: "-final".to_string(),
            },
        ]);
        let (base, ext) = apply_rules(&compiled, "IMG_001", Some("jpg"), &ctx(0));
        assert_eq!(base, "vacation-IMG_001-final");
        assert_eq!(ext.as_deref(), Some("jpg"));
    }

    #[test]
    fn replace_does_not_touch_extension() {
        let compiled = compile(&[RenameRule::Replace {
            find: "jpg".to_string(),
            replace_with: "png".to_string(),
        }]);
        // base "photo" 里没有 jpg → 不变；扩展名不被误替换（§17）
        let (base, ext) = apply_rules(&compiled, "photo", Some("jpg"), &ctx(0));
        assert_eq!(base, "photo");
        assert_eq!(ext.as_deref(), Some("jpg"));
    }

    #[test]
    fn regex_replace_with_invalid_pattern_is_structured_error() {
        let err = compile_rules(&[RenameRule::RegexReplace {
            pattern: "(unclosed".to_string(),
            replacement: "x".to_string(),
        }])
        .expect_err("invalid regex");
        assert_eq!(err.code, "rename.invalidRegex");

        let compiled = compile(&[RenameRule::RegexReplace {
            pattern: "^IMG_".to_string(),
            replacement: "photo_".to_string(),
        }]);
        let (base, ext) = apply_rules(&compiled, "IMG_001", Some("jpg"), &ctx(0));
        assert_eq!(base, "photo_001");
        assert_eq!(ext.as_deref(), Some("jpg"));
    }

    #[test]
    fn counter_replaces_base_with_padded_sequence() {
        let compiled = compile(&[RenameRule::Counter {
            start: 1,
            step: 1,
            width: 3,
        }]);
        let (b0, _) = apply_rules(&compiled, "IMG_001", Some("jpg"), &ctx(0));
        let (b1, _) = apply_rules(&compiled, "IMG_002", Some("jpg"), &ctx(1));
        let (b2, _) = apply_rules(&compiled, "IMG_003", Some("jpg"), &ctx(2));
        assert_eq!(
            (b0.as_str(), b1.as_str(), b2.as_str()),
            ("001", "002", "003")
        );
    }

    #[test]
    fn counter_respects_start_step_width() {
        let compiled = compile(&[RenameRule::Counter {
            start: 10,
            step: 5,
            width: 4,
        }]);
        let (a, _) = apply_rules(&compiled, "x", Some("jpg"), &ctx(0));
        let (b, _) = apply_rules(&compiled, "x", Some("jpg"), &ctx(1));
        assert_eq!((a.as_str(), b.as_str()), ("0010", "0015"));
    }

    #[test]
    fn date_rule_formats_modified_time() {
        // 2026-10-04 00:00 UTC
        let time = SystemTime::UNIX_EPOCH + Duration::from_secs(1791542400);
        let compiled = compile(&[RenameRule::Date {
            field: DateField::Modified,
            format: DateFormat::YmdDash,
        }]);
        let ctx = RenameContext {
            index: 0,
            date: Some(&time),
        };
        let (base, _) = apply_rules(&compiled, "report", Some("pdf"), &ctx);
        // 本地时区相关：仅断言形状为 YYYY-MM-DD
        assert_eq!(base.len(), 10);
        assert_eq!(base.as_bytes()[4], b'-');
    }

    #[test]
    fn case_forms_are_unicode_aware() {
        let compiled = compile(&[RenameRule::Case {
            form: CaseForm::Upper,
        }]);
        let (base, _) = apply_rules(&compiled, "café", Some("txt"), &ctx(0));
        assert_eq!(base, "CAFÉ");

        let compiled = compile(&[RenameRule::Case {
            form: CaseForm::Title,
        }]);
        let (base, _) = apply_rules(&compiled, "hello world", Some("txt"), &ctx(0));
        assert_eq!(base, "Hello World");
    }

    #[test]
    fn extension_rule_normalizes_dot() {
        let compiled = compile(&[RenameRule::Extension {
            new_extension: ".PNG".to_string(),
        }]);
        let (base, ext) = apply_rules(&compiled, "photo", Some("jpg"), &ctx(0));
        assert_eq!(base, "photo");
        assert_eq!(ext.as_deref(), Some("PNG"));
    }

    #[test]
    fn template_composes_final_name() {
        let compiled = compile(&[RenameRule::Template {
            template: "vacation-{counter}.{ext}".to_string(),
        }]);
        // 模板含 {ext}：扩展名并入最终名，外层 ext 置空（防 .jpg.jpg，§17）
        let (base, ext) = apply_rules(&compiled, "IMG_001", Some("jpg"), &ctx(0));
        assert_eq!(base, "vacation-001.jpg");
        assert_eq!(ext, None);
    }

    #[test]
    fn template_unknown_placeholder_is_plan_error_not_silent() {
        let err = compile_rules(&[RenameRule::Template {
            template: "{unknown}".to_string(),
        }])
        .expect_err("unknown placeholder");
        assert_eq!(err.code, "rename.invalidTemplate");
        assert!(err.message.contains("unknown"));

        let err = compile_rules(&[RenameRule::Template {
            template: "{name".to_string(),
        }])
        .expect_err("unclosed placeholder");
        assert_eq!(err.code, "rename.invalidTemplate");
    }

    #[test]
    fn rules_apply_in_list_order() {
        let compiled = compile(&[
            RenameRule::Suffix {
                text: "-1".to_string(),
            },
            RenameRule::Suffix {
                text: "-2".to_string(),
            },
        ]);
        let (base, _) = apply_rules(&compiled, "a", Some("txt"), &ctx(0));
        assert_eq!(base, "a-1-2");
    }

    #[test]
    fn split_name_handles_multi_dot_and_extensionless() {
        let (base, ext) = split_name("photo.final.jpg");
        assert_eq!(base, "photo.final");
        assert_eq!(ext.as_deref(), Some("jpg"));
        let (base, ext) = split_name("noext");
        assert_eq!(base, "noext");
        assert!(ext.is_none());
    }
}
