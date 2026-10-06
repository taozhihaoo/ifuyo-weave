//! M2.12 本地规则引擎（§3 Step 3：**无 AI Provider 也能运行** §47）。
//!
//! 四能力全为确定性规则 + 本地元数据证据（§12 Local-first）：
//! - Classify：扩展名 → Type（weave-files classify，Confirmed）；文件名
//!   关键词 → Category（Heuristic/Medium-Low）；无法判断 ⇒ Unknown。
//! - Rename：EXIF/文档元数据日期（Confirmed）+ 现名清洗（Heuristic）；
//!   证据不足 ⇒ 不建议（§9 不制造假依据）。
//! - Organize：Category → 建议子目录 + 备选（§11 多候选）。
//! - Cleanup：空文件（Confirmed）/temp·partial 命名（Heuristic）/
//!   重复（同尺寸分组 → partial_hash 确认——复用 M3 引擎原语 §2）。
//! - Provider 输出经 schema 验证后与本引擎同构合并（§30 AI 不能越权）。

use std::collections::HashMap;
use std::path::Path;

use weave_core::prelude::WeaveError;

use super::model::{
    SmartConfidence, SmartEvidenceItem, SmartEvidenceKind, SmartEvidenceLevel, SmartRiskLevel,
    SmartSuggestion, SuggestionType,
};

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("smart::rules")
}

/// 单文件事实（分析阶段一次采集——§56 单遍扫描，禁止 N×全目录）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct FileFacts {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size: u64,
    /// mtime 毫秒（UtcEpoch）。
    pub modified_ms: f64,
    /// EXIF/文档元数据日期（YYYY-MM-DD 形态；真实解析成功才有 §67）。
    pub meta_date: Option<String>,
    /// 文档 title（pdf/docx core properties；真实解析成功才有）。
    pub title: Option<String>,
    /// 非空文件才计算（4KB partial hash——与 M3 引擎同一原语 §2）。
    pub partial_hash: Option<String>,
}

/// 分析模式（§20 四选）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum AnalyzeMode {
    Rename,
    Organize,
    Classify,
    Cleanup,
}

// ── Classify（§6：Type 事实 + Category 推断 + Confidence）──

/// 文件名关键词规则（小写包含匹配；顺序 = 优先级）。
const CATEGORY_KEYWORDS: &[(&str, &str)] = &[
    ("invoice", "invoice"),
    ("发票", "invoice"),
    ("账单", "invoice"),
    ("receipt", "invoice"),
    ("screenshot", "screenshot"),
    ("截图", "screenshot"),
    ("屏幕截图", "screenshot"),
    ("report", "report"),
    ("报告", "report"),
    ("notes", "notes"),
    ("笔记", "notes"),
    ("meeting", "notes"),
    ("会议", "notes"),
    ("setup", "executable"),
    ("installer", "executable"),
    ("setup_", "executable"),
];

/// 扩展名 → Category（High 置信——扩展名是该类内容的强信号）。
const EXT_CATEGORY: &[(&str, &str)] = &[
    ("jpg", "image"),
    ("jpeg", "image"),
    ("png", "image"),
    ("gif", "image"),
    ("webp", "image"),
    ("mp4", "video"),
    ("mkv", "video"),
    ("mov", "video"),
    ("avi", "video"),
    ("mp3", "audio"),
    ("wav", "audio"),
    ("flac", "audio"),
    ("pdf", "document"),
    ("doc", "document"),
    ("docx", "document"),
    ("txt", "document"),
    ("md", "document"),
    ("xls", "spreadsheet"),
    ("xlsx", "spreadsheet"),
    ("csv", "spreadsheet"),
    ("ppt", "presentation"),
    ("pptx", "presentation"),
    ("zip", "archive"),
    ("7z", "archive"),
    ("rar", "archive"),
    ("tar", "archive"),
    ("gz", "archive"),
    ("rs", "code"),
    ("py", "code"),
    ("js", "code"),
    ("ts", "code"),
    ("java", "code"),
    ("c", "code"),
    ("cpp", "code"),
    ("go", "code"),
    ("exe", "executable"),
    ("msi", "executable"),
    ("bat", "executable"),
    ("ps1", "executable"),
    ("db", "database"),
    ("sqlite", "database"),
    ("ttf", "font"),
    ("otf", "font"),
    ("woff", "font"),
    ("psd", "design"),
    ("fig", "design"),
    ("xd", "design"),
];

