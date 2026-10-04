//! DocumentFacts / DocumentIdentity / Diagnostics / Limits / Capabilities
//! （M8 上 §9/§11/§13/§61/§87/§97/§100）。

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::detect::{Detection, DocumentFormat};

/// 字段三态（§11：解析失败 ≠ 0——绝不让 0 同时表示"零页"与"解析失败"）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state", content = "value", rename_all = "camelCase")]
pub enum Field<T> {
    Known(T),
    /// 库未暴露 / 文档不含该字段。
    Unknown,
    /// 该格式/能力不支持此字段。
    Unavailable,
    /// 近似值（如 word count 非精确 §34）。
    Estimated(T),
}

impl<T> Field<T> {
    pub fn known(v: T) -> Self {
        Field::Known(v)
    }
}

/// DocumentIdentity（§9）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentIdentity {
    pub path: PathBuf,
    pub file_name: String,
    pub extension: String,
    pub size: u64,
    /// UNIX epoch 毫秒（不可得 = None）。
    pub created_ms: Option<u64>,
    pub modified_ms: Option<u64>,
}

/// DocumentDiagnostic（§97）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentDiagnostic {
    /// info | warning | error
    pub severity: String,
    pub code: String,
    pub message: String,
    /// 可选上下文（part 名/页码等；不泄露绝对临时路径 §99）。
    pub context: Option<String>,
    /// recoverable | fatal
    pub recoverability: String,
}

pub fn diag(
    severity: &str,
    code: &str,
    message: impl Into<String>,
    context: Option<String>,
) -> DocumentDiagnostic {
    DocumentDiagnostic {
        severity: severity.into(),
        code: code.into(),
        message: message.into(),
        context,
        recoverability: "recoverable".into(),
    }
}

/// 文档统计（§34：标注 Exact/Approximate）。
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentStatistics {
    /// Exact：解析器逐个计数的字段。
    pub paragraphs: Option<Field<u64>>,
    pub headings: Option<Field<u64>>,
    pub tables: Option<Field<u64>>,
    pub images: Option<Field<u64>>,
    pub hyperlinks: Option<Field<u64>>,
    /// §34：word count 必须标 Approximate（whitespace split ≠ Word 统计）。
    pub words: Option<Field<u64>>,
    pub characters: Option<Field<u64>>,
}

/// 结构化 DocumentFacts（§11/§13）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentFacts {
    pub format: DocumentFormat,
    pub detection_reason: &'static str,
    pub extension_mismatch: bool,
    pub size: u64,
    /// PDF：页数 / 加密 / PDF 版本（§16 只报库真实暴露的字段）。
    pub pages: Option<Field<u64>>,
    pub encrypted: Option<Field<bool>>,
    pub pdf_version: Option<Field<String>>,
    /// 页面尺寸列表（pt，[w,h]）——有界：最多前 64 页（§100）。
    pub page_sizes: Option<Field<Vec<[f64; 2]>>>,
    /// Office：工作簿/幻灯片层事实。
    pub sheets: Option<Field<Vec<SheetFact>>>,
    pub slides: Option<Field<Vec<SlideFact>>>,
    /// Office core properties（§32：只报真实解析值）。
    pub metadata: Vec<(String, String)>,
    pub statistics: DocumentStatistics,
    /// §79：处理语义相关的元数据事实由操作层补充；inspect 只报告。
    pub warnings: Vec<String>,
    pub diagnostics: Vec<DocumentDiagnostic>,
}

/// XLSX Sheet 事实（§40-§44）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SheetFact {
    pub name: String,
    /// visible | hidden | veryHidden（§44：库能区分才区分）。
    pub visibility: String,
    /// §41：dimension 声明范围 ≠ populated cell 数——两者都报。
    pub dimension: Option<String>,
    pub populated_cells: Option<u64>,
    pub row_count: Option<u64>,
    pub column_count: Option<u64>,
    pub formula_cells: Option<u64>,
    pub merged_cells: Option<u64>,
}

/// PPTX Slide 事实（§48-§49）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SlideFact {
    pub index: u64,
    pub hidden: bool,
    /// 有文本占位的 slide（§50：文本提取 ≠ 视觉渲染）。
    pub has_text: bool,
    pub text_chars: u64,
    pub image_count: Option<u64>,
    pub shape_count: Option<u64>,
    pub has_notes: Option<bool>,
}

