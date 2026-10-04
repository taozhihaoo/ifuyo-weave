//! Regex Tester（M9 上 §65-§74）：唯一 engine = `regex` crate（与 M4
//! FindReplace 同源，§66 preview/execute 同语义）；能力矩阵如实（§67-§68：
//! lookaround/backref = Unsupported）；结果含 full/start/end/line/column/
//! groups/named（§69）；资源有界（§72：pattern/input/matches 上限；
//! regex crate 线性时间引擎 ⇒ §73 无灾难性回溯）。

use regex::RegexBuilder;

use crate::{UResult, UtilityError, UtilityErrorKind};

/// §67 能力矩阵（按 regex crate 真实能力，不模拟 §68）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexCapability {
    pub feature: &'static str,
    /// supported | unsupported
    pub status: &'static str,
    pub note: &'static str,
}

pub fn regex_capability_matrix() -> Vec<RegexCapability> {
    vec![
        RegexCapability {
            feature: "lookahead",
            status: "unsupported",
            note: "regex crate does not support lookahead (?=)",
        },
        RegexCapability {
            feature: "lookbehind",
            status: "unsupported",
            note: "regex crate does not support lookbehind (?<=)",
        },
        RegexCapability {
            feature: "backreferences",
            status: "unsupported",
            note: "\\1-style backreferences unsupported",
        },
        RegexCapability {
            feature: "named groups",
            status: "supported",
            note: "(?P<name>...)",
        },
        RegexCapability {
            feature: "unicode",
            status: "supported",
            note: "default on",
        },
        RegexCapability {
            feature: "case insensitive",
            status: "supported",
            note: "flag i",
        },
        RegexCapability {
            feature: "multiline",
            status: "supported",
            note: "flag m (^$ match line boundaries)",
        },
        RegexCapability {
            feature: "dot matches newline",
            status: "supported",
            note: "flag s",
        },
        RegexCapability {
            feature: "extended mode",
            status: "supported",
            note: "flag x (ignore whitespace)",
        },
        RegexCapability {
            feature: "linear time guarantee",
            status: "supported",
            note: "no catastrophic backtracking by construction (§73)",
        },
    ]
}

/// §70 flags（i/m/s/x —— regex crate 真实支持集）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexFlag {
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_newline: bool,
    pub ignore_whitespace: bool,
}

/// §72 资源上限。
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexLimits {
    pub max_pattern_len: usize,
    pub max_input_len: usize,
    pub max_matches: usize,
}

impl Default for RegexLimits {
    fn default() -> Self {
        Self {
            max_pattern_len: 4 * 1024,
            max_input_len: 4 * 1024 * 1024,
            max_matches: 10_000,
        }
    }
}

/// §69 匹配信息（含行/列，1-based；source order §103）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RegexMatchInfo {
    pub full: String,
    /// 字节偏移（输入为 UTF-8；同时给字符偏移供 UI 行/列使用）。
    pub start: f64,
    pub end: f64,
    pub line: f64,
    pub column: f64,
    /// 未命名捕获组（按序；未参与 = None）。
    pub groups: Vec<Option<String>>,
    /// 命名捕获组。
    pub named_groups: Vec<(String, String)>,
}

/// 查找（§65：Pattern + Input + Flags → Matches；§103 确定性：source
/// order；§72 有界；regex crate 线性时间 ⇒ §73 结构性安全）。
pub fn regex_find(
    pattern: &str,
    input: &str,
    flags: &RegexFlag,
    limits: &RegexLimits,
) -> UResult<Vec<RegexMatchInfo>> {
    if pattern.len() > limits.max_pattern_len {
        return Err(UtilityError::new(
            UtilityErrorKind::ResourceLimitExceeded,
            "regex.patternTooLong",
            format!(
                "pattern length {} over limit {}",
                pattern.len(),
                limits.max_pattern_len
            ),
        ));
    }
    if input.len() > limits.max_input_len {
        return Err(UtilityError::new(
            UtilityErrorKind::ResourceLimitExceeded,
            "regex.inputTooLong",
            format!(
                "input length {} over limit {}",
                input.len(),
                limits.max_input_len
            ),
        ));
    }
    let re = RegexBuilder::new(pattern)
        .case_insensitive(flags.case_insensitive)
        .multi_line(flags.multi_line)
        .dot_matches_new_line(flags.dot_matches_newline)
        .ignore_whitespace(flags.ignore_whitespace)
        .build()
        .map_err(|e| {
            UtilityError::new(
                UtilityErrorKind::InvalidPattern,
                "regex.invalidPattern",
                format!("invalid pattern: {e}"),
            )
        })?;
    let mut out = Vec::new();
    for caps in re.captures_iter(input) {
        let whole = caps.get(0).expect("whole match");
        let (line, column) = line_col(input, whole.start());
        let groups: Vec<Option<String>> = (1..caps.len())
            .map(|gi| caps.get(gi).map(|g| g.as_str().to_owned()))
            .collect();
        let named_groups: Vec<(String, String)> = re
            .capture_names()
            .flatten()
            .filter_map(|name| {
                caps.name(name)
                    .map(|g| (name.to_owned(), g.as_str().to_owned()))
            })
            .collect();
        out.push(RegexMatchInfo {
            full: whole.as_str().to_owned(),
            start: whole.start() as f64,
            end: whole.end() as f64,
            line: line as f64,
            column: column as f64,
            groups,
            named_groups,
        });
        if out.len() >= limits.max_matches {
            break; // §72/§97 有界（截断如实——不静默，见 warnings 由调用层提供）
        }
    }
    Ok(out)
}

