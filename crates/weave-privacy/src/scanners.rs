//! 格式扫描器（M2.4 §19-§21/§38/§40-§41/§44-§46）。
//!
//! 统一输出 PrivacyFinding；逐文件错误隔离（§44：99 成功 1 失败 ≠ 会话失败）；
//! 资源有界（§46：单文件大小/单文件与全会话 Finding 上限）。
//! 能力边界如实（§20/§62）：
//! - Image：JPEG/PNG/WebP 元数据（复用 M6 inspect 的 EXIF/GPS 事实）；清理 =
//!   像素重编码（方向已应用；ICC 会丢失——Finding 明示）。
//! - Office（DOCX/XLSX/PPTX）：core/app 元数据 + 隐藏工作表/修订/批注/隐藏
//!   幻灯片**扫描**；清理仅 core 元数据置空（隐藏内容删除 = scan-only，
//!   发现 ≠ 可安全删除 §9）。
//! - PDF（lopdf）：Info 字典元数据；清理 scan-only（§15 不粗暴重建）。
//! - Text/Data：位置级模式检测（复用共享原语 §53）。
//! - ZIP：文件名 + 有界内部文本扫描（深度 1，无嵌套递归 §40）。

use std::io::Read;

use crate::model::{FindingCategory, NotCheckedItem, PrivacyFinding, PrivacyScanSession, Severity};
use crate::patterns::{PatternKind, scan_line};

/// 扫描资源边界（§46：无界输入防护）。
#[derive(Debug, Clone)]
pub struct ScanLimits {
    /// 单文件检查上限（字节）。
    pub max_file_bytes: u64,
    /// 单文件 Finding 上限。
    pub max_findings_per_file: usize,
    /// 单次会话 Finding 上限。
    pub max_findings_total: usize,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_file_bytes: 32 * 1024 * 1024,
            max_findings_per_file: 200,
            max_findings_total: 5000,
        }
    }
}

/// 单文件自动分派（扩展名 → 扫描器；未知 ⇒ NotChecked 明示，不静默 §38）。
pub fn scan_bytes_auto(
    relative_path: &str,
    bytes: &[u8],
    session: &mut PrivacyScanSession,
    limits: &ScanLimits,
) {
    session.files_scanned += 1;
    if bytes.len() as u64 > limits.max_file_bytes {
        session.files_unsupported += 1;
        session.not_fully_checked.push(NotCheckedItem {
            relative_path: relative_path.to_string(),
            reason: format!(
                "file exceeds inspection size limit ({} bytes)",
                limits.max_file_bytes
            ),
        });
        return;
    }
    let ext = extension_of(relative_path);
    let before = session.findings.len();
    let supported = match ext.as_deref() {
        Some("jpg") | Some("jpeg") | Some("png") | Some("webp") => {
            scan_image(relative_path, bytes, session, limits)
        }
        Some("docx") | Some("xlsx") | Some("pptx") => {
            scan_office(relative_path, bytes, session, limits)
        }
        Some("pdf") => scan_pdf(relative_path, bytes, session, limits),
        Some("txt") | Some("md") | Some("log") | Some("csv") | Some("json") | Some("xml") => {
            scan_text(relative_path, bytes, session, limits)
        }
        Some("zip") => scan_zip(relative_path, bytes, session, limits),
        _ => false,
    };
    // 单文件 Finding 上限（§46）
    let file_findings = session.findings.len() - before;
    if file_findings > limits.max_findings_per_file {
        session
            .findings
            .truncate(before + limits.max_findings_per_file);
        session.warnings.push(format!(
            "findings truncated for {relative_path} (per-file limit {})",
            limits.max_findings_per_file
        ));
    }
    if supported {
        session.files_supported += 1;
    } else {
        session.files_unsupported += 1;
        session.not_fully_checked.push(NotCheckedItem {
            relative_path: relative_path.to_string(),
            reason: "unsupported format — not fully inspected".to_string(),
        });
    }
}

fn extension_of(path: &str) -> Option<String> {
    let base = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let ext = base.rsplit_once('.')?.1.to_lowercase();
    if ext.is_empty() { None } else { Some(ext) }
}

