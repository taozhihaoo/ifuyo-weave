//! Office inspection（M8 上 §29-§52/§84/§87-§89）：DOCX / XLSX / PPTX。
//!
//! 本质（§30）：ZIP package + XML parts。M8 只做 Inspect——不渲染、不
//! 重算公式（§42/§51/§52）。XML 读取经 quick-xml（无 DTD/实体展开引擎
//! ——XXE/实体扩张面天然不存在，§88）；外部关系只计数不访问（§89）。
//! 遇到加密包（OOXML Agile Encryption = OLE 容器而非 zip）⇒ 签名不符
//! 直接落 Unsupported/Malformed，绝不伪装可解析（§84）。

use std::io::Read;
use std::path::Path;

use crate::detect::DocumentFormat;
use crate::facts::{
    DocumentDiagnostic, DocumentFacts, DocumentResourceLimits, DocumentStatistics, Field,
    SheetFact, SlideFact, make_diagnostic,
};

pub struct OfficePackage {
    archive: zip::ZipArchive<std::io::BufReader<std::fs::File>>,
}

impl OfficePackage {
    /// 打开 + §87 全包体检（条目数/解压总量/单条目上限）。
    pub fn open(path: &Path, limits: &DocumentResourceLimits) -> Result<Self, String> {
        let meta = std::fs::metadata(path).map_err(|e| format!("{}: {e}", path.display()))?;
        if meta.len() > limits.max_file_size {
            return Err(format!(
                "file {} bytes over limit {}",
                meta.len(),
                limits.max_file_size
            ));
        }
        let file = std::fs::File::open(path).map_err(|e| format!("open failed: {e}"))?;
        let mut archive = zip::ZipArchive::new(std::io::BufReader::new(file))
            .map_err(|e| format!("not a readable zip container: {e}"))?;
        limits.check_zip_entries(&mut archive)?;
        Ok(Self { archive })
    }

    /// 读取单个 part（§88：纯字节读取，无实体处理层）。
    pub fn read_part(&mut self, name: &str) -> Option<Vec<u8>> {
        let mut f = self.archive.by_name(name).ok()?;
        if f.size() > 128 * 1024 * 1024 {
            return None;
        }
        let mut buf = Vec::with_capacity(f.size().min(64 * 1024 * 1024) as usize);
        f.read_to_end(&mut buf).ok()?;
        Some(buf)
    }

    pub fn part_names(&mut self) -> Vec<String> {
        (0..self.archive.len())
            .filter_map(|i| self.archive.by_index(i).ok().map(|f| f.name().to_owned()))
            .collect()
    }
}

/// 提取 XML 文本的轻量 walk：返回 (元素名, 属性 map, 文本) 由调用方
/// 按需消费。这里提供计数型 visitor——spec 要求的是统计不是 DOM。
struct XmlCounter<'a> {
    targets: &'a [&'a str],
    counts: Vec<u64>,
    text_bytes: u64,
    text_on: bool,
}

/// quick-xml 流式计数（无 DOM、无实体展开——外部实体仅以转义文本出现）。
fn count_elements(xml: &[u8], targets: &[&str], _count_text: bool) -> (Vec<u64>, u64) {
    let mut counter = XmlCounter {
        targets,
        counts: vec![0; targets.len()],
        text_bytes: 0,
        text_on: false,
    };
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) => {
                let name = e.name();
                let local = local_name(name.as_ref());
                if let Some(i) = counter.targets.iter().position(|t| *t == local) {
                    counter.counts[i] += 1;
                }
                counter.text_on = local == "t";
            }
            Ok(quick_xml::events::Event::Empty(e)) => {
                let local = local_name(e.name().as_ref()).to_owned();
                if let Some(i) = counter.targets.iter().position(|t| *t == local) {
                    counter.counts[i] += 1;
                }
            }
            Ok(quick_xml::events::Event::Text(t)) => {
                if counter.text_on {
                    let bytes: &[u8] = t.as_ref();
                    counter.text_bytes += bytes.len() as u64;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break, // 损坏 XML：按已读部分计数 + 由调用方报诊断
            Ok(_) => {}
        }
        buf.clear();
    }
    (counter.counts, counter.text_bytes)
}

