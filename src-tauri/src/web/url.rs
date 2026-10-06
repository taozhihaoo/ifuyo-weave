//! M2.10 Web URL 核心（§6/§7/§8/§9/§10/§38/§54）：
//! 规范化 / 校验 / 提取 / 参数分类 / 清理 / 脱敏 / 导出文件名。
//!
//! - 解析用 `url` crate（WhatWG URL 语义），不手写字符串拼接。
//! - Scheme 白名单 http/https；javascript:/file:/data:/blob:/ftp: 及未知
//!   scheme ⇒ 结构化拒绝，不静默转换（§7）。
//! - 参数分类保守（§8）：只有高置信度追踪参数允许默认删除；token/
//!   signature/session 类 = AuthRelated（保留 + 展示脱敏）；其余 = Unknown
//!   **必须保留**。Functional 类别保留给未来用户显式白名单，本版不猜。
//! - 隐私（§54）：`mask_sensitive` 供日志/历史/审计展示，绝不全量输出
//!   敏感 query 值。

use url::Url;

use super::model::{UrlCleanResult, UrlDisplay, UrlParameter, UrlParameterCategory};
use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("web::url")
}

/// 高置信度追踪参数（§8 已知清单；小写精确匹配）。
const TRACKING_PARAMS: &[&str] = &[
    "utm_source",
    "utm_medium",
    "utm_campaign",
    "utm_term",
    "utm_content",
    "utm_id",
    "utm_name",
    "fbclid",
    "gclid",
    "dclid",
    "msclkid",
    "twclid",
    "igshid",
    "li_fat_id",
    "mc_eid",
    "_hsenc",
    "_hsmi",
    "vero_id",
    "yclid",
    "wbraid",
    "gbraid",
    "spm",
    "scid",
    "s_kwcid",
];

/// 认证相关参数（§54；默认保留——误删会破坏可用性——但展示必须脱敏）。
const AUTH_PARAMS: &[&str] = &[
    "token",
    "access_token",
    "refresh_token",
    "id_token",
    "api_key",
    "apikey",
    "api-key",
    "signature",
    "sig",
    "session",
    "sessionid",
    "session_id",
    "sess",
    "secret",
    "password",
    "passwd",
    "auth",
    "authorization",
    "code",
    "key",
];

pub fn classify_parameter(name_lower: &str) -> UrlParameterCategory {
    if TRACKING_PARAMS.contains(&name_lower) {
        return UrlParameterCategory::Tracking;
    }
    if AUTH_PARAMS.contains(&name_lower) {
        return UrlParameterCategory::AuthRelated;
    }
    UrlParameterCategory::Unknown
}

/// 规范化 + 校验（§6/§7）：scheme 白名单；空 path ⇒ "/"；host 小写由
/// url crate 保证；Display URL 不用于请求（调用方区分三形态 §6）。
pub fn normalize(input: &str) -> Result<Url, WeaveError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(err("web.emptyUrl", "URL is empty"));
    }
    // 无 scheme 的输入如实拒绝（不猜测补 https://——§7 不能静默转换）
    let parsed = Url::parse(trimmed).map_err(|e| err("web.invalidUrl", format!("{e}")))?;
    match parsed.scheme() {
        "http" | "https" => {}
        other => {
            return Err(err(
                "web.unsupportedScheme",
                format!("scheme '{other}' is not supported (http/https only)"),
            ));
        }
    }
    Ok(parsed)
}

