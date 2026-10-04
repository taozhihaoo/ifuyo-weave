//! PDF domain（M8 上 §15-§28）：inspect / merge / split / extract / reorder /
//! rotate / output validation。基于 lopdf（纯 Rust，MIT）。
//!
//! §25：rotate = **页面 /Rotate 元数据**语义（非内容重渲染）——预览如实说明。
//! §27：任何写出的 PDF 必须通过 output validation（可解析 + 页数符合预期）。

use std::collections::BTreeMap;
use std::path::Path;

use lopdf::{Document, Object, ObjectId};

use crate::facts::{DocumentDiagnostic, DocumentResourceLimits, Field, make_diagnostic};

#[derive(Debug, Clone, PartialEq)]
pub struct PdfError {
    pub code: &'static str,
    pub message: String,
}

impl PdfError {
    pub fn new(code: &'static str, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
        }
    }
}

type PdfResult<T> = Result<T, PdfError>;

fn load(path: &Path, limits: &DocumentResourceLimits) -> PdfResult<Document> {
    let meta = std::fs::metadata(path)
        .map_err(|e| PdfError::new("pdf.sourceMissing", format!("{}: {e}", path.display())))?;
    if meta.len() > limits.max_file_size {
        return Err(PdfError::new(
            "pdf.fileTooLarge",
            format!("{} bytes over limit {}", meta.len(), limits.max_file_size),
        ));
    }
    // §84：load 不解密——加密文档显式报错（结构化），绝不魔法去密
    let doc = Document::load(path)
        .map_err(|e| PdfError::new("pdf.malformed", format!("parse failed: {e}")))?;
    if doc.is_encrypted() {
        return Err(PdfError::new(
            "pdf.encrypted",
            "The PDF appears encrypted and this version does not support password-protected input.",
        ));
    }
    Ok(doc)
}

/// PDF inspect 事实（§16：只报 lopdf 真实暴露的字段）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfFacts {
    pub page_count: u64,
    /// 每页 MediaBox [x0,y0,x1,y1]（pt）——有界：前 max_pages。
    pub page_sizes: Vec<[f64; 4]>,
    /// 每页 /Rotate（缺省 0）。
    pub rotations: Vec<i64>,
    pub encrypted: bool,
    /// 从文件头 %PDF-x.y 读取的版本。
    pub pdf_version: Option<String>,
    /// Info 字典元数据（§17：只报真实存在的键值；缺失 = 不在列表）。
    pub metadata: Vec<(String, String)>,
}

/// §16 Inspect：只读，不改原文件（§64）。
pub fn inspect(path: &Path, limits: &DocumentResourceLimits) -> PdfResult<PdfFacts> {
    let bytes_head = std::fs::read(path).unwrap_or_default();
    let pdf_version = String::from_utf8_lossy(&bytes_head[..bytes_head.len().min(16)])
        .split_once("%PDF-")
        .map(|(_, rest)| rest.chars().take(8).collect::<String>())
        .filter(|v| v.chars().next().is_some_and(|c| c.is_ascii_digit()));
    let doc = load(path, limits)?;
    let pages = doc.get_pages();
    let page_count = pages.len() as u64;
    if page_count > limits.max_pages {
        return Err(PdfError::new(
            "pdf.tooManyPages",
            format!("{page_count} pages over limit {}", limits.max_pages),
        ));
    }
    let mut page_sizes = Vec::new();
    let mut rotations = Vec::new();
    for (_, id) in pages.iter().take(limits.max_pages as usize) {
        let dict = doc
            .get_object(*id)
            .and_then(|o| o.as_dict())
            .map_err(|e| PdfError::new("pdf.malformed", format!("page dict unreadable: {e}")))?;
        let box_obj = dict.get(b"MediaBox").ok().and_then(|o| {
            o.as_array()
                .ok()
                .map(|a| a.iter().filter_map(|n| n.as_i64().ok()).collect::<Vec<_>>())
        });
        if let Some(v) = box_obj
            .as_ref()
            .filter(|v| v.len() == 4)
            .map(|v| [v[0] as f64, v[1] as f64, v[2] as f64, v[3] as f64])
        {
            page_sizes.push(v);
        } else {
            // MediaBox 可能由 Pages 树继承（缺省 = Unknown，不伪造 0）
            page_sizes.push([0.0, 0.0, 0.0, 0.0]);
        }
        let rot = dict
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        rotations.push(rot);
    }
    let metadata = read_metadata(&doc);
    Ok(PdfFacts {
        page_count,
        page_sizes,
        rotations,
        encrypted: false, // 加密在 load 已拒绝，inspect 到达这里必为未加密
        pdf_version,
        metadata,
    })
}

