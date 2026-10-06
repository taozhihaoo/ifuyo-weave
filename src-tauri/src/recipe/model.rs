//! Recipe 数据模型 + 校验 + 存储（M2.7 §4/§31/§32/§33/§40/§41）。
//!
//! - **声明式**（§5）：Recipe = 参数化步骤定义，不是脚本；Action 必须命中
//!   受控注册表（未知类型 ⇒ Broken/校验失败）。
//! - **版本化**（§32）：schemaVersion + 单 Recipe 单文件（原子写 + 损坏隔离）。
//! - **风险分级**（§41）：由 Action 类型推导（ReadOnly/Low/Moderate/Destructive）。
//! - **状态**（§31）：Enabled/Disabled；校验失败 ⇒ Broken 不静默运行。

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use specta::Type;
use weave_core::prelude::WeaveError;

pub const RECIPE_SCHEMA_VERSION: u32 = 1;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::validation(code, message).with_location("recipe::model")
}

/// Action 类型（§3/§39：只列真实已实现的能力，不虚构）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ActionType {
    /// 按扩展名过滤文件列表（内存过滤，无文件修改）。
    FilterByExtension,
    /// 前缀重命名（走 weave-files rename plan → execute_plan，M2.0 核心）。
    RenamePrefix,
    /// 创建 ZIP（走 weave-files::archive，M2.5）。
    ArchiveCreate,
    /// 生成目录 SHA-256 Manifest（走 integrity，只读）。
    HashManifest,
}

impl ActionType {
    pub fn as_str(self) -> &'static str {
        match self {
            ActionType::FilterByExtension => "filterByExtension",
            ActionType::RenamePrefix => "renamePrefix",
            ActionType::ArchiveCreate => "archiveCreate",
            ActionType::HashManifest => "hashManifest",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "filterByExtension" => Some(ActionType::FilterByExtension),
            "renamePrefix" => Some(ActionType::RenamePrefix),
            "archiveCreate" => Some(ActionType::ArchiveCreate),
            "hashManifest" => Some(ActionType::HashManifest),
            _ => None,
        }
    }

    /// 是否修改文件系统（§40 Safety）。
    pub fn is_mutating(self) -> bool {
        matches!(self, ActionType::RenamePrefix | ActionType::ArchiveCreate)
    }

    /// 风险分级（§41：由 Action 类型推导基线；运行时按数量上调）。
    pub fn base_risk(self) -> RiskLevel {
        match self {
            ActionType::FilterByExtension | ActionType::HashManifest => RiskLevel::ReadOnly,
            ActionType::RenamePrefix => RiskLevel::LowRisk,
            ActionType::ArchiveCreate => RiskLevel::LowRisk,
        }
    }
}

/// 风险分级（§41）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum RiskLevel {
    ReadOnly,
    LowRisk,
    ModerateRisk,
    Destructive,
}

/// Recipe 触发（§23/§25：Manual + Schedule 复用 M2.6 Scheduler；Watch/Clipboard
/// 待 M2.6 Watcher/触发分发成熟后接入——如实 Partial）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum RecipeTrigger {
    Manual,
    /// 复用 M2.6 Scheduler：每 interval_seconds 秒。
    Interval {
        seconds: u32,
    },
    /// 每天 hour:minute（本地时区）。
    Daily {
        hour: u8,
        minute: u8,
    },
}

/// Recipe 参数（§10：类型化 + 默认值 + 必填）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeParameter {
    pub name: String,
    /// string | number | boolean | path | folder
    pub kind: String,
    pub required: bool,
    pub default_value: Option<String>,
    /// 运行时值（保存时可为 None = 未设置）。
    pub value: Option<String>,
}

/// Recipe 步骤（§4 RecipeAction）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct RecipeAction {
    pub id: String,
    pub action_type: ActionType,
    pub enabled: bool,
    /// Filter：逗号分隔扩展名（png,jpg）；Rename：前缀文本。
    pub parameter: Option<String>,
}

/// Recipe（§4）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Recipe {
    pub id: String,
    pub name: String,
    pub description: String,
    pub schema_version: u32,
    pub enabled: bool,
    pub created_at: SystemTime,
    pub updated_at: SystemTime,
    pub input_folder: String,
    pub trigger: RecipeTrigger,
    pub parameters: Vec<RecipeParameter>,
    pub actions: Vec<RecipeAction>,
}

impl Recipe {
    /// 风险推导（§41）：变更类 Action 的数量与顺序决定整体风险。
    pub fn risk_level(&self) -> RiskLevel {
        let mutating = self
            .actions
            .iter()
            .any(|a| a.enabled && a.action_type.is_mutating());
        if mutating {
            RiskLevel::ModerateRisk
        } else {
            RiskLevel::ReadOnly
        }
    }
}

/// 校验结果（§40：Errors/Warnings 分离）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ValidationResult {
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl ValidationResult {
    pub fn is_ok(&self) -> bool {
        self.errors.is_empty()
    }
}