/// URL 清理（§9）：默认仅移除 Tracking；其余全保留并如实分类呈现。
pub fn clean(input: &str, remove_tracking: bool) -> Result<UrlCleanResult, WeaveError> {
    let mut url = normalize(input)?;
    let mut removed = Vec::new();
    let mut preserved = Vec::new();
    let mut unknown = Vec::new();
    let mut warnings = Vec::new();
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    if remove_tracking {
        let kept: Vec<(String, String)> = pairs
            .into_iter()
            .filter(|(k, v)| match classify_parameter(&k.to_lowercase()) {
                UrlParameterCategory::Tracking => {
                    removed.push(UrlParameter {
                        name: k.clone(),
                        value: v.clone(),
                        category: UrlParameterCategory::Tracking,
                    });
                    false
                }
                category => {
                    let param = UrlParameter {
                        name: k.clone(),
                        value: v.clone(),
                        category,
                    };
                    if param.category == UrlParameterCategory::Unknown {
                        unknown.push(param.clone());
                    }
                    preserved.push(param);
                    true
                }
            })
            .collect();
        set_query(&mut url, &kept);
        if !removed.is_empty() {
            warnings.push(format!("removed {} tracking parameter(s)", removed.len()));
        }
    } else {
        for (k, v) in pairs {
            let param = UrlParameter {
                name: k.clone(),
                value: v,
                category: classify_parameter(&k.to_lowercase()),
            };
            if param.category == UrlParameterCategory::Unknown {
                unknown.push(param.clone())
            }
            preserved.push(param);
        }
    }
    for name in preserved
        .iter()
        .filter(|p| p.category == UrlParameterCategory::AuthRelated)
        .map(|p| p.name.clone())
    {
        warnings.push(format!(
            "'{name}' looks authentication-related — kept, masked in display"
        ));
    }
    Ok(UrlCleanResult {
        original_url: input.trim().to_string(),
        normalized_url: url.to_string(),
        cleaned_url: url.to_string(),
        removed_parameters: removed,
        preserved_parameters: preserved,
        unknown_parameters: unknown,
        warnings,
    })
}

fn set_query(url: &mut Url, kept: &[(String, String)]) {
    let mut serializer = url.query_pairs_mut();
    serializer.clear();
    for (k, v) in kept {
        serializer.append_pair(k, v);
    }
}

/// 敏感 query 掩码（§54）：AuthRelated 参数值 ⇒ `***`；其余原样。
pub fn mask_sensitive(input: &str) -> String {
    let Ok(mut url) = normalize(input) else {
        // 无法解析的输入：不试图猜测——原样返回（调用方多在错误路径）
        return input.to_string();
    };
    let pairs: Vec<(String, String)> = url
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let masked: Vec<(String, String)> = pairs
        .into_iter()
        .map(|(k, v)| {
            if classify_parameter(&k.to_lowercase()) == UrlParameterCategory::AuthRelated {
                (k, "***".to_string())
            } else {
                (k, v)
            }
        })
        .collect();
    set_query(&mut url, &masked);
    url.to_string()
}

/// 展示形态（§54：原始 + 脱敏 + 敏感参数名单——UI/历史用）。
pub fn display(input: &str) -> Result<UrlDisplay, WeaveError> {
    let url = normalize(input)?;
    let sensitive: Vec<String> = url
        .query_pairs()
        .map(|(k, _)| k.into_owned())
        .filter(|k| classify_parameter(&k.to_lowercase()) == UrlParameterCategory::AuthRelated)
        .collect();
    Ok(UrlDisplay {
        original: input.trim().to_string(),
        masked: mask_sensitive(input),
        sensitive_parameter_names: sensitive,
    })
}

