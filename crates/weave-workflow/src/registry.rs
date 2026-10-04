//! Tool Registry（M10 上 §12-§25）：workflow 可用 Tool/Filter 的单一
//! 事实来源（§25 禁止 UI/后端各维护一份）。
//!
//! v1 注册 = **真实已实现** 的 M7 Stage 能力（§80 模板必须可执行）：
//! - tool `document.inspect` → M7 DocumentInspect（§65 read-only）
//! - tool `pdf.rotate`       → M7 PdfRotate（mutating 产物）
//! - tool `image.resize`     → M7 ImageResize
//! - tool `image.encode`     → M7 Encode
//! - filter `filter.extension` → M7 Filter
//!
//! §13/§15：每个 Tool 声明 Consumes/Produces/Mutates/Supports。

/// §15 Tool Contract。
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    /// §13：如 "pdf.rotate"（服从既有命名：域.动作）。
    pub tool_id: &'static str,
    pub name: &'static str,
    pub consumes: &'static str,
    pub produces: &'static str,
    /// §16：Pure = 只计算；Mutating = 产物写盘（经 M7/M2）。
    pub mutating: bool,
    pub preview_supported: bool,
    pub batch_supported: bool,
    /// §24 config schema（精简描述性 JSON；校验在 Validation）。
    pub config_schema: serde_json::Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilterDescriptor {
    pub filter_id: &'static str,
    pub name: &'static str,
}

/// §25 Registry（v1 = 静态注册表；全部映射到真实 M7 Stage）。
pub struct ToolRegistry;

impl ToolRegistry {
    /// 全部可注册 Tool（单一事实来源）。
    pub fn tools() -> Vec<ToolDescriptor> {
        vec![
            ToolDescriptor {
                tool_id: "document.inspect",
                name: "Document Inspect",
                consumes: "bytes",
                produces: "bytes",
                mutating: false,
                preview_supported: true,
                batch_supported: true,
                config_schema: serde_json::json!({}),
            },
            ToolDescriptor {
                tool_id: "pdf.rotate",
                name: "PDF Rotate",
                consumes: "bytes",
                produces: "bytes",
                mutating: true,
                preview_supported: true,
                batch_supported: true,
                config_schema: serde_json::json!({
                    "degrees": "number (90|180|270)",
                    "pages": "string (page ranges, empty = all)"
                }),
            },
            ToolDescriptor {
                tool_id: "image.resize",
                name: "Image Resize",
                consumes: "bytes",
                produces: "bytes",
                mutating: true,
                preview_supported: true,
                batch_supported: true,
                config_schema: serde_json::json!({
                    "width": "number",
                    "height": "number",
                    "mode": "string (fit|fill|exact|scale)",
                    "preventUpscale": "boolean"
                }),
            },
            ToolDescriptor {
                tool_id: "image.encode",
                name: "Image Encode",
                consumes: "bytes",
                produces: "bytes",
                mutating: true,
                preview_supported: true,
                batch_supported: true,
                config_schema: serde_json::json!({
                    "format": "string (png|jpeg|webp|bmp|tiff)",
                    "quality": "number|null"
                }),
            },
        ]
    }

    pub fn filters() -> Vec<FilterDescriptor> {
        vec![FilterDescriptor {
            filter_id: "filter.extension",
            name: "Extension filter",
        }]
    }

    pub fn tool(tool_id: &str) -> Option<ToolDescriptor> {
        Self::tools().into_iter().find(|t| t.tool_id == tool_id)
    }

    pub fn filter(filter_id: &str) -> Option<FilterDescriptor> {
        Self::filters()
            .into_iter()
            .find(|f| f.filter_id == filter_id)
    }
}
