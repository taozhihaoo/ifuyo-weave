//! weave-app 的单元测试（以集成测试目标运行：Windows 上测试可执行文件
//! 必须嵌入 common-controls manifest，见 build.rs）。

use weave_app_lib::app_info;
use weave_app_lib::commands::{IpcError, ping};
use weave_app_lib::config::{self, AppConfig};
use weave_app_lib::{export_bindings, logging};
use weave_core::prelude::{Recoverability, WeaveError, validate_absolute_path};

#[test]
fn export_bindings_regenerates_ipc_contract() {
    export_bindings();
}

// ─── brand（M0 §10）───

#[test]
fn embedded_brand_json_parses_with_required_fields() {
    let brand = app_info::brand().expect("embedded brand.json must parse");
    assert_eq!(brand.name, "Weave");
    assert!(!brand.vendor.is_empty());
    assert!(brand.site.starts_with("https://"));
}

// ─── IPC 契约（M0 §9）───

#[test]
fn ping_contract_shape_is_stable() {
    assert_eq!(ping().message, "pong");
}

#[test]
fn ipc_error_translates_all_structured_fields() {
    let err = WeaveError::validation("path.parentTraversal", "escapes root")
        .with_location("weave-core::path")
        .with_suggestion("stay inside the workspace");
    let dto: IpcError = err.into();
    assert_eq!(dto.kind, "validation");
    assert_eq!(dto.code, "path.parentTraversal");
    assert_eq!(dto.recoverability, "userActionRequired");
    assert_eq!(dto.location.as_deref(), Some("weave-core::path"));
    assert!(dto.suggestion.is_some());
}

#[test]
fn path_probe_rejects_traversal_before_touching_fs() {
    // 穿越 → 结构化校验错误，且永远不会触达文件系统。
    assert!(validate_absolute_path(r"C:\..\windows").is_err());
}

// ─── 配置（M0 §41）───

#[test]
fn default_config_is_valid() {
    AppConfig::default()
        .validate()
        .expect("default must be valid");
}

#[test]
fn validation_rejects_unknown_language_theme_and_version() {
    let config = AppConfig {
        language: "fr".to_string(),
        ..AppConfig::default()
    };
    assert_eq!(
        config.validate().expect_err("unknown language").code,
        "config.invalidLanguage"
    );

    let config = AppConfig {
        theme: "neon".to_string(),
        ..AppConfig::default()
    };
    assert_eq!(
        config.validate().expect_err("unknown theme").code,
        "config.invalidTheme"
    );

    let config = AppConfig {
        version: 99,
        ..AppConfig::default()
    };
    assert_eq!(
        config.validate().expect_err("future version").code,
        "config.unsupportedVersion"
    );
}

#[test]
fn config_round_trips_through_json_with_camel_case() {
    let config = AppConfig {
        language: "en".to_string(),
        ..AppConfig::default()
    };
    let json = serde_json::to_value(&config).expect("serialize");
    for key in ["version", "language", "theme"] {
        assert!(json.get(key).is_some(), "missing field {key}");
    }
    let back: AppConfig = serde_json::from_value(json).expect("deserialize");
    assert_eq!(back, config);
}

#[test]
fn corrupted_json_fails_loudly_not_silently() {
    let ws = weave_testkit::TempWorkspace::new("config").expect("workspace");
    let path = ws.file("config.json", "{ not json").expect("write");
    let raw = std::fs::read_to_string(path).expect("read");
    let parsed: Result<AppConfig, _> = serde_json::from_str(&raw);
    assert!(
        parsed.is_err(),
        "corrupted config must not parse to defaults"
    );
}

#[test]
fn config_save_and_load_round_trip_on_disk() {
    let ws = weave_testkit::TempWorkspace::new("config-disk").expect("workspace");
    let path = ws.path().join("config.json");
    let config = AppConfig {
        language: "en".to_string(),
        ..AppConfig::default()
    };
    config.validate().expect("valid");
    let json = serde_json::to_string_pretty(&config).expect("serialize");
    std::fs::write(&path, json).expect("write");
    let loaded: AppConfig =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read")).expect("parse");
    assert_eq!(loaded, config);
    let _ = config::CONFIG_VERSION;
}

// ─── 日志位置（M0 §13）───

#[test]
fn charter_logs_path_matches_spec() {
    let base = std::path::PathBuf::from(r"C:\Users\someone\AppData\Roaming");
    assert_eq!(
        logging::charter_logs_path(&base),
        base.join("ifuyo").join("Weave").join("logs")
    );
}

#[test]
fn io_errors_default_to_retryable_recoverability() {
    let err = WeaveError::io("io.readFailed", "cannot read");
    assert_eq!(err.recoverability, Recoverability::Retryable);
}

// ─── M1：files DTO 转换 ───

