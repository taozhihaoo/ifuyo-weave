//! 条目负载与上下文（M7 上 §17/§18）。
//!
//! ItemPayload 是流水线的有界负载表示（Bytes/Text/Image）；
//! ItemContext 携带稳定 ItemId（§18）与阶段日志，不在阶段间复制大对象
//! ——payload 按 move 传递。

use image::DynamicImage;

/// 流水线条目负载。
#[derive(Debug, Clone)]
pub enum ItemPayload {
    Bytes(Vec<u8>),
    Text(String),
    Image(DynamicImage),
}

impl ItemPayload {
    pub fn type_name(&self) -> &'static str {
        match self {
            ItemPayload::Bytes(_) => "bytes",
            ItemPayload::Text(_) => "text",
            ItemPayload::Image(_) => "image",
        }
    }

    pub fn as_bytes(&self) -> Option<&[u8]> {
        match self {
            ItemPayload::Bytes(b) => Some(b.as_slice()),
            _ => None,
        }
    }
}

/// 条目上下文（§17）：稳定 ItemId（§18）+ 当前负载 + 阶段日志。
#[derive(Debug, Clone)]
pub struct ItemContext {
    /// 稳定且 Job 内唯一：`item_{n}`（n = 快照序，0-based）。
    pub item_id: String,
    pub source_path: std::path::PathBuf,
    pub payload: ItemPayload,
    /// 当前编码扩展名（Encode 阶段更新；Export 命名依据，§76）。
    pub current_ext: String,
    /// 阶段日志（§28 stage diagnostics 摘要）。
    pub stage_log: Vec<(String, String)>,
}