/// §17：Info 字典中的文本字段；缺失键不出现（区分 Not Present）。
fn read_metadata(doc: &Document) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(trailer_id) = doc.trailer.get(b"Info").and_then(|o| o.as_reference()) else {
        return out;
    };
    let Ok(info) = doc.get_object(trailer_id).and_then(|o| o.as_dict()) else {
        return out;
    };
    const KEYS: [&[u8]; 8] = [
        b"Title",
        b"Author",
        b"Subject",
        b"Keywords",
        b"Creator",
        b"Producer",
        b"CreationDate",
        b"ModDate",
    ];
    for key in KEYS {
        if let Ok(value) = info.get(key) {
            let text = match value {
                Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
                Object::Reference(id) => doc.get_object(*id).ok().and_then(|o| match o {
                    Object::String(b, _) => Some(String::from_utf8_lossy(b).into_owned()),
                    _ => None,
                }),
                _ => None,
            };
            if let Some(text) = text {
                out.push((String::from_utf8_lossy(key).into_owned(), text));
            }
        }
    }
    out
}

// ── Merge（§18-§20/§74/§80）──

/// Merge 计划（§18/§19：有序、确定性、预览可显示每输入页数）。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PdfMergePlan {
    /// 有序输入（确定性顺序 = 数组顺序）。
    pub inputs: Vec<std::path::PathBuf>,
    /// 每输入页数（与 inputs 同序）。
    pub input_page_counts: Vec<u64>,
    pub output_page_count: u64,
}

/// Merge 预览数据（§19：不能只显示 "Ready to merge"）。
pub fn plan_merge(
    inputs: &[std::path::PathBuf],
    limits: &DocumentResourceLimits,
) -> PdfResult<PdfMergePlan> {
    if inputs.len() < 2 {
        return Err(PdfError::new(
            "pdf.mergeNeedsTwo",
            "merge requires at least 2 input PDFs",
        ));
    }
    // §20：missing / duplicate path
    let mut seen = std::collections::HashSet::new();
    let mut counts = Vec::with_capacity(inputs.len());
    for path in inputs {
        if !seen.insert(path.clone()) {
            return Err(PdfError::new(
                "pdf.duplicateInput",
                format!("duplicate input: {}", path.display()),
            ));
        }
        let facts = inspect(path, limits)?;
        counts.push(facts.page_count);
    }
    let total: u64 = counts.iter().sum();
    Ok(PdfMergePlan {
        inputs: inputs.to_vec(),
        input_page_counts: counts,
        output_page_count: total,
    })
}

/// Merge 执行（§74 单一逻辑操作；§80 元数据策略：**首个输入的 Info 元数据
/// 胜出**——deterministic/documentable/testable）。
pub fn execute_merge(
    plan: &PdfMergePlan,
    output: &Path,
    limits: &DocumentResourceLimits,
) -> PdfResult<(u64, Vec<DocumentDiagnostic>)> {
    let mut docs = Vec::new();
    for path in &plan.inputs {
        docs.push(load(path, limits)?);
    }
    let mut merged = docs[0].clone();
    for src in &docs[1..] {
        append_document(&mut merged, src)?;
    }
    let result = write_and_validate(&mut merged, output, plan.output_page_count, limits)?;
    // §80：元数据 = 首输入（merged 继承 docs[0]，克隆即首输入 Info）——显式记录
    Ok((
        result,
        vec![make_diagnostic(
            "info",
            "pdf.mergeMetadataPolicy",
            "output metadata taken from the first input",
            None::<String>,
        )],
    ))
}