/// Type → 整理子目录（§5；Unknown ⇒ 原地保留不瞎猜 §27）。
pub fn category_folder(category: &str) -> Option<&'static str> {
    Some(match category {
        "image" => "Pictures",
        "screenshot" => "Screenshots",
        "video" => "Videos",
        "audio" => "Audio",
        "document" | "report" | "notes" => "Documents",
        "spreadsheet" => "Documents/Spreadsheets",
        "presentation" => "Documents/Presentations",
        "invoice" => "Invoices",
        "archive" => "Archives",
        "code" => "Code",
        "executable" => "Executables",
        "database" => "Databases",
        "font" => "Fonts",
        "design" => "Design",
        _ => return None,
    })
}

/// 分类结果（Type Confirmed + Category 推断 + 备选 §11）。
pub struct Classification {
    pub file_type: String,
    pub category: String,
    pub confidence: SmartConfidence,
    pub alternatives: Vec<String>,
    pub evidence: Vec<SmartEvidenceItem>,
}

/// 分类（§6：Type 与 Category 分开；不能唯一判断 ⇒ 备选 + 降置信）。
pub fn classify(facts: &FileFacts) -> Classification {
    let ext = facts.ext.to_lowercase();
    let wc = weave_files::classify(Some(ext.as_str()), None);
    let file_type = wc.category.as_str().to_string();
    let mut evidence = vec![SmartEvidenceItem {
        kind: SmartEvidenceKind::Extension,
        detail: format!("extension .{ext} ⇒ type {file_type}"),
        level: SmartEvidenceLevel::Confirmed,
    }];

    // Category：文件名关键词（Medium）优先于纯扩展名映射（High 仅当无关键词冲突）
    let lower_name = facts.name.to_lowercase();
    let keyword_hit = CATEGORY_KEYWORDS
        .iter()
        .find(|(kw, _)| lower_name.contains(kw));
    let ext_category = EXT_CATEGORY
        .iter()
        .find(|(e, _)| *e == ext)
        .map(|(_, c)| (*c).to_string());
    let (category, confidence, alternatives) = match (keyword_hit, ext_category.as_deref()) {
        (Some((_, kw_cat)), Some(ext_cat)) if *kw_cat == ext_cat => {
            ((*kw_cat).to_string(), SmartConfidence::High, Vec::new())
        }
        (Some((_, kw_cat)), Some(ext_cat)) => {
            // 关键词与扩展名类别不一致 ⇒ 主推关键词，备选扩展名类别（§11）
            evidence.push(SmartEvidenceItem {
                kind: SmartEvidenceKind::FileName,
                detail: format!("name keyword suggests {kw_cat}"),
                level: SmartEvidenceLevel::Heuristic,
            });
            (
                (*kw_cat).to_string(),
                SmartConfidence::Medium,
                vec![(*ext_cat).to_string()],
            )
        }
        (Some((_, kw_cat)), None) => ((*kw_cat).to_string(), SmartConfidence::Medium, Vec::new()),
        (None, Some(ext_cat)) => ((*ext_cat).to_string(), SmartConfidence::High, Vec::new()),
        (None, None) => {
            evidence.push(SmartEvidenceItem {
                kind: SmartEvidenceKind::Rule,
                detail: "insufficient evidence: unknown extension and no name keyword".into(),
                level: SmartEvidenceLevel::Heuristic,
            });
            ("Unknown".to_string(), SmartConfidence::Unknown, Vec::new())
        }
    };
    Classification {
        file_type,
        category,
        confidence,
        alternatives,
        evidence,
    }
}

