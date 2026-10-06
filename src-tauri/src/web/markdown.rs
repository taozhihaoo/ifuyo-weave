//! M2.10 HTML → Markdown（§24/§25）：基于 scraper DOM 的确定性转换。
//!
//! - 覆盖 H1-H6/Paragraph/Bold/Italic/Strike/Link/Image/OL/UL/Quote/
//!   Code/CodeBlock/Table/HR（§24）；不支持结构保留文本不丢失（§24）。
//! - 安全（§25）：javascript:/data: href 一律丢弃（保留文本）；Markdown
//!   是纯文本产物，不进 WebView 执行（§40）。
//! - Enter/Exit 帧栈遍历（无递归 §79）；输出端 pending-space 状态机保证
//!   行内标记闭合处不吃空格（`**bold**` 而非 `**bold **`）。

use ego_tree::NodeRef as EgoNodeRef;
use scraper::{ElementRef, Html, Node};

use super::html::resolve_relative;
use super::url;

/// 表格状态（GFM：首行 = 表头 ⇒ 其后补 `---` 分隔行）。
struct TableState {
    rows_done: u32,
    header_cols: u32,
    cur_cols: u32,
}

enum Frame<'a> {
    Enter(EgoNodeRef<'a, Node>, u32),
    Exit(EgoNodeRef<'a, Node>, u32),
}

/// 输出端：`pending_space` 使文本间空格在行内标记闭合处被吃掉。
struct Emitter {
    out: String,
    pending_space: bool,
}

impl Emitter {
    fn text(&mut self, raw: &str) {
        let collapsed = raw.split_whitespace().collect::<Vec<_>>().join(" ");
        if collapsed.is_empty() {
            return;
        }
        if self.pending_space && self.needs_separator() {
            self.out.push(' ');
        }
        self.pending_space = false;
        self.out.push_str(&collapsed);
        self.pending_space = true;
    }

    /// 标记前缀：与前一词之间补空格（`see[x](y)` ⇒ `see [x](y)`）。
    fn markup_open(&mut self, markup: &str) {
        if markup.is_empty() {
            return;
        }
        if self.pending_space && self.needs_separator() {
            self.out.push(' ');
        }
        self.pending_space = false;
        self.out.push_str(markup);
    }

    /// 标记后缀：吃掉 pending space（`**bold**` 而非 `**bold **`）。
    fn markup_close(&mut self, markup: &str) {
        self.pending_space = false;
        self.out.push_str(markup);
    }

    /// 表格行关闭：pending space 落地为真格再补 `|`（`| H2 |`）。
    fn close_row(&mut self) {
        if self.pending_space {
            self.out.push(' ');
            self.pending_space = false;
        }
        self.out.push('|');
    }

    /// 块级前缀/分隔：重置 pending。
    fn block(&mut self, markup: &str) {
        self.pending_space = false;
        self.out.push_str(markup);
    }

    fn needs_separator(&self) -> bool {
        !self.out.is_empty()
            && !self.out.ends_with(' ')
            && !self.out.ends_with('\n')
            && !self.out.ends_with('[')
            && !self.out.ends_with('(')
            && !self.out.ends_with('>')
            && !self.out.ends_with('|')
            && !self.out.ends_with('#')
    }
}

/// DOM → Markdown。`base` 用于相对链接/图片解析（§12 join 语义）。
pub fn to_markdown(dom: &Html, base: Option<&str>) -> String {
    let root = dom.tree.root();
    let mut em = Emitter {
        out: String::with_capacity(64 * 1024),
        pending_space: false,
    };
    let mut list_counters: Vec<u32> = Vec::new();
    let mut list_kinds: Vec<bool> = Vec::new(); // true = ordered
    let mut tables: Vec<TableState> = Vec::new();
    let mut stack: Vec<Frame<'_>> = vec![Frame::Enter(root, 0)];
    while let Some(frame) = stack.pop() {
        match frame {
            Frame::Enter(node, depth) => {
                if depth > 1024 {
                    continue;
                }
                match node.value() {
                    Node::Text(text) => em.text(text),
                    Node::Element(el) => {
                        if matches!(
                            el.name(),
                            "script" | "style" | "noscript" | "template" | "head"
                        ) {
                            continue; // 子树整体剔除（§22/§77）
                        }
                        let element = ElementRef::wrap(node);
                        enter_markup(
                            el.name(),
                            &element,
                            base,
                            &mut em,
                            &mut list_counters,
                            &mut list_kinds,
                            &mut tables,
                        );
                        stack.push(Frame::Exit(node, depth));
                        for child in node.children().rev() {
                            stack.push(Frame::Enter(child, depth + 1));
                        }
                    }
                    _ => {
                        for child in node.children().rev() {
                            stack.push(Frame::Enter(child, depth + 1));
                        }
                    }
                }
            }
            Frame::Exit(node, depth) => {
                if depth > 1024 {
                    continue;
                }
                if let Node::Element(el) = node.value() {
                    let element = ElementRef::wrap(node);
                    exit_markup(
                        el.name(),
                        &element,
                        base,
                        &mut em,
                        &mut list_counters,
                        &mut list_kinds,
                        &mut tables,
                    );
                }
            }
        }
    }
    post_process(&em.out)
}

fn is_in_pre(el: &Option<ElementRef<'_>>) -> bool {
    el.as_ref()
        .and_then(|e| e.parent())
        .and_then(|p| ElementRef::wrap(p))
        .is_some_and(|p| p.value().name() == "pre")
}

fn enter_markup(
    name: &str,
    el: &Option<ElementRef<'_>>,
    base: Option<&str>,
    em: &mut Emitter,
    counters: &mut Vec<u32>,
    list_kinds: &mut Vec<bool>,
    tables: &mut Vec<TableState>,
) {
    match name {
        "h1" => em.block("\n\n# "),
        "h2" => em.block("\n\n## "),
        "h3" => em.block("\n\n### "),
        "h4" => em.block("\n\n#### "),
        "h5" => em.block("\n\n##### "),
        "h6" => em.block("\n\n###### "),
        "p" | "div" | "section" | "article" | "header" | "footer" | "figure" | "figcaption" => {
            em.block("\n\n");
        }
        "strong" | "b" => em.markup_open("**"),
        "em" | "i" => em.markup_open("*"),
        "del" | "s" | "strike" => em.markup_open("~~"),
        "code" => {
            // pre>code 由 pre 围栏负责；行内 code 用反引号
            if !is_in_pre(el) {
                em.markup_open("`");
            }
        }
        "pre" => em.block("\n\n```\n"),
        "blockquote" => em.block("\n\n> "),
        "hr" => em.block("\n\n---\n"),
        "br" => em.block("  \n"),
        "ul" => {
            counters.push(0);
            list_kinds.push(false);
            em.block("\n");
        }
        "ol" => {
            counters.push(0);
            list_kinds.push(true);
            em.block("\n");
        }
        "li" => {
            let ordered = list_kinds.last().copied().unwrap_or(false);
            let marker = if ordered {
                if let Some(last) = counters.last_mut() {
                    *last += 1;
                    format!("\n{}. ", last)
                } else {
                    "\n1. ".to_string()
                }
            } else {
                "\n- ".to_string()
            };
            em.block(&marker);
        }
        "table" => {
            tables.push(TableState {
                rows_done: 0,
                header_cols: 0,
                cur_cols: 0,
            });
            em.block("\n\n");
        }
        "tr" => {
            if let Some(t) = tables.last_mut() {
                t.cur_cols = 0;
            }
        }
        "td" | "th" => {
            if let Some(t) = tables.last_mut() {
                t.cur_cols += 1;
            }
            em.markup_open("| ");
        }
        "a" => {
            // 安全（§25）：javascript:/data: href 丢弃——只保留文本
            let href = el
                .as_ref()
                .and_then(|e| e.value().attr("href"))
                .unwrap_or("");
            if safe_link_target(base, href) {
                em.markup_open("[");
            }
        }
        "img" => {
            let src = el
                .as_ref()
                .and_then(|e| e.value().attr("src"))
                .unwrap_or("");
            let alt = el
                .as_ref()
                .and_then(|e| e.value().attr("alt"))
                .unwrap_or("");
            let markup = match resolve_relative(base, src) {
                Some(u) if u.starts_with("http") => format!("![{alt}]({u})"),
                _ => format!("![{alt}]"),
            };
            em.markup_open(&markup);
        }
        _ => {}
    }
}

fn exit_markup(
    name: &str,
    el: &Option<ElementRef<'_>>,
    base: Option<&str>,
    em: &mut Emitter,
    counters: &mut Vec<u32>,
    list_kinds: &mut Vec<bool>,
    tables: &mut Vec<TableState>,
) {
    match name {
        "h1" | "h2" | "h3" | "h4" | "h5" | "h6" => em.block("\n"),
        "strong" | "b" => em.markup_close("**"),
        "em" | "i" => em.markup_close("*"),
        "del" | "s" | "strike" => em.markup_close("~~"),
        "code" => {
            if !is_in_pre(el) {
                em.markup_close("`");
            }
        }
        "pre" => em.block("\n```"),
        "ul" | "ol" => {
            counters.pop();
            list_kinds.pop();
            em.block("\n");
        }
        "td" | "th" => {}
        "tr" => {
            if let Some(t) = tables.last_mut() {
                t.rows_done += 1;
                if t.rows_done == 1 {
                    // 首行 = 表头 ⇒ GFM 分隔行（按表头列数）
                    t.header_cols = t.cur_cols;
                    em.close_row();
                    let mut sep = String::from("\n");
                    for _ in 0..t.header_cols.max(1) {
                        sep.push_str("|---");
                    }
                    sep.push_str("|\n");
                    em.block(&sep);
                } else {
                    em.close_row();
                    em.block("\n");
                }
            }
        }
        "table" => {
            tables.pop();
            em.block("\n");
        }
        "a" => {
            let href = el
                .as_ref()
                .and_then(|e| e.value().attr("href"))
                .unwrap_or("");
            if safe_link_target(base, href) {
                let target = resolve_relative(base, href).unwrap_or_default();
                em.markup_close(&format!("]({target})"));
            }
        }
        _ => {}
    }
}

/// 链接目标白名单（§25）：http/https/mailto/tel 可输出；其余（javascript:
/// /data:/相对不可解析）只保留文本。
fn safe_link_target(base: Option<&str>, href: &str) -> bool {
    match resolve_relative(base, href) {
        Some(u) => {
            u.starts_with("http://")
                || u.starts_with("https://")
                || u.starts_with("mailto:")
                || u.starts_with("tel:")
        }
        None => false,
    }
}

/// 终清理：行尾空格、多空行折叠、整体 trim。
fn post_process(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed_end = line.trim_end();
        let collapsed = trimmed_end.split_whitespace().collect::<Vec<_>>().join(" ");
        out.push_str(&collapsed);
        out.push('\n');
    }
    // 折叠 3+ 连续换行
    let mut final_out = String::with_capacity(out.len());
    let mut blank = 0;
    for line in out.lines() {
        if line.is_empty() {
            blank += 1;
            if blank > 1 {
                continue;
            }
        } else {
            blank = 0;
        }
        final_out.push_str(line);
        final_out.push('\n');
    }
    final_out.trim().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn md(html: &str) -> String {
        to_markdown(&Html::parse_document(html), Some("https://example.com/"))
    }

    #[test]
    fn headings_paragraphs_and_inline_marks() {
        let out = md(
            "<body><h1>Title</h1><p>plain <b>bold</b> and <i>it</i> and <del>gone</del>.</p></body>",
        );
        assert!(out.contains("# Title"), "{out}");
        assert!(out.contains("**bold**"), "{out}");
        assert!(out.contains("*it*"), "{out}");
        assert!(out.contains("~~gone~~"), "{out}");
        assert!(!out.contains("** "), "标记闭合处不吃空格: {out}");
    }

    #[test]
    fn links_resolved_and_dangerous_schemes_dropped() {
        let out = md(concat!(
            r#"<body><a href="/x">rel</a>"#,
            r#"<a href="https://other.example/y">abs</a>"#,
            r#"<a href="javascript:alert(1)">evil</a></body>"#
        ));
        assert!(out.contains("[rel](https://example.com/x)"), "{out}");
        assert!(out.contains("[abs](https://other.example/y)"), "{out}");
        // javascript: href 丢弃但文本保留（§24/§25）
        assert!(out.contains("evil"), "{out}");
        assert!(!out.contains("javascript"), "{out}");
        assert!(out.ends_with("evil"), "危险链接只保留文本: {out}");
    }

    #[test]
    fn lists_ordered_and_unordered() {
        let out = md("<body><ul><li>a</li><li>b</li></ul><ol><li>one</li><li>two</li></ol></body>");
        assert!(out.contains("- a"), "{out}");
        assert!(out.contains("- b"), "{out}");
        assert!(out.contains("1. one"), "{out}");
        assert!(out.contains("2. two"), "{out}");
    }

    #[test]
    fn code_blocks_and_inline_code() {
        let out =
            md("<body><pre><code>fn main() {}</code></pre><p>use <code>x</code> here</p></body>");
        assert!(out.contains("```\nfn main() {}\n```"), "{out}");
        assert!(out.contains("`x`"), "{out}");
    }

    #[test]
    fn images_use_resolved_src_with_alt() {
        let out = md(r#"<body><img src="pic.png" alt="A pic"></body>"#);
        assert!(
            out.contains("![A pic](https://example.com/pic.png)"),
            "{out}"
        );
    }

    #[test]
    fn blockquote_and_hr() {
        let out = md("<body><blockquote>wise words</blockquote><hr></body>");
        assert!(out.contains("> wise words"), "{out}");
        assert!(out.contains("---"), "{out}");
    }

    #[test]
    fn table_keeps_pipe_rows_with_gfm_header_separator() {
        let out = md(
            "<body><table><tr><th>H1</th><th>H2</th></tr><tr><td>v1</td><td>v2</td></tr></table></body>",
        );
        assert!(out.contains("| H1 | H2 |"), "{out}");
        assert!(out.contains("|---|---|"), "{out}");
        assert!(out.contains("| v1 | v2 |"), "{out}");
    }

    #[test]
    fn script_content_excluded_entirely() {
        let out = md("<body><script>var x = 1;</script><p>ok</p></body>");
        assert!(!out.contains("var x"), "{out}");
        assert!(out.contains("ok"), "{out}");
    }

    #[test]
    fn hostile_nesting_does_not_crash() {
        let deep = format!("{}{}", "<div>".repeat(2000), "</div>".repeat(2000));
        let out = md(&format!("<body>{deep}<p>tail</p></body>"));
        assert!(out.contains("tail"), "{out}");
    }
}