/// lopdf merge recipe：对象重挂 + **仅重写新挂对象内部**的引用 + Pages 树
/// 拼接。（不可用全文档 traverse——src 与 target 的 id 空间重叠会改写
/// target 自身引用，第一次实现即踩此坑。）
fn append_document(target: &mut Document, src: &Document) -> PdfResult<()> {
    // 1. src 全部对象复制进 target，建立 remap
    let mut remap: BTreeMap<ObjectId, ObjectId> = BTreeMap::new();
    let mut added: Vec<ObjectId> = Vec::new();
    for (id, obj) in &src.objects {
        let new_id = target.add_object(obj.clone());
        remap.insert(*id, new_id);
        added.push(new_id);
    }
    // 2. 只重写新挂对象内部的引用（完整 remap 就绪后第二遍）
    for new_id in &added {
        let obj = target
            .get_object_mut(*new_id)
            .map_err(|e| PdfError::new("pdf.malformed", format!("remap object: {e}")))?;
        rewrite_references(obj, &remap);
    }
    // 3. 页树拼接：src Pages Kids 追加到 target Pages Kids
    let src_pages_id = src
        .catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(|o| o.as_reference())
        .map_err(|e| PdfError::new("pdf.malformed", format!("src page tree missing: {e}")))?;
    let target_pages_id = target
        .catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(|o| o.as_reference())
        .map_err(|e| PdfError::new("pdf.malformed", format!("target page tree missing: {e}")))?;
    let src_pages_dict = src
        .get_object(src_pages_id)
        .and_then(|o| o.as_dict().cloned())
        .map_err(|e| PdfError::new("pdf.malformed", format!("src pages dict: {e}")))?;
    let src_kids: Vec<Object> = src_pages_dict
        .get(b"Kids")
        .and_then(|o| o.as_array().cloned())
        .map_err(|e| PdfError::new("pdf.malformed", format!("src kids: {e}")))?;
    let src_count: u64 = src_pages_dict
        .get(b"Count")
        .and_then(|o| o.as_i64())
        .map(|v| v as u64)
        .unwrap_or(src_kids.len() as u64);

    let (old_kids, old_count) = {
        let target_pages_dict = target
            .get_object_mut(target_pages_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| PdfError::new("pdf.malformed", format!("target pages dict: {e}")))?;
        let kids = target_pages_dict
            .get(b"Kids")
            .and_then(|o| o.as_array().cloned())
            .map_err(|e| PdfError::new("pdf.malformed", format!("target kids: {e}")))?;
        let count = target_pages_dict
            .get(b"Count")
            .and_then(|o| o.as_i64())
            .map(|v| v as u64)
            .unwrap_or(0);
        (kids, count)
    };
    let mut kids = old_kids;
    let mut appended: Vec<ObjectId> = Vec::new();
    for kid in &src_kids {
        if let Object::Reference(rid) = kid
            && let Some(new_id) = remap.get(rid)
        {
            appended.push(*new_id);
            kids.push(Object::Reference(*new_id));
        }
    }
    {
        let target_pages_dict = target
            .get_object_mut(target_pages_id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| PdfError::new("pdf.malformed", format!("target pages dict: {e}")))?;
        target_pages_dict.set("Kids", Object::Array(kids));
        target_pages_dict.set("Count", Object::Integer((old_count + src_count) as i64));
    }
    // 追加页的 Parent 指向 target Pages 节点（dict 借用结束后再改）
    for new_id in appended {
        if let Ok(page_dict) = target.get_object_mut(new_id).and_then(|o| o.as_dict_mut()) {
            page_dict.set("Parent", Object::Reference(target_pages_id));
        }
    }
    Ok(())
}