fn push(session: &mut PrivacyScanSession, finding: PrivacyFinding, limits: &ScanLimits) {
    if session.findings.len() < limits.max_findings_total {
        session.findings.push(finding);
    }
}

// ── Image（复用 M6 inspect 的 EXIF/GPS 事实；§6 GPS 优先）──

fn scan_image(
    rel: &str,
    bytes: &[u8],
    session: &mut PrivacyScanSession,
    limits: &ScanLimits,
) -> bool {
    let inspected = weave_media::inspect_bytes(
        bytes,
        extension_of(rel).as_deref(),
        &weave_media::ImageLimits::default(),
    );
    let Ok((_decoded, facts)) = inspected else {
        let reason = inspected
            .err()
            .unwrap_or_else(|| "image could not be decoded".to_string());
        session.not_fully_checked.push(NotCheckedItem {
            relative_path: rel.to_string(),
            reason,
        });
        return true;
    };
    let md = &facts.metadata;
    let facts = |sub: &str,
                 severity: Severity,
                 location: &str,
                 description: String,
                 summary: String,
                 identity: String,
                 cleanup: crate::model::CleanupAction| {
        PrivacyFinding::new(crate::model::FindingFacts {
            relative_path: rel.to_string(),
            category: FindingCategory::ImageMetadata,
            sub_category: sub.to_string(),
            severity,
            evidence: crate::model::EvidenceLevel::Confirmed,
            location: location.to_string(),
            description,
            evidence_summary: summary,
            cleanup,
            evidence_identity: identity,
        })
    };
    if md.gps_present {
        push(
            session,
            facts(
                "gps",
                Severity::High,
                "EXIF GPS",
                "图片包含 GPS 位置信息——可能暴露拍摄地点。".to_string(),
                "EXIF GPS present".to_string(),
                "exif-gps".to_string(),
                crate::model::CleanupAction::StripImageMetadata,
            ),
            limits,
        );
    }
    if md.exif_present {
        let detail = match (&md.camera_make, &md.camera_model, &md.datetime) {
            (Some(make), Some(model), Some(dt)) => format!("{make} {model} @ {dt}"),
            (Some(make), Some(model), None) => format!("{make} {model}"),
            _ => "EXIF".to_string(),
        };
        push(
            session,
            facts(
                "exif",
                Severity::Medium,
                "EXIF",
                "图片包含 EXIF 元数据（设备/拍摄时间等）。".to_string(),
                detail,
                "exif-general".to_string(),
                crate::model::CleanupAction::StripImageMetadata,
            ),
            limits,
        );
    }
    if md.icc_present {
        push(
            session,
            facts(
                "icc",
                Severity::Info,
                "ICC profile",
                "图片包含色彩配置文件；元数据剥离重编码会丢失 ICC（像素颜色可能轻微变化）。"
                    .to_string(),
                "ICC profile present".to_string(),
                "exif-icc".to_string(),
                crate::model::CleanupAction::ScanOnly,
            ),
            limits,
        );
    }
    true
}

// ── Office（ZIP package；§8/§9）──

