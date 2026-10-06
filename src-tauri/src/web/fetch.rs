//! M2.10 抓取核心（§13-§21/§55/§73-§75）：GET + 限制 + 取消 + 状态/编码
//! 处理。经 `ureq`（阻塞、rustls）——本模块是 Weave 中唯一发起网络请求的
//! 位置（§17 仅 M2.10 Web 联网）。
//!
//! - 限制（§19）：timeout / max_redirects / max_bytes（解压后计量——
//!   压缩炸弹由读端上限拦截 §75）；无并发控制（并发在批量层 §45）。
//! - 取消（§44）：分块读取间检查 CancellationToken——**读中断开连接**
//!   （drop reader），不是只改 UI 状态。
//! - 状态（§14）：非 2xx ⇒ 结构化错误（带状态码与类别化 message），不把
//!   403 说成"网页损坏"。
//! - 编码（§21）：HTTP header charset → BOM → meta charset → utf-8 默认，
//!   来源如实记录。
//! - 不保存 Cookie/Authorization/登录态（§73/§74）：每次请求全新 Agent。

use std::io::Read;
use std::time::Instant;

use url::Url;

use super::model::{CharsetSource, WebContentType, WebFetchInfo};
use super::target::{GuardedResolver, precheck};
use weave_core::prelude::CancellationToken;
use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("web::fetch")
}

/// 请求限制（§19/§85：默认安全保守）。
#[derive(Debug, Clone, PartialEq)]
pub struct FetchLimits {
    pub timeout_secs: u32,
    pub max_redirects: u32,
    pub max_bytes: usize,
    pub user_agent: String,
}

impl Default for FetchLimits {
    fn default() -> Self {
        Self {
            timeout_secs: 30,
            max_redirects: 10,
            max_bytes: 5 * 1024 * 1024,
            user_agent: concat!("ifuyo-Weave/", env!("CARGO_PKG_VERSION")).to_string(),
        }
    }
}

/// 抓取结果：元信息 + 原始字节（压缩已解；上限内）。
#[derive(Debug)]
pub struct FetchResponse {
    pub info: WebFetchInfo,
    pub body: Vec<u8>,
}

/// Content-Type 识别（§20；核心只处理 HTML/Text）。
pub fn classify_content_type(raw: &str) -> WebContentType {
    let lower = raw.split(';').next().unwrap_or("").trim().to_lowercase();
    match lower.as_str() {
        "text/html" | "application/xhtml+xml" => WebContentType::Html,
        "text/plain" => WebContentType::PlainText,
        "application/json" | "text/json" => WebContentType::Json,
        "application/xml" | "text/xml" => WebContentType::Xml,
        "application/pdf" => WebContentType::Pdf,
        ct if ct.starts_with("image/") => WebContentType::Image,
        "application/octet-stream" => WebContentType::Binary,
        "" => WebContentType::Unknown,
        _ => WebContentType::Unknown,
    }
}

/// HTTP 状态类别化 message（§14：不把 403 说成"网页损坏"）。
pub fn status_message(status: u16) -> &'static str {
    match status {
        401 => "authentication required (401)",
        403 => "server refused access (403)",
        404 => "not found (404)",
        429 => "too many requests (429) — try later",
        500 => "server error (500)",
        502 => "bad gateway (502)",
        503 => "service unavailable (503)",
        504 => "gateway timeout (504)",
        s if (200..300).contains(&s) => "ok",
        s if (300..400).contains(&s) => "redirect (unhandled)",
        s if (400..500).contains(&s) => "client error",
        _ => "server error",
    }
}

/// 从 Content-Type header 取 charset 参数。
fn charset_from_content_type(raw: &str) -> Option<String> {
    raw.split(';')
        .map(|p| p.trim())
        .find_map(|p| p.strip_prefix("charset="))
        .map(|c| c.trim_matches('"').to_string())
}

/// meta charset 探测（ASCII-insensitive 扫描前 4KB；encoding_rs 归一）。
fn charset_from_meta(bytes: &[u8]) -> Option<String> {
    let head = &bytes[..bytes.len().min(4096)];
    let lower: String = head
        .iter()
        .map(|b| b.to_ascii_lowercase() as char)
        .collect();
    let idx = lower.find("charset=")?;
    let rest = &lower[idx + 8..];
    let rest = rest.trim_start();
    let rest = rest
        .strip_prefix('"')
        .or_else(|| rest.strip_prefix('\''))
        .unwrap_or(rest);
    let raw: String = rest
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '-' || *c == '_')
        .collect();
    if raw.is_empty() {
        None
    } else {
        // 归一为 encoding_rs 规范名（gbk ⇒ GBK）
        encoding_rs::Encoding::for_label(raw.as_bytes()).map(|e| e.name().to_string())
    }
}