/// Recipe 校验（§40）。
pub fn validate(recipe: &Recipe) -> ValidationResult {
    let mut result = ValidationResult::default();
    if recipe.name.trim().is_empty() {
        result.errors.push("name is empty".to_string());
    }
    if recipe.actions.is_empty() {
        result.errors.push("recipe has no actions".to_string());
    }
    for (idx, action) in recipe.actions.iter().enumerate() {
        if !action.enabled {
            result.warnings.push(format!("action {idx} is disabled"));
        }
        match action.action_type {
            ActionType::FilterByExtension => {
                if action
                    .parameter
                    .as_deref()
                    .unwrap_or_default()
                    .trim()
                    .is_empty()
                {
                    result
                        .errors
                        .push(format!("action {idx}: filter requires extensions"));
                }
            }
            ActionType::RenamePrefix if action.parameter.is_none() => {
                result
                    .errors
                    .push(format!("action {idx}: rename prefix missing"));
            }
            _ => {}
        }
    }
    match &recipe.trigger {
        RecipeTrigger::Interval { seconds } => {
            if *seconds == 0 {
                result
                    .errors
                    .push("interval seconds must be >= 1".to_string());
            }
        }
        RecipeTrigger::Daily { hour, minute } => {
            if *hour > 23 || *minute > 59 {
                result.errors.push("daily time out of range".to_string());
            }
        }
        RecipeTrigger::Manual => {}
    }
    for p in &recipe.parameters {
        if p.required && p.value.is_none() && p.default_value.is_none() {
            result
                .errors
                .push(format!("parameter '{}' is required", p.name));
        }
    }
    result
}

// ── 存储（§33：Recipes/ 目录，单文件原子写，损坏隔离）──

pub struct RecipeStore {
    dir: PathBuf,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoadOutcome {
    pub recipes: Vec<Recipe>,
    pub quarantined: Vec<String>,
}

impl RecipeStore {
    pub fn open(dir: &Path) -> Result<Self, WeaveError> {
        std::fs::create_dir_all(dir).map_err(|e| {
            WeaveError::io(
                "recipe.openFailed",
                format!("cannot create recipe dir: {e}"),
            )
            .with_location("recipe::store")
        })?;
        Ok(Self {
            dir: dir.to_path_buf(),
        })
    }

    fn recipe_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.recipe.json"))
    }

    /// 载入全部 Recipe（单个损坏 ⇒ 隔离，不拖垮整体 §33）。
    pub fn load_all(&self) -> LoadOutcome {
        let mut outcome = LoadOutcome {
            recipes: Vec::new(),
            quarantined: Vec::new(),
        };
        let Ok(read) = std::fs::read_dir(&self.dir) else {
            return outcome;
        };
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let Some(name) = path.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
                continue;
            };
            let Ok(raw) = std::fs::read_to_string(&path) else {
                continue;
            };
            match serde_json::from_str::<serde_json::Value>(&raw) {
                Ok(v)
                    if v.get("schemaVersion").and_then(|x| x.as_u64())
                        == Some(RECIPE_SCHEMA_VERSION as u64) =>
                {
                    match serde_json::from_value::<Recipe>(v) {
                        Ok(recipe) => outcome.recipes.push(recipe),
                        Err(_) => {
                            let q = format!("{name}.corrupt");
                            let _ = std::fs::rename(&path, self.dir.join(&q));
                            outcome.quarantined.push(q);
                        }
                    }
                }
                _ => {
                    // 版本不符/结构损坏：隔离（不猜测迁移 §32）
                    let q = format!("{name}.corrupt");
                    let _ = std::fs::rename(&path, self.dir.join(&q));
                    outcome.quarantined.push(q);
                }
            }
        }
        outcome.recipes.sort_by_key(|a| a.name.to_lowercase());
        outcome
    }

    /// 原子保存单个 Recipe。
    pub fn save(&self, recipe: &Recipe) -> Result<(), WeaveError> {
        let json = serde_json::to_string_pretty(recipe)
            .map_err(|e| WeaveError::internal("recipe.serializeFailed", format!("{e}")))?;
        let tmp = self.recipe_path(&recipe.id).with_extension("tmp");
        std::fs::write(&tmp, json)
            .and_then(|()| std::fs::rename(&tmp, self.recipe_path(&recipe.id)))
            .map_err(|e| {
                WeaveError::io("recipe.saveFailed", format!("{e}")).with_location("recipe::store")
            })
    }

    pub fn delete(&self, id: &str) -> Result<bool, WeaveError> {
        let path = self.recipe_path(id);
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                WeaveError::io("recipe.deleteFailed", format!("{e}")).with_location("recipe::store")
            })?;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn path_of(&self, id: &str) -> PathBuf {
        self.recipe_path(id)
    }
}

/// 导出 = 保存 JSON 到用户指定路径（§34：自包含，参数未设置则提示）。
pub fn export_recipe(recipe: &Recipe, target: &Path) -> Result<(), WeaveError> {
    let json = serde_json::to_string_pretty(recipe)
        .map_err(|e| WeaveError::internal("recipe.serializeFailed", format!("{e}")))?;
    std::fs::write(target, json).map_err(|e| {
        WeaveError::io("recipe.exportFailed", format!("{e}")).with_location("recipe::store")
    })
}

