//! weave-documents — Document tools（M8 上 §0-§118）。
//!
//! 职责：文档检测/身份/事实（§9-§13）、PDF 域操作（§15-§28：inspect/
//! merge/split/extract/reorder/rotate + 输出校验）、Office 检查（§29-§52：
//! DOCX/XLSX/PPTX 结构统计）、能力矩阵（§8/§61）、资源限额（§87/§100）、
//! 诊断模型（§97）。
//! 不负责：Office 编辑器/公式引擎/渲染器（§0.2）、M5 数据表、M6 图像——
//! 批量执行一律经 M7（§58-§60），单文档 mutation 经 M2 TextTransform
//! 管线（历史/undo 复用）。

pub mod detect;
pub mod facts;
pub mod office;
pub mod page_range;
pub mod pdf;

#[cfg(test)]
mod office_tests;
#[cfg(test)]
mod pdf_tests;

pub use detect::{Detection, DocumentFormat, detect};
pub use facts::{
    DocumentDiagnostic, DocumentFacts, DocumentIdentity, DocumentResourceLimits,
    DocumentStatistics, Field, FormatCapabilities, SheetFact, SlideFact, capabilities, identity,
    make_diagnostic,
};
pub use office::{inspect_docx, inspect_pptx, inspect_xlsx};
pub use page_range::{PageRangeError, parse_page_ranges, split_every_n};
pub use pdf::{
    PdfError, PdfFacts, PdfMergePlan, Rotation, execute_extract_pages, execute_merge,
    execute_reorder, execute_rotate, execute_split_every_n, inspect as pdf_inspect, plan_merge,
    split_output_name, validate_reorder,
};

use std::path::Path;

/// 统一 Inspect 入口（§12 流程：detect → capability check → safe read →
/// parse → facts + diagnostics）。只读，不修改原文件（§64）。
pub fn inspect_document(path: &Path, limits: &DocumentResourceLimits) -> DocumentFacts {
    let detection = detect(path);
    let id = identity(path, &detection);
    let mut facts = match detection.format {
        DocumentFormat::Pdf => match pdf::inspect(path, limits) {
            Ok(pdf_facts) => DocumentFacts {
                format: DocumentFormat::Pdf,
                detection_reason: detection.reason,
                extension_mismatch: detection.extension_mismatch,
                size: id.size,
                pages: Some(pdf_facts.pages_field()),
                encrypted: Some(Field::Known(pdf_facts.encrypted)),
                pdf_version: pdf_facts
                    .pdf_version
                    .map(Field::Known)
                    .or(Some(Field::Unknown)),
                page_sizes: Some(Field::Known(
                    pdf_facts
                        .page_sizes
                        .iter()
                        .map(|b| [b[2] - b[0], b[3] - b[1]])
                        .collect(),
                )),
                sheets: None,
                slides: None,
                metadata: pdf_facts.metadata,
                statistics: DocumentStatistics::default(),
                warnings: Vec::new(),
                diagnostics: Vec::new(),
            },
            Err(e) => failure_facts(&detection, id, &e),
        },
        DocumentFormat::Docx => match office::inspect_docx(path, limits) {
            Ok(f) => f,
            Err(diags) => failure_facts_with(&detection, id, diags),
        },
        DocumentFormat::Xlsx => match office::inspect_xlsx(path, limits) {
            Ok(f) => f,
            Err(diags) => failure_facts_with(&detection, id, diags),
        },
        DocumentFormat::Pptx => match office::inspect_pptx(path, limits) {
            Ok(f) => f,
            Err(diags) => failure_facts_with(&detection, id, diags),
        },
        DocumentFormat::Txt | DocumentFormat::Markdown => DocumentFacts {
            format: detection.format,
            detection_reason: detection.reason,
            extension_mismatch: detection.extension_mismatch,
            size: id.size,
            pages: Some(Field::Unavailable),
            encrypted: Some(Field::Known(false)),
            pdf_version: None,
            page_sizes: None,
            sheets: None,
            slides: None,
            metadata: Vec::new(),
            statistics: DocumentStatistics::default(),
            warnings: Vec::new(),
            diagnostics: Vec::new(),
        },
        other => DocumentFacts {
            format: other,
            detection_reason: detection.reason,
            extension_mismatch: detection.extension_mismatch,
            size: id.size,
            pages: Some(Field::Unavailable),
            encrypted: Some(Field::Unknown),
            pdf_version: None,
            page_sizes: None,
            sheets: None,
            slides: None,
            metadata: Vec::new(),
            statistics: DocumentStatistics::default(),
            warnings: Vec::new(),
            diagnostics: vec![make_diagnostic(
                "error",
                "document.unsupported",
                "format not supported for inspection (§7: no fake capability claims)",
                None::<String>,
            )],
        },
    };
    if detection.extension_mismatch {
        facts.diagnostics.push(make_diagnostic(
            "warning",
            "document.extensionMismatch",
            format!(
                "extension '.{}' does not match detected format {}",
                detection.extension,
                detection.format.as_str()
            ),
            None::<String>,
        ));
    }
    facts
}

fn failure_facts(detection: &Detection, id: DocumentIdentity, e: &pdf::PdfError) -> DocumentFacts {
    failure_facts_with(
        detection,
        id,
        vec![make_diagnostic(
            "error",
            Box::leak(format!("pdf.{}", e.code).into_boxed_str()),
            e.message.clone(),
            None::<String>,
        )],
    )
}

fn failure_facts_with(
    detection: &Detection,
    id: DocumentIdentity,
    diags: Vec<DocumentDiagnostic>,
) -> DocumentFacts {
    DocumentFacts {
        format: detection.format,
        detection_reason: detection.reason,
        extension_mismatch: detection.extension_mismatch,
        size: id.size,
        // §11：解析失败 ≠ 0——Unknown 如实
        pages: Some(Field::Unknown),
        encrypted: Some(Field::Unknown),
        pdf_version: Some(Field::Unknown),
        page_sizes: Some(Field::Unknown),
        sheets: Some(Field::Unknown),
        slides: Some(Field::Unknown),
        metadata: Vec::new(),
        statistics: DocumentStatistics::default(),
        warnings: Vec::new(),
        diagnostics: diags,
    }
}
