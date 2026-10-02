//! Tool Registry（M0 §5.3）。
//!
//! 未来 Command Palette / Quick Actions / Tool Search / Workflow 都从这里读。
//! M0 只需要 register / lookup / list，且必须 deterministic（BTreeMap 有序遍历）。

use crate::error::WeaveError;
use crate::id::ToolId;
use crate::tool::Tool;
use std::collections::BTreeMap;
use std::sync::Arc;

/// 全局工具注册表。注册表自身是纯内存结构，不绑定 Tauri 或线程池。
#[derive(Default)]
pub struct ToolRegistry {
    tools: BTreeMap<ToolId, Arc<dyn Tool>>,
}

impl ToolRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    /// 注册工具。工具 ID 不可重复（charter #24）。
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> Result<(), WeaveError> {
        let id = tool.id();
        if self.tools.contains_key(&id) {
            return Err(WeaveError::conflict(
                "registry.duplicateToolId",
                format!("tool id '{id}' is already registered"),
            )
            .with_suggestion("pick a unique, stable tool id"));
        }
        self.tools.insert(id, tool);
        Ok(())
    }

    pub fn lookup(&self, id: &ToolId) -> Option<Arc<dyn Tool>> {
        self.tools.get(id).cloned()
    }

    /// 按 ToolId 字典序列出全部已注册工具（deterministic）。
    pub fn list(&self) -> Vec<ToolId> {
        self.tools.keys().cloned().collect()
    }

    pub fn len(&self) -> usize {
        self.tools.len()
    }

    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::id::InputKind;
    use crate::tool::UnsupportedTool;

    fn tool(raw: &str) -> Arc<dyn Tool> {
        let id = ToolId::parse(raw).expect("valid tool id");
        let category = id.category();
        Arc::new(UnsupportedTool::new(
            id,
            category,
            vec![InputKind::FilePath],
        ))
    }

    #[test]
    fn register_lookup_list_round_trip() {
        let mut registry = ToolRegistry::new();
        assert!(registry.is_empty());

        registry.register(tool("files.rename")).expect("register");
        registry.register(tool("data.csv")).expect("register");
        registry.register(tool("text.compare")).expect("register");

        let list = registry.list();
        // BTreeMap 保证 deterministic 排序，与注册顺序无关。
        assert_eq!(
            list.iter().map(|id| id.as_str()).collect::<Vec<_>>(),
            vec!["data.csv", "files.rename", "text.compare"]
        );

        let id = ToolId::parse("files.rename").expect("valid");
        let found = registry.lookup(&id).expect("found");
        assert_eq!(found.id(), id);
        assert_eq!(registry.len(), 3);
    }

    #[test]
    fn duplicate_tool_id_is_rejected_with_conflict_error() {
        let mut registry = ToolRegistry::new();
        registry.register(tool("files.rename")).expect("first");
        let err = registry
            .register(tool("files.rename"))
            .expect_err("duplicate must fail");
        assert_eq!(err.code, "registry.duplicateToolId");
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn unknown_tool_id_lookups_are_none_not_panics() {
        let registry = ToolRegistry::new();
        let id = ToolId::parse("files.missing").expect("valid");
        assert!(registry.lookup(&id).is_none());
    }
}
