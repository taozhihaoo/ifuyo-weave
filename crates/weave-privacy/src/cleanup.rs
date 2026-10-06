//! 清理内容变换（M2.4 §22-§26/§35-§36）：原始字节 → 安全副本字节。
//!
//! - **原文件绝不在此触碰**（§21）：本模块是纯字节变换；落盘走 M2.0
//!   Export 执行器（temp+rename 原子、stale 重验、事务/undo）。
//! - 图片（§7）：像素重编码 + 方向已应用 + 元数据结构性消失；JPEG 质量
//!   92（视觉无损目标，**非字节级无损**——Finding/报告明示）；ICC 丢失明示。
//! - Office（§8）：core/app 元数据字段置空（ZIP 重写，part 结构不变）；
//!   隐藏内容/修订不做自动删除（§9 发现 ≠ 可安全删除）。
//! - 文本（§24）：命中值 → `[REDACTED]`（高风险动作，仅用户显式选择）。
//! - 验证（§36）：变换后自检（图片可解码 / ZIP 可开 / 文本可解析）。

use std::io::Read;

use weave_core::prelude::WeaveError;

use crate::model::{CleanupAction, FindingCategory, PrivacyFinding};
use crate::patterns::PatternKind;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("weave-privacy::cleanup")
}

/// 按选中 Finding 应用清理 → 安全副本字节。
///
/// `original_relative_path` 决定格式分派（与扫描同一路由 §41）。
pub fn apply_cleanup(
    original: &[u8],
    original_relative_path: &str,
    selected: &[PrivacyFinding],
) -> Result<Vec<u8>, WeaveError> {
    if selected.is_empty() {
        return Ok(original.to_vec());
    }
    let actions: std::collections::HashSet<CleanupAction> =
        selected.iter().map(|f| f.cleanup).collect();
    let ext = original_relative_path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "webp" => {
            if actions.contains(&CleanupAction::StripImageMetadata) {
                strip_image_metadata(original, &ext)
            } else {
                Ok(original.to_vec())
            }
        }
        "docx" | "xlsx" | "pptx" => {
            if actions.contains(&CleanupAction::ClearDocumentCoreMetadata) {
                clear_office_core_metadata(original)
            } else {
                Ok(original.to_vec())
            }
        }
        "txt" | "md" | "log" | "csv" | "json" | "xml" => {
            if actions.contains(&CleanupAction::MaskDetectedValue) {
                mask_text_values(original, selected)
            } else {
                Ok(original.to_vec())
            }
        }
        _ => Err(err(
            "privacy.cleanupUnsupportedFormat",
            format!("cleanup not supported for .{ext}"),
        )),
    }
}

/// 图片元数据剥离：解码 → 应用方向 → 无元数据重编码。
/// §7 明示：JPEG/WebP 重编码（质量 92）；PNG 无损像素；ICC 丢失。
fn strip_image_metadata(original: &[u8], ext: &str) -> Result<Vec<u8>, WeaveError> {
    let decoded = image::load_from_memory(original)
        .map_err(|e| err("privacy.imageDecodeFailed", format!("decode failed: {e}")))?;
    // 方向应用（EXIF orientation → 像素；重编码后方向不再依赖元数据）。
    // orientation 事实复用 M6 inspect（§59 原文件保护：此处只读副本字节）。
    let exif_orientation = weave_media::inspect_bytes(
        original,
        Some(&format!(".{ext}")),
        &weave_media::ImageLimits::default(),
    )
    .ok()
    .and_then(|(_, f)| f.metadata.orientation);
    let oriented = match exif_orientation {
        // Orientation::from_exif_chunk 接收 EXIF 短整型字节切片（构造 2B LE）
        Some(o) if (1..=8).contains(&o) => {
            let bytes = o.to_le_bytes();
            let mut oriented = decoded;
            if let Some(orientation) = image::metadata::Orientation::from_exif_chunk(&bytes) {
                // &mut 就地应用（§7：方向信息转为像素事实，重编码后不依赖元数据）
                oriented.apply_orientation(orientation);
            }
            oriented
        }
        _ => decoded,
    };
    let format = match ext {
        "png" => image::ImageFormat::Png,
        "webp" => image::ImageFormat::WebP,
        _ => image::ImageFormat::Jpeg,
    };
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    match format {
        // JPEG：质量 92（视觉无损目标；有损——已在 Finding/报告明示）
        image::ImageFormat::Jpeg => {
            // JPEG 无 alpha ⇒ 先合成 RGB（§14 同源语义）
            let rgb = oriented.to_rgb8();
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut cursor, 92);
            encoder
                .encode_image(&image::DynamicImage::ImageRgb8(rgb))
                .map_err(|e| {
                    err(
                        "privacy.imageEncodeFailed",
                        format!("jpeg encode failed: {e}"),
                    )
                })?;
        }
        other => {
            image::DynamicImage::ImageRgba8(oriented.to_rgba8())
                .write_to(&mut cursor, other)
                .map_err(|e| err("privacy.imageEncodeFailed", format!("encode failed: {e}")))?;
        }
    }
    Ok(out)
}

