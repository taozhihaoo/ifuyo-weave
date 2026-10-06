//! M2.10 HTML 解析与提取（§22-§29/§76/§77）：html5ever（scraper crate）
//! 容错解析——**不使用正则当 parser（§23）**；节点遍历迭代无递归
//! （§79）。
//!
//! - Plain Text：文本节点按祖先链过滤 script/style/noscript/template
//!   （§22/§77 不无差别删 hidden——交由正文启发式并保留证据）；块级祖先
//!   断行；空白归一；实体由 html5ever 解码（§76）。
//! - Links：a[href] 经 base URL `join` 解析（§12 不字符串拼接）；
//!   LinkType 不可靠 ⇒ Unknown（§11）；上限 5000（§91 资源上限）。
//! - Metadata：title/og/canonical/lang/author/published——缺失即缺席
//!   （§28），每字段带证据来源（§67）。
//! - Main Content：可解释启发式（§26/§27）——语义容器（article/main/
//!   [role=main]）⇒ High；否则文本密度打分 ⇒ Medium；body 兜底 ⇒ Low；
//!   永远返回结果 + 置信度 + 证据，UI 呈现"推断"。

use scraper::{ElementRef, Html, Node, Selector};

use super::model::{ExtractionConfidence, MetaField, PageMetadata, WebLink, WebLinkType};
use super::url;
use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("web::html")
}

const MAX_LINKS: usize = 5000;
const NONCONTENT: [&str; 4] = ["script", "style", "noscript", "template"];

/// 解析（恶意/破损 HTML ⇒ html5ever 容错；不 panic §78）。
pub fn parse(html: &str) -> Html {
    Html::parse_document(html)
}

/// 块级元素（断行语义——标题/段落/列表/表格/引用 §22）。
fn is_block(name: &str) -> bool {
    matches!(
        name,
        "p" | "div"
            | "section"
            | "article"
            | "header"
            | "footer"
            | "main"
            | "aside"
            | "nav"
            | "h1"
            | "h2"
            | "h3"
            | "h4"
            | "h5"
            | "h6"
            | "li"
            | "tr"
            | "blockquote"
            | "pre"
            | "figure"
            | "figcaption"
            | "address"
            | "dt"
            | "dd"
            | "table"
            | "ul"
            | "ol"
            | "thead"
            | "tbody"
            | "hr"
            | "form"
            | "fieldset"
    )
}

/// Plain Text（§22）：文本节点直出（祖先链过滤非内容元素）；块级祖先
/// 断行；空白归一。祖先链上行是迭代（无递归 §79）。
pub fn plain_text(dom: &Html) -> String {
    let mut out = String::with_capacity(64 * 1024);
    for node in dom.tree.nodes() {
        let Node::Text(text) = node.value() else {
            continue;
        };
        let mut in_noncontent = false;
        let mut nearest_block = false;
        let mut anc = node.parent();
        while let Some(a) = anc {
            if let Some(el) = ElementRef::wrap(a) {
                let name = el.value().name();
                if NONCONTENT.contains(&name) {
                    in_noncontent = true;
                    break;
                }
                if !nearest_block && is_block(name) {
                    nearest_block = true;
                }
            }
            anc = a.parent();
        }
        if in_noncontent {
            continue;
        }
        out.push_str(text);
        if nearest_block {
            out.push('\n');
        }
    }
    normalize_whitespace(&out)
}

/// 空白归一：行内空白折叠为单空格；空行折叠为单空行（§22 空白处理）。
fn normalize_whitespace(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut blank_pending = false;
    let mut wrote_any = false;
    for raw_line in text.lines() {
        let line = raw_line.split_whitespace().collect::<Vec<_>>().join(" ");
        if line.is_empty() {
            if wrote_any {
                blank_pending = true;
            }
        } else {
            if blank_pending {
                out.push('\n');
                blank_pending = false;
            } else if wrote_any {
                out.push('\n');
            }
            wrote_any = true;
            out.push_str(&line);
        }
    }
    out
}

/// 相对 URL 解析（§12：URI join 语义，不字符串拼接）。
pub fn resolve_relative(base: Option<&str>, href: &str) -> Option<String> {
    let base = base?;
    let base_url = url::normalize(base).ok()?;
    if href.starts_with('#') {
        return Some(format!("{base_url}{href}"));
    }
    base_url.join(href).ok().map(|u| u.to_string())
}

