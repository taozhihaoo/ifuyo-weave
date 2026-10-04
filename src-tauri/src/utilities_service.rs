//! M9（上）：Utilities IPC（§17 分层：UI → Command → 域函数直调）。
//!
//! 全部为无状态同步命令（§91/§92：纯转换不进 History/Undo；§93 文件
//! Export 不在本批——文本工具 v1 只做剪贴板复制）。错误映射：域
//! UtilityError → IpcError（code/message 原样保留）。

use serde::{Deserialize, Serialize};
use specta::Type;
use weave_core::prelude::{TextEncoding, WeaveError};
use weave_utilities::{
    Base64Alphabet, Base64DecodeLimits, Base64Padding, ColorHsl, ColorHsv, ColorInput, ColorRgb,
    RegexFlag, RegexLimits, SystemClock, TimestampConversion, TimestampUnit, UuidBatchLimits,
    UuidFormat, UuidInfo, UuidVersion, base64_decode, base64_encode, color_contrast,
    hash_algorithm_matrix, hash_text, now_utc, parse_color, parse_timestamp,
    regex_capability_matrix, regex_find, regex_replace, url_decode_component, url_decode_query,
    url_encode_component, url_encode_query, url_parse, uuid_generate, uuid_validate,
};

use crate::commands::IpcError;

// IPC 边界与 D10 同理。
#[allow(clippy::result_large_err)] // D10
fn err(e: &weave_utilities::UtilityError) -> IpcError {
    WeaveError::validation(e.code, e.message.clone())
        .with_location("utilities_service")
        .into()
}

fn sys_clock() -> SystemClock {
    SystemClock
}