/// 行/列（1-based；列 = 字符数，UTF-8 安全）。
fn line_col(input: &str, byte_pos: usize) -> (usize, usize) {
    let prefix = &input[..byte_pos];
    let line = prefix.matches('\n').count() + 1;
    let line_start = prefix.rfind('\n').map(|i| i + 1).unwrap_or(0);
    let column = input[line_start..byte_pos].chars().count() + 1;
    (line, column)
}

/// §71 Replace：**复用 M4 语义**——regex crate replace_all 与 M4
/// TransformKind::FindReplace 同 engine；此处提供预览（结果 + 替换计数）。
pub fn regex_replace(
    pattern: &str,
    input: &str,
    replacement: &str,
    flags: &RegexFlag,
    limits: &RegexLimits,
) -> UResult<(String, f64)> {
    let re = RegexBuilder::new(pattern)
        .case_insensitive(flags.case_insensitive)
        .multi_line(flags.multi_line)
        .dot_matches_new_line(flags.dot_matches_newline)
        .ignore_whitespace(flags.ignore_whitespace)
        .build()
        .map_err(|e| {
            UtilityError::new(
                UtilityErrorKind::InvalidPattern,
                "regex.invalidPattern",
                format!("invalid pattern: {e}"),
            )
        })?;
    if input.len() > limits.max_input_len {
        return Err(UtilityError::new(
            UtilityErrorKind::ResourceLimitExceeded,
            "regex.inputTooLong",
            format!(
                "input length {} over limit {}",
                input.len(),
                limits.max_input_len
            ),
        ));
    }
    let mut count = 0f64;
    let output = re.replace_all(input, |_: &regex::Captures| {
        count += 1.0;
        replacement
    });
    Ok((output.into_owned(), count))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FLAGS: RegexFlag = RegexFlag {
        case_insensitive: false,
        multi_line: false,
        dot_matches_newline: false,
        ignore_whitespace: false,
    };

    #[test]
    fn finds_matches_with_positions_and_groups() {
        let matches = regex_find(
            r"(\w+)=([0-9]+)",
            "alpha=1\nbeta=22",
            &FLAGS,
            &RegexLimits::default(),
        )
        .expect("ok");
        assert_eq!(matches.len(), 2);
        assert_eq!(matches[0].full, "alpha=1");
        assert_eq!(matches[0].line, 1.0);
        assert_eq!(
            matches[0].groups,
            vec![Some("alpha".into()), Some("1".into())]
        );
        // §103 source order（第二匹配行 2）
        assert_eq!(matches[1].line, 2.0);
        assert_eq!(matches[1].column, 1.0);
    }

    #[test]
    fn named_groups_and_flags() {
        let flags = RegexFlag {
            case_insensitive: true,
            ..FLAGS
        };
        let matches = regex_find(
            r"(?P<word>hello)",
            "Hello HELLO hello",
            &flags,
            &RegexLimits::default(),
        )
        .expect("ok");
        assert_eq!(matches.len(), 3);
        assert_eq!(
            matches[0].named_groups,
            vec![("word".into(), "Hello".into())]
        );
        // §67：能力矩阵如实——lookaround 编译失败 = 结构化错误（不模拟）
        let e = regex_find(r"foo(?=bar)", "foobar", &FLAGS, &RegexLimits::default())
            .expect_err("lookahead unsupported");
        assert_eq!(e.kind, UtilityErrorKind::InvalidPattern);
    }

    #[test]
    fn catastrophic_backtracking_is_structurally_safe() {
        // §73：((a+)+)+b 对 a*30（经典指数回溯 pattern）——线性引擎
        // 恒定时间内完成且确定性无匹配
        let evil = "a".repeat(30);
        let matches = regex_find(r"((a+)+)+b", &evil, &FLAGS, &RegexLimits::default())
            .expect("no catastrophic backtracking");
        assert!(matches.is_empty());
    }

    #[test]
    fn limits_are_enforced() {
        let tight = RegexLimits {
            max_pattern_len: 4,
            ..RegexLimits::default()
        };
        let e = regex_find(r"abcdef", "abc", &FLAGS, &tight).expect_err("pattern too long");
        assert_eq!(e.kind, UtilityErrorKind::ResourceLimitExceeded);
    }

    #[test]
    fn replace_preview_counts() {
        let (out, count) =
            regex_replace(r"\d+", "a1 b22 c333", "N", &FLAGS, &RegexLimits::default()).expect("ok");
        assert_eq!(out, "aN bN cN");
        assert_eq!(count, 3.0);
    }

    #[test]
    fn determinism_same_input_same_matches() {
        // §103
        let a = regex_find(r"\d+", "x1 y22 z333", &FLAGS, &RegexLimits::default()).expect("ok");
        let b = regex_find(r"\d+", "x1 y22 z333", &FLAGS, &RegexLimits::default()).expect("ok");
        assert_eq!(a, b);
    }
}