fn link_origin(url_str: &str) -> String {
    url::normalize(url_str)
        .map(|u| {
            format!(
                "{}://{}{}",
                u.scheme(),
                u.host_str().unwrap_or(""),
                u.port().map(|p| format!(":{p}")).unwrap_or_default()
            )
        })
        .unwrap_or_default()
}

/// 链接提取（§11/§12/§55）：a[href]；原文与解析结果分离保留。
pub fn extract_links(dom: &Html, base: Option<&str>) -> Vec<WebLink> {
    let Ok(a_selector) = Selector::parse("a[href]") else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for el in dom.select(&a_selector) {
        if out.len() >= MAX_LINKS {
            break;
        }
        let raw_href = el.value().attr("href").unwrap_or("").trim().to_string();
        if raw_href.is_empty() {
            continue;
        }
        let display_text = el.text().collect::<String>();
        let display_text = display_text
            .split_whitespace()
            .collect::<Vec<_>>()
            .join(" ");
        let scheme = raw_href.split(':').next().unwrap_or("");
        let link_type = match scheme {
            "mailto" => WebLinkType::Mail,
            "tel" => WebLinkType::Tel,
            "javascript" | "data" | "blob" | "file" => WebLinkType::Unknown,
            _ if raw_href.starts_with('#') => WebLinkType::Anchor,
            _ => {
                let is_http_ref =
                    raw_href.starts_with("http://") || raw_href.starts_with("https://");
                if !is_http_ref && resolve_relative(base, &raw_href).is_some() {
                    WebLinkType::Relative
                } else {
                    let absolute = resolve_relative(base, &raw_href);
                    match (absolute.as_deref(), base) {
                        (Some(abs), Some(b))
                            if link_origin(abs) == link_origin(b)
                                && !link_origin(abs).is_empty() =>
                        {
                            WebLinkType::Internal
                        }
                        (Some(_), Some(_)) => WebLinkType::External,
                        _ => WebLinkType::Unknown,
                    }
                }
            }
        };
        let absolute_url = resolve_relative(base, &raw_href).unwrap_or_else(|| raw_href.clone());
        out.push(WebLink {
            absolute_url,
            display_text,
            link_type,
            raw_href,
        });
    }
    out
}

/// 页面元数据（§28/§67：缺失即缺席 + 证据来源）。
pub fn extract_metadata(dom: &Html) -> PageMetadata {
    let meta_content = |selector: &str, source: &str| -> Option<MetaField> {
        Selector::parse(selector).ok().and_then(|sel| {
            dom.select(&sel)
                .next()
                .and_then(|el| el.value().attr("content"))
                .map(|c| c.trim().to_string())
                .filter(|c| !c.is_empty())
                .map(|value| MetaField {
                    value,
                    source: source.to_string(),
                })
        })
    };
    let title = Selector::parse("title").ok().and_then(|sel| {
        dom.select(&sel)
            .next()
            .map(|el| el.text().collect::<String>().trim().to_string())
            .filter(|t| !t.is_empty())
            .map(|v| MetaField {
                value: v,
                source: "title-tag".to_string(),
            })
    });
    let language = Selector::parse("html").ok().and_then(|sel| {
        dom.select(&sel)
            .next()
            .and_then(|el| el.value().attr("lang"))
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .map(|value| MetaField {
                value,
                source: "html-lang".to_string(),
            })
    });
    let canonical_url = Selector::parse("link[rel=canonical]").ok().and_then(|sel| {
        dom.select(&sel)
            .next()
            .and_then(|el| el.value().attr("href"))
            .map(|h| h.trim().to_string())
            .filter(|h| !h.is_empty())
            .map(|value| MetaField {
                value,
                source: "canonical".to_string(),
            })
    });
    PageMetadata {
        title,
        description: meta_content("meta[name=description]", "meta[name=description]"),
        author: meta_content("meta[name=author]", "meta[name=author]"),
        published_time: meta_content(
            "meta[property=article\\:published_time]",
            "article:published_time",
        )
        .or_else(|| meta_content("meta[name=date]", "meta[name=date]")),
        modified_time: meta_content(
            "meta[property=article\\:modified_time]",
            "article:modified_time",
        ),
        language,
        canonical_url,
        og_title: meta_content("meta[property=og\\:title]", "og:title"),
        og_description: meta_content("meta[property=og\\:description]", "og:description"),
        og_image: meta_content("meta[property=og\\:image]", "og:image"),
    }
}

