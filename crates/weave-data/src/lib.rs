//! weave-data — Structured data tools: CSV / JSON / TSV / JSONL (M5).
//!
//! 职责（§82）：DataDocument、CSV/TSV/JSON/JSONL 解析、Schema、Type
//! Detection（potential，非保证）、Profiler、过滤/排序/变换/去重、转换、
//! 导出序列化、Data Diagnostics。不负责：Tauri、React、filesystem 实现、
//! 路径校验、历史持久化。
//!
//! 核心纪律（§5/§7）：CSV 单元格是**原始文本**；类型只是 potential 观测；
//! 整数精度不得因 f64 转换静默丢失——保留原始文本或显式报告。

pub mod csv_parse;
pub mod diagnostics;
pub mod json_doc;
pub mod jsonl;
pub mod limits;
pub mod table;

pub use csv_parse::{CsvDialect, HeaderDecision, parse_csv};
pub use diagnostics::{DataDiagnostic, DataSeverity};
pub use json_doc::{JsonProfile, JsonRootKind, JsonSchemaPath, profile_json};
pub use jsonl::{JsonlErrorMode, JsonlReport, parse_jsonl};
pub use limits::DataLimits;
pub use table::{ColumnDefinition, DataTable};

/// 数据格式（§0.2：CSV 与 TSV 在模型层面显式区分，不靠扩展名）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataFormat {
    Csv,
    Tsv,
    Json,
    Jsonl,
}

impl DataFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            DataFormat::Csv => "csv",
            DataFormat::Tsv => "tsv",
            DataFormat::Json => "json",
            DataFormat::Jsonl => "jsonl",
        }
    }

    /// 按扩展名推断（仅供 UI 预填；真正格式由用户/检测确认）。
    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "csv" => Some(DataFormat::Csv),
            "tsv" | "tab" => Some(DataFormat::Tsv),
            "json" => Some(DataFormat::Json),
            "jsonl" | "ndjson" => Some(DataFormat::Jsonl),
            _ => None,
        }
    }
}

/// 数据单元 = 原始文本（§5：CSV 无原生类型；"00123" 永远是 "00123"，
/// potential type 只存在于 schema/profile 层）。
pub type Cell = String;

/// 数据文档：解析产物（§4 DataDocument）。
#[derive(Debug, Clone, PartialEq)]
pub struct DataDocument {
    pub format: DataFormat,
    pub table: DataTable,
    pub diagnostics: Vec<DataDiagnostic>,
    /// 原始字节大小（加载事实）。
    pub source_bytes: u64,
}
