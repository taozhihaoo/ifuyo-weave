//! 格式扫描器（M2.4 §19-§21；P2 提交实现）。
use crate::model::PrivacyScanSession;

/// 扫描资源边界（§46：无界输入防护）。
#[derive(Debug, Clone)]
pub struct ScanLimits {
    /// 单文件检查上限（字节）。
    pub max_file_bytes: u64,
    /// 单文件 Finding 上限。
    pub max_findings_per_file: usize,
    /// 单次会话 Finding 上限。
    pub max_findings_total: usize,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 32 * 1024 * 1024,
            max_findings_per_file: 200,
            max_findings_total: 5000,
        }
    }
}

/// 按路径扫描（P2 实现；当前占位以保证 crate 编译边界）。
pub fn scan_path(_root: &str, _session: &mut PrivacyScanSession, _limits: &ScanLimits) {
    // implemented in P2
}

/// 按字节自动分派扫描（P2 实现）。
pub fn scan_bytes_auto(
    _relative_path: &str,
    _bytes: &[u8],
    _session: &mut PrivacyScanSession,
    _limits: &ScanLimits,
) {
    // implemented in P2
}
