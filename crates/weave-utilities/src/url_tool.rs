//! URL（M9 上 §59-§64/§102）：Component / Query 语义分离（§61：space =
//! %20 与 + 不混淆）、Full parse（§62 轻量）、Decode 严格（§63 lenient
//! 显式）、Unicode = UTF-8 percent（§64，确定性 §102）。

use percent_encoding::{AsciiSet, CONTROLS, percent_decode_str, utf8_percent_encode};

use crate::{UResult, UtilityError, UtilityErrorKind};

/// §60 Component：保留字母数字与 -_.~（RFC 3986 unreserved），其余全部
/// 百分号编码（含 RFC 3986 保留字符）。
const COMPONENT_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'<')
    .add(b'=')
    .add(b'>')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'^')
    .add(b'`')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'\'');

/// §61 Query（application/x-www-form-urlencoded 语义）：空格 = +。
const QUERY_SET: &AsciiSet = &CONTROLS
    .add(b' ')
    .add(b'"')
    .add(b'#')
    .add(b'$')
    .add(b'%')
    .add(b'&')
    .add(b'+')
    .add(b',')
    .add(b'/')
    .add(b':')
    .add(b';')
    .add(b'=')
    .add(b'?')
    .add(b'@')
    .add(b'[')
    .add(b'\\')
    .add(b']')
    .add(b'<')
    .add(b'>')
    .add(b'{')
    .add(b'|')
    .add(b'}')
    .add(b'\'')
    .add(b'\'');

/// §60 encode component："hello world" ⇒ "hello%20world"（UTF-8 字节，
/// §64 确定性 §102）。
pub fn url_encode_component(input: &str) -> String {
    utf8_percent_encode(input, COMPONENT_SET).to_string()
}

/// §61 encode query（x-www-form-urlencoded）：空格 = +。
pub fn url_encode_query(input: &str) -> String {
    utf8_percent_encode(input, QUERY_SET)
        .to_string()
        .replace("%20", "+")
}

/// §63 decode（strict：malformed % / 非 UTF-8 输出 = 结构化错误；
/// lenient：容错 % 序列原样保留 + 非法字节 lossy）。
pub fn url_decode_component(input: &str, lenient: bool) -> UResult<String> {
    // §63 strict：malformed % 序列（% 后不足两位 hex）先手工检出
    // （percent_decode_str 对坏序列是透传语义）
    if !lenient {
        let bytes = input.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'%' {
                let hex_ok = i + 2 < bytes.len()
                    && bytes[i + 1].is_ascii_hexdigit()
                    && bytes[i + 2].is_ascii_hexdigit();
                if !hex_ok {
                    let tail = String::from_utf8_lossy(&bytes[i + 1..]).into_owned();
                    return Err(UtilityError::invalid_input(
                        "url.invalidPercentEncoding",
                        format!("invalid escape at byte {i}: trailing '{}'", tail),
                    ));
                }
                i += 3;
            } else {
                i += 1;
            }
        }
    }
    let decoded = percent_decode_str(input);
    if lenient {
        return Ok(decoded.decode_utf8_lossy().into_owned());
    }
    decoded.decode_utf8().map(|s| s.into_owned()).map_err(|e| {
        UtilityError::invalid_input(
            "url.invalidPercentEncoding",
            format!("invalid percent-encoding or non-UTF-8 output: {e}"),
        )
    })
}

/// §61 Query decode：+ ⇒ 空格（与 encode 对称；component decode 不做此
/// 转换——两者不得混淆）。
pub fn url_decode_query(input: &str, lenient: bool) -> UResult<String> {
    url_decode_component(&input.replace('+', " "), lenient)
}

/// §62 Full URL parse（轻量；§59 不成为 HTTP Client）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UrlPart {
    pub scheme: String,
    pub userinfo: Option<String>,
    pub host: String,
    pub port: Option<u16>,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
    /// §61：query 以 k=v 对呈现（不解释语义，只拆分）。
    pub query_pairs: Vec<(String, String)>,
}

