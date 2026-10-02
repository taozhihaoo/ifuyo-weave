//! 稳定、唯一、可序列化的标识类型（charter #20 / #24）。

use serde::{Deserialize, Serialize};
use std::fmt;
use std::sync::atomic::{AtomicU64, Ordering};

/// 工具类别（charter #4 的产品结构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ToolCategory {
    Files,
    Text,
    Data,
    Documents,
    Media,
    Batch,
    Utilities,
}

impl ToolCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ToolCategory::Files => "files",
            ToolCategory::Text => "text",
            ToolCategory::Data => "data",
            ToolCategory::Documents => "documents",
            ToolCategory::Media => "media",
            ToolCategory::Batch => "batch",
            ToolCategory::Utilities => "utilities",
        }
    }

    pub fn from_slug(slug: &str) -> Option<Self> {
        Some(match slug {
            "files" => ToolCategory::Files,
            "text" => ToolCategory::Text,
            "data" => ToolCategory::Data,
            "documents" => ToolCategory::Documents,
            "media" => ToolCategory::Media,
            "batch" => ToolCategory::Batch,
            "utilities" => ToolCategory::Utilities,
            _ => return None,
        })
    }
}

/// 工具接受的输入类型（charter #24 `inputTypes`）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum InputKind {
    FilePath,
    DirectoryPath,
    MultipleFilePaths,
    Text,
}

fn validate_identifier_segment(segment: &str, what: &str) -> Result<(), String> {
    if segment.is_empty() {
        return Err(format!("{what} segment must not be empty"));
    }
    if segment.len() > 64 {
        return Err(format!("{what} segment must be at most 64 characters"));
    }
    let first = segment.chars().next().expect("segment is not empty");
    if !first.is_ascii_lowercase() {
        return Err(format!(
            "{what} segment must start with a lowercase ASCII letter"
        ));
    }
    if !segment
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
    {
        return Err(format!(
            "{what} segment may only contain lowercase ASCII letters, digits and '_'"
        ));
    }
    Ok(())
}

fn stable_id_from_str(raw: &str, prefix: &str, what: &str) -> Result<String, String> {
    let body = raw
        .strip_prefix(prefix)
        .ok_or_else(|| format!("{what} must start with '{prefix}'"))?;
    if body.is_empty() {
        return Err(format!("{what} must not be empty after '{prefix}'"));
    }
    if body.len() > 64 {
        return Err(format!("{what} body must be at most 64 characters"));
    }
    if !body
        .chars()
        .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase() || c == '_')
    {
        return Err(format!(
            "{what} body may only contain lowercase ASCII letters, digits and '_'"
        ));
    }
    Ok(raw.to_string())
}

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

fn generate_id_body(prefix: &str) -> String {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let counter = ID_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}{nanos:012x}{counter:04x}")
}

macro_rules! stable_id {
    ($name:ident, $prefix:literal, $what:literal) => {
        #[doc = concat!("Stable identifier with prefix `", $prefix, "`.")]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub const PREFIX: &'static str = $prefix;

            #[doc = concat!("Creates a new ", $what, " with a generated unique body.")]
            pub fn generate() -> Self {
                Self(generate_id_body(Self::PREFIX))
            }

            #[doc = concat!("Parses an existing ", $what, " from its string form.")]
            pub fn parse(raw: &str) -> Result<Self, String> {
                stable_id_from_str(raw, Self::PREFIX, $what).map(Self)
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl TryFrom<String> for $name {
            type Error = String;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::parse(&value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> String {
                value.0
            }
        }
    };
}

stable_id!(OperationId, "op_", "operation id");
stable_id!(JobId, "job_", "job id");

/// 机器可读、稳定、适合持久化的工具 ID：`"<category>.<name>"`（charter #24）。
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct ToolId(String);

impl ToolId {
    /// 校验并构造工具 ID。第一段必须是合法类别 slug，第二段是 `[a-z][a-z0-9_]*`。
    pub fn parse(raw: &str) -> Result<Self, String> {
        let (category, name) = raw
            .split_once('.')
            .ok_or_else(|| "tool id must be '<category>.<name>'".to_string())?;
        ToolCategory::from_slug(category)
            .ok_or_else(|| format!("unknown tool category '{category}'"))?;
        validate_identifier_segment(name, "tool id name")
            .map_err(|e| format!("invalid tool id '{raw}': {e}"))?;
        Ok(Self(raw.to_string()))
    }

    pub fn new(category: ToolCategory, name: &str) -> Result<Self, String> {
        Self::parse(&format!("{}.{}", category.as_str(), name))
    }

    pub fn category(&self) -> ToolCategory {
        let (category, _) = self
            .0
            .split_once('.')
            .expect("ToolId is validated to contain '.'");
        ToolCategory::from_slug(category).expect("ToolId category is validated")
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ToolId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl TryFrom<String> for ToolId {
    type Error = String;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        ToolId::parse(&value)
    }
}

impl From<ToolId> for String {
    fn from(value: ToolId) -> String {
        value.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_id_accepts_valid_ids() {
        for raw in [
            "files.rename",
            "text.compare",
            "data.csv",
            "media.batch_image",
        ] {
            let id = ToolId::parse(raw).expect("valid tool id");
            assert_eq!(id.as_str(), raw);
        }
    }

    #[test]
    fn tool_id_rejects_invalid_ids() {
        for raw in [
            "",
            "rename",
            "wrongcategory.rename",
            "files.",
            "files.Rename",
            "files.rename-x",
            ".rename",
            "files.rename.extra",
        ] {
            assert!(ToolId::parse(raw).is_err(), "expected rejection: {raw}");
        }
    }

    #[test]
    fn tool_id_serializes_as_plain_string() {
        let id = ToolId::parse("files.inspect").expect("valid");
        let json = serde_json::to_string(&id).expect("serialize");
        assert_eq!(json, "\"files.inspect\"");
        let back: ToolId = serde_json::from_str(&json).expect("deserialize");
        assert_eq!(back, id);
    }

    #[test]
    fn tool_id_category_is_derived() {
        let id = ToolId::new(ToolCategory::Media, "resize").expect("valid");
        assert_eq!(id.category(), ToolCategory::Media);
    }

    #[test]
    fn operation_id_generation_is_unique_and_prefixed() {
        let a = OperationId::generate();
        let b = OperationId::generate();
        assert!(a.as_str().starts_with("op_"));
        assert_ne!(a, b);
    }

    #[test]
    fn operation_id_parses_existing_value() {
        let id = OperationId::parse("op_deadbeef0001").expect("valid");
        assert_eq!(id.to_string(), "op_deadbeef0001");
        assert!(OperationId::parse("wrong_deadbeef").is_err());
        assert!(OperationId::parse("op_").is_err());
        assert!(OperationId::parse("op_UPPER").is_err());
    }

    #[test]
    fn job_id_round_trips_through_serde() {
        let id = JobId::parse("job_cafe0001").expect("valid");
        let back: JobId =
            serde_json::from_str(&serde_json::to_string(&id).expect("ser")).expect("de");
        assert_eq!(back, id);
    }
}