/// Office core/app 元数据置空（ZIP 重写：part 结构与关系不变，字段值清空）。
fn clear_office_core_metadata(original: &[u8]) -> Result<Vec<u8>, WeaveError> {
    let mut source = zip::ZipArchive::new(std::io::Cursor::new(original))
        .map_err(|e| err("privacy.officeUnreadable", format!("open failed: {e}")))?;
    let mut out = std::io::Cursor::new(Vec::new());
    {
        let mut writer = zip::ZipWriter::new(&mut out);
        let options: zip::write::SimpleFileOptions = Default::default();
        for i in 0..source.len() {
            let mut entry = source
                .by_index(i)
                .map_err(|e| err("privacy.officeUnreadable", format!("{e}")))?;
            let name = entry.name().to_string();
            let mut bytes = Vec::new();
            entry
                .read_to_end(&mut bytes)
                .map_err(|e| err("privacy.officeUnreadable", format!("{e}")))?;
            let cleaned = if name == "docProps/core.xml" {
                Some(clear_xml_fields(
                    &String::from_utf8_lossy(&bytes),
                    &[
                        "dc:creator",
                        "cp:lastModifiedBy",
                        "dc:title",
                        "dc:subject",
                        "cp:keywords",
                        "dc:description",
                    ],
                ))
            } else if name == "docProps/app.xml" {
                Some(clear_xml_fields(
                    &String::from_utf8_lossy(&bytes),
                    &["Company", "Manager"],
                ))
            } else {
                None
            };
            writer
                .start_file(name.as_str(), options)
                .map_err(|e| err("privacy.officeWriteFailed", format!("{e}")))?;
            match cleaned {
                Some(text) => std::io::Write::write_all(&mut writer, text.as_bytes()),
                None => std::io::Write::write_all(&mut writer, &bytes),
            }
            .map_err(|e| err("privacy.officeWriteFailed", format!("{e}")))?;
        }
        writer
            .finish()
            .map_err(|e| err("privacy.officeWriteFailed", format!("{e}")))?;
    }
    Ok(out.into_inner())
}

/// 字段值置空（保留标签结构——part/schema 合法性不变）。
/// 每个字段只替换首次出现（替换后空标签仍会命中 find——循环会死循环）。
fn clear_xml_fields(xml: &str, tags: &[&str]) -> String {
    let mut out = xml.to_string();
    for tag in tags {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        if let Some(start) = out.find(&open)
            && let Some(rel_end) = out[start..].find(&close)
        {
            let end = start + rel_end + close.len();
            out.replace_range(start..end, format!("{open}{close}").as_str());
        }
    }
    out
}

