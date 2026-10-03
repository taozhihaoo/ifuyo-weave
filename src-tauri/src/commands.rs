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
use crate::files_dto::{
    FileInspectionDto, JobHandleDto, JobStatusDto, ScanOptionsDto, ToolDescriptorDto,
};
use serde::Serialize;
use specta::Type;
use weave_core::prelude::{
    ErrorKind, Recoverability, WeaveError, validate_absolute_path,
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

/// 统一错误在 IPC 边界的 DTO。weave-core 不依赖 specta；
/// 错误在 Application 层显式翻译（Domain Error → Application → UI）。
#[derive(Debug, Clone, Serialize, Type)]
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

// ─── File Core（M1）───
//
// 命令只做 orchestration（M1 §19）：校验输入 → 调 weave-files 服务 → 结构化返回。
// inspect 是同步快速路径（stat + 有界嗅探）；hash/scan 是显式任务（进度 + 取消）。

fn validate_input_path(raw_path: &str) -> Result<(), IpcError> {
    weave_core::prelude::validate_absolute_path(raw_path)
        .map(|_| ())
        .map_err(Into::into)
}

/// 同步检查：默认不读文件内容（除 ≤8 KiB 有界嗅探），立即返回。
#[tauri::command]
#[specta::specta]
pub fn inspect_file(raw_path: String) -> Result<FileInspectionDto, IpcError> {
    let inspection = weave_files::inspect_file(
        &weave_files::fs::StdFilesystem,
        &raw_path,
        &weave_files::InspectOptions::default(),
    )?;
    Ok(FileInspectionDto::from_inspection(inspection))
}

/// 显式哈希任务（M1 §34：单独触发，带进度与取消）。
#[tauri::command]
#[specta::specta]
pub fn hash_file(app: tauri::AppHandle, raw_path: String) -> Result<JobHandleDto, IpcError> {
    use tauri::Manager;
    validate_input_path(&raw_path)?;
    let (job_id, cancel, sink, state_cell) = app.state::<crate::state::AppState>().jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let result = weave_files::hash_file(
            &weave_files::fs::StdFilesystem,
            std::path::Path::new(&raw_path),
            &cancel,
            &mut |p| sink.report(&p),
        );
        match result {
            Ok(hash) => app_state
                .jobs
                .finish(&job_id_for_task, crate::jobs::JobOutcome::Hash(hash.into())),
            Err(e) => app_state.jobs.fail(&job_id_for_task, e),
        }
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 目录扫描任务（非阻塞；进度经 get_job 轮询，取消经 cancel_job）。
#[tauri::command]
#[specta::specta]
pub fn analyze_directory(
    app: tauri::AppHandle,
    raw_path: String,
    options: Option<ScanOptionsDto>,
) -> Result<JobHandleDto, IpcError> {
    use tauri::Manager;
    validate_input_path(&raw_path)?;
    let domain_options = options.unwrap_or_default().into_domain().map_err(|e| {
        WeaveError::validation(
            "scan.invalidOptions",
            format!("option '{}' = {} is out of range", e.what, e.value),
        )
    })?;

    let (job_id, cancel, sink, state_cell) = app.state::<crate::state::AppState>().jobs.register();
    let handle = app.clone();
    let job_id_for_task = job_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let app_state = handle.state::<crate::state::AppState>();
        let result = weave_files::scan_directory(
            &weave_files::fs::StdFilesystem,
            &raw_path,
            &domain_options,
            &cancel,
            &mut |p| sink.report(&p),
        );
        match result {
            Ok(report) => app_state.jobs.finish(
                &job_id_for_task,
                crate::jobs::JobOutcome::Scan(report.into()),
            ),
            Err(e) => app_state.jobs.fail(&job_id_for_task, e),
        }
    });
    drop(state_cell);
    Ok(JobHandleDto {
        job_id: job_id.to_string(),
    })
}

/// 轮询任务状态（running 带 progress；completed 带 hash/scan 结果）。
#[tauri::command]
#[specta::specta]
pub fn get_job(app: tauri::AppHandle, job_id: String) -> Result<JobStatusDto, IpcError> {
    use tauri::Manager;
    app.state::<crate::state::AppState>()
        .jobs
        .status(&job_id)
        .ok_or_else(|| {
            WeaveError::validation("job.unknown", format!("unknown job id '{job_id}'")).into()
        })
}

/// 协作式取消：立即返回，任务在安全点自行收尾为 Cancelled 结果。
#[tauri::command]
#[specta::specta]
pub fn cancel_job(app: tauri::AppHandle, job_id: String) -> Result<bool, IpcError> {
    use tauri::Manager;
    let cancelled = app.state::<crate::state::AppState>().jobs.cancel(&job_id);
    if cancelled {
        tracing::info!(%job_id, "job cancel requested");
        Ok(true)
    } else {
        Err(WeaveError::validation("job.unknown", format!("unknown job id '{job_id}'")).into())
    }
}

/// 统一工具发现（Command Palette / Quick Drop 的单一事实源）。
#[tauri::command]
#[specta::specta]
pub fn list_tools(app: tauri::AppHandle) -> Vec<ToolDescriptorDto> {
    use tauri::Manager;
    crate::tools::describe_registry(&app.state::<crate::state::AppState>().tools)
}