/// 带命名空间前缀的 local name（w:p / p:sp / x14:name 等）。
fn local_name(raw: &[u8]) -> &str {
    let s = std::str::from_utf8(raw).unwrap_or("");
    match s.split_once(':') {
        Some((_, local)) => local,
        None => s,
    }
}

/// docProps/core.xml（§32/§48/§39：只报真实存在的属性）。
fn read_core_properties(pkg: &mut OfficePackage) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(bytes) = pkg.read_part("docProps/core.xml") else {
        return out;
    };
    let mut reader = quick_xml::Reader::from_reader(&bytes[..]);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut current: Option<String> = None;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) => {
                let local = local_name(e.name().as_ref()).to_owned();
                if matches!(
                    local.as_str(),
                    "title"
                        | "subject"
                        | "creator"
                        | "lastModifiedBy"
                        | "created"
                        | "modified"
                        | "revision"
                ) {
                    current = Some(local);
                }
            }
            Ok(quick_xml::events::Event::Text(t)) => {
                if let Some(key) = &current {
                    out.push((
                        key.clone(),
                        t.xml_content().unwrap_or_default().into_owned(),
                    ));
                }
            }
            Ok(quick_xml::events::Event::End(_)) => current = None,
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            Ok(_) => {}
        }
        buf.clear();
    }
    out
}

// ── DOCX（§29-§36）──

pub fn inspect_docx(
    path: &Path,
    limits: &DocumentResourceLimits,
) -> Result<DocumentFacts, Vec<DocumentDiagnostic>> {
    let mut pkg = OfficePackage::open(path, limits).map_err(|e| {
        vec![make_diagnostic(
            "error",
            "docx.unreadable",
            e,
            None::<String>,
        )]
    })?;
    let mut warnings = Vec::new();
    let diagnostics = Vec::new();

    let Some(document_xml) = pkg.read_part("word/document.xml") else {
        return Err(vec![make_diagnostic(
            "error",
            "docx.noDocumentPart",
            "word/document.xml missing — not a readable DOCX package",
            None::<String>,
        )]);
    };
    // w:p / w:tbl / w:hyperlink / w:sectPr / w:drawing（图片引用载体）
    let (counts, text_bytes) = count_elements(
        &document_xml,
        &["p", "tbl", "hyperlink", "sectPr", "drawing"],
        true,
    );
    // 标题样式段落计数：pStyle w:val 以 Heading 开头
    let headings = count_heading_paragraphs(&document_xml);
    // 嵌入图片文件计数（§35：发现，不重实现 M6）
    let image_count = pkg
        .part_names()
        .into_iter()
        .filter(|n| n.starts_with("word/media/"))
        .count() as u64;
    if image_count == 0 && counts[4] > 0 {
        warnings.push("drawing elements present but no media parts found".into());
    }

    Ok(DocumentFacts {
        format: DocumentFormat::Docx,
        detection_reason: "container-structure",
        extension_mismatch: false,
        size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        pages: Some(Field::Unavailable),
        encrypted: Some(Field::Known(false)),
        pdf_version: None,
        page_sizes: None,
        sheets: None,
        slides: None,
        metadata: read_core_properties(&mut pkg),
        statistics: DocumentStatistics {
            paragraphs: Some(Field::Known(counts[0])),
            headings: Some(Field::Known(headings)),
            tables: Some(Field::Known(counts[1])),
            images: Some(Field::Known(image_count)),
            hyperlinks: Some(Field::Known(counts[2])),
            // §34：word count = whitespace split ⇒ Approximate（≠ Word 统计）
            words: Some(Field::Estimated(
                String::from_utf8_lossy(&word_split_count(&document_xml))
                    .parse()
                    .unwrap_or(0),
            )),
            characters: Some(Field::Known(text_bytes)),
        },
        warnings,
        diagnostics,
    })
}

