//! PdfPageRange（M8 上 §21-§22）：解析 / 校验 / 归一 / 去重 / 排序。
//!
//! 输入 `1-3,5,8-10` ⇒ [1,2,3,5,8,9,10]；0/负数/逆序区间/越界/坏 token
//! 一律结构化错误（§22）。

use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub struct PageRangeError {
    pub code: &'static str,
    pub message: String,
}

impl fmt::Display for PageRangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

fn err(code: &'static str, message: impl Into<String>) -> PageRangeError {
    PageRangeError {
        code,
        message: message.into(),
    }
}

/// 解析页码范围（1-based）。`max_page` 用于越界校验（None = 不校验）。
/// 输出：升序去重页码（§21：除非显式要求保留重复——本解析器不做）。
pub fn parse_page_ranges(input: &str, max_page: Option<u64>) -> Result<Vec<u64>, PageRangeError> {
    let text = input.trim();
    if text.is_empty() {
        return Err(err("range.empty", "page range is empty"));
    }
    let mut pages: Vec<u64> = Vec::new();
    for token in text.split(',') {
        let token = token.trim();
        if token.is_empty() {
            return Err(err(
                "range.malformedToken",
                format!("empty token in page range '{input}'"),
            ));
        }
        if let Some((start_s, end_s)) = token.split_once('-') {
            let start = parse_num(start_s, input)?;
            let end = parse_num(end_s, input)?;
            if start == 0 || end == 0 {
                return Err(err(
                    "range.zeroPage",
                    format!("page numbers are 1-based; got '{token}'"),
                ));
            }
            if end < start {
                return Err(err(
                    "range.reversedRange",
                    format!("reversed range '{token}' (use ascending order)"),
                ));
            }
            if let Some(max) = max_page
                && end > max
            {
                return Err(err(
                    "range.outOfBounds",
                    format!("range '{token}' exceeds page count {max}"),
                ));
            }
            pages.extend(start..=end);
        } else {
            let page = parse_num(token, input)?;
            if page == 0 {
                return Err(err("range.zeroPage", "page numbers are 1-based; got 0"));
            }
            if let Some(max) = max_page
                && page > max
            {
                return Err(err(
                    "range.outOfBounds",
                    format!("page {page} exceeds page count {max}"),
                ));
            }
            pages.push(page);
        }
    }
    pages.sort_unstable();
    pages.dedup();
    Ok(pages)
}

fn parse_num(s: &str, input: &str) -> Result<u64, PageRangeError> {
    s.trim().parse::<u64>().map_err(|_| {
        err(
            "range.malformedToken",
            format!("malformed token '{s}' in page range '{input}'"),
        )
    })
}

/// §23 Split Every N：切分保证 no missing / no duplicate / deterministic。
pub fn split_every_n(total: u64, n: u64) -> Result<Vec<(u64, u64)>, PageRangeError> {
    if n == 0 {
        return Err(err("range.zeroChunk", "N must be >= 1"));
    }
    if total == 0 {
        return Err(err("range.emptyDocument", "document has no pages"));
    }
    let mut out = Vec::new();
    let mut start = 1;
    while start <= total {
        let end = (start + n - 1).min(total);
        out.push((start, end));
        start = end + 1;
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ranges_sorted_deduped() {
        // §22：1-3,5,8-10 ⇒ 1,2,3,5,8,9,10；重复去重
        assert_eq!(
            parse_page_ranges("1-3,5,8-10", None).expect("ok"),
            vec![1, 2, 3, 5, 8, 9, 10]
        );
        assert_eq!(
            parse_page_ranges("3,1,2,2", None).expect("ok"),
            vec![1, 2, 3]
        );
        assert_eq!(
            parse_page_ranges(" 7 - 9 , 12 ", None).expect("ok"),
            vec![7, 8, 9, 12]
        );
    }

    #[test]
    fn rejects_structured_errors() {
        for (input, code) in [
            ("0", "range.zeroPage"),
            ("-3", "range.malformedToken"),
            ("5-2", "range.reversedRange"),
            ("1-3,x", "range.malformedToken"),
            ("1-3,", "range.malformedToken"),
            ("", "range.empty"),
        ] {
            let e = parse_page_ranges(input, None).expect_err(input);
            assert_eq!(e.code, code, "{input}: {e:?}");
        }
        let e = parse_page_ranges("1-5", Some(4)).expect_err("oob");
        assert_eq!(e.code, "range.outOfBounds");
    }

    #[test]
    fn split_every_n_covers_exactly() {
        // §23：100 / N=20 ⇒ 5 块无缺失无重复
        let chunks = split_every_n(100, 20).expect("ok");
        assert_eq!(
            chunks,
            vec![(1, 20), (21, 40), (41, 60), (61, 80), (81, 100)]
        );
        let tail = split_every_n(103, 40).expect("ok");
        assert_eq!(tail, vec![(1, 40), (41, 80), (81, 103)]);
        assert!(split_every_n(10, 0).is_err());
        assert!(split_every_n(0, 5).is_err());
    }
}