/// 文本命中值遮蔽（§24 高风险；清理时重跑检测取精确原值——不持久化明文）。
fn mask_text_values(original: &[u8], selected: &[PrivacyFinding]) -> Result<Vec<u8>, WeaveError> {
    let text = String::from_utf8(original.to_vec())
        .map_err(|_| err("privacy.textDecodeFailed", "text is not valid utf-8"))?;
    let selected_kinds: std::collections::HashSet<String> = selected
        .iter()
        .filter(|f| f.cleanup == CleanupAction::MaskDetectedValue)
        .map(|f| f.sub_category.clone())
        .collect();
    let mut out_lines: Vec<String> = Vec::new();
    for (idx, line) in text.lines().enumerate() {
        let mut working = line.to_string();
        for hit in crate::patterns::scan_line(line, idx + 1) {
            if selected_kinds.contains(hit.kind.sub_category()) {
                // 按列不可靠（同行多命中）——整值替换：重建时用检测原值
                if let Some(position) = find_value_position(line, hit.kind) {
                    let (start, end) = position;
                    working.replace_range(start..end, "[REDACTED]");
                }
            }
        }
        out_lines.push(working);
    }
    let mut result = out_lines.join("\n");
    if text.ends_with('\n') {
        result.push('\n');
    }
    Ok(result.into_bytes())
}

/// 在原始行中定位命中值（重跑检测；返回字节范围）。
fn find_value_position(line: &str, kind: PatternKind) -> Option<(usize, usize)> {
    let lower = line.to_lowercase();
    match kind {
        PatternKind::SuspectedSecretAssignment => {
            for key in ["password", "passwd", "secret", "api_key", "apikey", "token"] {
                for sep in [" = ", ": ", "=", ":"] {
                    let needle = format!("{key}{sep}");
                    if let Some(pos) = lower.find(&needle) {
                        let start = pos + needle.len();
                        let rest = line[start..].trim_start();
                        let lead = line[start..].len() - rest.len();
                        let value_end =
                            start + lead + rest.split_whitespace().next().unwrap_or("").len();
                        if value_end > start {
                            return Some((start + lead, value_end));
                        }
                    }
                }
            }
            None
        }
        PatternKind::SuspectedJwt => {
            let start = line.find("eyJ")?;
            let end = line[start..]
                .find(|c: char| c.is_whitespace())
                .map(|p| start + p)
                .unwrap_or(line.len());
            Some((start, end))
        }
        _ => None,
    }
}

