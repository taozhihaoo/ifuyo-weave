//! 最小配置模型（M0 §41 / §42）。
//!
//! - versioned + validated + migration-ready；不用裸 `serde_json::Value`。
//! - M0 不引入 SQLite：JSON 结构化配置足够，数据库留到真正需要时（charter #38）。
//! - 配置文件损坏或版本不识别时明确报错，不静默重置（charter #47 精神）。

use serde::{Deserialize, Serialize};
use specta::Type;
use std::path::PathBuf;
use tauri::Manager;
use weave_core::prelude::WeaveError;

pub const CONFIG_VERSION: u32 = 1;
pub const LANGUAGES: [&str; 2] = ["zh-CN", "en"];
pub const THEMES: [&str; 3] = ["system", "light", "dark"];

/// 收藏条目（M11 §37）：只存 type/id（引用），label 等从当前 Registry
/// 重新解析；数组顺序 = 用户顺序（§39）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct FavoriteEntry {
    /// "view"（工具页）| "workflow"（工作流定义）。
    pub kind: String,
    /// view id（如 "image"）或 workflow id。
    pub id: String,
}

/// 应用级最小配置。工具参数永远跟随工具本身，不进入全局配置（charter #37）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub version: u32,
    pub language: String,
    pub theme: String,
    /// M11 §34：用户收藏（本地配置持久化；无云无同步 §36）。
    pub favorites: Vec<FavoriteEntry>,
    /// M2.3：剪贴板历史设置（§42：设置持久化，变更立即生效）。
    #[serde(default)]
    pub clipboard: ClipboardConfig,
    /// M2.9：Wisp / Paste Guard 设置（§32）。
    #[serde(default)]
    pub wisp: WispConfig,
    /// M2.10：Web 设置（§84）。
    #[serde(default)]
    pub web: WebConfig,
    /// M2.11：Website Monitor 设置（§68）。
    #[serde(default)]
    pub monitor: MonitorConfig,
}

/// 剪贴板捕获设置（§17/§20/§42：隐私 P0——可停、可清、增长有界）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct ClipboardConfig {
    pub enabled: bool,
    pub capture_text: bool,
    pub capture_images: bool,
    pub capture_files: bool,
    pub max_entries: u32,
    pub retention_days: u32,
}

impl Default for ClipboardConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            capture_text: true,
            capture_images: true,
            capture_files: true,
            max_entries: 500,
            retention_days: 30,
        }
    }
}

/// 每应用粘贴策略（§21：可执行身份 + 模式 + 启用；不存窗口标题 §22）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WispAppRule {
    pub process: String,
    /// default | allow | warn | mask | block
    pub mode: String,
    pub enabled: bool,
}

impl Default for WispAppRule {
    fn default() -> Self {
        Self {
            process: String::new(),
            mode: "default".to_string(),
            enabled: true,
        }
    }
}

/// Wisp / Paste Guard 设置（§32；走既有 Settings 系统持久化）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WispConfig {
    pub enabled: bool,
    pub detect_credentials: bool,
    pub detect_local_paths: bool,
    pub detect_personal_info: bool,
    pub detect_secret_config: bool,
    /// Critical + 显式开启才 Block（§10：默认 Warn 不 Block）。
    pub block_critical: bool,
    /// 保护会话 TTL（§19）。
    pub session_ttl_secs: u32,
    /// 敏感原文是否落盘（§25：默认关——不伪装保险库；关时崩溃后不可恢复）。
    pub retain_sensitive_payload: bool,
    pub app_rules: Vec<WispAppRule>,
}

impl Default for WispConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            detect_credentials: true,
            detect_local_paths: true,
            detect_personal_info: true,
            detect_secret_config: true,
            block_critical: false,
            session_ttl_secs: 300,
            retain_sensitive_payload: false,
            app_rules: Vec::new(),
        }
    }
}

/// M2.11 Website Monitor 设置（§68：默认保守；target 专属设置优先）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct MonitorConfig {
    pub default_interval_secs: u32,
    pub timeout_secs: u32,
    pub max_response_mb: u32,
    pub max_concurrent_checks: u32,
    pub max_snapshots_per_target: u32,
    pub retention_days: u32,
    pub notify_on_change: bool,
    pub notify_on_error: bool,
    pub notify_on_recovery: bool,
}

