//! Timestamp（M9 上 §51-§58/§100-§101）：显式单位（§53 不靠位数盲猜）、
//! RFC 3339（§54）、UTC + Fixed Offset + System Local（§55；Named
//! Timezone/DST ⇒ NOT SUPPORTED 如实声明 §56）、Clock trait（§100/§101
//! 可注入）。

use time::OffsetDateTime;
use time::format_description::well_known::Rfc3339;

use crate::{UResult, UtilityError, UtilityErrorKind};

/// §100/§101：Clock trait（生产 SystemClock / 测试 FixedClock）。
pub trait Clock: Send + Sync {
    fn now(&self) -> OffsetDateTime;
}

pub struct SystemClock;

impl Clock for SystemClock {
    fn now(&self) -> OffsetDateTime {
        OffsetDateTime::now_utc()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TimestampUnit {
    Seconds,
    Milliseconds,
    Microseconds,
    Nanoseconds,
}

impl TimestampUnit {
    /// §52：各单位的可靠范围（纳秒超 i64 精度边界如实处理——只接受
    /// 可无损表达的值）。
    fn to_unix_nanos(self, value: f64) -> Option<i128> {
        let scaled = match self {
            TimestampUnit::Seconds => value * 1_000_000_000.0,
            TimestampUnit::Milliseconds => value * 1_000_000.0,
            TimestampUnit::Microseconds => value * 1_000.0,
            TimestampUnit::Nanoseconds => value,
        };
        if !scaled.is_finite() || scaled.abs() >= 9.3e18 {
            return None;
        }
        Some(scaled as i128)
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TimestampConversion {
    /// Unix 秒（浮点，含小数部分）。
    pub unix_seconds: f64,
    /// RFC 3339 UTC（§54 核心=exact datetime）。
    pub utc_rfc3339: String,
    /// Fixed offset 变体（offset_minutes 由调用方给定；None = 未请求）。
    pub with_offset: Option<String>,
    /// §58 Relative 仅 presentation——独立字段（不参与 exact 结果）。
    pub relative_hint: Option<String>,
    /// §53 auto-detect 判定（显式单位时为 None）。
    pub detected_unit: Option<String>,
}

fn to_datetime(unix_nanos: i128) -> UResult<OffsetDateTime> {
    OffsetDateTime::from_unix_timestamp_nanos(unix_nanos).map_err(|e| {
        UtilityError::new(
            UtilityErrorKind::InvalidTimestamp,
            "timestamp.outOfRange",
            format!("value out of representable range: {e}"),
        )
    })
}

fn rfc3339_utc(dt: &OffsetDateTime) -> String {
    // §54：核心结果一律归一 UTC 展示（带偏移输入亦然）
    dt.to_offset(time::UtcOffset::UTC)
        .format(&Rfc3339)
        .unwrap_or_else(|_| dt.to_string())
}

/// 数值 → 转换（unit 显式指定，§53）。
pub fn convert_number(value: f64, unit: TimestampUnit) -> UResult<TimestampConversion> {
    let nanos = unit.to_unix_nanos(value).ok_or_else(|| {
        UtilityError::new(
            UtilityErrorKind::InvalidTimestamp,
            "timestamp.outOfRange",
            format!("{value} {unit:?} exceeds reliable range (§52)"),
        )
    })?;
    let dt = to_datetime(nanos)?;
    let seconds = value
        / match unit {
            TimestampUnit::Seconds => 1.0,
            TimestampUnit::Milliseconds => 1_000.0,
            TimestampUnit::Microseconds => 1_000_000.0,
            TimestampUnit::Nanoseconds => 1_000_000_000.0,
        };
    Ok(TimestampConversion {
        unix_seconds: seconds,
        utc_rfc3339: rfc3339_utc(&dt),
        with_offset: None,
        relative_hint: None,
        detected_unit: None,
    })
}

/// §53 Auto detect：必须显示判定结果（不静默）。
/// 规则：整数且 < 1e11 ⇒ 秒；< 1e14 ⇒ 毫秒；< 1e17 ⇒ 微秒；否则纳秒。
/// 非整数（含小数部分）⇒ 视为秒。
pub fn auto_detect_unit(value: f64) -> TimestampUnit {
    if value.fract() != 0.0 {
        return TimestampUnit::Seconds;
    }
    let magnitude = value.abs();
    if magnitude < 1e11 {
        TimestampUnit::Seconds
    } else if magnitude < 1e14 {
        TimestampUnit::Milliseconds
    } else if magnitude < 1e17 {
        TimestampUnit::Microseconds
    } else {
        TimestampUnit::Nanoseconds
    }
}

/// RFC 3339 / ISO 8601 兼容形态（§54：明确 UTC 语义）→ Unix 秒。
pub fn parse_rfc3339(input: &str) -> UResult<TimestampConversion> {
    let dt = OffsetDateTime::parse(input.trim(), &Rfc3339).map_err(|e| {
        UtilityError::new(
            UtilityErrorKind::InvalidTimestamp,
            "timestamp.invalidRfc3339",
            format!("'{input}' is not a valid RFC 3339 timestamp: {e}"),
        )
    })?;
    let unix_seconds = dt.unix_timestamp() as f64 + dt.nanosecond() as f64 / 1e9;
    Ok(TimestampConversion {
        unix_seconds,
        utc_rfc3339: rfc3339_utc(&dt),
        with_offset: None,
        relative_hint: None,
        detected_unit: None,
    })
}

/// §55 Fixed offset 变体展示（分钟；DST 语义不适用 = 显式固定偏移）。
pub fn with_fixed_offset(unix_seconds: f64, offset_minutes: i32) -> UResult<String> {
    let nanos = (unix_seconds * 1e9) as i128;
    let dt = to_datetime(nanos)?;
    let offset =
        time::UtcOffset::from_hms((offset_minutes / 60) as i8, (offset_minutes % 60) as i8, 0)
            .map_err(|e| {
                UtilityError::new(
                    UtilityErrorKind::InvalidTimezone,
                    "timestamp.invalidOffset",
                    format!("offset out of range: {e}"),
                )
            })?;
    let local = dt.to_offset(offset);
    Ok(local.format(&Rfc3339).unwrap_or_else(|_| local.to_string()))
}

/// §101 Now（Clock 注入；§100 测试不断言精确输出）。
pub fn now_utc(clock: &dyn Clock) -> TimestampConversion {
    let dt = clock.now();
    TimestampConversion {
        unix_seconds: dt.unix_timestamp() as f64 + dt.nanosecond() as f64 / 1e9,
        utc_rfc3339: rfc3339_utc(&dt),
        with_offset: None,
        relative_hint: None,
        detected_unit: None,
    }
}

/// 统一入口：数值（unit 或 auto）或 RFC 3339 字符串。
pub fn parse_timestamp(
    input: &str,
    unit: Option<TimestampUnit>,
    offset_minutes: Option<i32>,
) -> UResult<TimestampConversion> {
    let trimmed = input.trim();
    let mut conversion = if let Ok(value) = trimmed.parse::<f64>() {
        match unit {
            Some(u) => convert_number(value, u)?,
            None => {
                let detected = auto_detect_unit(value);
                let mut c = convert_number(value, detected)?;
                c.detected_unit = Some(format!("{detected:?}"));
                c
            }
        }
    } else {
        parse_rfc3339(trimmed)?
    };
    if let Some(minutes) = offset_minutes {
        conversion.with_offset = Some(with_fixed_offset(conversion.unix_seconds, minutes)?);
    }
    Ok(conversion)
}

#[cfg(test)]
mod tests {
    use super::*;

    struct FixedClock(OffsetDateTime);
    impl Clock for FixedClock {
        fn now(&self) -> OffsetDateTime {
            self.0
        }
    }

    #[test]
    fn explicit_units_differ() {
        // §53：1720000000 秒 vs 1720000000000 毫秒
        let s = parse_timestamp("1720000000", Some(TimestampUnit::Seconds), None).expect("s");
        let ms =
            parse_timestamp("1720000000000", Some(TimestampUnit::Milliseconds), None).expect("ms");
        assert_eq!(s.utc_rfc3339, ms.utc_rfc3339, "同一时刻");
        assert!(s.utc_rfc3339.starts_with("2024-07-03"));
    }

    #[test]
    fn auto_detect_states_its_decision() {
        // §53：auto detect 必须显示判定
        let c = parse_timestamp("1720000000", None, None).expect("ok");
        assert_eq!(c.detected_unit.as_deref(), Some("Seconds"));
        let c = parse_timestamp("1720000000000", None, None).expect("ok");
        assert_eq!(c.detected_unit.as_deref(), Some("Milliseconds"));
        let c = parse_timestamp("1720000000.5", None, None).expect("ok");
        assert_eq!(c.detected_unit.as_deref(), Some("Seconds"), "小数 ⇒ 秒");
    }

    #[test]
    fn rfc3339_roundtrip_and_utc_semantics() {
        let c = parse_rfc3339("2026-10-03T00:00:00Z").expect("ok");
        assert!((c.unix_seconds - 1790985600.0).abs() < 1.0);
        // 带偏移的输入归一到 UTC
        let c = parse_rfc3339("2026-10-03T08:00:00+08:00").expect("ok");
        assert!(
            c.utc_rfc3339.starts_with("2026-10-03T00:00:00"),
            "§54 UTC 归一"
        );
        let e = parse_rfc3339("not a time").expect_err("bad");
        assert_eq!(e.code, "timestamp.invalidRfc3339");
    }

    #[test]
    fn fixed_offset_variant() {
        let c = parse_timestamp("0", Some(TimestampUnit::Seconds), Some(480)).expect("ok");
        assert_eq!(c.utc_rfc3339, "1970-01-01T00:00:00Z");
        assert_eq!(
            c.with_offset.as_deref(),
            Some("1970-01-01T08:00:00+08:00"),
            "§55 Fixed offset ≠ 本地 ≠ UTC"
        );
    }

    #[test]
    fn fixed_clock_injection() {
        // §100/§101：测试注入 FixedClock，不断言真实"现在"
        let fixed = FixedClock(to_datetime(1_720_000_000_000_000_000).expect("dt"));
        let c = now_utc(&fixed);
        assert_eq!(c.utc_rfc3339, "2024-07-03T09:46:40Z");
    }

    #[test]
    fn out_of_range_is_structured() {
        let e = parse_timestamp("1e30", Some(TimestampUnit::Nanoseconds), None).expect_err("range");
        assert_eq!(e.kind, UtilityErrorKind::InvalidTimestamp);
    }
}
