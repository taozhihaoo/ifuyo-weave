//! 统一操作结果（charter #25 / M0 §5.1）。
//!
//! 字段为 charter 钦定的七字段：success / processed / skipped / failed /
//! warnings / outputs / duration。所有工具统一返回，禁止另造变体。

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// 所有工具统一返回的操作结果（charter #25）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OperationResult {
    pub success: bool,
    pub processed: u64,
    pub skipped: u64,
    pub failed: u64,
    pub warnings: Vec<String>,
    pub outputs: Vec<String>,
    /// 实际耗时，毫秒（跨 IPC 用整数，避免 Duration 序列化歧义）。
    pub duration_ms: u64,
}

impl OperationResult {
    pub fn empty() -> Self {
        Self {
            success: true,
            processed: 0,
            skipped: 0,
            failed: 0,
            warnings: Vec::new(),
            outputs: Vec::new(),
            duration_ms: 0,
        }
    }

    pub fn with_duration(mut self, duration: Duration) -> Self {
        self.duration_ms = duration.as_millis() as u64;
        self
    }

    /// 部分失败不改变 success 语义由调用方决定；这里只提供统计口径。
    pub fn total_items(&self) -> u64 {
        self.processed + self.skipped + self.failed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn result_has_seven_charter_fields() {
        let result = OperationResult {
            success: true,
            processed: 997,
            skipped: 0,
            failed: 3,
            warnings: vec!["3 items failed".to_string()],
            outputs: vec![],
            duration_ms: 1200,
        };
        let json = serde_json::to_value(&result).expect("serialize");
        for key in [
            "success",
            "processed",
            "skipped",
            "failed",
            "warnings",
            "outputs",
            "durationMs",
        ] {
            assert!(json.get(key).is_some(), "missing charter field {key}");
        }
        assert_eq!(result.total_items(), 1000);
    }

    #[test]
    fn duration_is_recorded_in_millis() {
        let result = OperationResult::empty().with_duration(Duration::from_millis(1500));
        assert_eq!(result.duration_ms, 1500);
    }
}
