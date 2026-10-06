//! Recipe 运行历史（M2.7 §20/§21/§64-§66）。
//!
//! - 每次 Run 记录一行（含 recipe 版本快照——历史指向运行时版本，不随
//!   Recipe 编辑漂移 §64）。
//! - 容量有界（500 条，最旧淘汰）；原子写。
//! - 删除 Recipe 不删 Run History（§66 审计保留）。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use specta::Type;
use weave_core::prelude::WeaveError;

use crate::recipe::model::Recipe;
use crate::recipe::runner::RecipeRunReport;

pub const MAX_RUN_HISTORY: usize = 500;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeRunRecord {
    pub run_id: String,
    pub recipe_id: String,
    pub recipe_name: String,
    /// 运行时的 Recipe 内容快照（§64/§65：版本冻结）。
    pub recipe_snapshot: Recipe,
    pub trigger: String,
    pub dry_run: bool,
    pub status: String,
    pub matched_files: f64,
    pub total_processed: f64,
    pub total_skipped: f64,
    pub total_failed: f64,
    pub duration_ms: f64,
    pub finished_at: SystemTime,
}

pub struct RunHistoryStore {
    path: PathBuf,
}

impl RunHistoryStore {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Vec<RecipeRunRecord> {
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn append(&self, record: &RecipeRunRecord) -> Result<(), WeaveError> {
        let mut list = self.load();
        list.insert(0, record.clone());
        list.truncate(MAX_RUN_HISTORY);
        let json = serde_json::to_string_pretty(&list)
            .map_err(|e| WeaveError::internal("recipe.serializeFailed", format!("{e}")))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json)
            .and_then(|()| std::fs::rename(&tmp, &self.path))
            .map_err(|e| {
                WeaveError::io("recipe.historyWriteFailed", format!("{e}"))
                    .with_location("recipe::history")
            })
    }

    /// 记录一次运行（从 RunReport 构建；trigger/dry_run 由调用方传入）。
    pub fn record_from_report(
        &self,
        recipe: &Recipe,
        report: &RecipeRunReport,
        trigger: &str,
        run_id: &str,
    ) -> Result<(), WeaveError> {
        self.append(&RecipeRunRecord {
            run_id: run_id.to_string(),
            recipe_id: recipe.id.clone(),
            recipe_name: recipe.name.clone(),
            recipe_snapshot: recipe.clone(),
            trigger: trigger.to_string(),
            dry_run: report.dry_run,
            status: format!("{:?}", report.status).to_lowercase(),
            matched_files: report.matched_files as f64,
            total_processed: report.total_processed as f64,
            total_skipped: report.total_skipped as f64,
            total_failed: report.total_failed as f64,
            duration_ms: report.duration_ms as f64,
            finished_at: SystemTime::now(),
        })
    }
}

/// 从目录载入 Recipe（便利封装）。
pub fn load_recipe_file(path: &Path) -> Option<Recipe> {
    let raw = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&raw).ok()
}
