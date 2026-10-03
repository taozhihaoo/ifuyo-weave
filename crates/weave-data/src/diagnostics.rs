//! Data Diagnostics（M5 上 §42/§63/§64）：严格区分
//! Parse Error / Validation Error / Anomaly / Warning / Info。
//!
//! "一个坏字段不能让整个任务崩溃"（§64）的落地：
//! 能安全隔离的局部问题 ⇒ 收集诊断继续；会导致数据错位的损坏 ⇒ Stop。

/// 诊断严重级（§42：Anomaly ≠ Error）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSeverity {
    /// 解析失败（结构损坏，通常伴随 Stop）。
    ParseError,
    /// 规则/参数校验失败。
    ValidationError,
    /// 值得报告的数据异常（非错误，§41/§42）。
    Anomaly,
    Warning,
    Info,
}

impl DataSeverity {
    pub fn as_str(self) -> &'static str {
        match self {
            DataSeverity::ParseError => "parseError",
            DataSeverity::ValidationError => "validationError",
            DataSeverity::Anomaly => "anomaly",
            DataSeverity::Warning => "warning",
            DataSeverity::Info => "info",
        }
    }
}

/// 判断依据分类（§41：Fact / Rule / Heuristic / Inference——禁止把启发式
/// 伪装成确定事实）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Basis {
    /// 直接观测事实。
    Fact,
    /// 用户/规则显式定义。
    Rule,
    /// 启发式推断（必须如实标注）。
    Heuristic,
}

impl Basis {
    pub fn as_str(self) -> &'static str {
        match self {
            Basis::Fact => "fact",
            Basis::Rule => "rule",
            Basis::Heuristic => "heuristic",
        }
    }
}

/// 数据诊断（行/列为 1-based；无定位时 None，不伪造）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataDiagnostic {
    pub code: String,
    pub severity: DataSeverity,
    pub message: String,
    /// 1-based 数据行号（不含 header）。
    pub row: Option<u64>,
    /// 1-based 列号。
    pub column: Option<u64>,
    /// 判断依据（Anomaly 类必填）。
    pub basis: Option<Basis>,
}

impl DataDiagnostic {
    pub fn new(
        code: impl Into<String>,
        severity: DataSeverity,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            severity,
            message: message.into(),
            row: None,
            column: None,
            basis: None,
        }
    }

    pub fn at(mut self, row: u64, column: u64) -> Self {
        self.row = Some(row);
        self.column = Some(column);
        self
    }

    pub fn with_basis(mut self, basis: Basis) -> Self {
        self.basis = Some(basis);
        self
    }
}