fn scan_office(
    rel: &str,
    bytes: &[u8],
    session: &mut PrivacyScanSession,
    limits: &ScanLimits,
) -> bool {
    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        session.files_failed += 1;
        session.errors.push(
            weave_core::prelude::WeaveError::io(
                "privacy.officeUnreadable",
                format!("office package cannot be opened: {rel}"),
            )
            .with_location("weave-privacy::scanners"),
        );
        return true;
    };
    let office_facts = |sub: String,
                        severity: Severity,
                        location: &str,
                        description: String,
                        summary: String,
                        identity: String,
                        cleanup: crate::model::CleanupAction| {
        PrivacyFinding::new(crate::model::FindingFacts {
            relative_path: rel.to_string(),
            category: FindingCategory::DocumentMetadata,
            sub_category: sub,
            severity,
            evidence: crate::model::EvidenceLevel::Confirmed,
            location: location.to_string(),
            description,
            evidence_summary: summary,
            cleanup,
            evidence_identity: identity,
        })
    };
    if let Some(core) = read_entry(&mut archive, "docProps/core.xml") {
        for (tag, sub, severity) in [
            ("dc:creator", "author", Severity::Medium),
            ("cp:lastModifiedBy", "last_modified_by", Severity::Medium),
            ("dc:title", "title", Severity::Low),
            ("dc:subject", "subject", Severity::Low),
            ("cp:keywords", "keywords", Severity::Low),
            ("dc:description", "description", Severity::Low),
        ] {
            if let Some(value) = xml_text(&core, tag)
                && !value.trim().is_empty()
            {
                push(
                    session,
                    office_facts(
                        sub.to_string(),
                        severity,
                        "docProps/core.xml",
                        format!("文档元数据包含 {sub}（可能暴露作者/组织信息）。"),
                        crate::model::masked_preview(&value),
                        format!("core-{sub}"),
                        crate::model::CleanupAction::ClearDocumentCoreMetadata,
                    ),
                    limits,
                );
            }
        }
    }
    if let Some(app) = read_entry(&mut archive, "docProps/app.xml") {
        for (tag, sub) in [("Company", "company"), ("Manager", "manager")] {
            if let Some(value) = xml_text(&app, tag)
                && !value.trim().is_empty()
            {
                push(
                    session,
                    office_facts(
                        sub.to_string(),
                        Severity::Low,
                        "docProps/app.xml",
                        format!("文档属性包含 {sub}。"),
                        crate::model::masked_preview(&value),
                        format!("app-{sub}"),
                        crate::model::CleanupAction::ClearDocumentCoreMetadata,
                    ),
                    limits,
                );
            }
        }
    }
    // 隐藏内容（§9：发现 ≠ 可安全删除 ⇒ scan-only）
    let hidden_facts = |sub: String,
                        severity: Severity,
                        location: &str,
                        description: String,
                        summary: String,
                        identity: String| {
        PrivacyFinding::new(crate::model::FindingFacts {
            relative_path: rel.to_string(),
            category: FindingCategory::HiddenContent,
            sub_category: sub,
            severity,
            evidence: crate::model::EvidenceLevel::Confirmed,
            location: location.to_string(),
            description,
            evidence_summary: summary,
            cleanup: crate::model::CleanupAction::ScanOnly,
            evidence_identity: identity,
        })
    };
    if rel.to_lowercase().ends_with(".xlsx")
        && let Some(workbook) = read_entry(&mut archive, "xl/workbook.xml")
    {
        for state in ["hidden", "veryHidden"] {
            let needle = format!("state=\"{state}\"");
            let count = workbook.matches(&needle).count();
            if count > 0 {
                push(
                    session,
                    hidden_facts(
                        format!("hidden_sheet_{state}"),
                        Severity::High,
                        "xl/workbook.xml",
                        format!(
                            "工作簿包含 {count} 个 {state} 工作表——可能包含未预期的业务数据；是否删除需人工判断。"
                        ),
                        format!("{count} sheet(s)"),
                        format!("hidden-sheet-{state}-{count}"),
                    ),
                    limits,
                );
            }
        }
    }
    if rel.to_lowercase().ends_with(".docx") {
        if entry_exists(&mut archive, "word/comments.xml") {
            push(
                session,
                hidden_facts(
                    "comments".to_string(),
                    Severity::Medium,
                    "word/comments.xml",
                    "文档包含批注——可能暴露审阅过程与内部意见。".to_string(),
                    "comments part present".to_string(),
                    "docx-comments".to_string(),
                ),
                limits,
            );
        }
        if let Some(document) = read_entry(&mut archive, "word/document.xml") {
            let revisions =
                document.matches("<w:ins ").count() + document.matches("<w:del ").count();
            if revisions > 0 {
                push(
                    session,
                    hidden_facts(
                        "tracked_changes".to_string(),
                        Severity::High,
                        "word/document.xml",
                        format!(
                            "文档包含 {revisions} 处修订记录——可能暴露以前删除或修改过的文本。"
                        ),
                        format!("{revisions} revision mark(s)"),
                        format!("docx-revisions-{revisions}"),
                    ),
                    limits,
                );
            }
        }
    }
    if rel.to_lowercase().ends_with(".pptx") {
        let mut hidden_slides = 0usize;
        for i in 1..=999usize {
            let name = format!("ppt/slides/slide{i}.xml");
            if let Some(slide) = read_entry(&mut archive, &name)
                && slide.contains("show=\"0\"")
            {
                hidden_slides += 1;
            }
        }
        if hidden_slides > 0 {
            push(
                session,
                hidden_facts(
                    "hidden_slide".to_string(),
                    Severity::Medium,
                    "ppt/slides",
                    format!("演示文稿包含 {hidden_slides} 张隐藏幻灯片。"),
                    format!("{hidden_slides} hidden slide(s)"),
                    format!("pptx-hidden-{hidden_slides}"),
                ),
                limits,
            );
        }
        if entry_exists(&mut archive, "ppt/notesSlides/notesSlide1.xml") {
            push(
                session,
                hidden_facts(
                    "notes".to_string(),
                    Severity::Low,
                    "ppt/notesSlides",
                    "演示文稿包含演讲者备注。".to_string(),
                    "notes present".to_string(),
                    "pptx-notes".to_string(),
                ),
                limits,
            );
        }
    }
    true
}

