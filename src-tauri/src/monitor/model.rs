//! M2.11 Website Monitor 数据模型（§4/§7/§9/§47）。
//!
//! - specta 纪律：无 u64/SystemTime/usize（毫秒 f64、计数 u32）。
//! - 状态机（§47）业务层统一定义，UI 不自行推导。
//! - ChangeType 保持"一眼可懂"（§9 不人为复杂分类）；Added/Removed/
//!   Modified 由 diff 统计表达，不进顶层枚举。
//! - 隐私（§62）：模型只含元数据；正文仅存 snapshot payload 文件。

/// 监控范围（§5：WholePage/MainContent/Element——三种全真实可用，
/// 无空壳枚举；Element = CSS Selector，scraper 现成能力）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum MonitorMode {
    WholePage,
    MainContent,
    Element,
}

/// 目标运行状态（§47 状态机；业务层唯一事实源）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum TargetStatus {
    /// 已创建但从未检查。
    Waiting,
    /// 检查进行中。
    Checking,
    /// 已启用，等待下次计划检查。
    Idle,
    /// 最近一次检查发现内容变化（§47 状态机含 Changed）。
    Changed,
    /// 暂停（不自动检查；历史保留 §64）。
    Paused,
    /// 已禁用（§65 与 Paused/Deleted 区分）。
    Disabled,
    /// 最近一次检查失败（Recoverable——不覆盖成功快照 §15）。
    Failed,
    /// 被拒绝访问（403 等 §19）。
    Blocked,
    /// 内容类型/动态渲染不支持（§36/§54）。
    Unsupported,
}

/// 检查结果 / 变化类型（§9/§14/§15/§73）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum CheckResult {
    /// 首次检查建立基线（不是 Changed §14）。
    Initial,
    NoChange,
    /// 内容指纹变化（Added/Removed/Modified 由 diff 统计表达）。
    ContentChanged,
    /// 失败后恢复成功（§73；是否通知由设置决定）。
    Recovered,
    /// 抓取失败（网络/超时/5xx）——不污染成功快照 §15。
    FetchFailed,
    /// 访问被拒（403 等 §19）。
    Blocked,
    /// 不支持的内容/动态渲染（§36/§54）。
    Unsupported,
    Cancelled,
}

/// 变化强度（§57：变化量指标，非语义重要性——不说"重大新闻"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum ChangeMagnitude {
    None,
    Small,
    Medium,
    Large,
}

/// 监控目标（§4 精简为真实使用的字段）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MonitorTarget {
    pub id: String,
    pub name: String,
    pub url: String,
    /// enabled=false ⇒ Disabled（§65）；paused 独立（§64）。
    pub enabled: bool,
    pub paused: bool,
    pub mode: MonitorMode,
    /// Element 模式的 CSS selector（§34 最低要求：可输入可测试）。
    pub selector: Option<String>,
    /// 检查间隔秒（下限 60——§18 低频克制）。
    pub interval_secs: u32,
    /// 变化时通知（§22）。
    pub notify_on_change: bool,
    /// 失败时通知。
    pub notify_on_error: bool,
    /// 失败后恢复成功时通知（§73）。
    pub notify_on_recovery: bool,
    /// 连续失败计数（backoff 依据 §18：interval × 2^min(n,4)）。
    pub consecutive_failures: u32,
    pub last_checked_at_ms: Option<f64>,
    pub last_success_at_ms: Option<f64>,
    /// 最近一次成功快照指纹（快速比较 §10）。
    pub last_fingerprint: Option<String>,
    pub last_snapshot_seq: Option<f64>,
    pub status: TargetStatus,
    pub last_error_code: Option<String>,
    pub created_at_ms: f64,
    pub updated_at_ms: f64,
}

/// 快照元数据（§7；正文 payload 按需存文件——NoChange 不存正文 §46）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSnapshot {
    pub target_id: String,
    pub seq: f64,
    pub fetched_at_ms: f64,
    pub status_code: u32,
    pub final_url: String,
    pub content_type: String,
    /// 规范化文本指纹（§10/§55：基于真正比较对象——规范化后的提取文本）。
    pub fingerprint: String,
    pub text_size: u32,
    pub duration_ms: f64,
    /// 该次检查结果（历史列表"是否变化"列 §8）。
    pub result: CheckResult,
    /// 正文 payload 相对文件名；None = metadata-only（NoChange/失败 §46）。
    pub payload_file: Option<String>,
    pub truncated: bool,
}

/// 变化事件（§9/§13：确定性摘要，无 AI §60）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct MonitorChangeEvent {
    pub id: String,
    pub target_id: String,
    pub detected_at_ms: f64,
    pub result: CheckResult,
    pub magnitude: ChangeMagnitude,
    /// diff 统计（Added/Removed/Modified 的行级表达 §9）。
    pub added_lines: u32,
    pub removed_lines: u32,
    pub previous_seq: Option<f64>,
    pub current_seq: f64,
    /// 确定性摘要（规则生成 §13，不含正文 §62）。
    pub summary: String,
}

/// 检查间隔下限（§18：用户设 1 分钟不会变成每秒多次请求）。
pub const MIN_INTERVAL_SECS: u32 = 60;

/// 失败退避（§18）：interval × 2^min(failures, 4)。
pub fn backoff_delay_secs(base_interval_secs: u32, consecutive_failures: u32) -> u64 {
    let shift = consecutive_failures.min(4);
    (base_interval_secs.max(MIN_INTERVAL_SECS) as u64) << shift
}

/// 变化强度判定（§57：按行级增删占原行数比例；确定性规则）。
pub fn magnitude_for(added_lines: u32, removed_lines: u32, previous_lines: u32) -> ChangeMagnitude {
    let changed = added_lines + removed_lines;
    if changed == 0 {
        return ChangeMagnitude::None;
    }
    let base = previous_lines.max(1) as f64;
    let ratio = changed as f64 / base;
    if ratio >= 1.0 {
        ChangeMagnitude::Large
    } else if ratio >= 0.25 {
        ChangeMagnitude::Medium
    } else {
        ChangeMagnitude::Small
    }
}

/// 确定性摘要（§13：规则化、可解释、不含正文内容 §62）。
pub fn summarize(result: CheckResult, added: u32, removed: u32, status_code: u32) -> String {
    match result {
        CheckResult::Initial => format!("baseline established (HTTP {status_code})"),
        CheckResult::NoChange => format!("no change (HTTP {status_code})"),
        CheckResult::ContentChanged => {
            format!("content changed: +{added} / -{removed} lines (HTTP {status_code})")
        }
        CheckResult::Recovered => format!("recovered after failure (HTTP {status_code})"),
        CheckResult::FetchFailed => format!("check failed (HTTP {status_code})"),
        CheckResult::Blocked => format!("access refused (HTTP {status_code})"),
        CheckResult::Unsupported => format!("unsupported content (HTTP {status_code})"),
        CheckResult::Cancelled => "check cancelled".to_string(),
    }
}
