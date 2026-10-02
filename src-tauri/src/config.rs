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

/// 应用级最小配置。工具参数永远跟随工具本身，不进入全局配置（charter #37）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct AppConfig {
    pub version: u32,
    pub language: String,
    pub theme: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            version: CONFIG_VERSION,
            language: "zh-CN".to_string(),
            theme: "system".to_string(),
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
