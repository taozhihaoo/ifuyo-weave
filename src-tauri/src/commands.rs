//! IPC 命令层（M0 §9）。
//!
//! 职责边界：这里只做"接收 UI 请求 → 验证输入 → 调用 domain → 返回结构化结果"。
//! 禁止在 command 里堆业务代码；文件系统访问在 M0 作为最小探针存在，
//! M1 起移入 Filesystem Adapter。

// IPC 边界的每次调用都要跨越序列化边界，~144 字节的 Err DTO 不构成
// result_large_err 针对的热路径问题；这是记录在 DECISIONS.md 的有意取舍。
#![expect(clippy::result_large_err)]

use crate::app_info;
use crate::config::{self, AppConfig};
use serde::Serialize;
use specta::Type;
use weave_core::prelude::{
    ErrorKind, PathValidation, Recoverability, WeaveError, validate_absolute_path,
};

/// `ping` 的应答。真实 IPC 往返，不是前端本地 mock。
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Pong {
    pub message: String,
}

/// 应用信息（品牌 + 版本 + 运行环境）。
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppInfo {
    pub name: String,
    pub vendor: String,
    pub site: String,
    pub version: String,
    pub environment: String,
}

/// `inspect_path` 的结果。这是 M0 架构探针，不是正式 File Inspector 工具（M0 §38）。
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PathProbe {
    pub requested: String,
    pub normalized: String,
    pub exists: bool,
    pub kind: String,
    pub name: Option<String>,
    pub extension: Option<String>,
    /// IPC 边界用 f64（JSON number 的实际类型，≤2^53 无损）；领域层保持 u64。
    /// specta-typescript 0.0.12 禁止导出 u64 且无配置项，见 DECISIONS.md。
    pub size_bytes: Option<f64>,
}

/// 统一错误在 IPC 边界的 DTO。weave-core 不依赖 specta；
/// 错误在 Application 层显式翻译（Domain Error → Application → UI）。
#[derive(Debug, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct IpcError {
    pub kind: String,
    pub code: String,
    pub message: String,
    pub location: Option<String>,
    pub recoverability: String,
    pub suggestion: Option<String>,
}

impl From<WeaveError> for IpcError {
    fn from(e: WeaveError) -> Self {
        let kind = match e.kind {
            ErrorKind::Validation => "validation",
            ErrorKind::Io => "io",
            ErrorKind::Permission => "permission",
            ErrorKind::Conflict => "conflict",
            ErrorKind::Cancelled => "cancelled",
            ErrorKind::Unsupported => "unsupported",
            ErrorKind::Internal => "internal",
        };
        let recoverability = match e.recoverability {
            Recoverability::Retryable => "retryable",
            Recoverability::Fatal => "fatal",
            Recoverability::UserActionRequired => "userActionRequired",
        };
        Self {
            kind: kind.to_string(),
            code: e.code,
            message: e.message,
            location: e.location,
            recoverability: recoverability.to_string(),
            suggestion: e.suggestion,
        }
    }
}

fn environment_name() -> &'static str {
    if cfg!(debug_assertions) {
        "development"
    } else {
        "production"
    }
}

/// 最小 IPC 往返验证：React → IPC → Rust → 应答。
#[tauri::command]
#[specta::specta]
pub fn ping() -> Pong {
    Pong {
        message: "pong".to_string(),
    }
}

/// 品牌与应用信息。version 取自真实应用包信息（brand.json versionSource = package）。
#[tauri::command]
#[specta::specta]
pub fn get_app_info(app: tauri::AppHandle) -> Result<AppInfo, IpcError> {
    let brand = app_info::brand()?;
    let version = app.package_info().version.to_string();
    tracing::info!(%version, "app info requested");
    Ok(AppInfo {
        name: brand.name,
        vendor: brand.vendor,
        site: brand.site,
        version,
        environment: environment_name().to_string(),
    })
}

/// 最小路径探针：validate → metadata → typed result。
/// 拒绝相对路径、穿越、保留名等一切不安全输入。
#[tauri::command]
#[specta::specta]
pub fn inspect_path(raw_path: String) -> Result<PathProbe, IpcError> {
    let validation: PathValidation = validate_absolute_path(&raw_path)?;
    let target = std::path::PathBuf::from(&validation.normalized);

    let metadata = std::fs::metadata(&target).map_err(|e| {
        let code = match e.kind() {
            std::io::ErrorKind::NotFound => "path.notFound",
            std::io::ErrorKind::PermissionDenied => "path.permissionDenied",
            _ => "path.statFailed",
        };
        let error = match e.kind() {
            std::io::ErrorKind::PermissionDenied => WeaveError::permission(
                code,
                format!("cannot stat '{}': {e}", validation.normalized),
            ),
            _ => WeaveError::io(
                code,
                format!("cannot stat '{}': {e}", validation.normalized),
            ),
        };
        error
            .with_location("inspect_path")
            .with_recoverability(Recoverability::UserActionRequired)
    })?;

    let name = target.file_name().map(|n| n.to_string_lossy().into_owned());
    let extension = target.extension().map(|e| e.to_string_lossy().into_owned());
    let kind = if metadata.is_dir() {
        "directory"
    } else if metadata.is_file() {
        "file"
    } else {
        "other"
    };

    tracing::info!(normalized = %validation.normalized, kind, "path inspected");
    Ok(PathProbe {
        requested: raw_path,
        normalized: validation.normalized,
        exists: true,
        kind: kind.to_string(),
        name,
        extension,
        size_bytes: metadata.is_file().then_some(metadata.len() as f64),
    })
}

/// 读取应用配置（不存在时返回默认值）。
#[tauri::command]
#[specta::specta]
pub fn get_app_config(app: tauri::AppHandle) -> Result<AppConfig, IpcError> {
    config::load(&app).map_err(IpcError::from)
}

/// 校验并保存应用配置。
#[tauri::command]
#[specta::specta]
pub fn set_app_config(app: tauri::AppHandle, config: AppConfig) -> Result<AppConfig, IpcError> {
    config::save(&app, &config)?;
    tracing::info!(language = %config.language, theme = %config.theme, "config saved");
    Ok(config)
}