/// DocumentCapabilities（§8/§61：按真实代码声明，绝不伪装）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FormatCapabilities {
    pub format: DocumentFormat,
    pub inspect: bool,
    pub metadata: bool,
    /// PDF mutation 集。
    pub merge: bool,
    pub split: bool,
    pub reorder: bool,
    pub rotate: bool,
    /// 渲染级预览（PDF raster）——M8 上 NOT SUPPORTED。
    pub visual_preview: bool,
}

/// DocumentResourceLimits（§87/§100：集中定义，值可测可调）。
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentResourceLimits {
    /// 单文档最大字节数。
    pub max_file_size: u64,
    /// Office zip：最大条目数。
    pub max_archive_entries: usize,
    /// Office zip：单条目解压上限。
    pub max_entry_size: u64,
    /// Office zip：解压总量上限。
    pub max_decompressed_total: u64,
    /// 单个 XML part 解析上限。
    pub max_xml_part_size: u64,
    /// PDF 最大页数（inspect 遍历/操作）。
    pub max_pages: u64,
    pub max_sheets: usize,
    pub max_slides: usize,
    // §100：值来自保守初值 + 测试校准（PERF.md 记录），非拍脑袋宣称。
}

impl Default for DocumentResourceLimits {
    fn default() -> Self {
        Self {
            max_file_size: 512 * 1024 * 1024,
            max_archive_entries: 65_536,
            max_entry_size: 256 * 1024 * 1024,
            max_decompressed_total: 1024 * 1024 * 1024,
            max_xml_part_size: 128 * 1024 * 1024,
            max_pages: 10_000,
            max_sheets: 1_024,
            max_slides: 2_048,
        }
    }
}

impl DocumentResourceLimits {
    /// §87：zip 打开时全包体检（条目数/解压总量/单条目）。
    pub fn check_zip_entries<R: std::io::Read + std::io::Seek>(
        &self,
        archive: &mut zip::ZipArchive<R>,
    ) -> Result<(), String> {
        if archive.len() > self.max_archive_entries {
            return Err(format!(
                "archive has {} entries, over limit {}",
                archive.len(),
                self.max_archive_entries
            ));
        }
        let mut total: u64 = 0;
        for i in 0..archive.len() {
            let f = archive
                .by_index_raw(i)
                .map_err(|e| format!("archive entry {i} unreadable: {e}"))?;
            let uncompressed = f.size();
            total = total.saturating_add(uncompressed);
            if uncompressed > self.max_entry_size {
                return Err(format!(
                    "entry '{}' decompressed size {uncompressed} over limit {}",
                    f.name(),
                    self.max_entry_size
                ));
            }
        }
        if total > self.max_decompressed_total {
            return Err(format!(
                "archive total decompressed {total} over limit {}",
                self.max_decompressed_total
            ));
        }
        Ok(())
    }
}

/// 能力矩阵（§8：由真实实现生成——非 inspect 能力的字段为 false）。
pub fn capabilities(format: DocumentFormat) -> FormatCapabilities {
    match format {
        DocumentFormat::Pdf => FormatCapabilities {
            format,
            inspect: true,
            metadata: true,
            merge: true,
            split: true,
            reorder: true,
            rotate: true,
            visual_preview: false,
        },
        DocumentFormat::Docx | DocumentFormat::Xlsx | DocumentFormat::Pptx => FormatCapabilities {
            format,
            inspect: true,
            metadata: true,
            merge: false,
            split: false,
            reorder: false,
            rotate: false,
            visual_preview: false,
        },
        DocumentFormat::Txt | DocumentFormat::Markdown => FormatCapabilities {
            format,
            inspect: true,
            metadata: false,
            merge: false,
            split: false,
            reorder: false,
            rotate: false,
            visual_preview: false,
        },
        _ => FormatCapabilities {
            format,
            inspect: false,
            metadata: false,
            merge: false,
            split: false,
            reorder: false,
            rotate: false,
            visual_preview: false,
        },
    }
}

pub use diag as make_diagnostic;

/// 检测结果的 identity 部分（§9）。
pub fn identity(path: &std::path::Path, detection: &Detection) -> DocumentIdentity {
    let meta = std::fs::metadata(path).ok();
    let ms = |t: Option<std::time::SystemTime>| {
        t.and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_millis() as u64)
    };
    DocumentIdentity {
        path: path.to_path_buf(),
        file_name: path
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default(),
        extension: detection.extension.clone(),
        size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
        created_ms: meta.as_ref().and_then(|m| ms(m.created().ok())),
        modified_ms: meta.and_then(|m| ms(m.modified().ok())),
    }
}