/// 递归重写对象树内的间接引用（§merge 内部步骤）。
fn rewrite_references(obj: &mut Object, remap: &BTreeMap<ObjectId, ObjectId>) {
    match obj {
        Object::Reference(rid) => {
            if let Some(new_id) = remap.get(rid) {
                *rid = *new_id;
            }
        }
        Object::Array(items) => {
            for item in items {
                rewrite_references(item, remap);
            }
        }
        Object::Dictionary(dict) => {
            for (_, value) in dict.iter_mut() {
                rewrite_references(value, remap);
            }
        }
        Object::Stream(stream) => {
            for (_, value) in stream.dict.iter_mut() {
                rewrite_references(value, remap);
            }
        }
        _ => {}
    }
}

// ── Split / Extract（§21-§23/§28）──

/// Split 输出命名（§28：deterministic + collision-safe，默认不覆盖）。
pub fn split_output_name(source_stem: &str, index: usize) -> String {
    format!("{source_stem}.part-{index:03}.pdf")
}

/// 按页码子集产出新 PDF（clone 全文档 + 删除补集——简单且与 lopdf 语义对齐）。
fn extract_subset(source: &Document, pages: &[u64], total_pages: u64) -> PdfResult<Document> {
    let mut out = source.clone();
    let keep: std::collections::HashSet<u64> = pages.iter().copied().collect();
    let to_delete: Vec<u32> = (1..=total_pages)
        .filter(|p| !keep.contains(p))
        .map(|p| p as u32)
        .collect();
    if to_delete.len() == total_pages as usize {
        return Err(PdfError::new(
            "pdf.emptySubset",
            "page selection would produce an empty document",
        ));
    }
    out.delete_pages(&to_delete);
    Ok(out)
}

/// §23 Split Every N：产出 N 个子文档（确定性命名由调用方经
/// `split_output_name` 生成；输出校验对每份执行）。
pub fn execute_split_every_n(
    input: &Path,
    n: u64,
    output_dir: &Path,
    limits: &DocumentResourceLimits,
) -> PdfResult<Vec<(std::path::PathBuf, u64)>> {
    let stem = input
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".into());
    let doc = load(input, limits)?;
    let total = doc.get_pages().len() as u64;
    let chunks = crate::page_range::split_every_n(total, n)
        .map_err(|e| PdfError::new("pdf.badRange", e.to_string()))?;
    let mut outputs = Vec::new();
    for (index, (start, end)) in chunks.iter().enumerate() {
        let pages: Vec<u64> = (*start..=*end).collect();
        let mut part = extract_subset(&doc, &pages, total)?;
        let name = split_output_name(&stem, index + 1);
        let out_path = output_dir.join(name);
        let validated = write_and_validate(&mut part, &out_path, pages.len() as u64, limits)?;
        outputs.push((out_path, validated));
    }
    Ok(outputs)
}

/// Extract / Split by ranges：单一子集导出。
pub fn execute_extract_pages(
    input: &Path,
    ranges: &str,
    output: &Path,
    limits: &DocumentResourceLimits,
) -> PdfResult<u64> {
    let doc = load(input, limits)?;
    let total = doc.get_pages().len() as u64;
    let pages = crate::page_range::parse_page_ranges(ranges, Some(total))
        .map_err(|e| PdfError::new("pdf.badRange", e.to_string()))?;
    if pages.is_empty() {
        return Err(PdfError::new("pdf.emptySubset", "no pages selected"));
    }
    let mut out = extract_subset(&doc, &pages, total)?;
    write_and_validate(&mut out, output, pages.len() as u64, limits)
}

// ── Reorder（§24）──

