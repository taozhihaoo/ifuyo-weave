//! 统一执行摘要（M2.0 §18）：模块无关的 Report 词汇。
//!
//! 各模块已有自己的执行报告（`weave-files::ExecutionReport`、batch 结果、
//! UndoReport）；本模型是它们**派生**出的共同摘要形状，供 UI 摘要、JSON
//! 导出与后续格式扩展使用——不建立第二份持久化，也不替代模块报告
//! （§18 Report 与模块解耦；镜像纪律见 DECISIONS D63）。

use crate::error::WeaveError;
use crate::operation::{OperationKind, ReversibilityLevel, RiskLevel};
use serde::{Deserialize, Serialize};

/// 计数汇总（M2.0 §18 Statistics）。
///
/// 语义注记：weave-files 执行器按 M2 §44 把取消条目计入 skipped；
/// 该事实在派生适配器里如实保留，不在摘要层伪造分离计数。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryStatistics {
    pub total: u64,
    pub succeeded: u64,
    pub failed: u64,
    pub skipped: u64,
    /// 当前模块报告中与 skipped 分离计数的取消数（无分离事实时为 0）。
    pub cancelled: u64,
    pub no_op: u64,
}

impl SummaryStatistics {
    /// 变更类条目数（succeeded + failed）——审计关心的实际作用面。
    pub fn mutations(&self) -> u64 {
        self.succeeded + self.failed
    }
}

/// 一次执行的统一摘要。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ExecutionSummary {
    pub operation_id: String,
    pub kind: OperationKind,
    /// 人类可读一句话摘要（模块生成，如 "Rename 3 files"）。
    pub summary: String,
    pub statistics: SummaryStatistics,
    pub warnings: Vec<String>,
    pub errors: Vec<WeaveError>,
    pub duration_ms: u64,
    pub reversibility: ReversibilityLevel,
    pub risk: RiskLevel,
}

impl ExecutionSummary {
    /// 机器可读 JSON（M2.0 §18：UI 摘要 + JSON 起步，后续格式可扩展）。
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// 单行文本摘要（通知/日志行用）。
    pub fn summary_line(&self) -> String {
        format!(
            "{} — ok {}, failed {}, skipped {}, noop {} ({} ms)",
            self.summary,
            self.statistics.succeeded,
            self.statistics.failed,
            self.statistics.skipped,
            self.statistics.no_op,
            self.duration_ms
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ExecutionSummary {
        ExecutionSummary {
            operation_id: "op_test".to_string(),
            kind: OperationKind::Rename,
            summary: "Rename 3 files".to_string(),
            statistics: SummaryStatistics {
                total: 4,
                succeeded: 3,
                failed: 0,
                skipped: 1,
                cancelled: 0,
                no_op: 0,
            },
            warnings: vec![],
            errors: vec![],
            duration_ms: 42,
            reversibility: ReversibilityLevel::OperationLog,
            risk: RiskLevel::Low,
        }
    }

    #[test]
    fn summary_json_round_trip_and_summary_line() {
        let s = sample();
        let json = s.to_json().expect("json");
        let back: ExecutionSummary = serde_json::from_str(&json).expect("parse");
        assert_eq!(back, s);
        let line = s.summary_line();
        assert!(line.contains("Rename 3 files"));
        assert!(line.contains("ok 3"));
        assert!(line.contains("(42 ms)"));
    }

    #[test]
    fn mutations_count_successes_and_failures_only() {
        assert_eq!(sample().statistics.mutations(), 3);
    }
}