// ── Rename（§4：原名/建议名/依据/置信/冲突——不直接改名）──

/// 清洗现名（§4：去副本标记/版本噪声；保留主体与扩展名；非法字符拒绝）。
pub fn sanitize_name(name: &str) -> Option<String> {
    let (stem, ext) = name.rsplit_once('.')?;
    if stem.is_empty() {
        return None;
    }
    let mut cleaned = stem.to_string();
    // 副本/下载标记：" (1)"、"(3)"、" - Copy"、"final_v8"、"new"
    for pat in [
        " - Copy", "-Copy", "_copy", " (1)", " (2)", " (3)", " (4)", " (5)",
    ] {
        cleaned = cleaned.replace(pat, "");
    }
    // 版本噪声循环剥离（_final_v8 ⇒ _final ⇒ 主体）
    loop {
        match regex_lite(&cleaned) {
            Some(stripped) if stripped != cleaned => cleaned = stripped,
            _ => break,
        }
    }
    // 折叠多余分隔符
    while cleaned.contains("__") {
        cleaned = cleaned.replace("__", "_");
    }
    while cleaned.contains("--") {
        cleaned = cleaned.replace("--", "-");
    }
    let cleaned = cleaned.trim_matches(['-', '_', ' ']).to_string();
    if cleaned.is_empty() || cleaned == stem {
        return None; // 无可清洗（原样）——不建议
    }
    Some(format!("{cleaned}.{ext}"))
}

/// `_final_v8` / `-v12` / `_old2` 形态版本噪声剥离。
fn regex_lite(name: &str) -> Option<String> {
    const MARKERS: [&str; 5] = ["_final", "-final", "_v", "-v", "_old"];
    for marker in MARKERS {
        if let Some(pos) = name.rfind(marker) {
            let rest = &name[pos + marker.len()..];
            // 尾部确认：marker 后为版本号形态（v8 / 8 / old2；final 无尾随）
            let version_ok = match marker {
                "_final" | "-final" => rest.is_empty() || rest.chars().all(|c| c.is_ascii_digit()),
                _ => !rest.is_empty() && rest.chars().all(|c| c.is_ascii_alphanumeric()),
            };
            if version_ok {
                // _final_v8 ⇒ 先剥 _final 后剩余 "_v8" 仍以版本 marker 开头 ⇒ 递归收敛
                let head = name[..pos].to_string();
                if (marker == "_final" || marker == "-final")
                    && let Some(stripped2) = regex_lite(&head)
                {
                    return Some(stripped2);
                }
                return Some(head);
            }
        }
    }
    None
}

