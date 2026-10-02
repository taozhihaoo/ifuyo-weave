//! 统一 Tool 抽象（charter #24 / M0 §5.2）。
//!
//! M0 只建立 Rust 类型签名：方法体不实现。`UnsupportedTool` 是参考实现，
//! 证明签名可用并为契约测试提供夹具。这个 trait 是 M1～M14 的核心扩展点。

use crate::cancellation::CancellationToken;
use crate::error::WeaveError;
use crate::id::{InputKind, OperationId, ToolCategory, ToolId};
use crate::preview::Preview;
use crate::progress::Progress;
use crate::result::OperationResult;

/// 工具选项的描述（charter #24 `options`）。M0 只声明形状，不做选项求值。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolOptionSpec {
    pub key: String,
    pub description: String,
    pub required: bool,
}

/// 工具的输入。M0 的输入是枚举而不是 `map[string, any]`。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ToolInput {
    Path { path: String },
    MultiplePaths { paths: Vec<String> },
    Text { content: String },
}

/// 统一 Tool 契约：8 个成员（charter #24）。
pub trait Tool: Send + Sync {
    fn id(&self) -> ToolId;

    fn category(&self) -> ToolCategory;

    fn input_types(&self) -> Vec<InputKind>;

    fn options(&self) -> Vec<ToolOptionSpec>;

    /// 预览必须无副作用（charter #11）。
    fn preview(&self, input: &ToolInput) -> Result<Preview, WeaveError>;

    /// 执行接受协作式取消令牌与进度回报。
    fn execute(
        &self,
        input: &ToolInput,
        cancel: &CancellationToken,
        report: &mut dyn FnMut(Progress),
    ) -> Result<OperationResult, WeaveError>;

    /// 是否可撤销；默认不可撤销（不可撤销必须如实声明，charter #12）。
    fn can_undo(&self) -> bool {
        false
    }

    /// 撤销一次已完成的操作。
    fn undo(&self, operation: OperationId) -> Result<(), WeaveError>;
}

/// 参考实现：签名成立、行为一律返回 Unsupported。
/// M1 起的真实工具替换它，而不是修改 trait。
#[derive(Debug)]
pub struct UnsupportedTool {
    id: ToolId,
    category: ToolCategory,
    input_types: Vec<InputKind>,
}

impl UnsupportedTool {
    pub fn new(id: ToolId, category: ToolCategory, input_types: Vec<InputKind>) -> Self {
        Self {
            id,
            category,
            input_types,
        }
    }
}

impl Tool for UnsupportedTool {
    fn id(&self) -> ToolId {
        self.id.clone()
    }

    fn category(&self) -> ToolCategory {
        self.category
    }

    fn input_types(&self) -> Vec<InputKind> {
        self.input_types.clone()
    }

    fn options(&self) -> Vec<ToolOptionSpec> {
        Vec::new()
    }

    fn preview(&self, _input: &ToolInput) -> Result<Preview, WeaveError> {
        Err(WeaveError::unsupported(
            "tool.previewUnsupported",
            "this tool is registered as a signature placeholder in M0",
        ))
    }

    fn execute(
        &self,
        _input: &ToolInput,
        _cancel: &CancellationToken,
        _report: &mut dyn FnMut(Progress),
    ) -> Result<OperationResult, WeaveError> {
        Err(WeaveError::unsupported(
            "tool.executeUnsupported",
            "this tool is registered as a signature placeholder in M0",
        ))
    }

    fn undo(&self, _operation: OperationId) -> Result<(), WeaveError> {
        Err(WeaveError::unsupported(
            "tool.undoUnsupported",
            "this tool is registered as a signature placeholder in M0",
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    fn placeholder() -> Arc<dyn Tool> {
        Arc::new(UnsupportedTool::new(
            ToolId::parse("files.rename").expect("valid"),
            ToolCategory::Files,
            vec![InputKind::MultipleFilePaths],
        ))
    }

    #[test]
    fn tool_signature_is_usable_through_dyn_tool() {
        let tool = placeholder();
        assert_eq!(tool.id().as_str(), "files.rename");
        assert_eq!(tool.category(), ToolCategory::Files);
        assert_eq!(tool.input_types(), vec![InputKind::MultipleFilePaths]);
        assert!(tool.options().is_empty());
        assert!(!tool.can_undo());
    }

    #[test]
    fn placeholder_returns_unsupported_for_all_actions() {
        let tool = placeholder();
        let input = ToolInput::MultiplePaths { paths: vec![] };
        assert_eq!(
            tool.preview(&input).expect_err("preview").code,
            "tool.previewUnsupported"
        );

        let token = CancellationToken::new();
        let mut reports = Vec::new();
        let err = tool
            .execute(&input, &token, &mut |p| reports.push(p))
            .expect_err("execute");
        assert_eq!(err.code, "tool.executeUnsupported");
        assert!(reports.is_empty());

        let err = tool.undo(OperationId::generate()).expect_err("undo");
        assert_eq!(err.code, "tool.undoUnsupported");
    }

    #[test]
    fn option_spec_serializes_camel_case() {
        let spec = ToolOptionSpec {
            key: "includeHidden".to_string(),
            description: "include hidden files".to_string(),
            required: false,
        };
        let json = serde_json::to_value(&spec).expect("serialize");
        assert!(json.get("required").is_some());
    }
}