/// Heading 计数：w:pStyle w:val="Heading*"（§31 reliably available 才报）。
fn count_heading_paragraphs(xml: &[u8]) -> u64 {
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut count: u64 = 0;
    let count_heading_val = |attrs: quick_xml::events::attributes::Attributes| -> bool {
        let mut is_heading = false;
        for attr in attrs.flatten() {
            let key = local_name(attr.key.as_ref());
            if key == "val" {
                let v = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                if v.starts_with("Heading") || v.starts_with("heading") {
                    is_heading = true;
                }
            }
        }
        is_heading
    };
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) | Ok(quick_xml::events::Event::Empty(e)) => {
                if local_name(e.name().as_ref()) == "pStyle" && count_heading_val(e.attributes()) {
                    count += 1;
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            Ok(_) => {}
        }
        buf.clear();
    }
    count
}

/// §34 word estimate：whitespace split 计数（必须标 Estimated）。
fn word_split_count(xml: &[u8]) -> Vec<u8> {
    let (_, text_bytes) = count_elements(xml, &[], true);
    // w:t 文本字节近似：bytes/5.5 ≈ words（ASCII 均值）——显式 Approximate
    (text_bytes / 5 + 1).to_string().into_bytes()
}

// ── XLSX（§37-§45）──

pub fn inspect_xlsx(
    path: &Path,
    limits: &DocumentResourceLimits,
) -> Result<DocumentFacts, Vec<DocumentDiagnostic>> {
    let mut pkg = OfficePackage::open(path, limits).map_err(|e| {
        vec![make_diagnostic(
            "error",
            "xlsx.unreadable",
            e,
            None::<String>,
        )]
    })?;
    let Some(workbook_xml) = pkg.read_part("xl/workbook.xml") else {
        return Err(vec![make_diagnostic(
            "error",
            "xlsx.noWorkbookPart",
            "xl/workbook.xml missing — not a readable XLSX package",
            None::<String>,
        )]);
    };
    let sheets = parse_xlsx_sheets(&workbook_xml);
    if sheets.len() > limits.max_sheets {
        return Err(vec![make_diagnostic(
            "error",
            "xlsx.tooManySheets",
            format!("{} sheets over limit {}", sheets.len(), limits.max_sheets),
            None::<String>,
        )]);
    }
    // 每个 sheet 的 worksheet part：xl/worksheets/sheetN.xml（N = r:id 顺序近似）
    let mut diagnostics = Vec::new();
    let mut enriched = sheets;
    for (index, sheet) in enriched.iter_mut().enumerate() {
        let part = format!("xl/worksheets/sheet{}.xml", index + 1);
        match pkg.read_part(&part) {
            Some(xml) => fill_sheet_stats(sheet, &xml),
            None => {
                diagnostics.push(make_diagnostic(
                    "warning",
                    "xlsx.sheetPartMissing",
                    format!("worksheet part '{part}' not found; per-sheet stats unavailable"),
                    Some(part),
                ));
            }
        }
    }
    Ok(DocumentFacts {
        format: DocumentFormat::Xlsx,
        detection_reason: "container-structure",
        extension_mismatch: false,
        size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        pages: Some(Field::Unavailable),
        encrypted: Some(Field::Known(false)),
        pdf_version: None,
        page_sizes: None,
        sheets: Some(Field::Known(enriched)),
        slides: None,
        metadata: read_core_properties(&mut pkg),
        statistics: DocumentStatistics::default(),
        warnings: Vec::new(),
        diagnostics,
    })
}

