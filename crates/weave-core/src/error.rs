//! 统一错误模型（charter #25 / M0 §17）。
//!
//! 所有错误必须携带 `Code / Message / Location / Recoverability / Suggestion`
//! 五个结构化字段，禁止只返回 "Something went wrong"。
//! 领域错误不夹杂 UI 文案：`Domain Error → Application Translation → UI Message`。

use serde::{Deserialize, Serialize};
use std::fmt;

/// 错误类别（M0 §17）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ErrorKind {
    Validation,
    Io,
    Permission,
    Conflict,
    Cancelled,
    Unsupported,
    Internal,
}

/// 错误的可恢复性（charter #25）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Recoverability {
    /// 重试同一操作可能成功。
    Retryable,
    /// 不可恢复，只能放弃。
    Fatal,
    /// 需要用户决策后才能继续。
    UserActionRequired,
}

/// Weave 统一结构化错误。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeaveError {
    pub kind: ErrorKind,
    /// 机器可读错误码，如 `"path.parentTraversal"`。
    pub code: String,
    /// 开发者可读消息；不含 UI 文案。
    pub message: String,
    pub location: Option<String>,
    pub recoverability: Recoverability,
    pub suggestion: Option<String>,
}

impl WeaveError {
    fn new(kind: ErrorKind, code: &str, message: impl Into<String>) -> Self {
        Self {
            kind,
            code: code.to_string(),
            message: message.into(),
            location: None,
            recoverability: Recoverability::Fatal,
            suggestion: None,
        }
    }

    pub fn validation(code: &str, message: impl Into<String>) -> Self {
        let mut e = Self::new(ErrorKind::Validation, code, message);
        e.recoverability = Recoverability::UserActionRequired;
        e
    }

    pub fn io(code: &str, message: impl Into<String>) -> Self {
        let mut e = Self::new(ErrorKind::Io, code, message);
        e.recoverability = Recoverability::Retryable;
        e
    }

    pub fn permission(code: &str, message: impl Into<String>) -> Self {
        let mut e = Self::new(ErrorKind::Permission, code, message);
        e.recoverability = Recoverability::UserActionRequired;
        e
    }

    pub fn conflict(code: &str, message: impl Into<String>) -> Self {
        let mut e = Self::new(ErrorKind::Conflict, code, message);
        e.recoverability = Recoverability::UserActionRequired;
        e
    }

    pub fn cancelled(code: &str, message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Cancelled, code, message)
    }

    pub fn unsupported(code: &str, message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Unsupported, code, message)
    }

    pub fn internal(code: &str, message: impl Into<String>) -> Self {
        Self::new(ErrorKind::Internal, code, message)
    }

    pub fn with_location(mut self, location: impl Into<String>) -> Self {
        self.location = Some(location.into());
        self
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        self.suggestion = Some(suggestion.into());
        self
    }

    pub fn with_recoverability(mut self, recoverability: Recoverability) -> Self {
        self.recoverability = recoverability;
        self
    }

    /// io 错误 → 结构化错误（M2.0 §14）。code = `{base}.` + 分类后缀：
    /// `sourceNotFound` / `permissionDenied` / `fileLocked` / `insufficientSpace` /
    /// `filesystemError`。locked/space/permission/missing 需要 用户介入或
    /// 重新计划 ⇒ recoverability = UserActionRequired；其余 Retryable。
    pub fn from_io(e: &std::io::Error, base_code: &str, message: impl Into<String>) -> Self {
        let class = classify_io_error(e);
        let suffix = match class {
            IoErrorClass::NotFound => "sourceNotFound",
            IoErrorClass::Permission => "permissionDenied",
            IoErrorClass::Locked => "fileLocked",
            IoErrorClass::InsufficientSpace => "insufficientSpace",
            IoErrorClass::Other => "filesystemError",
        };
        let mut err = WeaveError::io(
            &format!("{base_code}.{suffix}"),
            format!("{}: {}", message.into(), e),
        );
        if class != IoErrorClass::Other {
            err.recoverability = Recoverability::UserActionRequired;
        }
        err
    }
}

impl fmt::Display for WeaveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "[{}:{}] {}", self.kind_name(), self.code, self.message)?;
        if let Some(location) = &self.location {
            write!(f, " (at {location})")?;
        }
        Ok(())
    }
}

/// 平台 io 错误的可识别分类（M2.0 §14）：locked / 空间不足等不再笼统
/// 归入 `filesystemError`，调用方据此给出精确错误码与恢复建议。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IoErrorClass {
    NotFound,
    Permission,
    /// 文件被占用（Windows ERROR_SHARING_VIOLATION / ERROR_LOCK_VIOLATION）。
    Locked,
    /// 磁盘空间不足（Windows ERROR_DISK_FULL / POSIX ENOSPC）。
    InsufficientSpace,
    Other,
}