pub fn url_parse(input: &str) -> UResult<UrlPart> {
    let parsed = url::Url::parse(input.trim()).map_err(|e| {
        UtilityError::new(
            UtilityErrorKind::InvalidUrl,
            "url.invalid",
            format!("'{input}' is not a valid absolute URL: {e}"),
        )
    })?;
    let query_pairs = parsed
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    Ok(UrlPart {
        scheme: parsed.scheme().to_owned(),
        userinfo: if parsed.username().is_empty() {
            None
        } else {
            Some(parsed.username().to_owned())
        },
        host: parsed.host_str().unwrap_or_default().to_owned(),
        port: parsed.port(),
        path: parsed.path().to_owned(),
        query: parsed.query().map(|q| q.to_owned()),
        fragment: parsed.fragment().map(|f| f.to_owned()),
        query_pairs,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn component_space_is_percent20() {
        // §60："hello world" ⇒ "hello%20world"（不是 +）
        assert_eq!(url_encode_component("hello world"), "hello%20world");
    }

    #[test]
    fn query_space_is_plus_and_they_differ() {
        // §61：两种语义不得混淆
        assert_eq!(url_encode_query("hello world"), "hello+world");
        assert_ne!(
            url_encode_component("hello world"),
            url_encode_query("hello world")
        );
        // 保留字符处理
        assert_eq!(url_encode_component("a&b=c"), "a%26b%3Dc");
        assert_eq!(url_encode_query("a&b=c"), "a%26b%3Dc");
    }

    #[test]
    fn unicode_is_utf8_percent_encoded_deterministic() {
        // §64：你好 world ⇒ UTF-8 字节 percent；确定性（§102 两次一致）
        let once = url_encode_component("你好 world");
        let twice = url_encode_component("你好 world");
        assert_eq!(once, twice);
        assert_eq!(once, "%E4%BD%A0%E5%A5%BD%20world");
        // decode 对称
        assert_eq!(
            url_decode_component("%E4%BD%A0%E5%A5%BD%20world", false).expect("ok"),
            "你好 world"
        );
    }

    #[test]
    fn strict_decode_rejects_malformed_and_lenient_preserves() {
        // §63：malformed % ⇒ strict error / lenient 原样保留
        let e = url_decode_component("100%", false).expect_err("strict");
        assert_eq!(e.code, "url.invalidPercentEncoding");
        assert_eq!(url_decode_component("100%", true).expect("lenient"), "100%");
        // 非 UTF-8 输出（%FF）strict 报错
        let e = url_decode_component("%FF", false).expect_err("non-utf8");
        assert_eq!(e.code, "url.invalidPercentEncoding");
    }

    #[test]
    fn query_decode_converts_plus_but_component_does_not() {
        assert_eq!(url_decode_query("a+b", false).expect("ok"), "a b");
        assert_eq!(url_decode_component("a+b", false).expect("ok"), "a+b");
    }

    #[test]
    fn full_parse_splits_all_parts() {
        let p = url_parse("https://user@example.com:8443/p/a?k1=v1&k2=v2#frag").expect("ok");
        assert_eq!(p.scheme, "https");
        assert_eq!(p.userinfo.as_deref(), Some("user"));
        assert_eq!(p.host, "example.com");
        assert_eq!(p.port, Some(8443));
        assert_eq!(p.path, "/p/a");
        assert_eq!(p.query.as_deref(), Some("k1=v1&k2=v2"));
        assert_eq!(p.fragment.as_deref(), Some("frag"));
        assert_eq!(
            p.query_pairs,
            vec![("k1".into(), "v1".into()), ("k2".into(), "v2".into())]
        );
        let e = url_parse("not a url").expect_err("bad");
        assert_eq!(e.kind, UtilityErrorKind::InvalidUrl);
    }
}