/// 自由文本提取 URL（§10）：保留原文与偏移，供批量与未来管道使用。
/// 返回 (url, utf-8 字节偏移)。
pub fn extract_urls(text: &str) -> Vec<(String, usize)> {
    static RE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let re = RE.get_or_init(|| {
        // http(s) 起始；终止于空白/引号/尖括号/反括号；容许常见 URL 字符
        regex::Regex::new(r#"https?://[^\s<>"'\[\]{}\\]+"#).expect("url regex")
    });
    let mut out = Vec::new();
    for m in re.find_iter(text) {
        // 去尾随标点（句子边界），保留偏移按 trimmed 结束位置
        let raw = m.as_str();
        let trimmed = raw.trim_end_matches(['.', ',', ';', ':', '!', '?', ')', '」', '』', '。']);
        if trimmed.len() < 11 {
            continue; // 短于 "http://" + host 最小形态
        }
        if normalize(trimmed).is_err() {
            continue; // 无法解析（如 https://）——如实跳过
        }
        let end = m.start() + trimmed.len();
        out.push((trimmed.to_string(), m.start()));
        let _ = end;
    }
    out
}

/// 导出文件名（§38）：host + path slug；非法字符替换；限长；保留扩展名
/// 由调用方附加。重名处理由导出层 output-first/调用方负责。
pub fn filename_from_url(input: &str, max_len: usize) -> Result<String, WeaveError> {
    let url = normalize(input)?;
    let host = url.host_str().unwrap_or("page");
    let path_slug: String = url
        .path()
        .split('/')
        .filter(|s| !s.is_empty())
        .map(|s| {
            s.chars()
                .map(|c| {
                    if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
                        c
                    } else {
                        '_'
                    }
                })
                .collect::<String>()
        })
        .filter(|s| !s.is_empty() && s != "_")
        .collect::<Vec<_>>()
        .join("_");
    let mut base = if path_slug.is_empty() {
        host.to_string()
    } else {
        format!("{host}_{path_slug}")
    };
    // Windows 保留名/非法字符兜底
    for c in ['<', '>', '"', '|', '?', '*', ':'] {
        base = base.replace(c, "_");
    }
    if base.len() > max_len {
        base = base.chars().take(max_len).collect();
    }
    Ok(base)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_accepts_http_https_and_lowercases() {
        let u = normalize("HTTPS://EXAMPLE.COM").expect("ok");
        assert_eq!(u.scheme(), "https");
        assert_eq!(u.host_str(), Some("example.com"));
        // 空路径 ⇒ "/"
        assert_eq!(normalize("https://example.com").unwrap().path(), "/");
        assert_eq!(
            normalize("  https://example.com?a=1 ").unwrap().query(),
            Some("a=1")
        );
    }

    #[test]
    fn unsupported_schemes_are_rejected_not_converted() {
        for bad in [
            "javascript:alert(1)",
            "file:///C:/x.txt",
            "data:text/html,<b>hi</b>",
            "blob:https://x/y",
            "ftp://example.com/f",
            "gopher://old",
        ] {
            let e = normalize(bad).expect_err(bad);
            assert!(e.code.starts_with("web.unsupportedScheme"), "{bad}: {e}");
        }
        assert!(normalize("").is_err());
        assert!(normalize("not a url at all").is_err());
    }

    #[test]
    fn tracking_params_removed_others_preserved() {
        let r = clean(
            "https://example.com/a?utm_source=x&id=123&token=abc&page=2",
            true,
        )
        .expect("clean");
        assert_eq!(
            r.cleaned_url,
            "https://example.com/a?id=123&token=abc&page=2"
        );
        assert_eq!(r.removed_parameters.len(), 1);
        assert_eq!(r.removed_parameters[0].name, "utm_source");
        // Unknown 必须保留
        assert!(r.unknown_parameters.iter().any(|p| p.name == "page"));
        assert!(r.unknown_parameters.iter().any(|p| p.name == "id"));
        // AuthRelated 保留但给警告
        assert!(
            r.preserved_parameters
                .iter()
                .any(|p| p.name == "token" && p.category == UrlParameterCategory::AuthRelated)
        );
        assert!(r.warnings.iter().any(|w| w.contains("token")));
    }

    #[test]
    fn clean_without_removal_classifies_only() {
        let r = clean("https://example.com/?utm_campaign=y&q=book", false).expect("clean");
        assert!(r.removed_parameters.is_empty());
        assert!(r.unknown_parameters.iter().any(|p| p.name == "q"));
    }

    #[test]
    fn sensitive_query_masked_in_display() {
        let masked = mask_sensitive("https://api.example.com/v1?token=supersecret&id=42");
        assert!(!masked.contains("supersecret"), "{masked}");
        assert!(masked.contains("token=***"));
        assert!(masked.contains("id=42"));
        let d = display("https://api.example.com/v1?token=supersecret").expect("display");
        assert_eq!(d.sensitive_parameter_names, vec!["token".to_string()]);
    }

    #[test]
    fn extract_urls_finds_and_positions() {
        let text = "这是我的网站：https://example.com/a。\n另一个：https://example.com/b?utm_source=x 结束";
        let found = extract_urls(text);
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].0, "https://example.com/a");
        assert!(text[found[1].1..].starts_with("https://"));
        // 无法解析的半截 URL 跳过
        assert_eq!(extract_urls("see https:// for details").len(), 0);
    }

    #[test]
    fn filename_from_url_is_safe_and_bounded() {
        let f = filename_from_url("https://example.com/article/123?a=b", 120).expect("name");
        assert_eq!(f, "example.com_article_123");
        let long = filename_from_url(&format!("https://example.com/{}", "x".repeat(300)), 80)
            .expect("name");
        assert!(long.chars().count() <= 80);
        // 非法字符不出现
        assert!(!f.contains('?'));
    }
}