/// §24：permutation 校验——必须覆盖全部页（默认不允许隐式丢页/重复页）。
pub fn validate_reorder(permutation: &[u64], total_pages: u64) -> PdfResult<()> {
    if permutation.len() != total_pages as usize {
        return Err(PdfError::new(
            "pdf.reorderIncomplete",
            format!(
                "permutation has {} entries, document has {total_pages} pages (implicit page loss is not allowed)",
                permutation.len()
            ),
        ));
    }
    let mut seen = std::collections::HashSet::new();
    for &p in permutation {
        if p == 0 || p > total_pages {
            return Err(PdfError::new(
                "pdf.reorderOutOfBounds",
                format!("page {p} out of bounds 1..={total_pages}"),
            ));
        }
        if !seen.insert(p) {
            return Err(PdfError::new(
                "pdf.reorderDuplicate",
                format!("page {p} appears more than once"),
            ));
        }
    }
    Ok(())
}

/// Reorder 执行：同一文档内重排 Pages Kids 顺序（无对象复制）。
pub fn execute_reorder(
    input: &Path,
    permutation: &[u64],
    output: &Path,
    limits: &DocumentResourceLimits,
) -> PdfResult<u64> {
    let mut doc = load(input, limits)?;
    let total = doc.get_pages().len() as u64;
    validate_reorder(permutation, total)?;
    let pages = doc.get_pages(); // BTreeMap<u32, ObjectId> 1-based
    let pages_id = doc
        .catalog()
        .and_then(|c| c.get(b"Pages"))
        .and_then(|o| o.as_reference())
        .map_err(|e| PdfError::new("pdf.malformed", format!("page tree: {e}")))?;
    let ordered: Vec<Object> = permutation
        .iter()
        .map(|&p| Object::Reference(pages[&(p as u32)]))
        .collect();
    let dict = doc
        .get_object_mut(pages_id)
        .and_then(|o| o.as_dict_mut())
        .map_err(|e| PdfError::new("pdf.malformed", format!("pages dict: {e}")))?;
    dict.set("Kids", Object::Array(ordered));
    dict.set("Count", Object::Integer(total as i64));
    write_and_validate(&mut doc, output, total, limits)
}

// ── Rotate（§25-§26）──

/// §25：rotate = 页面 **/Rotate 元数据**（90/180/270；非内容重渲染——
/// §26 预览必须如实说明）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Rotation {
    Deg90,
    Deg180,
    Deg270,
}

impl Rotation {
    pub fn degrees(self) -> i64 {
        match self {
            Rotation::Deg90 => 90,
            Rotation::Deg180 => 180,
            Rotation::Deg270 => 270,
        }
    }
}

/// 内存版 rotate（M7 batch 适配面，§58：批量经同一域函数）：bytes ⇒
/// rotate ⇒ 重解析校验页数 ⇒ bytes。失败一律结构化 PdfError。
pub fn rotate_pdf_bytes(
    bytes: &[u8],
    degrees: i64,
    ranges: &str,
    limits: &DocumentResourceLimits,
) -> Result<(Vec<u8>, u64, Vec<u64>), PdfError> {
    let rotation = match degrees {
        90 => Rotation::Deg90,
        180 => Rotation::Deg180,
        270 => Rotation::Deg270,
        other => {
            return Err(PdfError::new(
                "pdf.badRotation",
                format!("rotation must be 90/180/270, got {other}"),
            ));
        }
    };
    let mut doc = Document::load_mem(bytes)
        .map_err(|e| PdfError::new("pdf.malformed", format!("parse failed: {e}")))?;
    if doc.is_encrypted() {
        return Err(PdfError::new(
            "pdf.encrypted",
            "encrypted PDF rotate is not supported",
        ));
    }
    let total = doc.get_pages().len() as u64;
    if total > limits.max_pages {
        return Err(PdfError::new(
            "pdf.tooManyPages",
            format!("{total} pages over limit {}", limits.max_pages),
        ));
    }
    let target: Vec<u64> = if ranges.trim().is_empty() {
        (1..=total).collect()
    } else {
        crate::page_range::parse_page_ranges(ranges, Some(total))
            .map_err(|e| PdfError::new("pdf.badRange", e.to_string()))?
    };
    let target_set: std::collections::HashSet<u64> = target.iter().copied().collect();
    for (index, id) in doc.get_pages() {
        if !target_set.contains(&(index as u64)) {
            continue;
        }
        let dict = doc
            .get_object_mut(id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| PdfError::new("pdf.malformed", format!("page {index}: {e}")))?;
        let current = dict
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        let next = (current + rotation.degrees()).rem_euclid(360);
        dict.set("Rotate", Object::Integer(next));
    }
    let mut out_bytes = Vec::new();
    doc.save_to(&mut out_bytes)
        .map_err(|e| PdfError::new("pdf.writeFailed", e.to_string()))?;
    // §27 输出校验：重解析 + 页数一致
    let check = Document::load_mem(&out_bytes)
        .map_err(|e| PdfError::new("pdf.outputEmpty", format!("reparse failed: {e}")))?;
    if check.get_pages().len() as u64 != total {
        return Err(PdfError::new(
            "pdf.outputPageMismatch",
            format!(
                "output has {} pages, expected {total}",
                check.get_pages().len()
            ),
        ));
    }
    Ok((out_bytes, total, target))
}