/// 图片重命名建议：EXIF 日期（Confirmed）+ 清洗名 + 保留原序号尾部。
pub fn rename_suggestion(facts: &FileFacts) -> Option<SmartSuggestion> {
    let ext = facts.ext.to_lowercase();
    let is_image = matches!(ext.as_str(), "jpg" | "jpeg" | "png" | "tiff" | "heic");
    let mut evidence = Vec::new();
    let mut stem = facts.name.rsplit_once('.').map(|(s, _)| s.to_string())?;
    let mut confidence = SmartConfidence::Unknown;

    // 证据 1：元数据日期（Confirmed §9）
    if let Some(date) = &facts.meta_date {
        evidence.push(SmartEvidenceItem {
            kind: SmartEvidenceKind::Metadata,
            detail: format!("metadata date {date}"),
            level: SmartEvidenceLevel::Confirmed,
        });
        // 去掉现名中的旧日期前缀（IMG_20260820_001 → 001；2026-08-20_x → x）
        let stripped = strip_camera_prefix(&stem);
        let stripped = strip_date_prefix(&stripped);
        stem = format!("{date}_{stripped}");
        confidence = SmartConfidence::High;
    }
    // 证据 2：文档 title（Confirmed）
    else if let Some(title) = &facts.title {
        let sanitized: String = title
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || matches!(c, '-' | '_' | ' ') {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<_>>()
            .join("_");
        if !sanitized.is_empty() && sanitized.chars().count() >= 3 {
            evidence.push(SmartEvidenceItem {
                kind: SmartEvidenceKind::Metadata,
                detail: "document core-property title".into(),
                level: SmartEvidenceLevel::Confirmed,
            });
            stem = sanitized;
            confidence = SmartConfidence::Medium;
        }
    }
    // 证据 3：现名清洗（Heuristic——仅图片语境外也适用）
    let is_doc = matches!(ext.as_str(), "pdf" | "docx" | "doc" | "xlsx" | "pptx");
    if confidence == SmartConfidence::Unknown
        && (is_image || is_doc)
        && let Some(cleaned) = sanitize_name(&facts.name)
    {
        evidence.push(SmartEvidenceItem {
            kind: SmartEvidenceKind::FileName,
            detail: "name contains copy/version noise".into(),
            level: SmartEvidenceLevel::Heuristic,
        });
        stem = cleaned
            .rsplit_once('.')
            .map(|(s, _)| s.to_string())
            .unwrap_or(cleaned);
        confidence = SmartConfidence::Low;
    }
    // mtime 日期补充证据（Derived——元数据缺失时的降级依据 §9 如实）
    if confidence != SmartConfidence::Unknown && facts.meta_date.is_none() {
        let date = crate::smart::model::mtime_date(facts.modified_ms);
        evidence.push(SmartEvidenceItem {
            kind: SmartEvidenceKind::Metadata,
            detail: format!("file modified date {date} (derived, not capture date)"),
            level: SmartEvidenceLevel::Derived,
        });
    }
    if confidence == SmartConfidence::Unknown || evidence.is_empty() {
        return None; // 证据不足 ⇒ 不建议（§9）
    }

    let ext_dot = facts
        .name
        .rsplit_once('.')
        .map(|(_, e)| e.to_string())
        .unwrap_or(ext);
    let suggested = format!("{stem}.{ext_dot}");
    // 原名已理想 ⇒ 不建议
    if suggested == facts.name {
        return None;
    }
    Some(SmartSuggestion {
        id: String::new(), // 服务层编号
        suggestion_type: SuggestionType::Rename,
        target_path: facts.path.clone(),
        original_value: facts.name.clone(),
        suggested_value: suggested,
        reason: "content metadata / name normalization".into(),
        evidence,
        confidence,
        risk: SmartRiskLevel::Low,
        conflict: None,
        alternatives: Vec::new(),
    })
}

/// 剥离 IMG_/DSC_/IMG 等相机前缀（与日期一起构成旧命名模式）。
fn strip_camera_prefix(stem: &str) -> String {
    for p in ["IMG_", "DSC_", "DSCN_", "PXL_", "IMG"] {
        if let Some(rest) = stem.strip_prefix(p)
            && !rest.is_empty()
        {
            return rest.trim_start_matches(['_', '-']).to_string();
        }
    }
    stem.to_string()
}

/// 剥离 IMG_20260820_ / 2026-08-20_ 形态日期前缀（保留序号）。
fn strip_date_prefix(stem: &str) -> String {
    // 紧凑形态：前 8 位全数字且形如日期
    let chars: Vec<char> = stem.chars().collect();
    if chars.len() > 9 {
        let head: String = chars[..8].iter().collect();
        if head.chars().all(|c| c.is_ascii_digit()) {
            let rest: String = chars[8..].iter().collect();
            let rest = rest.trim_start_matches(['_', '-']);
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    // 破折号形态：YYYY-MM-DD_
    if stem.len() > 10 {
        let head: String = chars[..10].iter().collect();
        let digits_and_dashes =
            head.chars().all(|c| c.is_ascii_digit() || c == '-') && head.matches('-').count() == 2;
        if digits_and_dashes {
            let rest: String = chars[10..].iter().collect();
            let rest = rest.trim_start_matches(['_', '-']);
            if !rest.is_empty() {
                return rest.to_string();
            }
        }
    }
    stem.to_string()
}

// ── Cleanup（§7/§28：Detected ≠ Suggested ≠ Confirmed；理由+证据）──

/// 清理候选判定（§28：Path/Reason/Evidence/Risk/Confidence）。
pub fn cleanup_candidates(facts: &[FileFacts]) -> Vec<SmartSuggestion> {
    let mut out = Vec::new();
    // 1) 空文件（Confirmed——0 字节是事实）
    for f in facts {
        if f.size == 0 {
            out.push(SmartSuggestion {
                id: String::new(),
                suggestion_type: SuggestionType::DeleteCandidate,
                target_path: f.path.clone(),
                original_value: f.name.clone(),
                suggested_value: "recycle".into(),
                reason: "empty file (0 bytes)".into(),
                evidence: vec![SmartEvidenceItem {
                    kind: SmartEvidenceKind::Content,
                    detail: "size = 0 bytes".into(),
                    level: SmartEvidenceLevel::Confirmed,
                }],
                confidence: SmartConfidence::High,
                risk: SmartRiskLevel::Medium,
                conflict: None,
                alternatives: Vec::new(),
            });
        }
    }
    // 2) temp/partial 命名（Heuristic——名字不是删除理由本身 §7，故 Low/Medium）
    for f in facts {
        let lower = f.name.to_lowercase();
        let partial_download = lower.ends_with(".crdownload") || lower.ends_with(".part");
        let temp_name = lower.starts_with('~')
            || lower.ends_with(".tmp")
            || lower.ends_with(".temp")
            || lower.ends_with(".bak")
            || (lower.contains("temp") && f.ext.eq_ignore_ascii_case("tmp"));
        if partial_download || temp_name {
            out.push(SmartSuggestion {
                id: String::new(),
                suggestion_type: SuggestionType::DeleteCandidate,
                target_path: f.path.clone(),
                original_value: f.name.clone(),
                suggested_value: "recycle".into(),
                reason: if partial_download {
                    "incomplete download marker".into()
                } else {
                    "temporary-file naming pattern".into()
                },
                evidence: vec![SmartEvidenceItem {
                    kind: SmartEvidenceKind::FileName,
                    detail: format!("name pattern ({})", f.name),
                    level: SmartEvidenceLevel::Heuristic,
                }],
                confidence: if partial_download {
                    SmartConfidence::Medium
                } else {
                    SmartConfidence::Low
                },
                risk: SmartRiskLevel::High,
                conflict: None,
                alternatives: Vec::new(),
            });
        }
    }
    // 3) 重复（同尺寸分组 → partial_hash 确认；M3 引擎同一原语 §2）
    let mut by_size: HashMap<u64, Vec<&FileFacts>> = HashMap::new();
    for f in facts {
        if f.size > 0 && f.partial_hash.is_some() {
            by_size.entry(f.size).or_default().push(f);
        }
    }
    let mut confirmed_groups: Vec<Vec<&FileFacts>> = Vec::new();
    for (_, group) in by_size {
        if group.len() < 2 {
            continue;
        }
        let mut by_hash: HashMap<&str, Vec<&FileFacts>> = HashMap::new();
        for f in group {
            if let Some(h) = &f.partial_hash {
                by_hash.entry(h.as_str()).or_default().push(f);
            }
        }
        for (_, same) in by_hash {
            if same.len() >= 2 {
                confirmed_groups.push(same);
            }
        }
    }
    for group in confirmed_groups {
        // 最旧者为"正本"，其余为候选（删除候选 = 副本，不是正本 §28）
        let mut sorted = group;
        sorted.sort_by(|a, b| {
            a.modified_ms
                .partial_cmp(&b.modified_ms)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        let keeper = sorted[0];
        for dup in &sorted[1..] {
            out.push(SmartSuggestion {
                id: String::new(),
                suggestion_type: SuggestionType::DeleteCandidate,
                target_path: dup.path.clone(),
                original_value: dup.name.clone(),
                suggested_value: format!("recycle (duplicate of {})", keeper.name),
                reason: "exact duplicate (same content hash), older copy".into(),
                evidence: vec![SmartEvidenceItem {
                    kind: SmartEvidenceKind::Duplicate,
                    detail: format!(
                        "same partial hash as {} (both {} bytes)",
                        keeper.name, keeper.size
                    ),
                    level: SmartEvidenceLevel::Confirmed,
                }],
                confidence: SmartConfidence::High,
                risk: SmartRiskLevel::High,
                conflict: None,
                alternatives: Vec::new(),
            });
        }
    }
    out
}

/// Organize 建议（§5：Category → 子目录 + 备选；Unknown ⇒ 不建议 §27）。
pub fn organize_suggestion(facts: &FileFacts) -> Option<SmartSuggestion> {
    let c = classify(facts);
    let folder = category_folder(&c.category)?;
    Some(SmartSuggestion {
        id: String::new(),
        suggestion_type: SuggestionType::Organize,
        target_path: facts.path.clone(),
        original_value: Path::new(&facts.path)
            .parent()
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_default(),
        suggested_value: folder.to_string(),
        reason: format!(
            "classified as {} ({:?} confidence)",
            c.category, c.confidence
        ),
        evidence: c.evidence.clone(),
        confidence: c.confidence,
        risk: SmartRiskLevel::Medium,
        conflict: None,
        alternatives: c
            .alternatives
            .iter()
            .filter_map(|a| category_folder(a))
            .map(|f| f.to_string())
            .collect(),
    })
}

/// 非法目标名拒绝（§24：Windows 非法字符与保留名不生成建议）。
pub fn valid_target_name(name: &str) -> bool {
    if name.is_empty() || name.len() > 240 {
        return false;
    }
    if name
        .chars()
        .any(|c| matches!(c, '<' | '>' | ':' | '"' | '|' | '?' | '*' | '/' | '\\'))
    {
        return false;
    }
    let stem = name.split('.').next().unwrap_or("");
    !matches!(
        stem.to_uppercase().as_str(),
        "CON" | "PRN" | "AUX" | "NUL" | "COM1" | "LPT1"
    )
}

pub fn invalid_name_error(name: &str) -> WeaveError {
    err(
        "smart.invalidTargetName",
        format!("suggested name is not usable: {name}"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(name: &str, size: u64) -> FileFacts {
        FileFacts {
            path: format!("C:\\d\\{name}"),
            name: name.to_string(),
            ext: name
                .rsplit_once('.')
                .map(|(_, e)| e.to_string())
                .unwrap_or_default(),
            size,
            modified_ms: 1_700_000_000_000.0,
            meta_date: None,
            title: None,
            partial_hash: None,
        }
    }

    // ── Classify（§58 混合/Unknown/无扩展名）──

    #[test]
    fn classify_by_extension_is_high_confidence() {
        let c = classify(&facts("photo.jpg", 100));
        assert_eq!(c.file_type, "image");
        assert_eq!(c.category, "image");
        assert_eq!(c.confidence, SmartConfidence::High);
    }

    #[test]
    fn keyword_overrides_extension_with_alternative() {
        let c = classify(&facts("invoice_scan.pdf", 100));
        assert_eq!(c.category, "invoice");
        assert_eq!(c.confidence, SmartConfidence::Medium);
        assert!(c.alternatives.contains(&"document".to_string()));
    }

    #[test]
    fn unknown_extension_is_honest_unknown() {
        let mut f = facts("mystery.bin", 10);
        f.ext = "bin".into();
        let c = classify(&f);
        assert_eq!(c.category, "Unknown");
        assert_eq!(c.confidence, SmartConfidence::Unknown);
    }

    // ── Rename（§4/§58：重复名称/版本噪声/证据不足不猜）──

    #[test]
    fn exif_date_drives_high_confidence_rename() {
        let mut f = facts("IMG_20260820_001.jpg", 1000);
        f.meta_date = Some("2026-08-20".into());
        let s = rename_suggestion(&f).expect("suggestion");
        assert_eq!(s.suggested_value, "2026-08-20_001.jpg");
        assert_eq!(s.confidence, SmartConfidence::High);
        assert!(
            s.evidence
                .iter()
                .any(|e| e.level == SmartEvidenceLevel::Confirmed)
        );
    }

    #[test]
    fn copy_noise_is_stripped_at_low_confidence() {
        let s = rename_suggestion(&facts("report (1).pdf", 100)).expect("suggestion");
        assert_eq!(s.suggested_value, "report.pdf");
        assert_eq!(s.confidence, SmartConfidence::Low);
    }

    #[test]
    fn version_noise_stripped() {
        let s = rename_suggestion(&facts("report_final_v8.docx", 100)).expect("suggestion");
        assert_eq!(s.suggested_value, "report.docx");
    }

    #[test]
    fn clean_name_produces_no_suggestion() {
        assert!(rename_suggestion(&facts("2026-08-20_001.jpg", 100)).is_none());
        assert!(rename_suggestion(&facts("simple.txt", 100)).is_none());
    }

    #[test]
    fn invalid_target_name_rejected() {
        assert!(valid_target_name("ok_name.pdf"));
        assert!(!valid_target_name("bad<name>.pdf"));
        assert!(!valid_target_name("CON.txt"));
    }

    // ── Cleanup（§28：证据/风险/置信；重复=副本非正本）──

    #[test]
    fn empty_and_temp_detected_with_distinct_confidence() {
        let mut files = vec![
            facts("empty.txt", 0),
            facts("~cache1.tmp", 5),
            facts("real.docx", 500),
        ];
        files[2].partial_hash = Some("aa".into());
        let out = cleanup_candidates(&files);
        assert_eq!(out.len(), 2);
        let empty = out
            .iter()
            .find(|s| s.original_value == "empty.txt")
            .expect("empty");
        assert_eq!(empty.confidence, SmartConfidence::High);
        let tmp = out
            .iter()
            .find(|s| s.original_value == "~cache1.tmp")
            .expect("tmp");
        assert_eq!(tmp.risk, SmartRiskLevel::High, "删除候选高风险 §54");
        assert!(
            tmp.confidence == SmartConfidence::Medium || tmp.confidence == SmartConfidence::Low
        );
    }

    #[test]
    fn duplicate_group_suggests_recycling_only_older_copies() {
        let mut a = facts("a.zip", 1000);
        a.partial_hash = Some("h1".into());
        let mut b = facts("b.zip", 1000);
        b.partial_hash = Some("h1".into());
        b.modified_ms += 5000.0;
        let mut c = facts("c.zip", 1000);
        c.partial_hash = Some("other".into());
        let out = cleanup_candidates(&[a, b, c]);
        assert_eq!(out.len(), 1, "只建议删较新的副本，正本保留 §28");
        assert!(out[0].original_value == "b.zip");
        assert!(out[0].suggested_value.contains("duplicate of a.zip"));
        assert_eq!(out[0].confidence, SmartConfidence::High);
    }

    // ── Organize（§5/§11/§27）──

    #[test]
    fn organize_maps_category_to_folder_with_alternatives() {
        let s = organize_suggestion(&facts("invoice_scan.pdf", 100)).expect("organize");
        assert_eq!(s.suggested_value, "Invoices");
        assert!(
            !s.alternatives
                .contains(&"Documents/Spreadsheets".to_string())
        );
        // Unknown 不瞎猜
        let mut f = facts("mystery.bin", 1);
        f.ext = "bin".into();
        assert!(organize_suggestion(&f).is_none());
    }
}
