//! 统一故障注入框架（M0 §23.3，charter #43）。
//!
//! M0 只建立机制：未来 Adapter 层在真实故障点调用 [`FaultInjector::trip`]，
//! 命中已布防的故障时返回失败路径。失败场景（Permission Denied / Disk Full /
//! File Locked / Rename Collision / Partial Write / Interrupted / Cancelled /
//! Malformed Input）是 Weave 文件安全模型的一部分。

use std::collections::{BTreeMap, BTreeSet};

/// 可注入的故障类别。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Fault {
    PermissionDenied,
    DiskFull,
    FileLocked,
    RenameCollision,
    PartialWrite,
    InterruptedOperation,
    CancelledOperation,
    MalformedInput,
}

impl Fault {
    pub fn as_str(self) -> &'static str {
        match self {
            Fault::PermissionDenied => "permission_denied",
            Fault::DiskFull => "disk_full",
            Fault::FileLocked => "file_locked",
            Fault::RenameCollision => "rename_collision",
            Fault::PartialWrite => "partial_write",
            Fault::InterruptedOperation => "interrupted_operation",
            Fault::CancelledOperation => "cancelled_operation",
            Fault::MalformedInput => "malformed_input",
        }
    }
}

/// 故障注入器：测试中布防故障，被测代码在故障点询问是否应当失败。
#[derive(Debug, Default)]
pub struct FaultInjector {
    armed: BTreeSet<Fault>,
    trips: BTreeMap<Fault, u64>,
}

impl FaultInjector {
    pub fn new() -> Self {
        Self::default()
    }

    /// 布防一个故障类别。
    pub fn arm(&mut self, fault: Fault) -> &mut Self {
        self.armed.insert(fault);
        self
    }

    pub fn disarm(&mut self, fault: Fault) -> &mut Self {
        self.armed.remove(&fault);
        self
    }

    pub fn is_armed(&self, fault: Fault) -> bool {
        self.armed.contains(&fault)
    }

    /// 被测代码在故障点调用：返回 `true` 表示该故障应当触发。
    /// 每次命中都会计数，便于断言"故障真的走到了"。
    pub fn trip(&mut self, fault: Fault) -> bool {
        if self.armed.contains(&fault) {
            *self.trips.entry(fault).or_insert(0) += 1;
            true
        } else {
            false
        }
    }

    /// 某故障被触发的次数。
    pub fn trips(&self, fault: Fault) -> u64 {
        self.trips.get(&fault).copied().unwrap_or(0)
    }

    /// 清空布防与计数。
    pub fn reset(&mut self) {
        self.armed.clear();
        self.trips.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unarmed_faults_never_trip() {
        let mut injector = FaultInjector::new();
        assert!(!injector.trip(Fault::DiskFull));
        assert_eq!(injector.trips(Fault::DiskFull), 0);
    }

    #[test]
    fn armed_faults_trip_and_count() {
        let mut injector = FaultInjector::new();
        injector.arm(Fault::PermissionDenied);
        assert!(injector.is_armed(Fault::PermissionDenied));
        assert!(injector.trip(Fault::PermissionDenied));
        assert!(injector.trip(Fault::PermissionDenied));
        assert_eq!(injector.trips(Fault::PermissionDenied), 2);

        injector.disarm(Fault::PermissionDenied);
        assert!(!injector.trip(Fault::PermissionDenied));
    }

    #[test]
    fn reset_clears_everything() {
        let mut injector = FaultInjector::new();
        injector.arm(Fault::FileLocked);
        assert!(injector.trip(Fault::FileLocked));
        injector.reset();
        assert!(!injector.is_armed(Fault::FileLocked));
        assert_eq!(injector.trips(Fault::FileLocked), 0);
    }

    #[test]
    fn all_fault_kinds_have_stable_names() {
        let faults = [
            Fault::PermissionDenied,
            Fault::DiskFull,
            Fault::FileLocked,
            Fault::RenameCollision,
            Fault::PartialWrite,
            Fault::InterruptedOperation,
            Fault::CancelledOperation,
            Fault::MalformedInput,
        ];
        for fault in faults {
            assert!(!fault.as_str().is_empty());
        }
    }
}