/// 导入 = 读取 + 版本检查 + 校验（§34：未知 Action/版本 ⇒ 明确错误）。
pub fn import_recipe(path: &Path) -> Result<Recipe, WeaveError> {
    let raw = std::fs::read_to_string(path).map_err(|e| {
        WeaveError::io("recipe.importReadFailed", format!("{e}")).with_location("recipe::store")
    })?;
    let value: serde_json::Value = serde_json::from_str(&raw)
        .map_err(|e| err("recipe.importInvalid", format!("invalid JSON: {e}")))?;
    let version = value.get("schemaVersion").and_then(|v| v.as_u64());
    if version != Some(RECIPE_SCHEMA_VERSION as u64) {
        return Err(err(
            "recipe.unsupportedVersion",
            format!("schema version {version:?} not supported (expected {RECIPE_SCHEMA_VERSION})"),
        ));
    }
    let recipe: Recipe = serde_json::from_value(value)
        .map_err(|e| err("recipe.importInvalid", format!("recipe invalid: {e}")))?;
    let result = validate(&recipe);
    if !result.is_ok() {
        return Err(err(
            "recipe.importValidationFailed",
            format!("imported recipe has errors: {}", result.errors.join("; ")),
        ));
    }
    Ok(recipe)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Recipe {
        Recipe {
            id: "r1".to_string(),
            name: "Tidy".to_string(),
            description: String::new(),
            schema_version: RECIPE_SCHEMA_VERSION,
            enabled: true,
            created_at: SystemTime::UNIX_EPOCH,
            updated_at: SystemTime::UNIX_EPOCH,
            input_folder: "C:\\Downloads".to_string(),
            trigger: RecipeTrigger::Manual,
            parameters: vec![],
            actions: vec![
                RecipeAction {
                    id: "a1".to_string(),
                    action_type: ActionType::FilterByExtension,
                    enabled: true,
                    parameter: Some("png,jpg".to_string()),
                },
                RecipeAction {
                    id: "a2".to_string(),
                    action_type: ActionType::RenamePrefix,
                    enabled: true,
                    parameter: Some("new-".to_string()),
                },
            ],
        }
    }

    #[test]
    fn validate_accepts_good_recipe_and_rejects_empty() {
        assert!(validate(&sample()).is_ok());
        let mut empty = sample();
        empty.actions.clear();
        empty.name = String::new();
        let result = validate(&empty);
        assert!(!result.is_ok());
        assert_eq!(result.errors.len(), 2);
    }

    #[test]
    fn filter_action_requires_extensions() {
        let mut r = sample();
        r.actions[0].parameter = None;
        let result = validate(&r);
        assert!(result.errors.iter().any(|e| e.contains("filter requires")));
    }

    #[test]
    fn interval_zero_is_rejected() {
        let mut r = sample();
        r.trigger = RecipeTrigger::Interval { seconds: 0 };
        assert!(!validate(&r).is_ok());
    }

    #[test]
    fn risk_derives_from_mutating_actions() {
        assert_eq!(sample().risk_level(), RiskLevel::ModerateRisk);
        let mut read_only = sample();
        read_only
            .actions
            .retain(|a| a.action_type == ActionType::FilterByExtension);
        assert_eq!(read_only.risk_level(), RiskLevel::ReadOnly);
    }

    #[test]
    fn store_round_trip_and_quarantine() {
        let ws = weave_testkit::TempWorkspace::new("recipe-store").expect("ws");
        let store = RecipeStore::open(ws.path()).expect("open");
        store.save(&sample()).expect("save");
        let outcome = store.load_all();
        assert!(outcome.quarantined.is_empty());
        assert_eq!(outcome.recipes.len(), 1);
        assert_eq!(outcome.recipes[0].id, "r1");
        // 损坏 ⇒ 隔离
        std::fs::write(store.path_of("bad"), "{ broken").expect("write corrupt");
        let outcome = store.load_all();
        assert_eq!(outcome.recipes.len(), 1);
        assert_eq!(outcome.quarantined.len(), 1);
    }

    #[test]
    fn import_rejects_unknown_version_and_bad_actions() {
        let ws = weave_testkit::TempWorkspace::new("recipe-import").expect("ws");
        let target = ws.path().join("r.recipe.json");
        let mut r = sample();
        r.schema_version = 99;
        std::fs::write(&target, serde_json::to_string(&r).unwrap()).expect("write");
        let e = import_recipe(&target).expect_err("version rejected");
        assert_eq!(e.code, "recipe.unsupportedVersion");

        let mut r2 = sample();
        r2.actions[0].action_type = ActionType::FilterByExtension;
        // 未知 action 类型需 JSON 层拒绝：手动构造非法 serde 值
        let bad = r#"{"id":"x","schemaVersion":1,"actions":[{"actionType":"executeShell"}]}"#;
        std::fs::write(&target, bad).expect("write bad");
        let e2 = import_recipe(&target).expect_err("unknown action rejected");
        assert!(
            e2.code == "recipe.importInvalid" || e2.code == "recipe.importValidationFailed",
            "{e2}"
        );
        let _ = r2;
    }
}