/// 依据 std io 错误分类：原始 OS 错误码优先于粗粒度 ErrorKind。
pub fn classify_io_error(e: &std::io::Error) -> IoErrorClass {
    match e.raw_os_error() {
        // Windows: 32 ERROR_SHARING_VIOLATION / 33 ERROR_LOCK_VIOLATION
        Some(32) | Some(33) => return IoErrorClass::Locked,
        // Windows: 112 ERROR_DISK_FULL；POSIX: 28 ENOSPC
        Some(112) | Some(28) => return IoErrorClass::InsufficientSpace,
        _ => {}
    }
    match e.kind() {
        std::io::ErrorKind::NotFound => IoErrorClass::NotFound,
        std::io::ErrorKind::PermissionDenied => IoErrorClass::Permission,
        std::io::ErrorKind::StorageFull => IoErrorClass::InsufficientSpace,
        _ => IoErrorClass::Other,
    }
}

impl std::error::Error for WeaveError {}

impl WeaveError {
    fn kind_name(&self) -> &'static str {
        match self.kind {
            ErrorKind::Validation => "validation",
            ErrorKind::Io => "io",
            ErrorKind::Permission => "permission",
            ErrorKind::Conflict => "conflict",
            ErrorKind::Cancelled => "cancelled",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::Internal => "internal",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_carries_all_five_structured_fields() {
        let e = WeaveError::validation("path.parentTraversal", "path escapes the allowed root")
            .with_location("Preview.plan")
            .with_suggestion("choose a path inside the workspace");
        assert_eq!(e.kind, ErrorKind::Validation);
        assert_eq!(e.code, "path.parentTraversal");
        assert!(e.location.is_some());
        assert!(e.suggestion.is_some());
        assert_eq!(e.recoverability, Recoverability::UserActionRequired);
    }

    #[test]
    fn error_serializes_with_camel_case_fields() {
        let e = WeaveError::io("io.readFailed", "cannot read file");
        let json = serde_json::to_value(&e).expect("serialize");
        for key in [
            "kind",
            "code",
            "message",
            "location",
            "recoverability",
            "suggestion",
        ] {
            assert!(json.get(key).is_some(), "missing field {key}");
        }
        assert_eq!(json["recoverability"], "retryable");
    }

    #[test]
    fn error_display_is_machine_and_human_readable() {
        let e = WeaveError::conflict("rename.targetExists", "target already exists");
        let text = e.to_string();
        assert!(text.contains("[conflict:rename.targetExists]"));
    }

    #[test]
    fn io_classification_maps_precise_codes_and_recoverability() {
        // Windows sharing violation（32）→ fileLocked + 需要用户介入
        let locked = WeaveError::from_io(
            &std::io::Error::from_raw_os_error(32),
            "rename",
            "rename failed",
        );
        assert_eq!(locked.code, "rename.fileLocked");
        assert_eq!(locked.kind, ErrorKind::Io);
        assert_eq!(locked.recoverability, Recoverability::UserActionRequired);
        assert!(locked.message.contains("rename failed"));

        // 磁盘满（112）→ insufficientSpace
        let full = WeaveError::from_io(
            &std::io::Error::from_raw_os_error(112),
            "move",
            "copy failed",
        );
        assert_eq!(full.code, "move.insufficientSpace");

        // POSIX ENOSPC（28）同样识别
        let enospac = WeaveError::from_io(
            &std::io::Error::from_raw_os_error(28),
            "move",
            "copy failed",
        );
        assert_eq!(enospac.code, "move.insufficientSpace");

        // NotFound → sourceNotFound（保持既有约定），需要用户重新计划
        let missing = WeaveError::from_io(
            &std::io::Error::from(std::io::ErrorKind::NotFound),
            "rename",
            "stat failed",
        );
        assert_eq!(missing.code, "rename.sourceNotFound");
        assert_eq!(missing.recoverability, Recoverability::UserActionRequired);

        // 其他错误 → filesystemError + Retryable（保持既有默认）
        let other = WeaveError::from_io(&std::io::Error::other("boom"), "rename", "io");
        assert_eq!(other.code, "rename.filesystemError");
        assert_eq!(other.recoverability, Recoverability::Retryable);
    }

    #[test]
    fn error_implements_std_error() {
        fn assert_std_error<E: std::error::Error>(_: &E) {}
        assert_std_error(&WeaveError::internal("internal.boom", "boom"));
    }
}
