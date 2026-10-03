//! Tool Registry 接线（charter #24 / M0 §5.3）。
//!
//! M1 的三个文件能力以 Tool 形态注册进 registry，证明 Command Palette /
//! Quick Drop / 未来 Workflow 所需的统一发现机制端到端可用。
//! 真实执行入口仍是 IPC 命令（它们直接调 Application 服务，保持低开销）；
//! Tool::execute 是同一服务的完整形态，供未来引擎复用。

use weave_core::prelude::{
    CancellationToken, FileKind, InputKind, OperationId, Progress, ScanStatus, Tool, ToolCategory,
    ToolId, ToolOptionSpec, ToolRegistry, WeaveError,
};
use weave_files::ScanOptions;

use crate::files_dto::ToolDescriptorDto;

/// 只读检查工具：preview == execute 语义（无副作用，M1 §11）。
pub struct InspectTool;

impl Tool for InspectTool {
    fn id(&self) -> ToolId {
        ToolId::parse("files.inspect").expect("valid tool id")
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Files
    }

    fn input_types(&self) -> Vec<InputKind> {
        vec![InputKind::FilePath, InputKind::DirectoryPath]
    }

    fn options(&self) -> Vec<ToolOptionSpec> {
        Vec::new()
    }

    fn preview(
        &self,
        input: &weave_core::prelude::ToolInput,
    ) -> Result<weave_core::prelude::Preview, WeaveError> {
        let path = tool_input_path(input)?;
        Ok(weave_core::prelude::Preview {
            operation: OperationId::generate(),
            tool: self.id(),
            affected_items: vec![weave_core::prelude::PreviewItem {
                source: path,
                target: None,
                summary: "read-only metadata inspection".to_string(),
            }],
            warnings: vec![],
            conflicts: vec![],
            estimated_output: None,
            // 只读操作没有可撤销的变更；reversible=false 表示"没有东西可撤销"。
            reversible: false,
        })
    }

    fn execute(
        &self,
        input: &weave_core::prelude::ToolInput,
        _cancel: &CancellationToken,
        _report: &mut dyn FnMut(Progress),
    ) -> Result<weave_core::prelude::OperationResult, WeaveError> {
        let path = tool_input_path(input)?;
        let inspection = weave_files::inspect_file(
            &weave_files::fs::StdFilesystem,
            &path,
            &weave_files::InspectOptions::default(),
        )?;
        Ok(weave_core::prelude::OperationResult {
            success: true,
            processed: 1,
            skipped: 0,
            failed: 0,
            warnings: inspection.warnings,
            outputs: vec![inspection.normalized_path],
            duration_ms: 0,
        })
    }

    fn undo(&self, _operation: OperationId) -> Result<(), WeaveError> {
        Err(WeaveError::unsupported(
            "tool.undoUnsupported",
            "inspection is read-only; there is nothing to undo",
        ))
    }
}

/// 目录分析工具。
pub struct AnalyzeTool;

impl Tool for AnalyzeTool {
    fn id(&self) -> ToolId {
        ToolId::parse("files.analyze_directory").expect("valid tool id")
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Files
    }

    fn input_types(&self) -> Vec<InputKind> {
        vec![InputKind::DirectoryPath]
    }

    fn options(&self) -> Vec<ToolOptionSpec> {
        vec![
            ToolOptionSpec {
                key: "maxDepth".to_string(),
                description: "maximum recursion depth (root = 0)".to_string(),
                required: false,
            },
            ToolOptionSpec {
                key: "maxEntries".to_string(),
                description: "maximum processed entries guard".to_string(),
                required: false,
            },
        ]
    }

    fn preview(
        &self,
        input: &weave_core::prelude::ToolInput,
    ) -> Result<weave_core::prelude::Preview, WeaveError> {
        let path = tool_input_path(input)?;
        Ok(weave_core::prelude::Preview {
            operation: OperationId::generate(),
            tool: self.id(),
            affected_items: vec![weave_core::prelude::PreviewItem {
                source: path,
                target: None,
                summary: "read-only directory scan".to_string(),
            }],
            warnings: vec![],
            conflicts: vec![],
            estimated_output: None,
            reversible: false,
        })
    }

    fn execute(
        &self,
        input: &weave_core::prelude::ToolInput,
        cancel: &CancellationToken,
        report: &mut dyn FnMut(Progress),
    ) -> Result<weave_core::prelude::OperationResult, WeaveError> {
        let path = tool_input_path(input)?;
        let scan = weave_files::scan_directory(
            &weave_files::fs::StdFilesystem,
            &path,
            &ScanOptions::default(),
            cancel,
            report,
        )?;
        Ok(weave_core::prelude::OperationResult {
            success: matches!(
                scan.status,
                ScanStatus::Completed | ScanStatus::CompletedWithWarnings
            ),
            processed: scan.entries_processed,
            skipped: scan.other_entries,
            failed: scan.error_count,
            warnings: scan.warnings,
            outputs: vec![scan.root],
            duration_ms: scan.duration_ms,
        })
    }