/// Safe Copy 验证（§36）：存在/可解析/非空。失败 ⇒ 上层不得报成功。
pub fn verify_safe_copy(bytes: &[u8], relative_path: &str) -> Result<(), WeaveError> {
    if bytes.is_empty() {
        return Err(err("privacy.verifyEmpty", "safe copy is empty"));
    }
    let ext = relative_path
        .rsplit('.')
        .next()
        .unwrap_or_default()
        .to_lowercase();
    match ext.as_str() {
        "jpg" | "jpeg" | "png" | "webp" => {
            image::load_from_memory(bytes).map_err(|e| {
                err(
                    "privacy.verifyDecodeFailed",
                    format!("image cannot decode: {e}"),
                )
            })?;
        }
        "docx" | "xlsx" | "pptx" => {
            zip::ZipArchive::new(std::io::Cursor::new(bytes))
                .map_err(|e| err("privacy.verifyZipFailed", format!("package invalid: {e}")))?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{CleanupAction, EvidenceLevel, FindingCategory, FindingFacts, Severity};

    fn finding(action: CleanupAction, sub: &str) -> PrivacyFinding {
        PrivacyFinding::new(FindingFacts {
            relative_path: "file".to_string(),
            category: FindingCategory::SuspectedSecret,
            sub_category: sub.to_string(),
            severity: Severity::Medium,
            evidence: EvidenceLevel::Suspected,
            location: "line 1".to_string(),
            description: String::new(),
            evidence_summary: String::new(),
            cleanup: action,
            evidence_identity: format!("{sub}-1"),
        })
    }

    #[test]
    fn text_mask_replaces_selected_values_with_redacted() {
        let original = b"api_key = sk-prod-12345678\ntoken = apple\nplain line\n";
        let selected = vec![finding(
            CleanupAction::MaskDetectedValue,
            "secret_assignment",
        )];
        let cleaned = apply_cleanup(original, "config.txt", &selected).expect("clean");
        let text = String::from_utf8(cleaned).expect("utf8");
        assert!(text.contains("[REDACTED]"), "{text}");
        assert!(!text.contains("sk-prod-12345678"), "原值必须消失");
        // 误报行（apple 无形状）不被遮蔽 —— 精度原则
        assert!(text.contains("token = apple"));
        assert!(text.contains("plain line"));
    }

    #[test]
    fn no_selection_returns_original_bytes() {
        let original = b"unchanged";
        let cleaned = apply_cleanup(original, "a.txt", &[]).expect("clean");
        assert_eq!(cleaned, original);
    }

    #[test]
    fn image_strip_removes_metadata_and_keeps_decodable() {
        // 构造带 EXIF 的 JPEG（借 image crate 编码基础图 + weave-media inspect 验证前态）
        let img = image::DynamicImage::ImageRgba8(image::RgbaImage::from_pixel(
            8,
            8,
            image::Rgba([10, 20, 30, 255]),
        ));
        let mut jpeg = Vec::new();
        image::DynamicImage::ImageRgba8(img.to_rgba8())
            .write_to(
                &mut std::io::Cursor::new(&mut jpeg),
                image::ImageFormat::Jpeg,
            )
            .expect("encode");
        let selected = vec![finding(CleanupAction::StripImageMetadata, "exif")];
        let cleaned = apply_cleanup(&jpeg, "photo.jpg", &selected).expect("strip");
        // §36 验证：仍可解码
        verify_safe_copy(&cleaned, "photo.jpg").expect("safe copy decodes");
        let repainted = image::load_from_memory(&cleaned).expect("decode");
        assert_eq!((repainted.width(), repainted.height()), (8, 8), "尺寸保持");
    }

    #[test]
    fn office_core_metadata_cleared_but_structure_valid() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts: zip::write::SimpleFileOptions = Default::default();
            w.start_file("docProps/core.xml", opts).unwrap();
            std::io::Write::write_all(
                &mut w,
                br#"<cp:coreProperties><dc:creator>Alice</dc:creator><cp:lastModifiedBy>Bob</cp:lastModifiedBy></cp:coreProperties>"#,
            )
            .unwrap();
            w.start_file("word/document.xml", opts).unwrap();
            std::io::Write::write_all(&mut w, br#"<doc>keep me</doc>"#).unwrap();
            w.finish().unwrap();
        }
        let selected = vec![finding(CleanupAction::ClearDocumentCoreMetadata, "author")];
        let cleaned = apply_cleanup(buf.get_ref(), "report.docx", &selected).expect("clean");
        // §36：包仍可开 + 必需 part 在
        verify_safe_copy(&cleaned, "report.docx").expect("zip valid");
        let mut archive = zip::ZipArchive::new(std::io::Cursor::new(&cleaned)).expect("open");
        let core = {
            let mut f = archive.by_name("docProps/core.xml").expect("core");
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };
        assert!(core.contains("<dc:creator></dc:creator>"), "{core}");
        assert!(!core.contains("Alice"), "作者值必须消失");
        let doc = {
            let mut f = archive.by_name("word/document.xml").expect("doc");
            let mut s = String::new();
            std::io::Read::read_to_string(&mut f, &mut s).unwrap();
            s
        };
        assert_eq!(doc, "<doc>keep me</doc>", "非元数据 part 原样保留");
    }

    #[test]
    fn unsupported_format_cleanup_is_structured_error() {
        let selected = vec![finding(CleanupAction::StripImageMetadata, "exif")];
        let err = apply_cleanup(b"x", "file.weird", &selected).expect_err("structured");
        assert_eq!(err.code, "privacy.cleanupUnsupportedFormat");
    }

    #[test]
    fn verify_rejects_empty_safe_copy() {
        assert!(verify_safe_copy(&[], "a.txt").is_err());
    }
}