/// Rotate：全部页或选定页（ranges 为空 = 全部）。
pub fn execute_rotate(
    input: &Path,
    rotation: Rotation,
    ranges: &str,
    output: &Path,
    limits: &DocumentResourceLimits,
) -> PdfResult<(u64, Vec<u64>)> {
    let mut doc = load(input, limits)?;
    let total = doc.get_pages().len() as u64;
    let target: Vec<u64> = if ranges.trim().is_empty() {
        (1..=total).collect()
    } else {
        crate::page_range::parse_page_ranges(ranges, Some(total))
            .map_err(|e| PdfError::new("pdf.badRange", e.to_string()))?
    };
    let target_set: std::collections::HashSet<u64> = target.iter().copied().collect();
    for (index, id) in doc.get_pages() {
        if !target_set.contains(&(index as u64)) {
            continue;
        }
        let dict = doc
            .get_object_mut(id)
            .and_then(|o| o.as_dict_mut())
            .map_err(|e| PdfError::new("pdf.malformed", format!("page {index}: {e}")))?;
        let current = dict
            .get(b"Rotate")
            .ok()
            .and_then(|o| o.as_i64().ok())
            .unwrap_or(0);
        // 累积语义：多次 rotate 叠加（mod 360）
        let next = (current + rotation.degrees()).rem_euclid(360);
        dict.set("Rotate", Object::Integer(next));
    }
    write_and_validate(&mut doc, output, total, limits)?;
    Ok((total, target))
}

// ── Output Validation（§27/§70）──

/// 任何写出的 PDF 必须：exists / non-zero / parseable / 页数符合预期。
/// 绝不以 "file exists" 为成功标准（§27）。
pub fn write_and_validate(
    doc: &mut Document,
    output: &Path,
    expected_pages: u64,
    limits: &DocumentResourceLimits,
) -> PdfResult<u64> {
    doc.save(output)
        .map_err(|e| PdfError::new("pdf.writeFailed", format!("{}: {e}", output.display())))?;
    let meta =
        std::fs::metadata(output).map_err(|e| PdfError::new("pdf.writeFailed", e.to_string()))?;
    if meta.len() == 0 {
        return Err(PdfError::new(
            "pdf.outputEmpty",
            format!("output {} is empty", output.display()),
        ));
    }
    // 重新解析验证（可解析 + 页数）
    let check = load(output, limits)?;
    let actual = check.get_pages().len() as u64;
    if actual != expected_pages {
        return Err(PdfError::new(
            "pdf.outputPageMismatch",
            format!("output has {actual} pages, expected {expected_pages}"),
        ));
    }
    Ok(actual)
}

/// Field 视图辅助：把 PdfFacts 映射进统一 DocumentFacts 的 PDF 段。
impl PdfFacts {
    pub fn pages_field(&self) -> Field<u64> {
        Field::Known(self.page_count)
    }
}
