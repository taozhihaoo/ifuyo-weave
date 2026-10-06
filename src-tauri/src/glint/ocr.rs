//! OCR（M2.8 §15-§17）：经 Windows.Media.Ocr（WinRT，windows-rs 安全封装）。
//!
//! - 引擎不可用/无语言包 ⇒ 结构化 `glint.ocrUnavailable`（不伪造结果 §16）。
//! - Confidence 缺失 ⇒ Unknown（不伪造）。
//! - 隐私（§17/§27）：识别文本只经返回值给 UI，不写日志。

use windows::core::HSTRING;
use windows::Globalization::Language;
use windows::Graphics::Imaging::BitmapDecoder;
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::{DataWriter, InMemoryRandomAccessStream};

use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("glint::ocr")
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrLineDto {
    pub text: String,
    /// WinRT 不提供行级 confidence ⇒ Unknown（§16 不伪造）。
    pub confidence: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrResultDto {
    pub text: String,
    pub lines: Vec<OcrLineDto>,
    /// 引擎语言（如 zh-Hans-CN / en-US）；不可用 ⇒ None。
    pub engine_language: Option<String>,
}

/// PNG/JPEG 字节 → WinRT 流（供 BitmapDecoder）。
fn to_stream(png: &[u8]) -> windows::core::Result<InMemoryRandomAccessStream> {
    let stream = InMemoryRandomAccessStream::new()?;
    let writer = DataWriter::CreateDataWriter(&stream)?;
    writer.WriteBytes(png)?;
    writer.StoreAsync()?.get()?;
    writer.FlushAsync()?.get()?;
    drop(writer);
    Ok(stream)
}

/// 对 PNG/JPEG 字节执行 OCR。
///
/// 引擎优先级：用户配置语言 → 用户档案语言 → Globalization 偏好。
/// 大图超过引擎 MaxImageDimension ⇒ 先缩放（保 OCR 可行）。
pub fn ocr_png(png: &[u8], language_hint: Option<&str>) -> Result<OcrResultDto, WeaveError> {
    let engine = create_engine(language_hint)
        .map_err(|e| err("glint.ocrUnavailable", format!("OCR engine unavailable: {e}")))?;
    let max_dim = OcrEngine::MaxImageDimension().unwrap_or(u32::MAX);
    let png = if let Some((w, h)) = png_dimensions_bytes(png) {
        if w > max_dim || h > max_dim {
            // 超限缩放：保持比例，fit 到 max_dim
            let scale = max_dim as f64 / w.max(h) as f64;
            let new_w = ((w as f64 * scale) as u32).max(1);
            let new_h = ((h as f64 * scale) as u32).max(1);
            resize_png(png, new_w, new_h)?
        } else {
            png.to_vec()
        }
    } else {
        png.to_vec()
    };

    let stream = to_stream(&png).map_err(|e| err("glint.ocrFailed", format!("{e}")))?;
    let decoder = BitmapDecoder::CreateAsync(&stream)
        .map_err(|e| err("glint.ocrFailed", format!("{e}")))?
        .get()
        .map_err(|e| err("glint.ocrFailed", format!("decode failed: {e}")))?;
    let bitmap = decoder
        .GetSoftwareBitmapAsync()
        .map_err(|e| err("glint.ocrFailed", format!("{e}")))?
        .get()
        .map_err(|e| err("glint.ocrFailed", format!("decode failed: {e}")))?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(|e| err("glint.ocrFailed", format!("{e}")))?
        .get()
        .map_err(|e| err("glint.ocrFailed", format!("recognize failed: {e}")))?;

    let engine_language = engine
        .RecognizerLanguage()
        .ok()
        .and_then(|l| l.LanguageTag().ok())
        .map(|t| t.to_string());
    let mut lines = Vec::new();
    let mut full = String::new();
    if let Ok(line_list) = result.Lines() {
        for line in line_list {
            let text_h = line.Text().unwrap_or_default();
            let text = String::from_utf16_lossy(&text_h.as_wide());
            if text.is_empty() {
                continue;
            }
            full.push_str(&text);
            full.push('\n');
            lines.push(OcrLineDto { text, confidence: None });
        }
    }
    Ok(OcrResultDto {
        text: full,
        lines,
        engine_language,
    })
}

fn png_dimensions_bytes(png: &[u8]) -> Option<(u32, u32)> {
    if png.len() < 24 || &png[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([png[16], png[17], png[18], png[19]]);
    let h = u32::from_be_bytes([png[20], png[21], png[22], png[23]]);
    Some((w, h))
}

fn create_engine(hint: Option<&str>) -> windows::core::Result<OcrEngine> {
    if let Some(lang) = hint {
        if let Ok(language) = Language::CreateLanguage(&HSTRING::from(lang)) {
            if let Ok(engine) = OcrEngine::TryCreateFromLanguage(&language) {
                return Ok(engine);
            }
        }
    }
    OcrEngine::TryCreateFromUserProfileLanguages()
}

fn resize_png(png: &[u8], new_w: u32, new_h: u32) -> Result<Vec<u8>, WeaveError> {
    let decoded = image::load_from_memory(png)
        .map_err(|e| err("glint.ocrFailed", format!("decode for resize failed: {e}")))?;
    let resized = decoded.resize(new_w, new_h, image::imageops::FilterType::Lanczos3);
    let mut out = Vec::new();
    image::DynamicImage::ImageRgba8(resized.to_rgba8())
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .map_err(|e| err("glint.ocrFailed", format!("encode failed: {e}")))?;
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engine_unavailable_returns_structured_error_not_crash() {
        // 无法保证 CI 有 OCR 语言包 ⇒ 只验证错误是结构化的
        let result = ocr_png(&minimal_png(), None);
        match result {
            Ok(_) => {} // 引擎可用的机器上成功也算通过
            Err(e) => {
                assert!(
                    e.code.starts_with("glint.ocr"),
                    "结构化错误码: {e}"
                );
            }
        }
    }

    fn minimal_png() -> Vec<u8> {
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            4,
            4,
            image::Rgba([255, 255, 255, 255]),
        ));
        let mut out = Vec::new();
        image::DynamicImage::ImageRgba8(img.to_rgba8())
            .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
            .expect("encode");
        out
    }
}