#[test]
fn inspection_dto_maps_all_facts() {
    use std::time::{Duration, SystemTime};
    use weave_app_lib::files_dto::FileInspectionDto;
    use weave_core::prelude::{FileKind, TextEncoding};
    use weave_files::{
        Classification, ClassificationEvidence, FileCategory, FileInspection, InspectionStatus,
    };

    let inspection = FileInspection {
        status: InspectionStatus::Complete,
        requested: r"C:\a.txt".to_string(),
        normalized_path: r"C:\a.txt".to_string(),
        name: "a.txt".to_string(),
        extension: Some("txt".to_string()),
        kind: FileKind::RegularFile,
        size: 11,
        readonly: false,
        hidden: Some(false),
        created: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(1000)),
        modified: Some(SystemTime::UNIX_EPOCH + Duration::from_millis(2000)),
        accessed: None,
        classification: Classification {
            category: FileCategory::Text,
            evidence: ClassificationEvidence::Extension,
            mime: None,
        },
        encoding: Some(TextEncoding::Ascii),
        warnings: vec![],
    };
    let dto = FileInspectionDto::from_inspection(inspection);
    assert_eq!(dto.status, "complete");
    assert_eq!(dto.kind, "file");
    assert_eq!(dto.size, 11.0);
    assert_eq!(dto.created_ms, Some(1000.0));
    assert_eq!(dto.accessed_ms, None);
    assert_eq!(dto.classification.category, "text");
    assert_eq!(dto.encoding.as_deref(), Some("ascii"));
}

#[test]
fn scan_options_validation_rejects_bad_input() {
    use weave_app_lib::files_dto::ScanOptionsDto;
    let ok = ScanOptionsDto {
        max_depth: Some(8.0),
        max_entries: Some(1000.0),
    };
    let domain = ok.into_domain().expect("valid");
    assert_eq!(domain.max_depth, Some(8));

    let fractional = ScanOptionsDto {
        max_depth: Some(1.5),
        max_entries: None,
    };
    assert!(fractional.into_domain().is_err());
}

// ─── M1：JobTracker 生命周期 ───

#[test]
fn job_lifecycle_running_to_done() {
    use weave_app_lib::jobs::{JobOutcome, JobState, JobTracker};
    use weave_core::prelude::HashResult;

    let tracker = JobTracker::new();
    let (id, _cancel, _sink, state) = tracker.register();
    assert_eq!(
        tracker.status(&id.to_string()).expect("running").state,
        "running"
    );

    *state.lock().expect("lock") = JobState::Running {
        progress_current: 512,
    };
    assert_eq!(
        tracker.status(&id.to_string()).expect("r").progress_current,
        Some(512.0)
    );

    tracker.finish(
        &id,
        JobOutcome::Hash(
            HashResult {
                algorithm: weave_core::prelude::HashAlgorithm::Sha256,
                digest_hex: Some("ab".repeat(32)),
                bytes_processed: 1024,
                duration_ms: 3,
                status: weave_core::prelude::HashStatus::Completed,
            }
            .into(),
        ),
    );
    let done = tracker.status(&id.to_string()).expect("done");
    assert_eq!(done.state, "completed");
    assert_eq!(done.hash.expect("hash").bytes_processed, 1024.0);
}

#[test]
fn job_cancel_sets_token_and_unknown_is_false() {
    use weave_app_lib::jobs::JobTracker;
    let tracker = JobTracker::new();
    let (id, cancel, _sink, _state) = tracker.register();
    assert!(tracker.cancel(&id.to_string()));
    assert!(cancel.is_cancelled());
    assert!(!tracker.cancel("job_nonexistent"));
}

// ─── M1：Tool registry ───

#[test]
fn file_tools_registry_is_deterministic_and_describable() {
    use weave_app_lib::tools::{build_file_tools_registry, describe_registry};
    let registry = build_file_tools_registry();
    let ids: Vec<String> = registry.list().iter().map(|i| i.to_string()).collect();
    assert_eq!(
        ids,
        vec!["files.analyze_directory", "files.hash", "files.inspect"]
    );

    let descriptors = describe_registry(&registry);
    let analyze = descriptors
        .iter()
        .find(|d| d.id == "files.analyze_directory")
        .expect("analyze");
    assert_eq!(analyze.category, "files");
    assert_eq!(analyze.input_kinds, vec!["directoryPath"]);
}

#[test]
fn inspect_tool_rejects_text_input() {
    use weave_app_lib::tools::InspectTool;
    use weave_core::prelude::{Tool, ToolInput};
    let tool = InspectTool;
    let err = tool
        .preview(&ToolInput::Text {
            content: "nope".to_string(),
        })
        .expect_err("text input rejected");
    assert_eq!(err.code, "tool.wrongInputKind");
}
