//! 品牌信息单一事实源（M0 §10 / §10.1）。
//!
//! `brand.json` 在编译期通过 `include_str!` 嵌入（build-time 策略），
//! Rust 与前端都不各自复制品牌信息。该策略记录于 DECISIONS.md。

use serde::Deserialize;
use weave_core::prelude::WeaveError;

/// brand.json 的编译期副本（仓库根目录，相对本文件 `../../brand.json`）。
const BRAND_JSON: &str = include_str!("../../brand.json");

/// brand.json 的反序列化形状。
#[derive(Debug, Clone, Deserialize)]
pub struct Brand {
    pub name: String,
    pub vendor: String,
    pub site: String,
}

/// 解析嵌入的 brand.json。只在内容损坏时失败（有测试守护）。
pub fn brand() -> Result<Brand, WeaveError> {
    serde_json::from_str(BRAND_JSON).map_err(|e| {
        WeaveError::internal(
            "brand.parseFailed",
            format!("embedded brand.json is invalid: {e}"),
        )
    })
}
