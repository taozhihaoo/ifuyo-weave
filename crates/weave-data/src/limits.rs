//! 统一限额（M5 上 §88 Memory Budget / §166 同源纪律，与 M4 TextLimits
//! 分档同思路）：每个核心操作的资源上限集中在此，数值基于本仓库测试
//! 校准（m5_perf），不随意拍数字。

/// 数据操作分档限额。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DataLimits {
    /// 输入文件上限（字节）。
    pub max_input_bytes: u64,
    /// 单会话驻留内存的行数上限（§88 Max Rows In Memory）。
    pub max_rows_in_memory: usize,
    /// 单次预览/分页返回的最大行数（§85 max page size）。
    pub max_page_rows: usize,
    /// Preview 采样行数（§77 representative rows）。
    pub preview_rows: usize,
    /// Exact unique 基数集合的键数上限（§47：达到后 Unavailable，不静默降级）。
    pub max_exact_unique: usize,
    /// JSON 最大嵌套深度（§91；serde_json 默认 128，此处用于预检）。
    pub max_json_depth: usize,
    /// 单次操作诊断数上限（§88 Max Error Count；超出停止收集并标注）。
    pub max_diagnostics: usize,
    /// Profiling 采样行数（§48）。
    pub profile_sample_rows: usize,
}

impl Default for DataLimits {
    fn default() -> Self {
        Self {
            max_input_bytes: 32 * 1024 * 1024,
            max_rows_in_memory: 500_000,
            max_page_rows: 1_000,
            preview_rows: 50,
            max_exact_unique: 100_000,
            max_json_depth: 128,
            max_diagnostics: 1_000,
            profile_sample_rows: 50_000,
        }
    }
}

impl DataLimits {
    /// 限额检查：超出 ⇒ (message, code)。
    pub fn check(&self, tool: &'static str, bytes: u64) -> Result<(), (String, &'static str)> {
        let max = match tool {
            "document" => self.max_input_bytes,
            "format" => self.max_input_bytes / 2,
            _ => self.max_input_bytes,
        };
        if bytes > max {
            Err((
                format!("input exceeds the {tool} size limit ({max} bytes)"),
                "text.tooLarge",
            ))
        } else {
            Ok(())
        }
    }
}