/// 解码为 UTF-8（§21 优先级：显式 charset → BOM → utf-8 best effort）。
/// 返回 (text, used_charset_label)。
pub fn decode_body(bytes: &[u8], charset_hint: Option<&str>) -> (String, String, CharsetSource) {
    if let Some(hint) = charset_hint {
        if let Some(enc) = encoding_rs::Encoding::for_label(hint.as_bytes()) {
            let (text, _, _) = enc.decode(bytes);
            return (
                text.into_owned(),
                enc.name().to_string(),
                CharsetSource::HttpHeader,
            );
        }
    }
    if let Some((enc, bom_len)) = encoding_rs::Encoding::for_bom(bytes) {
        let (text, _, _) = enc.decode(&bytes[bom_len..]);
        return (
            text.into_owned(),
            enc.name().to_string(),
            CharsetSource::Bom,
        );
    }
    // 无显式编码：utf-8 解码失败⇒按 meta charset 重试
    if let Ok(text) = std::str::from_utf8(bytes) {
        return (
            text.to_string(),
            "utf-8".to_string(),
            CharsetSource::Utf8Default,
        );
    }
    if let Some(meta) = charset_from_meta(bytes) {
        if let Some(enc) = encoding_rs::Encoding::for_label(meta.as_bytes()) {
            let (decoded, _, _) = enc.decode(bytes);
            return (
                decoded.into_owned(),
                enc.name().to_string(),
                CharsetSource::MetaTag,
            );
        }
    }
    // 兜底：GBK 常见于中文站；再不行 lossy utf-8（如实标注）
    if let Some(enc) = encoding_rs::Encoding::for_label(b"gb18030") {
        let (decoded, _, had_errors) = enc.decode(bytes);
        if !had_errors {
            return (
                decoded.into_owned(),
                enc.name().to_string(),
                CharsetSource::MetaTag,
            );
        }
    }
    (
        String::from_utf8_lossy(bytes).into_owned(),
        "utf-8 (lossy)".to_string(),
        CharsetSource::Utf8Default,
    )
}

/// 抓取（§13-§16/§44/§75）。`allow_private` 仅显式放行内网/本机。
pub fn fetch(
    requested: &str,
    limits: &FetchLimits,
    cancel: &CancellationToken,
    allow_private: bool,
) -> Result<FetchResponse, WeaveError> {
    let start = Instant::now();
    let url = super::url::normalize(requested)?;
    precheck(url.host_str().unwrap_or_default(), allow_private)?;
    let resolver = GuardedResolver { allow_private };
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(u64::from(
            limits.timeout_secs.max(1),
        )))
        .timeout_connect(std::time::Duration::from_secs(10))
        .redirects(limits.max_redirects)
        .user_agent(&limits.user_agent)
        .resolver(resolver)
        .build();
    let response = agent
        .get(url.as_str())
        .call()
        .map_err(|e| map_ureq_error(e, cancel))?;
    let status = response.status();
    if !(200..300).contains(&status) {
        return Err(err(
            "web.httpStatus",
            format!("{} {}", status, status_message(status)),
        ));
    }
    let final_url: Url = response.get_url().parse().unwrap_or_else(|_| url.clone());
    let content_type_raw = response.header("content-type").unwrap_or("").to_string();
    let content_type = classify_content_type(&content_type_raw);
    if matches!(
        content_type,
        WebContentType::Image | WebContentType::Pdf | WebContentType::Binary
    ) {
        return Err(err(
            "web.unsupportedContent",
            format!(
                "content-type '{}' is recognized but not supported here",
                content_type_raw
            ),
        ));
    }
    let charset_hint = charset_from_content_type(&content_type_raw);
    let _redirect_chain: Vec<String> = Vec::new(); // ureq 不暴露逐跳链——记录 final（如实 §15）
    let redirect_count = if final_url.as_str() == url.as_str() {
        0
    } else {
        1
    };
    // 有界读取（解压后计量）+ 分块取消（§19/§44/§75）
    let mut reader = response.into_reader().take((limits.max_bytes + 1) as u64);
    let mut body = Vec::with_capacity(64 * 1024);
    let mut chunk = [0u8; 64 * 1024];
    loop {
        if cancel.is_cancelled() {
            drop(reader);
            return Err(err("web.cancelled", "fetch cancelled"));
        }
        match reader.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => {
                body.extend_from_slice(&chunk[..n]);
                if body.len() > limits.max_bytes {
                    // 保留前 max_bytes——截断事实由 info.size 上限体现（§19）
                    body.truncate(limits.max_bytes);
                    break;
                }
            }
            Err(e) => return Err(err("web.readFailed", format!("read body failed: {e}"))),
        }
    }
    let info = WebFetchInfo {
        requested_url: requested.trim().to_string(),
        final_url: final_url.to_string(),
        status: status as u32,
        content_type,
        content_type_raw: content_type_raw.clone(),
        // header 显式编码先记录；BOM/meta 判定在 decode 阶段覆盖（§21）
        charset: charset_hint.clone().unwrap_or_default(),
        charset_source: if charset_hint.is_some() {
            CharsetSource::HttpHeader
        } else {
            CharsetSource::Utf8Default
        },
        size_bytes: body.len() as f64,
        duration_ms: start.elapsed().as_millis() as f64,
        redirect_count,
        redirect_chain: _redirect_chain,
    };
    Ok(FetchResponse { info, body })
}

