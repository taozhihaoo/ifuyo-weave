//! UUID（M9 上 §44-§50）：Generate（v4/v7，CSPRNG §46/§47）、Validate/
//! Parse（版本/variant）、Batch（§48 有界 1..=10_000）、Formats（§49）。

use uuid::Uuid;

use crate::{UResult, UtilityError, UtilityErrorKind};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UuidVersion {
    V4,
    V7,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum UuidFormat {
    /// 8-4-4-4-12 小写（默认）。
    LowerHyphenated,
    UpperHyphenated,
    /// 32 位十六进制无连字符（§49：必须明确）。
    Compact,
    /// {xxxxxxxx-...} 大括号。
    Braces,
}

/// §48 批量上限（防 UI/内存耗尽）。
#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UuidBatchLimits {
    pub max_count: f64,
}

impl Default for UuidBatchLimits {
    fn default() -> Self {
        Self {
            max_count: 10_000.0,
        }
    }
}

/// 单条结果（§49/§50：value + 解析事实）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UuidInfo {
    /// 按请求 format 格式化的值。
    pub value: String,
    /// 标准小写连字符形态（§98 展示归一）。
    pub canonical: String,
    /// v4 | v7（解析自位字段，§50）。
    pub version: String,
    /// RFC 4122 variant（解析自位字段）。
    pub variant: String,
}

fn format_uuid(u: Uuid, format: UuidFormat) -> String {
    match format {
        UuidFormat::LowerHyphenated => u.hyphenated().to_string(),
        UuidFormat::UpperHyphenated => u.hyphenated().to_string().to_uppercase(),
        UuidFormat::Compact => u.simple().to_string(),
        UuidFormat::Braces => format!("{{{}}}", u.hyphenated()),
    }
}

fn describe(u: Uuid) -> (String, String) {
    let version = match u.get_version_num() {
        4 => "v4",
        7 => "v7",
        other => return (format!("v{other}"), "unknown".into()),
    };
    // variant：RFC 4122 = 高两位 10
    let variant = match u.get_variant() {
        uuid::Variant::RFC4122 => "RFC 4122".to_owned(),
        other => format!("{other:?}"),
    };
    (version.to_owned(), variant)
}

/// §46/§47：v4/v7 都走 uuid crate（CSPRNG getrandom；v7 时间戳 + 随机）。
pub fn uuid_generate(
    version: UuidVersion,
    count: f64,
    format: UuidFormat,
    limits: &UuidBatchLimits,
) -> UResult<Vec<UuidInfo>> {
    if !(1.0..=limits.max_count).contains(&count) {
        return Err(UtilityError::new(
            UtilityErrorKind::ResourceLimitExceeded,
            "uuid.countOutOfRange",
            format!("count must be 1..={}", limits.max_count),
        ));
    }
    let count = count as usize;
    let mut out = Vec::with_capacity(count);
    for _ in 0..count {
        let u = match version {
            UuidVersion::V4 => Uuid::new_v4(),
            UuidVersion::V7 => Uuid::now_v7(),
        };
        let (ver, variant) = describe(u);
        out.push(UuidInfo {
            value: format_uuid(u, format),
            canonical: u.hyphenated().to_string(),
            version: ver,
            variant,
        });
    }
    Ok(out)
}

/// §50 Validate/Parse：输入任意 §49 形态（compact/braces 也接受）。
pub fn uuid_validate(input: &str) -> UResult<UuidInfo> {
    let trimmed = input.trim();
    let parsed = Uuid::parse_str(trimmed).map_err(|e| {
        UtilityError::invalid_input(
            "uuid.invalid",
            format!("'{input}' is not a valid UUID: {e}"),
        )
    })?;
    let (version, variant) = describe(parsed);
    if version == "unknown" {
        return Err(UtilityError::invalid_input(
            "uuid.unknownVersion",
            format!("UUID version bits indicate an unsupported version ({version})"),
        ));
    }
    Ok(UuidInfo {
        value: parsed.hyphenated().to_string(),
        canonical: parsed.hyphenated().to_string(),
        version,
        variant,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v4_is_rfc4122_with_csprng_uniqueness() {
        let infos = uuid_generate(
            UuidVersion::V4,
            100.0,
            UuidFormat::LowerHyphenated,
            &UuidBatchLimits::default(),
        )
        .expect("ok");
        assert_eq!(infos.len(), 100);
        // §98 例外：随机性是工具语义——但唯一性与位字段必须成立
        let set: std::collections::HashSet<&String> = infos.iter().map(|i| &i.canonical).collect();
        assert_eq!(set.len(), 100, "v4 100 条不得碰撞（概率 2^-2600）");
        for info in &infos {
            assert_eq!(info.version, "v4");
            assert_eq!(info.variant, "RFC 4122");
        }
    }

    #[test]
    fn v7_is_ordered_and_time_based() {
        // §47：v7 = 时间戳前缀 + 随机；同批单调不降（同 ms 内 rand_a 递增由 crate 保证）
        let infos = uuid_generate(
            UuidVersion::V7,
            50.0,
            UuidFormat::LowerHyphenated,
            &UuidBatchLimits::default(),
        )
        .expect("ok");
        for info in &infos {
            assert_eq!(info.version, "v7");
            assert_eq!(info.variant, "RFC 4122");
        }
        let mut canonicals: Vec<&str> = infos.iter().map(|i| i.canonical.as_str()).collect();
        canonicals.sort();
        // v7 生成序 = 字典序（时间前缀）——排序后与原序一致
        let original: Vec<&str> = infos.iter().map(|i| i.canonical.as_str()).collect();
        assert_eq!(canonicals, original, "v7 必须时间有序（§47）");
    }

    #[test]
    fn formats_roundtrip() {
        let infos = uuid_generate(
            UuidVersion::V4,
            1.0,
            UuidFormat::UpperHyphenated,
            &UuidBatchLimits::default(),
        )
        .expect("ok");
        let upper = &infos[0].value;
        assert_eq!(upper, &upper.to_uppercase());
        assert!(upper.contains('-'));
        let compact = uuid_generate(
            UuidVersion::V4,
            1.0,
            UuidFormat::Compact,
            &UuidBatchLimits::default(),
        )
        .expect("ok");
        assert!(!compact[0].value.contains('-'));
        assert_eq!(compact[0].value.len(), 32);
        let braces = uuid_generate(
            UuidVersion::V4,
            1.0,
            UuidFormat::Braces,
            &UuidBatchLimits::default(),
        )
        .expect("ok");
        assert!(braces[0].value.starts_with('{') && braces[0].value.ends_with('}'));
    }

    #[test]
    fn validate_accepts_all_formats_and_rejects_garbage() {
        // §50：接受 compact/braces/大写
        let info = uuid_validate("3D9C9DB6-9072-4F59-A04E-2CBEBF9A6F4B").expect("ok");
        assert_eq!(info.version, "v4");
        let info = uuid_validate("0192f0e2-7b7a-7cce-9f3a-9d9c1d1e0f3a").expect("v7 parse");
        assert_eq!(info.version, "v7");
        let e = uuid_validate("not-a-uuid").expect_err("garbage");
        assert_eq!(e.code, "uuid.invalid");
    }

    #[test]
    fn batch_is_bounded() {
        let e = uuid_generate(
            UuidVersion::V4,
            1_000_001.0,
            UuidFormat::LowerHyphenated,
            &UuidBatchLimits::default(),
        )
        .expect_err("too many");
        assert_eq!(e.kind, UtilityErrorKind::ResourceLimitExceeded);
    }
}
