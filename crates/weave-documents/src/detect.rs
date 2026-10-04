//! DocumentDetector（M8 上 §9-§10/§94-§96）：内容签名优先、容器结构次之、
//! 扩展名仅兜底。Extension 不是最终事实（§9）。

use serde::{Deserialize, Serialize};
#[cfg(feature = "specta")]
use specta::Type;
use std::path::Path;

/// 文档格式（§10：只有实际能力能判断的才加入枚举）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "specta", derive(Type))]
#[serde(rename_all = "camelCase")]
pub enum DocumentFormat {
    Pdf,
    Docx,
    Xlsx,
    Pptx,
    Txt,
    Markdown,
    /// 合法容器但不是 Weave 支持的文档类型（如纯 zip/rtf）。
    Unsupported,
    /// 结构损坏（签名存在但解析失败——由 inspect 层细分）。
    Malformed,
    Unknown,
}

impl DocumentFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            DocumentFormat::Pdf => "pdf",
            DocumentFormat::Docx => "docx",
            DocumentFormat::Xlsx => "xlsx",
            DocumentFormat::Pptx => "pptx",
            DocumentFormat::Txt => "txt",
            DocumentFormat::Markdown => "markdown",
            DocumentFormat::Unsupported => "unsupported",
            DocumentFormat::Malformed => "malformed",
            DocumentFormat::Unknown => "unknown",
        }
    }
}

/// 检测结论（§94：DetectedFormat + DetectionReason）。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Detection {
    pub format: DocumentFormat,
    /// 扩展名（小写，不含点；无扩展名 = ""）。
    pub extension: String,
    /// content-signature | container-structure | extension | none
    pub reason: &'static str,
    /// §95：扩展名与内容签名不一致。
    pub extension_mismatch: bool,
}

/// ZIP 内 OOXML 判别（容器结构级检测）。
fn detect_ooxml<R: std::io::Read + std::io::Seek>(
    reader: R,
) -> Result<DocumentFormat, std::io::Error> {
    let mut archive = match zip::ZipArchive::new(reader) {
        Ok(a) => a,
        // 非 zip 容器 ⇒ None（调用方继续走文本/未知启发，不短路）
        Err(_) => return Ok(DocumentFormat::Unknown),
    };
    // [Content_Types].xml 存在 ⇒ OOXML 包
    let mut has_content_types = false;
    for i in 0..archive.len() {
        if let Ok(f) = archive.by_index(i)
            && f.name().eq_ignore_ascii_case("[Content_Types].xml")
        {
            has_content_types = true;
            break;
        }
    }
    if !has_content_types {
        return Ok(DocumentFormat::Unsupported);
    }
    let _ = &mut archive;
    let format = DocumentFormat::Unknown;
    for i in 0..archive.len() {
        let Ok(file) = archive.by_index(i) else {
            continue;
        };
        let name = file.name();
        if name.starts_with("word/") {
            return Ok(DocumentFormat::Docx);
        }
        if name.starts_with("xl/") {
            return Ok(DocumentFormat::Xlsx);
        }
        if name.starts_with("ppt/") {
            return Ok(DocumentFormat::Pptx);
        }
    }
    let _ = format;
    Ok(DocumentFormat::Unsupported)
}

/// 主检测入口（§94：content signature → container structure → extension）。
pub fn detect(path: &Path) -> Detection {
    let extension = path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_ascii_lowercase())
        .unwrap_or_default();

    // 1. 内容签名：PDF magic（%PDF- 前 5 字节）
    let mut head = [0u8; 8];
    let head_len = std::fs::File::open(path)
        .and_then(|mut f| std::io::Read::read_exact(&mut f, &mut head))
        .map(|_| 8usize)
        .or_else(|_| {
            // 短文件（<8B）按实际长度读
            std::fs::read(path).map(|b| {
                let n = b.len().min(8);
                head[..n].copy_from_slice(&b[..n]);
                n
            })
        })
        .unwrap_or(0);
    let is_pdf_sig = head_len >= 5 && &head[..5] == b"%PDF-";

    // 2. 容器结构：ZIP → OOXML part 判别
    let zip_format = std::fs::File::open(path)
        .ok()
        .and_then(|f| detect_ooxml(f).ok())
        .filter(|f| *f != DocumentFormat::Unknown); // None/Unknown = 非 zip 容器

    let (format, reason) = if is_pdf_sig {
        (DocumentFormat::Pdf, "content-signature")
    } else if let Some(f) = zip_format {
        match f {
            DocumentFormat::Docx | DocumentFormat::Xlsx | DocumentFormat::Pptx => {
                (f, "container-structure")
            }
            _ => (DocumentFormat::Unsupported, "container-structure"),
        }
    } else if head_len > 0 && is_probably_text(path) {
        // 3. 文本类：内容可 UTF-8 解码 + 扩展名兜底区分 txt/markdown
        match extension.as_str() {
            "md" | "markdown" => (DocumentFormat::Markdown, "extension"),
            _ => (DocumentFormat::Txt, "extension"),
        }
    } else {
        match extension.as_str() {
            "pdf" | "docx" | "xlsx" | "pptx" | "md" | "markdown" => {
                // 扩展名声称支持格式但签名不符 ⇒ 交由 inspect 判 Malformed（§96）
                (DocumentFormat::Unknown, "none")
            }
            _ => (DocumentFormat::Unknown, "none"),
        }
    };

    // §95 mismatch：扩展名格式 ≠ 检测格式
    let claimed = match extension.as_str() {
        "pdf" => Some(DocumentFormat::Pdf),
        "docx" => Some(DocumentFormat::Docx),
        "xlsx" => Some(DocumentFormat::Xlsx),
        "pptx" => Some(DocumentFormat::Pptx),
        "txt" => Some(DocumentFormat::Txt),
        "md" | "markdown" => Some(DocumentFormat::Markdown),
        _ => None,
    };
    // §95：扩展名声称某格式而内容不符 ⇒ mismatch（含 Unsupported/Unknown）
    let extension_mismatch = claimed.is_some_and(|c| c != format);

    Detection {
        format,
        extension,
        reason,
        extension_mismatch,
    }
}

/// 文本启发（仅用于 txt/markdown 兜底）：前 8KB 可 UTF-8 且无 NUL。
fn is_probably_text(path: &Path) -> bool {
    let Ok(bytes) = std::fs::read(path) else {
        return false;
    };
    let sample = &bytes[..bytes.len().min(8 * 1024)];
    !sample.contains(&0) && std::str::from_utf8(sample).is_ok()
}
