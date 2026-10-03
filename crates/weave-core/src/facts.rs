//! 文件事实契约（M1 §5.1）。
//!
//! 这些类型是 weave-files 实现与 IPC DTO 之间的稳定契约层：
//! 平台无关、可序列化、只描述事实。

use serde::{Deserialize, Serialize};

/// 文件系统条目的种类（M1 §6.3 推荐的结果模型）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FileKind {
    RegularFile,
    Directory,
    Symlink,
    Other,
}

/// 轻量级文本编码识别结果（M1 §9）。
///
/// M1 范围：BOM 优先 + UTF-8 有效性；GB18030/GBK/Latin-1 属 charter #27，
/// 推迟到 M4 Encoding Detection（见 DECISIONS.md 与 Known Limitations）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextEncoding {
    Utf8,
    Utf8Bom,
    Utf16Le,
    Utf16Be,
    Ascii,
    /// 无法可靠判断时如实报告 Unknown，不猜测（charter：不伪造）。
    Unknown,
}

/// 哈希算法（M1 §10：只实现当前确认需要的 SHA-256）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HashAlgorithm {
    Sha256,
}

impl HashAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            HashAlgorithm::Sha256 => "SHA-256",
        }
    }
}

/// 哈希结果状态。取消 ≠ 失败（M1 §10.4）；内容不稳定 ≠ 静默成功（M1 §10.3）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HashStatus {
    /// 完整读取且前后 stat 一致。
    Completed,
    /// 协作式取消，digest 不存在。
    Cancelled,
    /// 哈希期间文件发生变化：digest 是 best-effort，调用方必须看到警告。
    Unstable,
}

/// 哈希结果（M1 §10.1：algorithm / digest / bytes_processed / duration）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashResult {
    pub algorithm: HashAlgorithm,
    /// 小写十六进制 digest；Cancelled 时为 None。
    pub digest_hex: Option<String>,
    pub bytes_processed: u64,
    pub duration_ms: u64,
    pub status: HashStatus,
}

/// 目录扫描的宏观状态（M1 §33：不允许只有 success/error 二态）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ScanStatus {
    Completed,
    CompletedWithWarnings,
    Failed,
    Cancelled,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fact_enums_serialize_camel_case() {
        assert_eq!(
            serde_json::to_value(FileKind::RegularFile).expect("ser"),
            serde_json::json!("regularFile")
        );
        assert_eq!(
            serde_json::to_value(TextEncoding::Utf8Bom).expect("ser"),
            serde_json::json!("utf8Bom")
        );
        assert_eq!(
            serde_json::to_value(ScanStatus::CompletedWithWarnings).expect("ser"),
            serde_json::json!("completedWithWarnings")
        );
        assert_eq!(
            serde_json::to_value(HashStatus::Unstable).expect("ser"),
            serde_json::json!("unstable")
        );
    }

    #[test]
    fn hash_result_contract_fields() {
        let result = HashResult {
            algorithm: HashAlgorithm::Sha256,
            digest_hex: Some("ab".repeat(32)),
            bytes_processed: 8,
            duration_ms: 1,
            status: HashStatus::Completed,
        };
        let json = serde_json::to_value(&result).expect("ser");
        for key in [
            "algorithm",
            "digestHex",
            "bytesProcessed",
            "durationMs",
            "status",
        ] {
            assert!(json.get(key).is_some(), "missing {key}");
        }
    }

    #[test]
    fn hash_algorithm_display_is_stable() {
        assert_eq!(HashAlgorithm::Sha256.as_str(), "SHA-256");
    }
}