// ── PDF（lopdf Info 字典；§15 scan+report only）──

fn scan_pdf(
    rel: &str,
    bytes: &[u8],
    session: &mut PrivacyScanSession,
    limits: &ScanLimits,
) -> bool {
    match lopdf::Document::load_mem(bytes) {
        Ok(doc) => {
            let info = doc.trailer.get(b"Info").ok().and_then(|o| match o {
                lopdf::Object::Reference(object_id) => doc.get_object(*object_id).ok().cloned(),
                lopdf::Object::Dictionary(dict) => Some(lopdf::Object::Dictionary(dict.clone())),
                _ => None,
            });
            if let Some(dict) = info {
                for (key, sub, severity) in [
                    (&b"Author"[..], "author", Severity::Medium),
                    (&b"Creator"[..], "creator", Severity::Medium),
                    (&b"Producer"[..], "producer", Severity::Low),
                    (&b"Title"[..], "title", Severity::Low),
                    (&b"Subject"[..], "subject", Severity::Low),
                    (&b"Keywords"[..], "keywords", Severity::Low),
                ] {
                    let value = match dict.as_dict().ok().and_then(|d| d.get(key).ok().cloned()) {
                        Some(lopdf::Object::String(text, _)) => {
                            String::from_utf8_lossy(&text).into_owned()
                        }
                        _ => String::new(),
                    };
                    if !value.trim().is_empty() {
                        push(
                            session,
                            PrivacyFinding::new(crate::model::FindingFacts {
                                relative_path: rel.to_string(),
                                category: FindingCategory::DocumentMetadata,
                                sub_category: format!("pdf_{sub}"),
                                severity,
                                evidence: crate::model::EvidenceLevel::Confirmed,
                                location: "PDF Info".to_string(),
                                description: format!("PDF 元数据包含 {sub}。"),
                                evidence_summary: crate::model::masked_preview(&value),
                                cleanup: crate::model::CleanupAction::ScanOnly,
                                evidence_identity: format!("pdf-{sub}"),
                            }),
                            limits,
                        );
                    }
                }
            }
            true
        }
        Err(e) => {
            // §38/§39：加密/损坏 ⇒ 明示不可检查（不鼓励绕过）
            let dbg = format!("{e:?}");
            let reason = if dbg.contains("assword") {
                "encrypted PDF cannot be inspected without a password".to_string()
            } else {
                format!("PDF could not be parsed: {e}")
            };
            session.not_fully_checked.push(NotCheckedItem {
                relative_path: rel.to_string(),
                reason,
            });
            session.files_failed += 1;
            true
        }
    }
}

// ── Text / Data（§16；位置级模式）──

