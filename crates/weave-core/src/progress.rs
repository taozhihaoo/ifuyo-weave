//! 统一进度模型（M0 §19，charter #26）。
//!
//! 一个 `Progress` 服务于所有工具；不为单个工具定制 `RenameProgress` 之类的变体。
//! Rate / ETA 属于派生显示字段，由消费方根据真实吞吐计算，不进入核心模型。

use crate::id::OperationId;
use serde::{Deserialize, Serialize};

/// 操作的宏观状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OperationStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

/// 统一进度：current / total / percentage / status。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub operation: OperationId,
    pub current: u64,
    pub total: Option<u64>,
    pub status: OperationStatus,
}

impl Progress {
    pub fn running(operation: OperationId, current: u64, total: Option<u64>) -> Self {
        Self {
            operation,
            current,
            total,
            status: OperationStatus::Running,
        }
    }

    /// total 存在且大于 0 时给出诚实百分比；否则 None（禁止伪造 63.7%）。
    pub fn percentage(&self) -> Option<f32> {
        let total = self.total?;
        if total == 0 {
            return None;
        }
        Some(((self.current as f32 / total as f32) * 100.0).min(100.0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn percentage_is_honest() {
        let p = Progress::running(OperationId::generate(), 35, Some(100));
        assert_eq!(p.percentage(), Some(35.0));

        let unknown = Progress::running(OperationId::generate(), 35, None);
        assert_eq!(unknown.percentage(), None);

        let zero = Progress::running(OperationId::generate(), 0, Some(0));
        assert_eq!(zero.percentage(), None);

        let over = Progress::running(OperationId::generate(), 120, Some(100));
        assert_eq!(over.percentage(), Some(100.0));
    }

    #[test]
    fn progress_serializes_camel_case() {
        let p = Progress::running(OperationId::generate(), 1, Some(2));
        let json = serde_json::to_value(&p).expect("serialize");
        assert!(json.get("current").is_some());
        assert!(json.get("total").is_some());
        assert!(json.get("status").is_some());
    }
}