// ── Hash（§24-§31）──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HashWarningDto {
    pub code: String,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TextHashResultDto {
    pub algorithm: String,
    pub bytes_processed: f64,
    pub digest_hex: String,
    pub warnings: Vec<HashWarningDto>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct HashAlgorithmInfoDto {
    pub id: String,
    pub name: String,
    pub security_note: Option<String>,
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_hash_text(
    text: String,
    algorithm_id: String,
    upper: bool,
) -> Result<TextHashResultDto, IpcError> {
    let algorithm = weave_utilities::require_algorithm(&algorithm_id).map_err(|e| err(&e))?;
    let r = hash_text(&text, algorithm, upper).map_err(|e| err(&e))?;
    Ok(TextHashResultDto {
        algorithm: r.algorithm.as_str().to_owned(),
        bytes_processed: r.bytes_processed,
        digest_hex: r.digest_hex,
        warnings: r
            .warnings
            .iter()
            .map(|w| HashWarningDto {
                code: w.code.clone(),
                message: w.message.clone(),
            })
            .collect(),
    })
}

/// TextHashAlgorithm 无 as_str —— 借矩阵 id。
#[tauri::command]
#[specta::specta]
pub fn utilities_hash_algorithms() -> Vec<HashAlgorithmInfoDto> {
    hash_algorithm_matrix()
        .iter()
        .map(|i| HashAlgorithmInfoDto {
            id: format!("{:?}", i.id).to_lowercase(),
            name: i.name.to_owned(),
            security_note: i.security_note.map(|s| s.to_owned()),
        })
        .collect()
}

// ── Checksum（§32-§36）──

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_checksum(text: String, algorithm_id: String) -> Result<String, IpcError> {
    let algorithm = weave_utilities::ChecksumAlgorithm::parse(&algorithm_id).ok_or_else(|| {
        err(&weave_utilities::UtilityError::new(
            weave_utilities::UtilityErrorKind::UnsupportedAlgorithm,
            "checksum.unknownAlgorithm",
            format!("unknown checksum algorithm '{algorithm_id}'"),
        ))
    })?;
    let bytes = text.as_bytes();
    weave_utilities::checksum_bytes(algorithm, bytes).map_err(|e| err(&e))
}

// ── Base64（§37-§43）──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Base64DecodeResultDto {
    /// 解码字节的 hex（二进制安全呈现）。
    pub bytes_hex: String,
    /// lossy UTF-8 预览（二进制内容可能不可读——如实）。
    pub text_preview: String,
    pub byte_count: f64,
    pub warnings: Vec<String>,
}

#[allow(clippy::result_large_err)] // D10
fn encoding_from_id(id: &str) -> Result<TextEncoding, IpcError> {
    match id.to_ascii_lowercase().as_str() {
        "utf8" => Ok(TextEncoding::Utf8),
        "utf16le" => Ok(TextEncoding::Utf16Le),
        "utf16be" => Ok(TextEncoding::Utf16Be),
        "latin1" => Ok(TextEncoding::Latin1),
        other => Err(WeaveError::validation(
            "base64.encodingUnsupportedHere",
            format!("encoding '{other}' not supported here (GB family → Text tool, M4/D38)"),
        )
        .with_location("utilities_service")
        .into()),
    }
}

fn alphabet_from_id(id: &str) -> Base64Alphabet {
    if id.eq_ignore_ascii_case("urlsafe") {
        Base64Alphabet::UrlSafe
    } else {
        Base64Alphabet::Standard
    }
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_base64_encode(
    text: String,
    encoding_id: String,
    alphabet_id: String,
    omit_padding: bool,
) -> Result<String, IpcError> {
    let encoding = encoding_from_id(&encoding_id)?;
    let alphabet = alphabet_from_id(&alphabet_id);
    let padding = if omit_padding {
        Base64Padding::Omit
    } else {
        Base64Padding::Include
    };
    base64_encode(&text, encoding, alphabet, padding).map_err(|e| err(&e))
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_base64_decode(
    input: String,
    alphabet_id: String,
    lenient: bool,
) -> Result<Base64DecodeResultDto, IpcError> {
    let alphabet = alphabet_from_id(&alphabet_id);
    let (bytes, warnings) =
        base64_decode(&input, alphabet, lenient, &Base64DecodeLimits::default())
            .map_err(|e| err(&e))?;
    Ok(Base64DecodeResultDto {
        bytes_hex: bytes.iter().map(|b| format!("{b:02x}")).collect(),
        text_preview: String::from_utf8_lossy(&bytes).into_owned(),
        byte_count: bytes.len() as f64,
        warnings: warnings.iter().map(|w| w.message.clone()).collect(),
    })
}

// ── UUID（§44-§50）──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UuidInfoDto {
    pub value: String,
    pub canonical: String,
    pub version: String,
    pub variant: String,
}

fn uuid_dto(info: &UuidInfo) -> UuidInfoDto {
    UuidInfoDto {
        value: info.value.clone(),
        canonical: info.canonical.clone(),
        version: info.version.clone(),
        variant: info.variant.clone(),
    }
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_uuid_generate(
    version: String,
    count: f64,
    format: String,
) -> Result<Vec<UuidInfoDto>, IpcError> {
    let version = match version.to_ascii_lowercase().as_str() {
        "v4" | "4" => UuidVersion::V4,
        "v7" | "7" => UuidVersion::V7,
        other => {
            return Err(err(&weave_utilities::UtilityError::new(
                weave_utilities::UtilityErrorKind::UnsupportedAlgorithm,
                "uuid.unknownVersion",
                format!("unsupported UUID version '{other}' (v4/v7 per §45)"),
            )));
        }
    };
    let format = match format.to_ascii_lowercase().as_str() {
        "upper" => UuidFormat::UpperHyphenated,
        "compact" => UuidFormat::Compact,
        "braces" => UuidFormat::Braces,
        _ => UuidFormat::LowerHyphenated,
    };
    let infos =
        uuid_generate(version, count, format, &UuidBatchLimits::default()).map_err(|e| err(&e))?;
    Ok(infos.iter().map(uuid_dto).collect())
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_uuid_validate(input: String) -> Result<UuidInfoDto, IpcError> {
    uuid_validate(&input)
        .map(|i| uuid_dto(&i))
        .map_err(|e| err(&e))
}

// ── Timestamp（§51-§58/§101）──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TimestampConversionDto {
    pub unix_seconds: f64,
    pub utc_rfc3339: String,
    pub with_offset: Option<String>,
    pub detected_unit: Option<String>,
}

fn ts_dto(c: &TimestampConversion) -> TimestampConversionDto {
    TimestampConversionDto {
        unix_seconds: c.unix_seconds,
        utc_rfc3339: c.utc_rfc3339.clone(),
        with_offset: c.with_offset.clone(),
        detected_unit: c.detected_unit.clone(),
    }
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_timestamp_convert(
    input: String,
    unit: Option<String>,
    offset_minutes: Option<f64>,
) -> Result<TimestampConversionDto, IpcError> {
    let unit = unit.and_then(|u| match u.to_ascii_lowercase().as_str() {
        "seconds" | "s" => Some(TimestampUnit::Seconds),
        "milliseconds" | "ms" => Some(TimestampUnit::Milliseconds),
        "microseconds" | "us" => Some(TimestampUnit::Microseconds),
        "nanoseconds" | "ns" => Some(TimestampUnit::Nanoseconds),
        _ => None,
    });
    let offset = offset_minutes.map(|m| m as i32);
    parse_timestamp(&input, unit, offset)
        .map(|c| ts_dto(&c))
        .map_err(|e| err(&e))
}

#[tauri::command]
#[specta::specta]
pub fn utilities_timestamp_now() -> TimestampConversionDto {
    ts_dto(&now_utc(&sys_clock()))
}

// ── URL（§59-§64）──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UrlPartDto {
    pub scheme: String,
    pub userinfo: Option<String>,
    pub host: String,
    pub port: Option<f64>,
    pub path: String,
    pub query: Option<String>,
    pub fragment: Option<String>,
    pub query_pairs: Vec<(String, String)>,
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_url_encode(mode: String, input: String) -> Result<String, IpcError> {
    match mode.as_str() {
        "component" => Ok(url_encode_component(&input)),
        "query" => Ok(url_encode_query(&input)),
        other => Err(WeaveError::validation(
            "url.unknownMode",
            format!("encode mode must be 'component' or 'query', got '{other}'"),
        )
        .with_location("utilities_service")
        .into()),
    }
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_url_decode(
    mode: String,
    input: String,
    lenient: bool,
) -> Result<String, IpcError> {
    match mode.as_str() {
        "component" => url_decode_component(&input, lenient).map_err(|e| err(&e)),
        "query" => url_decode_query(&input, lenient).map_err(|e| err(&e)),
        other => Err(WeaveError::validation(
            "url.unknownMode",
            format!("decode mode must be 'component' or 'query', got '{other}'"),
        )
        .with_location("utilities_service")
        .into()),
    }
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_url_parse(input: String) -> Result<UrlPartDto, IpcError> {
    let p = url_parse(&input).map_err(|e| err(&e))?;
    Ok(UrlPartDto {
        scheme: p.scheme,
        userinfo: p.userinfo,
        host: p.host,
        port: p.port.map(|v| v as f64),
        path: p.path,
        query: p.query,
        fragment: p.fragment,
        query_pairs: p.query_pairs,
    })
}

// ── Regex（§65-§74）──

#[derive(Debug, Clone, Deserialize, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegexFlagDto {
    pub case_insensitive: bool,
    pub multi_line: bool,
    pub dot_matches_newline: bool,
    pub ignore_whitespace: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegexMatchDto {
    pub full: String,
    pub start: f64,
    pub end: f64,
    pub line: f64,
    pub column: f64,
    pub groups: Vec<Option<String>>,
    pub named_groups: Vec<(String, String)>,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegexCapabilityDto {
    pub feature: String,
    pub status: String,
    pub note: String,
}

fn flags_from_dto(f: &RegexFlagDto) -> RegexFlag {
    RegexFlag {
        case_insensitive: f.case_insensitive,
        multi_line: f.multi_line,
        dot_matches_newline: f.dot_matches_newline,
        ignore_whitespace: f.ignore_whitespace,
    }
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_regex_find(
    pattern: String,
    input: String,
    flags: RegexFlagDto,
) -> Result<Vec<RegexMatchDto>, IpcError> {
    let matches = regex_find(
        &pattern,
        &input,
        &flags_from_dto(&flags),
        &RegexLimits::default(),
    )
    .map_err(|e| err(&e))?;
    Ok(matches
        .iter()
        .map(|m| RegexMatchDto {
            full: m.full.clone(),
            start: m.start,
            end: m.end,
            line: m.line,
            column: m.column,
            groups: m.groups.clone(),
            named_groups: m.named_groups.clone(),
        })
        .collect())
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RegexReplaceResultDto {
    pub output: String,
    pub count: f64,
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_regex_replace(
    pattern: String,
    input: String,
    replacement: String,
    flags: RegexFlagDto,
) -> Result<RegexReplaceResultDto, IpcError> {
    let (output, count) = regex_replace(
        &pattern,
        &input,
        &replacement,
        &flags_from_dto(&flags),
        &RegexLimits::default(),
    )
    .map_err(|e| err(&e))?;
    Ok(RegexReplaceResultDto { output, count })
}

#[tauri::command]
#[specta::specta]
pub fn utilities_regex_capabilities() -> Vec<RegexCapabilityDto> {
    regex_capability_matrix()
        .iter()
        .map(|c| RegexCapabilityDto {
            feature: c.feature.to_owned(),
            status: c.status.to_owned(),
            note: c.note.to_owned(),
        })
        .collect()
}

// ── Color（§75-§88）──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ColorResultDto {
    /// 大写 HEX（§85/§104 展示归一；alpha < 1 时 8 位）。
    pub hex: String,
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
    pub h: f64,
    pub s_hsl: f64,
    pub l: f64,
    pub s_hsv: f64,
    pub v: f64,
    /// §88 WCAG 对白底对比度。
    pub contrast_on_white: f64,
    /// §88 WCAG 对黑底对比度。
    pub contrast_on_black: f64,
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ColorRequestDto {
    Hex { value: String },
    Rgb { r: f64, g: f64, b: f64, a: f64 },
    Hsl { h: f64, s: f64, l: f64, a: f64 },
    Hsv { h: f64, s: f64, v: f64, a: f64 },
    Hwb { h: f64, w: f64, b: f64, a: f64 },
}

fn color_dto(c: &ColorRgb) -> ColorResultDto {
    let hsl = weave_utilities::rgb_to_hsl(c);
    let hsv = weave_utilities::rgb_to_hsv(c);
    let white = ColorRgb {
        r: 255.0,
        g: 255.0,
        b: 255.0,
        a: 1.0,
    };
    let black = ColorRgb {
        r: 0.0,
        g: 0.0,
        b: 0.0,
        a: 1.0,
    };
    ColorResultDto {
        hex: weave_utilities::to_hex(c, c.a < 1.0),
        r: c.r,
        g: c.g,
        b: c.b,
        a: c.a,
        h: hsl.h,
        s_hsl: hsl.s,
        l: hsl.l,
        s_hsv: hsv.s,
        v: hsv.v,
        contrast_on_white: color_contrast(c, &white),
        contrast_on_black: color_contrast(c, &black),
    }
}

// ColorHsl/Hsv 别名导入仅为文档可读性；消除 unused 警告
#[allow(unused)]
fn _touch(_a: ColorHsl, _b: ColorHsv) {}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_color_convert(input: ColorRequestDto) -> Result<ColorResultDto, IpcError> {
    let domain_input = match input {
        ColorRequestDto::Hex { value } => ColorInput::Hex(value),
        ColorRequestDto::Rgb { r, g, b, a } => ColorInput::Rgb { r, g, b, a },
        ColorRequestDto::Hsl { h, s, l, a } => ColorInput::Hsl { h, s, l, a },
        ColorRequestDto::Hsv { h, s, v, a } => ColorInput::Hsv { h, s, v, a },
        ColorRequestDto::Hwb { h, w, b, a } => ColorInput::Hwb { h, w, b, a },
    };
    let rgb = parse_color(domain_input).map_err(|e| err(&e))?;
    Ok(color_dto(&rgb))
}

#[tauri::command]
#[specta::specta]
#[allow(clippy::result_large_err)] // D10
pub fn utilities_color_contrast(hex_a: String, hex_b: String) -> Result<f64, IpcError> {
    let a = weave_utilities::parse_hex(&hex_a).map_err(|e| err(&e))?;
    let b = weave_utilities::parse_hex(&hex_b).map_err(|e| err(&e))?;
    Ok(color_contrast(&a, &b))
}
