//! 统一文本限额（M4 下 §104/§166）：所有工具的分档上限共用此单一来源。
//!
//! 分档原因（§104）：Formatter（parser 驱动）、Diff（O(ND)+窗口）、Regex、
//! Transformation 计算模型不同，不能共用一个魔法数。超出 ⇒ 各工具返回
//! 结构化 `text.tooLarge` 系错误，绝不静默截断或冻结。

/// 分档文本限额（字节；匹配数另计）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextLimits {
    /// 文档加载上限（load_text_document）。
    pub document: u64,
    /// Formatter 输入上限（parser 驱动，最保守）。
    pub format: u64,
    /// Compare 单侧输入上限（窗口内存有界）。
    pub compare: u64,
    /// Extractor 输入上限（线性扫描 + 结果上限）。
    pub extract: u64,
    /// Transformer 输入上限（线性）。
    pub transform: u64,
    /// Extractor 结果匹配数上限（§185 防 UI 爆炸；超出截断 + Warning）。
    pub max_extract_matches: usize,
}

impl Default for TextLimits {
    fn default() -> Self {
        Self {
            document: 2 * 1024 * 1024,
            format: 1024 * 1024,
            compare: 2 * 1024 * 1024,
            extract: 2 * 1024 * 1024,
            transform: 2 * 1024 * 1024,
            max_extract_matches: 10_000,
        }
    }
}

impl TextLimits {
    /// 限额检查：超出 ⇒ 结构化错误（code 带工具语境）。
    pub fn check(&self, tool: &'static str, bytes: u64) -> Result<(), (String, &'static str)> {
        let max = match tool {
            "document" => self.document,
            "format" => self.format,
            "compare" => self.compare,
            "extract" => self.extract,
            "transform" => self.transform,
            _ => self.document,
        };
        if bytes > max {
            Err((
                format!("input exceeds the {tool} size limit ({max} bytes)"),
                "text.tooLarge",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn per_tool_limits_differ() {
        let limits = TextLimits::default();
        assert!(limits.format < limits.document, "parser 驱动最保守（§104）");
        assert!(limits.check("format", limits.format).is_ok());
        let (message, code) = limits.check("format", limits.format + 1).unwrap_err();
        assert_eq!(code, "text.tooLarge");
        assert!(message.contains("format"));
    }
}