fn scan_text(
    rel: &str,
    bytes: &[u8],
    session: &mut PrivacyScanSession,
    limits: &ScanLimits,
) -> bool {
    let text = String::from_utf8_lossy(bytes);
    let mut file_hits = 0usize;
    for (idx, line) in text.lines().enumerate() {
        for hit in scan_line(line, idx + 1) {
            file_hits += 1;
            if file_hits > limits.max_findings_per_file {
                return true;
            }
            let severity = match hit.kind {
                PatternKind::SuspectedJwt
                | PatternKind::SuspectedSecretAssignment
                | PatternKind::SuspectedLongHexKey => Severity::Medium,
                _ => Severity::Low,
            };
            let cleanup = match hit.kind {
                PatternKind::SuspectedJwt
                | PatternKind::SuspectedSecretAssignment
                | PatternKind::SuspectedLongHexKey
                | PatternKind::Email
                | PatternKind::Phone => crate::model::CleanupAction::MaskDetectedValue,
                _ => crate::model::CleanupAction::ScanOnly,
            };
            push(
                session,
                PrivacyFinding::new(crate::model::FindingFacts {
                    relative_path: rel.to_string(),
                    category: match hit.kind {
                        PatternKind::LocalPath => FindingCategory::LocalPath,
                        PatternKind::Email | PatternKind::Phone => FindingCategory::Contact,
                        _ => FindingCategory::SuspectedSecret,
                    },
                    sub_category: hit.kind.sub_category().to_string(),
                    severity,
                    evidence: crate::model::EvidenceLevel::Suspected,
                    location: format!("line {}", hit.line),
                    description: hit.kind.description().to_string(),
                    evidence_summary: hit.masked_value.clone(),
                    cleanup,
                    // §28：证据 identity = kind+行号+masked（明文不进指纹）
                    evidence_identity: format!(
                        "{}:{}:{}",
                        hit.kind.sub_category(),
                        hit.line,
                        hit.masked_value
                    ),
                }),
                limits,
            );
        }
    }
    true
}

// ── ZIP（§40：深度 1 有界；§41 与 Office package 分流由扩展名路由保证）──

const ZIP_MAX_ENTRIES: usize = 1000;
const ZIP_MAX_TEXT_BYTES: usize = 1024 * 1024;
const ZIP_MAX_TEXT_FILES: usize = 20;

fn scan_zip(
    rel: &str,
    bytes: &[u8],
    session: &mut PrivacyScanSession,
    limits: &ScanLimits,
) -> bool {
    let Ok(mut archive) = zip::ZipArchive::new(std::io::Cursor::new(bytes)) else {
        session.files_failed += 1;
        session.not_fully_checked.push(NotCheckedItem {
            relative_path: rel.to_string(),
            reason: "archive could not be opened (corrupted?)".to_string(),
        });
        return true;
    };
    let names: Vec<String> = (0..archive.len().min(ZIP_MAX_ENTRIES))
        .filter_map(|i| archive.by_index(i).ok().map(|f| f.name().to_string()))
        .collect();
    if archive.len() > ZIP_MAX_ENTRIES {
        session.warnings.push(format!(
            "archive {rel} truncated at {ZIP_MAX_ENTRIES} entries"
        ));
    }
    // 压缩包内文本文件有界抽查（深度 1，不递归嵌套 §40）
    let mut scanned_inner = 0usize;
    for name in &names {
        if scanned_inner >= ZIP_MAX_TEXT_FILES {
            session
                .warnings
                .push(format!("archive {rel} inner text scan truncated"));
            break;
        }
        let ext_ok = matches!(
            extension_of(name).as_deref(),
            Some("txt") | Some("json") | Some("csv") | Some("xml") | Some("md") | Some("log")
        );
        if !ext_ok {
            continue;
        }
        if let Ok(mut file) = archive.by_name(name.as_str()) {
            if file.size() > ZIP_MAX_TEXT_BYTES as u64 {
                continue;
            }
            let mut buf = Vec::new();
            if file.read_to_end(&mut buf).is_ok() {
                scanned_inner += 1;
                scan_text(&format!("{rel}!{name}"), &buf, session, limits);
            }
        }
    }
    true
}

// ── ZIP/XML 小助手 ──

fn read_entry(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str) -> Option<String> {
    let mut file = archive.by_name(name).ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    Some(buf)
}

fn entry_exists(archive: &mut zip::ZipArchive<std::io::Cursor<&[u8]>>, name: &str) -> bool {
    archive.by_name(name).is_ok()
}

