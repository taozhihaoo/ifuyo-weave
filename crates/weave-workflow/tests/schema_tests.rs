//! M10（下）Schema / Serialization / Registry / Compatibility 测试
//! （§126-§132/§238-§239）。

use weave_workflow::{
    StepType, ToolRegistry, WORKFLOW_SCHEMA_VERSION, Workflow, WorkflowStep, validate,
};

fn step(id: &str, ty: StepType, kind: &str) -> WorkflowStep {
    WorkflowStep {
        id: id.into(),
        r#type: ty,
        name: id.into(),
        kind: kind.into(),
        config: serde_json::json!({}),
        enabled: true,
    }
}

fn minimal() -> Workflow {
    serde_json::from_str(
        r#"{
            "schemaVersion": 1,
            "id": "wf-min",
            "name": "Minimal",
            "description": "",
            "steps": [
                {"id": "i1", "type": "input", "name": "in", "kind": "files", "config": {}, "enabled": true},
                {"id": "e1", "type": "export", "name": "out", "kind": "export.files", "config": {}, "enabled": true}
            ],
            "parameters": []
        }"#,
    )
    .expect("minimal workflow fixture")
}

// ── §126 Schema Test ──

#[test]
fn schema_valid_and_minimal() {
    let v = validate(&minimal());
    assert!(!v.has_errors(), "{v:?}");
}

#[test]
fn schema_wrong_version_rejected() {
    let mut wf = minimal();
    wf.schema_version = 2;
    let v = validate(&wf);
    assert!(v.issues.iter().any(|i| i.code == "workflow.schemaVersion"));
}

#[test]
fn schema_unknown_step_type_rejected() {
    // §126 unknown step：serde 反序列化即拒绝（tag 严格）
    let raw = r#"{
        "schemaVersion": 1, "id": "x", "name": "x", "steps": [
            {"id": "s1", "type": "cron", "name": "c", "kind": "cron", "config": {}, "enabled": true}
        ], "parameters": []
    }"#;
    assert!(
        serde_json::from_str::<Workflow>(raw).is_err(),
        "unknown type must fail parse"
    );
}

#[test]
fn schema_missing_property_rejected() {
    // 缺 id
    let raw = r#"{"schemaVersion": 1, "name": "x", "steps": [], "parameters": []}"#;
    assert!(serde_json::from_str::<Workflow>(raw).is_err());
}

#[test]
fn schema_unknown_property_tolerated() {
    // serde 默认忽略未知字段（向前兼容读；写出不含未知字段）
    let raw = r#"{
        "schemaVersion": 1, "id": "x", "name": "x", "futureField": 1,
        "steps": [], "parameters": []
    }"#;
    let wf: Workflow = serde_json::from_str(raw).expect("unknown field tolerated on read");
    assert_eq!(wf.id, "x");
}

// ── §128/§129/§130 Golden + Round-trip ──

#[test]
fn serialization_golden_stable() {
    let mut wf = minimal();
    wf.steps.insert(
        1,
        WorkflowStep {
            id: "f1".into(),
            r#type: StepType::Filter,
            name: "pdf only".into(),
            kind: "filter.extension".into(),
            config: serde_json::json!({"extensionsIn": ["pdf"]}),
            enabled: true,
        },
    );
    wf.steps.insert(
        2,
        WorkflowStep {
            id: "t1".into(),
            r#type: StepType::Tool,
            name: "rot".into(),
            kind: "pdf.rotate".into(),
            config: serde_json::json!({"degrees": 90, "pages": ""}),
            enabled: false, // disabled step 亦须稳定序列化（§238）
        },
    );
    let a = serde_json::to_string_pretty(&wf).expect("a");
    let b = serde_json::to_string_pretty(&wf).expect("b");
    assert_eq!(a, b, "同模型 → 同序列化（§128 golden）");
    let back: Workflow = serde_json::from_str(&a).expect("round-trip");
    assert_eq!(back, wf);
    let again = serde_json::to_string_pretty(&back).expect("c");
    assert_eq!(a, again, "§129 双往返稳定");
}

#[test]
fn version_roundtrip_no_meaningful_change() {
    // §130：v1 保存 → load → save 不变
    let wf = minimal();
    let j1 = serde_json::to_string_pretty(&wf).expect("j1");
    let back: Workflow = serde_json::from_str(&j1).expect("load");
    let j2 = serde_json::to_string_pretty(&back).expect("j2");
    assert_eq!(j1, j2);
    assert_eq!(back.schema_version, WORKFLOW_SCHEMA_VERSION);
}

// ── §131 Tool Registry Tests ──

#[test]
fn registry_entries_have_unique_ids_and_schema() {
    let tools = ToolRegistry::tools();
    assert!(!tools.is_empty());
    let mut ids = std::collections::HashSet::new();
    for t in &tools {
        assert!(!t.tool_id.is_empty());
        assert!(ids.insert(t.tool_id), "duplicate tool id {}", t.tool_id);
        assert!(t.config_schema.is_object() || t.config_schema.is_string());
        assert!(matches!(t.mutating, true | false));
    }
    for f in ToolRegistry::filters() {
        assert!(
            ids.insert(f.filter_id),
            "duplicate filter id {}",
            f.filter_id
        );
    }
}

