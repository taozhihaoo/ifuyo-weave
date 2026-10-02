//! 日志基线（M0 §13，charter #36）。
//!
//! - `tracing` 结构化日志，rolling 写入 `%APPDATA%/ifuyo/Weave/logs`。
//! - 禁止记录文件内容、密码、token 等敏感数据（charter #36）。
//! - 业务路径禁止 `println!` / `eprintln!`。

use std::fs;
use std::path::PathBuf;
use tauri::Manager;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::EnvFilter;

/// 保持非阻塞日志 flush worker 存活；由 `AppState` 持有到进程结束。
pub struct LogGuard {
    /// 仅依赖其 Drop 侧效应（flush worker 生命周期）。
    _guard: WorkerGuard,
}

/// charter 规定的日志位置：`<APPDATA>/ifuyo/Weave/logs`。
pub fn charter_logs_path(appdata: &std::path::Path) -> PathBuf {
    appdata.join("ifuyo").join("Weave").join("logs")
}

/// 解析日志目录：优先 charter 规定的 `%APPDATA%/ifuyo/Weave/logs`，
/// APPDATA 不可用时回退到 Tauri app data 目录。
pub fn resolve_logs_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    if let Some(appdata) = std::env::var_os("APPDATA") {
        return Ok(charter_logs_path(std::path::Path::new(&appdata)));
    }
    app.path()
        .app_data_dir()
        .map(|p| p.join("logs"))
        .map_err(|e| format!("cannot resolve app data dir: {e}"))
}

/// 初始化全局 tracing subscriber。返回的 guard 必须由调用方持有。
pub fn init(app: &tauri::AppHandle) -> Result<LogGuard, String> {
    let logs_dir = resolve_logs_dir(app)?;
    fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("cannot create logs dir {}: {e}", logs_dir.display()))?;

    let file_appender = tracing_appender::rolling::daily(&logs_dir, "weave.log");
    let (writer, guard) = tracing_appender::non_blocking(file_appender);

    tracing_subscriber::fmt()
        .with_writer(writer)
        .with_ansi(false)
        .with_target(false)
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .try_init()
        .map_err(|e| format!("tracing subscriber init failed: {e}"))?;

    Ok(LogGuard { _guard: guard })
}
