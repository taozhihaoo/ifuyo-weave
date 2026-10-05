//! 分层验证策略（M2.0 §7）：Plan 时捕获文件状态快照，Apply 前按策略
//! 重新验证（Revalidate）。成本分档是契约的一部分——不允许"安全起见"
//! 把所有操作默认全量 Hash（M2.0 §25 性能原则）。
//!
//! - `Exists`：仅存在性——回收站恢复目标检查、批量跳过判断等场景。
//! - `SizeMtime`：存在 + size + mtime——Rename/Move Revalidate（与 M2
//!   既有行为一致，M2 §38）。
//! - `FullHash`：再加内容 Hash——跨卷移动的 Copy→Verify→Remove（M2.0
//!   §15）、未来完整性中心。必须由场景显式选择，成本 Expensive。

use crate::facts::HashAlgorithm;
use serde::{Deserialize, Serialize};
use std::time::SystemTime;

/// 验证策略（成本从低到高；场景显式选择，不默认升级）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationPolicy {
    Exists,
    SizeMtime,
    FullHash(HashAlgorithm),
}

/// 验证成本档位（策略选择的性能契约，M2.0 §25）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum VerificationCost {
    /// 纯元数据读取（stat），O(1)。
    Cheap,
    /// 元数据读取 + 字段比较。
    Moderate,
    /// 需要完整读取内容计算 Hash——大文件昂贵。
    Expensive,
}

impl VerificationPolicy {
    pub fn cost(&self) -> VerificationCost {
        match self {
            VerificationPolicy::Exists => VerificationCost::Cheap,
            VerificationPolicy::SizeMtime => VerificationCost::Moderate,
            VerificationPolicy::FullHash(_) => VerificationCost::Expensive,
        }
    }

    /// 该策略验证哪些字段（审计/报告可解释性用）。
    pub fn describes(&self) -> &'static str {
        match self {
            VerificationPolicy::Exists => "exists",
            VerificationPolicy::SizeMtime => "exists+size+mtime",
            VerificationPolicy::FullHash(_) => "exists+size+mtime+hash",
        }
    }
}

/// Plan 时的文件状态快照——Revalidate 的期望侧（M2.0 §7）。
///
/// 字段均为 Option：策略未覆盖（或快照时不可得）的字段为 None，
/// 验证时跳过该字段而不是猜测（诚实表达）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileFingerprint {
    pub path: String,
    pub size: Option<u64>,
    pub modified: Option<SystemTime>,
    /// FullHash 策略下的内容 digest（小写 hex）；其余策略为 None。
    pub digest_hex: Option<String>,
}

impl FileFingerprint {
    /// 仅存在性快照。
    pub fn exists_only(path: impl Into<String>) -> Self {
        Self {
            path: path.into(),
            size: None,
            modified: None,
            digest_hex: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_costs_are_ordered_by_expense() {
        assert!(VerificationPolicy::Exists.cost() < VerificationPolicy::SizeMtime.cost());
        assert!(
            VerificationPolicy::SizeMtime.cost()
                < VerificationPolicy::FullHash(HashAlgorithm::Sha256).cost()
        );
    }

    #[test]
    fn policy_describes_covered_fields() {
        assert_eq!(VerificationPolicy::Exists.describes(), "exists");
        assert_eq!(
            VerificationPolicy::SizeMtime.describes(),
            "exists+size+mtime"
        );
        assert_eq!(
            VerificationPolicy::FullHash(HashAlgorithm::Sha256).describes(),
            "exists+size+mtime+hash"
        );
    }

    #[test]
    fn fingerprint_serializes_and_defaults_to_exists_only() {
        let fp = FileFingerprint::exists_only("C:\\a.txt");
        assert_eq!(fp.size, None);
        assert_eq!(fp.digest_hex, None);
        let json = serde_json::to_value(&fp).expect("serialize");
        assert_eq!(json["path"], "C:\\a.txt");
        let back: FileFingerprint = serde_json::from_value(json).expect("deserialize");
        assert_eq!(back, fp);
    }
}