/// 极简 XML 文本提取（`<tag>value</tag>`）。
fn xml_text(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{tag}>");
    let close = format!("</{tag}>");
    let start = xml.find(&open)? + open.len();
    let end = xml[start..].find(&close)? + start;
    Some(xml[start..end].trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan(ext: &str, bytes: &[u8]) -> PrivacyScanSession {
        let mut session = PrivacyScanSession::new("R", "s".to_string());
        session.status = crate::model::SessionStatus::Scanning;
        scan_bytes_auto(
            &format!("file.{ext}"),
            bytes,
            &mut session,
            &ScanLimits::default(),
        );
        session
    }

    #[test]
    fn unsupported_format_is_not_fully_checked_not_silent() {
        let session = scan("weird", b"MAGIC");
        assert_eq!(session.files_unsupported, 1);
        assert_eq!(session.not_fully_checked.len(), 1);
        assert!(session.not_fully_checked[0].reason.contains("unsupported"));
    }

    #[test]
    fn oversized_file_is_limited_not_scanned() {
        let limits = ScanLimits {
            max_file_bytes: 10,
            ..ScanLimits::default()
        };
        let mut session = PrivacyScanSession::new("R", "s".to_string());
        scan_bytes_auto("big.txt", &[0u8; 100], &mut session, &limits);
        assert_eq!(session.files_unsupported, 1);
        assert!(session.not_fully_checked[0].reason.contains("size limit"));
    }

    #[test]
    fn corrupted_office_is_failed_not_crash() {
        let session = scan("docx", b"not a zip");
        assert_eq!(session.files_failed, 1, "§44：单文件失败 ≠ 会话崩溃");
        assert_eq!(session.files_scanned, 1);
    }

    #[test]
    fn corrupted_pdf_reports_reason() {
        let session = scan("pdf", b"%PDF-1.4 broken");
        assert!(
            session
                .not_fully_checked
                .iter()
                .any(|n| n.relative_path.ends_with(".pdf"))
        );
    }

    #[test]
    fn text_scan_finds_and_masks() {
        let session = scan(
            "txt",
            b"contact alice@example.com\napi_key = sk-prod-12345678\n",
        );
        assert!(session.findings.iter().any(|f| f.sub_category == "email"));
        assert!(
            session
                .findings
                .iter()
                .any(|f| f.sub_category == "secret_assignment"
                    && f.evidence == crate::model::EvidenceLevel::Suspected)
        );
        // §14：证据只含 masked
        assert!(
            session
                .findings
                .iter()
                .all(|f| !f.evidence_summary.contains("sk-prod-12345678"))
        );
    }

    #[test]
    fn zip_scans_inner_text_with_bounds() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts: zip::write::SimpleFileOptions = Default::default();
            w.start_file("inner/secret.txt", opts).unwrap();
            std::io::Write::write_all(&mut w, b"api_key = sk-zip-1234567890").unwrap();
            w.finish().unwrap();
        }
        let session = scan("zip", buf.get_ref());
        assert!(
            session
                .findings
                .iter()
                .any(|f| f.relative_path.contains("inner/secret.txt")
                    && f.sub_category == "secret_assignment")
        );
    }

    #[test]
    fn xlsx_hidden_sheet_is_high_scan_only() {
        let mut buf = std::io::Cursor::new(Vec::new());
        {
            let mut w = zip::ZipWriter::new(&mut buf);
            let opts: zip::write::SimpleFileOptions = Default::default();
            w.start_file("xl/workbook.xml", opts).unwrap();
            std::io::Write::write_all(
                &mut w,
                br#"<workbook><sheet name="S1" state="hidden"/><sheet name="S2" state="veryHidden"/></workbook>"#,
            )
            .unwrap();
            w.start_file("docProps/core.xml", opts).unwrap();
            std::io::Write::write_all(
                &mut w,
                br#"<cp:coreProperties><dc:creator>Alice</dc:creator></cp:coreProperties>"#,
            )
            .unwrap();
            w.finish().unwrap();
        }
        let session = scan("xlsx", buf.get_ref());
        assert!(
            session
                .findings
                .iter()
                .any(|f| f.sub_category == "hidden_sheet_hidden" && f.severity == Severity::High)
        );
        assert!(session
            .findings
            .iter()
            .any(|f| f.sub_category == "author" && f.evidence_summary.chars().all(|c| c == '•')));
    }
}
