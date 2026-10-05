//! 清理内容变换（M2.4 §22-§26；P3 提交实现）。
use weave_core::prelude::WeaveError;

use crate::model::PrivacyFinding;

/// 用户选择的清理项。
#[derive(Debug, Clone)]
pub struct CleanupSelection {
    pub findings: Vec<PrivacyFinding>,
}

/// 应用清理到原始字节 → 安全副本字节（P3 实现）。
pub fn apply_cleanup(
    _original: &[u8],
    _selection: &CleanupSelection,
) -> Result<Vec<u8>, WeaveError> {
    Ok(_original.to_vec())
}
