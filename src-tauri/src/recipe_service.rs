//! M2.7：Recipe IPC 服务（CRUD / Dry Run / Apply / Run History）。
//!
//! - **编排者而非执行引擎**（§6/§43）：rename 走 M2.0 rename plan+execute；
//!   archive 走 weave-files::archive；manifest 走 integrity——无第二套 Apply。
//! - **运行历史**（§20/§21/§64）：每次 Run 记录（含 Recipe 快照版本冻结）；
//!   删除 Recipe 保留历史（§66）。
//! - **调度接入**（§25）：Recipe 的 Schedule trigger 经 M2.6 Agent Scheduler
//!   （job_name = `recipe:<id>`）；handler 注册表在 Agent tick 分派。
//! - **隐私**（§46/§48）：Run 历史/日志只有计数与状态。

#![expect(clippy::result_large_err)] // D10

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::Manager;
use weave_core::prelude::{CancellationToken, WeaveError};

use crate::commands::IpcError;
use crate::recipe::model::{
    Recipe, RecipeAction, RecipeParameter, RecipeStore, RecipeTrigger, import_recipe, validate,
};
use crate::recipe::runner::{RecipeRunReport, run_recipe};
use crate::recipe::store_history::{RecipeRunRecord, RunHistoryStore};

fn err(code: &'static str, message: impl Into<String>) -> IpcError {
    WeaveError::validation(code, message)
        .with_location("recipe_service")
        .into()
}

fn store(app: &tauri::AppHandle) -> Result<RecipeStore, IpcError> {
    let dir = crate::rename_service::resolve_history_dir(app)?;
    RecipeStore::open(&dir.join("recipes")).map_err(IpcError::from)
}

fn history(app: &tauri::AppHandle) -> Result<RunHistoryStore, IpcError> {
    let dir = crate::rename_service::resolve_history_dir(app)?;
    Ok(RunHistoryStore::at(dir.join("recipe-runs.json")))
}

// ── CRUD（§3） ──

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeDto {
    pub id: String,
    pub name: String,
    pub description: String,
    pub enabled: bool,
    pub trigger: String,
    pub risk: String,
    pub action_count: f64,
    pub updated_at_ms: Option<f64>,
}

fn trigger_str(t: &RecipeTrigger) -> String {
    match t {
        RecipeTrigger::Manual => "manual".to_string(),
        RecipeTrigger::Interval { seconds } => format!("interval:{seconds}"),
        RecipeTrigger::Daily { hour, minute } => format!("daily:{hour:02}:{minute:02}"),
    }
}

fn recipe_dto(r: &Recipe) -> RecipeDto {
    RecipeDto {
        id: r.id.clone(),
        name: r.name.clone(),
        description: r.description.clone(),
        enabled: r.enabled,
        trigger: trigger_str(&r.trigger),
        risk: format!("{:?}", r.risk_level()).to_lowercase(),
        action_count: r.actions.len() as f64,
        updated_at_ms: r
            .updated_at
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .map(|d| d.as_millis() as f64),
    }
}

#[tauri::command]
#[specta::specta]
pub fn recipe_list(app: tauri::AppHandle) -> Result<Vec<RecipeDto>, IpcError> {
    let store = store(&app)?;
    let outcome = store.load_all();
    Ok(outcome.recipes.iter().map(recipe_dto).collect())
}

#[tauri::command]
#[specta::specta]
pub fn recipe_get(app: tauri::AppHandle, id: String) -> Result<Recipe, IpcError> {
    let store = store(&app)?;
    let outcome = store.load_all();
    outcome
        .recipes
        .into_iter()
        .find(|r| r.id == id)
        .ok_or_else(|| err("recipe.unknown", format!("no recipe '{id}'")))
}

#[tauri::command]
#[specta::specta]
pub fn recipe_save(app: tauri::AppHandle, recipe: Recipe) -> Result<(), IpcError> {
    // 保存前校验（§40）
    let result = validate(&recipe);
    if !result.is_ok() {
        return Err(err(
            "recipe.validationFailed",
            format!("{}", result.errors.join("; ")),
        ));
    }
    let store = store(&app)?;
    store.save(&recipe).map_err(IpcError::from)
}