impl Default for MonitorConfig {
    fn default() -> Self {
        Self {
            default_interval_secs: 1800,
            timeout_secs: 30,
            max_response_mb: 5,
            max_concurrent_checks: 4,
            max_snapshots_per_target: 50,
            retention_days: 30,
            notify_on_change: true,
            notify_on_error: false,
            notify_on_recovery: true,
        }
    }
}

/// M2.10 Web 设置（§84/§85：默认安全保守够用）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct WebConfig {
    /// 网络权限（§17：仅 Web 模块联网；关 ⇒ 全部抓取拒绝）。
    pub allow_network: bool,
    /// 内网/本机目标（§16：默认拒绝）。
    pub allow_private_targets: bool,
    pub timeout_secs: u32,
    pub max_response_mb: u32,
    pub max_redirects: u32,
    pub max_concurrent: u32,
    /// 敏感 query 掩码（§54；UI/日志/历史展示层）。
    pub sensitive_query_masking: bool,
}

impl Default for WebConfig {
    fn default() -> Self {
        Self {
            allow_network: true,
            allow_private_targets: false,
            timeout_secs: 30,
            max_response_mb: 5,
            max_redirects: 10,
            max_concurrent: 4,
            sensitive_query_masking: true,
        }
    }
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
            favorites: Vec::new(),
            clipboard: ClipboardConfig::default(),
            wisp: WispConfig::default(),
            web: WebConfig::default(),
            monitor: MonitorConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), WeaveError> {
        if self.version != CONFIG_VERSION {
            return Err(WeaveError::unsupported(
                "config.unsupportedVersion",
                format!(
                    "config version {} is not supported (expected {CONFIG_VERSION})",
                    self.version
                ),
            )
            .with_suggestion("remove or migrate the config file to a supported version"));
        }
        if !LANGUAGES.contains(&self.language.as_str()) {
            return Err(WeaveError::validation(
                "config.invalidLanguage",
                format!("unknown language '{}'", self.language),
            ));
        }
        if !THEMES.contains(&self.theme.as_str()) {
            return Err(WeaveError::validation(
                "config.invalidTheme",
                format!("unknown theme '{}'", self.theme),
            ));
        }
        Ok(())
    }
}

/// 配置文件位置：`%APPDATA%/ifuyo/Weave/config.json`（回退 app data 目录）。
pub fn resolve_config_path(app: &tauri::AppHandle) -> Result<PathBuf, WeaveError> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Ok(PathBuf::from(appdata)
            .join("ifuyo")
            .join("Weave")
            .join("config.json"));
    }
    app.path()
        .app_data_dir()
        .map(|p| p.join("config.json"))
        .map_err(|e| {
            WeaveError::internal(
                "config.pathUnavailable",
                format!("cannot resolve app data dir: {e}"),
            )
        })
}

/// 读取配置；文件不存在时返回默认值（不自动落盘，首次保存时写入）。
pub fn load(app: &tauri::AppHandle) -> Result<AppConfig, WeaveError> {
    let path = resolve_config_path(app)?;
    if !path.exists() {
        return Ok(AppConfig::default());
    }
    let raw = std::fs::read_to_string(&path).map_err(|e| {
        WeaveError::io(
            "config.readFailed",
            format!("cannot read {}: {e}", path.display()),
        )
    })?;
    let config: AppConfig = serde_json::from_str(&raw).map_err(|e| {
        WeaveError::internal(
            "config.parseFailed",
            format!("config file is corrupted: {e}"),
        )
        .with_location(path.display().to_string())
        .with_suggestion("fix or delete the config file; defaults are not applied silently")
    })?;
    config.validate()?;
    Ok(config)
}

/// 校验并保存配置。
pub fn save(app: &tauri::AppHandle, config: &AppConfig) -> Result<(), WeaveError> {
    config.validate()?;
    let path = resolve_config_path(app)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| {
            WeaveError::io(
                "config.writeFailed",
                format!("cannot create config dir: {e}"),
            )
        })?;
    }
    let json = serde_json::to_string_pretty(config).map_err(|e| {
        WeaveError::internal(
            "config.serializeFailed",
            format!("config serialization failed: {e}"),
        )
    })?;
    std::fs::write(&path, json).map_err(|e| {
        WeaveError::io(
            "config.writeFailed",
            format!("cannot write {}: {e}", path.display()),
        )
    })?;
    Ok(())
}