/// 正文提取（§26/§27 可解释启发式）。
/// 返回 (正文片段 HTML, 置信度, 证据来源)。
pub fn extract_main_content(dom: &Html) -> (String, ExtractionConfidence, String) {
    // 1) 语义容器 ⇒ High（§26 Confirmed 来源）
    for (sel, source) in [
        ("article", "article"),
        ("main", "main"),
        ("[role=main]", "[role=main]"),
    ] {
        if let Ok(selector) = Selector::parse(sel) {
            if let Some(el) = dom.select(&selector).next() {
                return (
                    el.inner_html(),
                    ExtractionConfidence::High,
                    source.to_string(),
                );
            }
        }
    }
    // 2) 文本密度启发式（readability-lite）：score = text_len × (1-link_density)
    let mut best: Option<(f64, String)> = None;
    if let Ok(link_sel) = Selector::parse("a") {
        for name in ["div", "section"] {
            let Ok(selector) = Selector::parse(name) else {
                continue;
            };
            for el in dom.select(&selector) {
                let text_len: usize = el.text().map(|t| t.trim().len()).sum();
                if text_len < 200 {
                    continue;
                }
                let link_len: usize = el
                    .select(&link_sel)
                    .map(|a| a.text().map(|t| t.trim().len()).sum::<usize>())
                    .sum();
                let density = link_len as f64 / text_len as f64;
                if density > 0.5 {
                    continue; // 链接主导（导航/目录）——非正文
                }
                let score = text_len as f64 * (1.0 - density);
                if best.as_ref().is_none_or(|(b, _)| score > *b) {
                    best = Some((score, el.inner_html()));
                }
            }
        }
    }
    if let Some((_, fragment)) = best {
        return (
            fragment,
            ExtractionConfidence::Medium,
            "heuristic:density".to_string(),
        );
    }
    // 3) 兜底：整个 body（Low——如实）
    let fallback = Selector::parse("body")
        .ok()
        .and_then(|sel| dom.select(&sel).next());
    match fallback {
        Some(el) => (
            el.inner_html(),
            ExtractionConfidence::Low,
            "heuristic:body-fallback".to_string(),
        ),
        None => (
            String::new(),
            ExtractionConfidence::Unknown,
            "heuristic:none".to_string(),
        ),
    }
}