#[tauri::command]
#[specta::specta]
pub fn recipe_delete(app: tauri::AppHandle, id: String) -> Result<bool, IpcError> {
    // §66：只删定义，保留 Run History
    let store = store(&app)?;
    store.delete(&id).map_err(IpcError::from)
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeImportDto {
    pub path: String,
}

/// 导入（§34：版本/未知 Action/校验失败 ⇒ 明确错误）。
#[tauri::command]
#[specta::specta]
pub fn recipe_import(app: tauri::AppHandle, request: RecipeImportDto) -> Result<String, IpcError> {
    let mut recipe = import_recipe(std::path::Path::new(&request.path)).map_err(IpcError::from)?;
    // 新 id 避免覆盖已有
    recipe.id = format!(
        "r_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let store = store(&app)?;
    store.save(&recipe).map_err(IpcError::from)?;
    Ok(recipe.id.clone())
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeExportDto {
    pub id: String,
    pub target: String,
}

/// 导出（§34/§35：自包含 JSON；含绝对路径时由 UI 提示用户）。
#[tauri::command]
#[specta::specta]
pub fn recipe_export(app: tauri::AppHandle, request: RecipeExportDto) -> Result<(), IpcError> {
    let store = store(&app)?;
    let outcome = store.load_all();
    let recipe = outcome
        .recipes
        .into_iter()
        .find(|r| r.id == request.id)
        .ok_or_else(|| err("recipe.unknown", format!("no recipe '{}'", request.id)))?;
    crate::recipe::model::export_recipe(&recipe, std::path::Path::new(&request.target))
        .map_err(IpcError::from)
}

#[tauri::command]
#[specta::specta]
pub fn recipe_set_enabled(
    app: tauri::AppHandle,
    id: String,
    enabled: bool,
) -> Result<(), IpcError> {
    let store = store(&app)?;
    let mut outcome = store.load_all();
    match outcome.recipes.iter_mut().find(|r| r.id == id) {
        Some(recipe) => {
            recipe.enabled = enabled;
            recipe.updated_at = std::time::SystemTime::now();
            store.save(&recipe).map_err(IpcError::from)
        }
        None => Err(err("recipe.unknown", format!("no recipe '{id}'"))),
    }
}

// ── Dry Run / Apply（§14/§15/§16） ──

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRunRequestDto {
    pub id: String,
    pub input_folder: String,
    pub dry_run: bool,
}

#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRunResultDto {
    pub run_id: String,
    pub recipe_id: String,
    pub dry_run: bool,
    pub status: String,
    pub matched_files: f64,
    pub total_processed: f64,
    pub total_skipped: f64,
    pub total_failed: f64,
    pub blocked_reason: Option<String>,
    pub duration_ms: f64,
    pub actions: Vec<(String, String, String, f64, f64, f64)>,
}

/// Dry Run / Apply（同一 Plan §15；同步——rename/archive 典型规模亚秒-秒级，
/// 巨量输入被 SafetyLimits.max_files=1000 有界）。
#[tauri::command]
#[specta::specta]
pub fn recipe_run(request: RecipeRunRequestDto) -> Result<RecipeRunResultDto, IpcError> {
    let _app_guard = ();
    recipe_run_impl(request, None)
}

fn recipe_run_impl(
    request: RecipeRunRequestDto,
    app: Option<&tauri::AppHandle>,
) -> Result<RecipeRunResultDto, IpcError> {
    let store = match app {
        Some(handle) => store(handle)?,
        None => {
            // 无 AppHandle 的测试路径不可达（IPC 总带 handle）；防御性分支
            return Err(err("recipe.internal", "missing app handle"));
        }
    };
    let outcome = store.load_all();
    let recipe = outcome
        .recipes
        .into_iter()
        .find(|r| r.id == request.id)
        .ok_or_else(|| err("recipe.unknown", format!("no recipe '{}'", request.id)))?;
    if !recipe.enabled {
        return Err(err("recipe.disabled", "recipe is disabled"));
    }
    let report = run_recipe(
        &recipe,
        std::path::Path::new(&request.input_folder),
        request.dry_run,
        &CancellationToken::new(),
        &mut |_, _| {},
    )
    .map_err(IpcError::from)?;

    // Run History（§20/§21/§64：含 Recipe 快照）
    let run_id = format!(
        "run_{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    if let Some(handle) = app {
        if let Ok(history) = history(handle) {
            let _ = history.record_from_report(&recipe, &report, "manual", &run_id);
        }
    }

    Ok(RecipeRunResultDto {
        run_id,
        recipe_id: report.recipe_id,
        dry_run: report.dry_run,
        status: format!("{:?}", report.status).to_lowercase(),
        matched_files: report.matched_files as f64,
        total_processed: report.total_processed as f64,
        total_skipped: report.total_skipped as f64,
        total_failed: report.total_failed as f64,
        blocked_reason: report.blocked_reason,
        duration_ms: report.duration_ms as f64,
        actions: report
            .actions
            .iter()
            .map(|a| {
                (
                    a.action_id.clone(),
                    a.action_type.clone(),
                    format!("{:?}", a.status).to_lowercase(),
                    a.processed as f64,
                    a.skipped as f64,
                    a.failed as f64,
                )
            })
            .collect(),
    })
}

#[derive(Debug, Clone, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRunWithAppDto {
    pub id: String,
    pub input_folder: String,
    pub dry_run: bool,
}

/// Apply 带历史落盘（IPC 入口）。
#[tauri::command]
#[specta::specta]
pub fn recipe_run_with_history(
    app: tauri::AppHandle,
    request: RecipeRunWithAppDto,
) -> Result<RecipeRunResultDto, IpcError> {
    recipe_run_impl(
        RecipeRunRequestDto {
            id: request.id,
            input_folder: request.input_folder,
            dry_run: request.dry_run,
        },
        Some(&app),
    )
}

/// Run History（§21；删除 Recipe 后仍保留 §66）。
#[tauri::command]
#[specta::specta]
pub fn recipe_run_history(app: tauri::AppHandle) -> Result<Vec<RecipeRunRecord>, IpcError> {
    let history = history(&app)?;
    Ok(history.load())
}
