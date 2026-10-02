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