#[test]
fn registry_resolves_registered_and_rejects_unknown() {
    assert!(ToolRegistry::tool("pdf.rotate").is_some());
    assert!(ToolRegistry::tool("document.inspect").is_some());
    assert!(
        ToolRegistry::tool("shell.exec").is_none(),
        "§2.4 无脚本工具"
    );
    assert!(ToolRegistry::filter("filter.extension").is_some());
}

// ── §132 Compatibility Matrix（经 Validation 的类型/结构约束）──

#[test]
fn compatibility_matrix() {
    // Input → Tool：VALID
    let mut wf = minimal();
    wf.steps
        .insert(1, step("t1", StepType::Tool, "document.inspect"));
    assert!(!validate(&wf).has_errors(), "Input→Tool");

    // Input → Filter → Tool：VALID
    let mut wf = minimal();
    wf.steps
        .insert(1, step("f1", StepType::Filter, "filter.extension"));
    wf.steps
        .insert(2, step("t1", StepType::Tool, "document.inspect"));
    assert!(!validate(&wf).has_errors(), "Input→Filter→Tool");

    // Tool → Tool：VALID
    let mut wf = minimal();
    wf.steps
        .insert(1, step("t1", StepType::Tool, "document.inspect"));
    let mut rot = step("t2", StepType::Tool, "pdf.rotate");
    rot.config = serde_json::json!({"degrees": 90});
    wf.steps.insert(2, rot);
    assert!(!validate(&wf).has_errors(), "Tool→Tool");

    // Tool → Export：VALID
    let mut wf = minimal();
    wf.steps
        .insert(1, step("t1", StepType::Tool, "document.inspect"));
    assert!(!validate(&wf).has_errors(), "Tool→Export");

    // 非法：Filter 在 Input 前（无 Input 起点）→ ERROR
    let mut wf = minimal();
    wf.steps.clear();
    wf.steps
        .push(step("f1", StepType::Filter, "filter.extension"));
    wf.steps.push(step("e1", StepType::Export, "export.files"));
    assert!(validate(&wf).has_errors(), "无 Input ⇒ ERROR");

    // 非法：disabled step 仍是 VALID（§239 Disabled Step = VALID）
    let mut wf = minimal();
    let mut t = step("t1", StepType::Tool, "document.inspect");
    t.enabled = false;
    wf.steps.insert(1, t);
    assert!(!validate(&wf).has_errors(), "disabled = VALID (§55)");
}

// ── §239 Validation Matrix（关键行固定为测试）──

#[test]
fn validation_matrix_rows() {
    // Tool Missing ⇒ ERROR（§48）
    let mut wf = minimal();
    wf.steps.insert(1, step("t", StepType::Tool, "gone.tool"));
    assert!(validate(&wf).has_errors());

    // Invalid Config ⇒ ERROR（§28）
    let mut wf = minimal();
    let mut t = step("t", StepType::Tool, "pdf.rotate");
    t.config = serde_json::json!({"degrees": 1});
    wf.steps.insert(1, t);
    assert!(validate(&wf).has_errors());

    // Empty steps ⇒ ERROR
    let mut wf = minimal();
    wf.steps.clear();
    assert!(validate(&wf).has_errors());

    // Warning only（空 name）⇒ 仍 VALID
    let mut wf = minimal();
    wf.name = String::new();
    let v = validate(&wf);
    assert!(!v.has_errors());
    assert!(v.issues.iter().any(|i| i.severity == "warning"));
}

// ── §127 Fuzz / Property（确定性种子，§117 同源）──

#[test]
fn fuzz_workflow_parser_never_panics() {
    let mut seed: u64 = 0x00BE_EFCA_FE20_2610;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let kinds = [
        "files",
        "export.files",
        "filter.extension",
        "document.inspect",
        "pdf.rotate",
        "unknown.kind",
        "",
    ];
    for _ in 0..300 {
        let steps: Vec<serde_json::Value> = (0..(next() % 6))
            .map(|_| {
                let kind = kinds[(next() as usize) % kinds.len()];
                let types = ["input", "filter", "tool", "export"];
                serde_json::json!({
                    "id": format!("s{}", next() % 1000),
                    "type": types[(next() % 4) as usize],
                    "name": "f",
                    "kind": kind,
                    "config": if next() % 2 == 0 { serde_json::json!({"degrees": next() % 400}) } else { serde_json::json!({}) },
                    "enabled": true,
                })
            })
            .collect();
        let raw = serde_json::json!({
            "schemaVersion": 1,
            "id": "fuzz",
            "name": "fuzz",
            "steps": steps,
            "parameters": [],
        })
        .to_string();
        // parse + validate 全路径无 panic 即目标
        if let Ok(wf) = serde_json::from_str::<Workflow>(&raw) {
            let _ = validate(&wf);
        }
    }
}