    fn undo(&self, _operation: OperationId) -> Result<(), WeaveError> {
        Err(WeaveError::unsupported(
            "tool.undoUnsupported",
            "directory analysis is read-only; there is nothing to undo",
        ))
    }
}

/// 显式哈希工具（昂贵操作，M1 §34）。
pub struct HashTool;

impl Tool for HashTool {
    fn id(&self) -> ToolId {
        ToolId::parse("files.hash").expect("valid tool id")
    }

    fn category(&self) -> ToolCategory {
        ToolCategory::Files
    }

    fn input_types(&self) -> Vec<InputKind> {
        vec![InputKind::FilePath]
    }

    fn options(&self) -> Vec<ToolOptionSpec> {
        Vec::new()
    }

    fn preview(
        &self,
        input: &weave_core::prelude::ToolInput,
    ) -> Result<weave_core::prelude::Preview, WeaveError> {
        let path = tool_input_path(input)?;
        Ok(weave_core::prelude::Preview {
            operation: OperationId::generate(),
            tool: self.id(),
            affected_items: vec![weave_core::prelude::PreviewItem {
                source: path,
                target: None,
                summary: "streaming SHA-256 over the full file (explicit cost)".to_string(),
            }],
            warnings: vec![],
            conflicts: vec![],
            estimated_output: None,
            reversible: false,
        })
    }

    fn execute(
        &self,
        input: &weave_core::prelude::ToolInput,
        cancel: &CancellationToken,
        report: &mut dyn FnMut(Progress),
    ) -> Result<weave_core::prelude::OperationResult, WeaveError> {
        let path = tool_input_path(input)?;
        let hash = weave_files::hash_file(
            &weave_files::fs::StdFilesystem,
            std::path::Path::new(&path),
            cancel,
            report,
        )?;
        let warnings = match hash.status {
            weave_core::prelude::HashStatus::Unstable => {
                vec!["file changed while hashing; digest is best-effort".to_string()]
            }
            _ => vec![],
        };
        Ok(weave_core::prelude::OperationResult {
            success: hash.status != weave_core::prelude::HashStatus::Cancelled,
            processed: u64::from(hash.status != weave_core::prelude::HashStatus::Cancelled),
            skipped: 0,
            failed: 0,
            warnings,
            outputs: hash.digest_hex.map(|d| vec![d]).unwrap_or_default(),
            duration_ms: hash.duration_ms,
        })
    }

    fn undo(&self, _operation: OperationId) -> Result<(), WeaveError> {
        Err(WeaveError::unsupported(
            "tool.undoUnsupported",
            "hashing is read-only; there is nothing to undo",
        ))
    }
}

fn tool_input_path(input: &weave_core::prelude::ToolInput) -> Result<String, WeaveError> {
    match input {
        weave_core::prelude::ToolInput::Path { path } => Ok(path.clone()),
        weave_core::prelude::ToolInput::MultiplePaths { paths } => paths
            .first()
            .cloned()
            .ok_or_else(|| WeaveError::validation("tool.emptyInput", "no paths given")),
        weave_core::prelude::ToolInput::Text { .. } => Err(WeaveError::validation(
            "tool.wrongInputKind",
            "file tools require a path input",
        )),
    }
}

/// 构建带 M1 文件工具的注册表（注册表自身确定性：BTreeMap）。
pub fn build_file_tools_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    for tool in [
        std::sync::Arc::new(InspectTool) as std::sync::Arc<dyn Tool>,
        std::sync::Arc::new(AnalyzeTool) as std::sync::Arc<dyn Tool>,
        std::sync::Arc::new(HashTool) as std::sync::Arc<dyn Tool>,
    ] {
        registry.register(tool).expect("unique tool ids");
    }
    registry
}

pub fn describe_registry(registry: &ToolRegistry) -> Vec<ToolDescriptorDto> {
    registry
        .list()
        .iter()
        .filter_map(|id| {
            let tool = registry.lookup(id)?;
            Some(ToolDescriptorDto {
                id: id.to_string(),
                category: tool.category().as_str().to_string(),
                input_kinds: tool
                    .input_types()
                    .into_iter()
                    .map(|k| match k {
                        InputKind::FilePath => "filePath".to_string(),
                        InputKind::DirectoryPath => "directoryPath".to_string(),
                        InputKind::MultipleFilePaths => "multipleFilePaths".to_string(),
                        InputKind::Text => "text".to_string(),
                    })
                    .collect(),
            })
        })
        .collect()
}

/// FileKind 在 tools 里只被 scan 结果间接引用；保留引用避免 unused 告警。
#[allow(unused_imports)]
use FileKind as _;