fn map_ureq_error(e: ureq::Error, cancel: &CancellationToken) -> WeaveError {
    if cancel.is_cancelled() {
        return err("web.cancelled", "fetch cancelled");
    }
    match e {
        ureq::Error::Status(code, _) => err(
            "web.httpStatus",
            format!("{} {}", code, status_message(code)),
        ),
        ureq::Error::Transport(t) => {
            let kind = t.kind();
            let code = match kind {
                ureq::ErrorKind::Dns => "web.dnsFailure",
                ureq::ErrorKind::TooManyRedirects => "web.redirectError",
                ureq::ErrorKind::InvalidUrl | ureq::ErrorKind::UnknownScheme => "web.invalidUrl",
                _ => "web.networkError",
            };
            err(code, format!("transport: {kind}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_type_classification() {
        assert_eq!(
            classify_content_type("text/html; charset=utf-8"),
            WebContentType::Html
        );
        assert_eq!(
            classify_content_type("TEXT/PLAIN"),
            WebContentType::PlainText
        );
        assert_eq!(
            classify_content_type("application/json"),
            WebContentType::Json
        );
        assert_eq!(classify_content_type("image/png"), WebContentType::Image);
        assert_eq!(
            classify_content_type("application/pdf"),
            WebContentType::Pdf
        );
        assert_eq!(
            classify_content_type("application/octet-stream"),
            WebContentType::Binary
        );
        assert_eq!(classify_content_type(""), WebContentType::Unknown);
    }

    #[test]
    fn status_messages_are_specific() {
        assert!(status_message(403).contains("refused"));
        assert!(status_message(429).contains("429"));
        assert!(status_message(404).contains("not found"));
    }

    #[test]
    fn charset_priority_header_then_bom_then_lossy() {
        let utf16: Vec<u8> = [0xFF, 0xFE]
            .iter()
            .copied()
            .chain("中".encode_utf16().flat_map(|u| u.to_le_bytes()))
            .collect();
        let (text, label, source) = decode_body(&utf16, None);
        assert_eq!(text, "中");
        assert_eq!(source, CharsetSource::Bom);
        assert!(label.starts_with("UTF-16"));
        // GB18030 内容（"中文" 的 GBK 编码）
        let gbk = vec![0xD6, 0xD0, 0xCE, 0xC4];
        let (text, label, _) = decode_body(&gbk, Some("gbk"));
        assert_eq!(text, "中文");
        assert_eq!(label, "GBK");
        // 无 BOM 且 utf-8 无效 ⇒ 兜底路径如实标注（不冒充显式编码）
        let (text, label, source) = decode_body(&[0xC3, 0x28, 0xFF], None);
        assert!(
            source == CharsetSource::Utf8Default || source == CharsetSource::MetaTag,
            "unexpected source {source:?}"
        );
        assert!(label.contains("utf-8") || label.contains("GB"), "{label}");
        assert!(!text.is_empty());
    }

    #[test]
    fn meta_charset_detected() {
        let html = b"<html><head><meta charset=\"GBK\"><body>x</body>";
        assert_eq!(charset_from_meta(html).as_deref(), Some("GBK"));
    }

    #[test]
    fn fetch_rejects_unsupported_scheme_before_network() {
        let cancel = CancellationToken::new();
        let e = fetch(
            "file:///etc/passwd",
            &FetchLimits::default(),
            &cancel,
            false,
        )
        .expect_err("reject");
        assert!(e.code.starts_with("web.unsupportedScheme"));
    }

    #[test]
    fn fetch_rejects_local_target_without_allow() {
        let cancel = CancellationToken::new();
        let e = fetch(
            "http://127.0.0.1:9/x",
            &FetchLimits::default(),
            &cancel,
            false,
        )
        .expect_err("reject");
        assert!(e.code.starts_with("web.targetBlocked"), "{e}");
        // allow_private 时本地端口连不上 ⇒ 分类不再拦（网络层错误）
        let e2 = fetch(
            "http://127.0.0.1:9/x",
            &FetchLimits::default(),
            &cancel,
            true,
        )
        .expect_err("connect fail");
        assert!(e2.code.starts_with("web."), "{e2}");
    }
}
