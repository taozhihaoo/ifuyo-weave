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
    fn error_implements_std_error() {
        fn assert_std_error<E: std::error::Error>(_: &E) {}
        assert_std_error(&WeaveError::internal("internal.boom", "boom"));
    }
}