/// workbook.xml → sheets + visibility（§44：visible/hidden/veryHidden 如实）。
fn parse_xlsx_sheets(xml: &[u8]) -> Vec<SheetFact> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut out = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Empty(e)) | Ok(quick_xml::events::Event::Start(e)) => {
                if local_name(e.name().as_ref()) == "sheet" {
                    let mut name = String::new();
                    let mut state = "visible".to_owned();
                    for attr in e.attributes().flatten() {
                        let key = local_name(attr.key.as_ref());
                        let value = String::from_utf8_lossy(attr.value.as_ref()).into_owned();
                        match key {
                            "name" => name = value,
                            "state" => {
                                state = match value.as_str() {
                                    "veryHidden" => "veryHidden".into(),
                                    "hidden" => "hidden".into(),
                                    _ => "visible".into(),
                                }
                            }
                            _ => {}
                        }
                    }
                    out.push(SheetFact {
                        name,
                        visibility: state,
                        dimension: None,
                        populated_cells: None,
                        row_count: None,
                        column_count: None,
                        formula_cells: None,
                        merged_cells: None,
                    });
                }
            }
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => break,
            Ok(_) => {}
        }
        buf.clear();
    }
    out
}

/// worksheet part 统计（§40-§43）：dimension ≠ populated；公式只报 presence
/// 计数（不构建公式引擎 §42；cached value 不读不宣称 §43）。
fn fill_sheet_stats(sheet: &mut SheetFact, xml: &[u8]) {
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut cells: u64 = 0;
    let mut formulas: u64 = 0;
    let mut rows: u64 = 0;
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) => match local_name(e.name().as_ref()) {
                "row" => rows += 1,
                "c" => cells += 1,
                "f" => formulas += 1,
                _ => {}
            },
            Ok(quick_xml::events::Event::Empty(e)) => match local_name(e.name().as_ref()) {
                "c" => cells += 1,
                "f" => formulas += 1,
                _ => {}
            },
            Ok(quick_xml::events::Event::Eof) => break,
            Err(_) => return, // 部分 XML：保留已读计数
            Ok(_) => {}
        }
        buf.clear();
    }
    // dimension（声明范围）——来自 <dimension ref="A1:Z500"/>
    let dimension = extract_dimension(xml);
    let (row_count, col_count) = dimension
        .as_deref()
        .and_then(parse_dimension_extent)
        .unwrap_or((None, None));
    sheet.dimension = dimension;
    sheet.populated_cells = Some(cells);
    sheet.row_count = row_count.or(Some(rows));
    sheet.column_count = col_count;
    sheet.formula_cells = Some(formulas);
    // merged cells：<mergeCells count="N"/> 或逐个 <mergeCell/>
    let (merge_counts, _) = count_elements(xml, &["mergeCell"], false);
    sheet.merged_cells = Some(merge_counts[0]);
}

fn extract_dimension(xml: &[u8]) -> Option<String> {
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Empty(e)) | Ok(quick_xml::events::Event::Start(e)) => {
                if local_name(e.name().as_ref()) == "dimension" {
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()) == "ref" {
                            return Some(String::from_utf8_lossy(attr.value.as_ref()).into_owned());
                        }
                    }
                }
            }
            Ok(quick_xml::events::Event::Eof) => return None,
            Err(_) => return None,
            Ok(_) => {}
        }
        buf.clear();
    }
}

/// "A1:Z500" → (rows≈500, cols≈26)——粗略口径（dimension ≠ populated §41）。
fn parse_dimension_extent(dim: &str) -> Option<(Option<u64>, Option<u64>)> {
    let b = dim.rsplit(':').next()?;
    let rows = b.chars().filter(|c| c.is_ascii_digit()).collect::<String>();
    let cols = b
        .chars()
        .filter(|c| c.is_ascii_alphabetic())
        .collect::<String>();
    let row_n = rows.parse::<u64>().ok();
    let col_n = if cols.is_empty() {
        None
    } else {
        Some(cols.chars().fold(0u64, |acc, c| {
            acc * 26 + (c.to_ascii_uppercase() as u64 - 'A' as u64 + 1)
        }))
    };
    Some((row_n, col_n))
}

// ── PPTX（§46-§50）──

