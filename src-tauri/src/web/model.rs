//! M2.10 Web 内容处理中心——领域模型（§5）。
//!
//! - specta 纪律：无 u64/SystemTime/usize（毫秒 f64、计数 u32、偏移 u32）。
//! - 证据可追踪（§67）：每个提取字段带 source；启发式结果带 confidence，
//!   **不把 heuristic 当事实（§27）**。
//! - 隐私（§53/§54）：展示/日志用 URL 一律经 sensitive masking。

/// 查询参数分类（§8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum UrlParameterCategory {
    /// 高置信度追踪参数（utm_* / fbclid / gclid…）——默认建议删除。
    Tracking,
    /// 功能参数（预留给未来用户显式白名单——本版不猜测赋值）。
    Functional,
    /// 认证相关（token/signature/session…）——**默认保留**，展示脱敏。
    AuthRelated,
    /// 未知业务参数——**必须保留（§8）**。
    Unknown,
}

/// 单个查询参数（§5 UrlParameter）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UrlParameter {
    pub name: String,
    pub value: String,
    pub category: UrlParameterCategory,
}

/// URL 清理结果（§9 UrlCleanResult）——删除了什么、保留了什么、为什么。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UrlCleanResult {
    pub original_url: String,
    /// 规范化 URL（请求/比较/缓存键用；§6 与 Display 区分）。
    pub normalized_url: String,
    /// 清理后 URL（默认仅移除 Tracking）。
    pub cleaned_url: String,
    pub removed_parameters: Vec<UrlParameter>,
    pub preserved_parameters: Vec<UrlParameter>,
    pub unknown_parameters: Vec<UrlParameter>,
    pub warnings: Vec<String>,
}

/// 展示用脱敏 URL（§54：token=***——日志/历史/报告一律用这个）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct UrlDisplay {
    pub original: String,
    pub masked: String,
    pub sensitive_parameter_names: Vec<String>,
}

/// 链接类型（§11；不可靠判断 ⇒ Unknown，不硬分类）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WebLinkType {
    Internal,
    External,
    Relative,
    Mail,
    Tel,
    Anchor,
    Unknown,
}

/// 页面链接（§11 WebLink；absolute 基于文档 base URL 解析 §12）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebLink {
    pub absolute_url: String,
    pub display_text: String,
    pub link_type: WebLinkType,
    /// href 原文（相对链接保留原文证据 §55）。
    pub raw_href: String,
}

/// 元数据字段证据（§67：值 + 来源；缺失 ⇒ 字段整个缺席，不猜 §28）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MetaField {
    pub value: String,
    /// 证据来源（如 "title-tag" / "og:title" / "meta[name=author]" /
    /// "canonical" / "html-lang" / "heuristic"）。
    pub source: String,
}

/// 页面元数据（§28；缺失即缺席）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct PageMetadata {
    pub title: Option<MetaField>,
    pub description: Option<MetaField>,
    pub author: Option<MetaField>,
    pub published_time: Option<MetaField>,
    pub modified_time: Option<MetaField>,
    pub language: Option<MetaField>,
    /// canonical（§29：仅记录；绝不静默替换 RequestedUrl）。
    pub canonical_url: Option<MetaField>,
    pub og_title: Option<MetaField>,
    pub og_description: Option<MetaField>,
    pub og_image: Option<MetaField>,
}

/// 正文提取置信度（§27：High/Medium/Low/Unknown——UI 必须呈现"这是推断"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ExtractionConfidence {
    High,
    Medium,
    Low,
    Unknown,
}

/// 编码判定来源（§21：不静默宣称 UTF-8）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum CharsetSource {
    HttpHeader,
    Bom,
    MetaTag,
    /// 合理默认（如实标注 default）。
    Utf8Default,
}

/// Content-Type 识别（§20；核心只处理 HTML/Text，其余 Recognized but
/// Not Supported ⇒ 交还用户/既有模块）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum WebContentType {
    Html,
    PlainText,
    Json,
    Xml,
    Image,
    Pdf,
    Binary,
    Unknown,
}

/// 抓取元信息（§5 WebFetchResult 的 metadata 部分；§13-§15/§29）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebFetchInfo {
    pub requested_url: String,
    pub final_url: String,
    pub status: u32,
    pub content_type: WebContentType,
    pub content_type_raw: String,
    pub charset: String,
    pub charset_source: CharsetSource,
    pub size_bytes: f64,
    pub duration_ms: f64,
    pub redirect_count: u32,
    pub redirect_chain: Vec<String>,
}

/// 抓取 + 提取 + 转换的统一文档（§5 WebDocument / §66 WebContent——
/// 按需生成：text/markdown/links 在 analyze 时一次算好，原始字节只存缓存）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct WebDocument {
    pub info: WebFetchInfo,
    pub metadata: PageMetadata,
    /// 正文提取置信（heuristic 不冒充事实 §27）。
    pub content_confidence: ExtractionConfidence,
    pub content_source: String,
    pub plain_text: String,
    pub markdown: String,
    pub links: Vec<WebLink>,
    pub truncated: bool,
}