/// 从 HTML 片段生成 Plain Text（正文片段复用同一 text 管线 §63/§65）。
pub fn fragment_to_text(fragment_html: &str) -> String {
    plain_text(&Html::parse_fragment(fragment_html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_text_strips_scripts_and_keeps_structure() {
        let html = r#"<html><head><style>body{color:red}</style></head>
            <body><h1>Hello</h1><script>alert("x")</script><p>World</p>
            <ul><li>one</li><li>two</li></ul>
            <table><tr><td>cell</td></tr></table>
            <blockquote>quoted</blockquote>
            <a href="/x">link</a></body></html>"#;
        let text = plain_text(&parse(html));
        assert!(text.contains("Hello"));
        assert!(text.contains("World"));
        assert!(text.contains("one"));
        assert!(text.contains("cell"));
        assert!(text.contains("quoted"));
        assert!(text.contains("link"));
        assert!(!text.contains("alert"), "script 内容不得混入正文: {text}");
        assert!(!text.contains("color:red"));
        // 每个文本只出现一次（无祖先重复展开）
        assert_eq!(text.matches("Hello").count(), 1);
        assert_eq!(text.matches("link").count(), 1);
        // 标题/段落分行
        assert!(text.lines().any(|l| l == "Hello"));
    }

    #[test]
    fn entities_are_decoded() {
        let text = plain_text(&parse("<p>A&amp;B&nbsp;C&#x4E2D;&#25991;</p>"));
        assert!(text.contains("A&B"));
        assert!(text.contains("中"));
        assert!(text.contains("文"));
    }

    #[test]
    fn links_resolved_against_base_with_types() {
        let dom = parse(concat!(
            r#"<a href="/about">About</a>"#,
            r#"<a href="https://other.example/x">Ext</a>"#,
            r#"<a href="mailto:a@b.c">Mail</a>"#,
            r##"<a href="#top">Anchor</a>"##,
            r#"<a href="javascript:void(0)">Js</a>"#,
            r#"<a href="https://example.com/next">Same</a>"#,
        ));
        let links = extract_links(&dom, Some("https://example.com/posts/1"));
        assert_eq!(links.len(), 6);
        assert_eq!(links[0].absolute_url, "https://example.com/about");
        assert_eq!(links[0].link_type, WebLinkType::Relative);
        assert_eq!(links[0].raw_href, "/about");
        assert_eq!(links[1].link_type, WebLinkType::External);
        assert_eq!(links[2].link_type, WebLinkType::Mail);
        assert_eq!(links[3].link_type, WebLinkType::Anchor);
        assert_eq!(links[4].link_type, WebLinkType::Unknown);
        assert_eq!(links[5].link_type, WebLinkType::Internal);
        // 无 base ⇒ 不硬拼（原文保留）
        let links2 = extract_links(&dom, None);
        assert_eq!(links2[0].absolute_url, "/about");
        assert_eq!(links2[0].link_type, WebLinkType::Unknown);
    }

    #[test]
    fn metadata_extracts_with_evidence_and_omits_missing() {
        let dom = parse(concat!(
            "<html lang=\"zh-CN\"><head><title>My Page</title>",
            "<meta name=\"description\" content=\"desc here\">",
            "<meta name=\"author\" content=\"Alice\">",
            "<meta property=\"article:published_time\" content=\"2026-10-01\">",
            "<meta property=\"og:title\" content=\"OG Title\">",
            "<link rel=\"canonical\" href=\"https://example.com/canonical\">",
            "</head><body>x</body></html>"
        ));
        let m = extract_metadata(&dom);
        assert_eq!(m.title.as_ref().expect("title").source, "title-tag");
        assert_eq!(m.title.as_ref().expect("title").value, "My Page");
        assert_eq!(m.description.as_ref().expect("desc").value, "desc here");
        assert_eq!(m.author.as_ref().expect("author").value, "Alice");
        assert_eq!(m.published_time.as_ref().expect("pub").value, "2026-10-01");
        assert_eq!(m.language.as_ref().expect("lang").value, "zh-CN");
        assert_eq!(
            m.canonical_url.as_ref().expect("canonical").value,
            "https://example.com/canonical"
        );
        assert_eq!(m.og_title.as_ref().expect("og").value, "OG Title");
        // 缺失 ⇒ 缺席（不猜 §28）
        assert!(m.modified_time.is_none());
        assert!(m.og_image.is_none());
    }

    #[test]
    fn main_content_semantic_container_is_high_confidence() {
        let html = "<body><nav>menu</nav><article><h1>T</h1><p>article body</p></article></body>";
        let (_, confidence, source) = extract_main_content(&parse(html));
        assert_eq!(confidence, ExtractionConfidence::High);
        assert_eq!(source, "article");
    }

    #[test]
    fn main_content_density_fallback_is_medium() {
        let body = format!("<p>{}</p>", "real content words ".repeat(60));
        let html =
            format!("<html><head><title>t</title></head><body><div>{body}</div></body></html>");
        let (fragment, confidence, source) = extract_main_content(&parse(&html));
        assert_eq!(confidence, ExtractionConfidence::Medium, "{source}");
        assert!(fragment_to_text(&fragment).contains("real content"));
    }

    #[test]
    fn hostile_deep_nesting_and_repetition_do_not_crash() {
        // 极深嵌套（html5ever 迭代构建；§78/§79）——不 StackOverflow
        let deep = format!("{}{}", "<div>".repeat(3000), "</div>".repeat(3000));
        let dom = parse(&format!("<html><body>{deep}</body></html>"));
        let _ = plain_text(&dom);
        // 10 万链接截断（§91 MAX_LINKS）
        let many = "<a href=\"/x\">x</a>".repeat(100_000);
        let dom2 = parse(&many);
        assert_eq!(extract_links(&dom2, None).len(), MAX_LINKS);
    }
}