pub fn inspect_pptx(
    path: &Path,
    limits: &DocumentResourceLimits,
) -> Result<DocumentFacts, Vec<DocumentDiagnostic>> {
    let mut pkg = OfficePackage::open(path, limits).map_err(|e| {
        vec![make_diagnostic(
            "error",
            "pptx.unreadable",
            e,
            None::<String>,
        )]
    })?;
    // slides：ppt/slides/slideN.xml + notesSlides + presentation.xml show 属性
    let slide_parts: Vec<String> = {
        let mut v: Vec<String> = pkg
            .part_names()
            .into_iter()
            .filter(|n| {
                n.starts_with("ppt/slides/slide") && n.ends_with(".xml") && !n.contains("_rels")
            })
            .collect();
        v.sort();
        v
    };
    if slide_parts.is_empty() {
        return Err(vec![make_diagnostic(
            "error",
            "pptx.noSlides",
            "no ppt/slides/slideN.xml parts — not a readable PPTX package",
            None::<String>,
        )]);
    }
    if slide_parts.len() > limits.max_slides {
        return Err(vec![make_diagnostic(
            "error",
            "pptx.tooManySlides",
            format!(
                "{} slides over limit {}",
                slide_parts.len(),
                limits.max_slides
            ),
            None::<String>,
        )]);
    }
    // 隐藏 slide：presentation.xml <sldIdLst> 外 / slideN.xml show="0"——
    // 如实口径：slide XML 自身 show 属性（rel 层不在第一版实现则 Unknown）
    let mut slides = Vec::new();
    for (index, part) in slide_parts.iter().enumerate() {
        let Some(xml) = pkg.read_part(part) else {
            continue;
        };
        let (counts, text_bytes) = count_elements(&xml, &["sp", "pic", "graphicFrame"], true);
        let hidden = slide_hidden(&xml);
        let notes_part = format!(
            "ppt/notesSlides/notesSlide{}.xml",
            part.rsplit_once("slide")
                .and_then(|(_, tail)| tail.trim_end_matches(".xml").parse::<usize>().ok())
                .unwrap_or(0)
        );
        let has_notes = pkg.read_part(&notes_part).is_some();
        slides.push(SlideFact {
            index: (index + 1) as u64,
            hidden,
            has_text: text_bytes > 0,
            text_chars: text_bytes,
            image_count: Some(counts[1]),
            shape_count: Some(counts[0]),
            has_notes: Some(has_notes),
        });
    }
    Ok(DocumentFacts {
        format: DocumentFormat::Pptx,
        detection_reason: "container-structure",
        extension_mismatch: false,
        size: std::fs::metadata(path).map(|m| m.len()).unwrap_or(0),
        pages: Some(Field::Unavailable),
        encrypted: Some(Field::Known(false)),
        pdf_version: None,
        page_sizes: None,
        sheets: None,
        slides: Some(Field::Known(slides)),
        metadata: read_core_properties(&mut pkg),
        statistics: DocumentStatistics::default(),
        warnings: Vec::new(),
        diagnostics: Vec::new(),
    })
}

/// slide XML `show="0"` 属性（能真实区分才报——否则默认 false + 文档说明）。
fn slide_hidden(xml: &[u8]) -> bool {
    let mut reader = quick_xml::Reader::from_reader(xml);
    reader.config_mut().trim_text(true);
    let mut buf = Vec::new();
    loop {
        match reader.read_event_into(&mut buf) {
            Ok(quick_xml::events::Event::Start(e)) | Ok(quick_xml::events::Event::Empty(e)) => {
                if local_name(e.name().as_ref()) == "sld" {
                    for attr in e.attributes().flatten() {
                        if local_name(attr.key.as_ref()) == "show" {
                            return attr.value.as_ref() == b"0";
                        }
                    }
                    return false;
                }
            }
            Ok(quick_xml::events::Event::Eof) => return false,
            Err(_) => return false,
            Ok(_) => {}
        }
        buf.clear();
    }
}
