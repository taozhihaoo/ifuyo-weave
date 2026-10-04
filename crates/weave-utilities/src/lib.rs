//! weave-utilities — 通用工具域（M9 上 §16-§104）。
//!
//! 八大工具的**语义层**：Hash / Checksum / Base64 / UUID / Timestamp /
//! URL / Regex / Color。全部确定性（§98；UUID 随机 = 显式工具语义例外
//! §98）、结构化输入输出、结构化错误（§22 → Weave Error 映射）、资源
//! 有界（§72/§97）。文件哈希复用 M1 流式能力（§5/§28 不建第二引擎）；
//! 正则替换复用 M4 Text Transformer（§71）；核心语义全在 Rust（§18）。

pub mod base64;
pub mod checksum;
pub mod color;
pub mod file_digest;
pub mod hash;
pub mod regex_tool;
pub mod timestamp;
pub mod url_tool;
pub mod uuid_tool;

pub use base64::{Base64Alphabet, Base64DecodeLimits, Base64Padding, base64_decode, base64_encode};
pub use checksum::{ChecksumAlgorithm, adler32, checksum_bytes, crc32_iso_hdlc, crc32c};
pub use color::{
    ColorHsl, ColorHsv, ColorHwb, ColorInput, ColorRgb, color_contrast, parse_color, parse_hex,
    rgb_to_hsl, rgb_to_hsv, to_hex,
};
pub use file_digest::{
    Crc32Incremental, ExportFormat, FileChecksumEntry, checksum_file, export_report,
};
pub use hash::{
    HashAlgorithmInfo, TextHashAlgorithm, TextHashResult, hash_algorithm_matrix, hash_text,
    require_algorithm,
};
pub use regex_tool::{
    RegexFlag, RegexLimits, RegexMatchInfo, regex_capability_matrix, regex_find, regex_replace,
};
pub use timestamp::{
    Clock, SystemClock, TimestampConversion, TimestampUnit, now_utc, parse_timestamp,
};
pub use url_tool::{
    UrlPart, url_decode_component, url_decode_query, url_encode_component, url_encode_query,
    url_parse,
};
pub use uuid_tool::{
    UuidBatchLimits, UuidFormat, UuidInfo, UuidVersion, uuid_generate, uuid_validate,
};

use serde::{Deserialize, Serialize};

/// 工具错误（§22：结构化，映射到 Weave Error 由 IPC 层完成）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtilityError {
    pub code: &'static str,
    pub kind: UtilityErrorKind,
    pub message: String,
    pub recoverable: bool,
}

impl std::fmt::Display for UtilityError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.code, self.message)
    }
}

/// §22 错误类别（有界枚举，不吞内部细节）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UtilityErrorKind {
    InvalidInput,
    UnsupportedEncoding,
    UnsupportedAlgorithm,
    InvalidPattern,
    InvalidTimestamp,
    InvalidTimezone,
    InvalidUrl,
    InvalidColor,
    ResourceLimitExceeded,
    InternalError,
}

impl UtilityError {
    pub fn new(kind: UtilityErrorKind, code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            kind,
            message: message.into(),
            recoverable: true,
        }
    }

    pub fn invalid_input(code: &'static str, message: impl Into<String>) -> Self {
        Self::new(UtilityErrorKind::InvalidInput, code, message)
    }
}

type UResult<T> = Result<T, UtilityError>;

/// 诊断（§23：Warning ≠ Error——如 MD5 legacy 用途）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UtilityDiagnostic {
    /// info | warning
    pub severity: String,
    pub code: String,
    pub message: String,
}

pub(crate) fn warn(code: &str, message: impl Into<String>) -> UtilityDiagnostic {
    UtilityDiagnostic {
        severity: "warning".into(),
        code: code.into(),
        message: message.into(),
    }
}
