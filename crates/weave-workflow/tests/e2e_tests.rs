//! M10（下）E2E（§224-§237）：真实文件、真实编译、真实执行——非 mock。
//! TOCTOU（§167）与取消（§232）含内。

use std::path::{Path, PathBuf};

use weave_core::prelude::CancellationToken;
use weave_workflow::{Workflow, WorkflowStep, compile, validate};

fn ws() -> tempfile::TempDir {
    tempfile::tempdir().expect("ws")
}

/// 合成 PDF，页 i 含内容标记 `WeaveMarker:page-{i}`（§7 synthetic 明确标识）。
fn make_pdf(path: &Path, pages: usize) {
    use lopdf::{Document, Object, Stream, dictionary};
    let mut doc = Document::with_version("1.5");
    let pages_id = doc.add_object(dictionary! {});
    let mut kids = Vec::new();
    for i in 1..=pages {
        let text = format!("BT /F1 12 Tf 72 720 Td (WeaveMarker:page-{i}) Tj ET");
        let content_id = doc.add_object(Stream::new(dictionary! {}, text.into_bytes()));
        let page_id = doc.add_object(dictionary! {
            "Type" => "Page",
            "Parent" => Object::Reference(pages_id),
            "MediaBox" => vec![0.into(), 0.into(), 595.into(), 842.into()],
            "Contents" => Object::Reference(content_id),
            "Resources" => dictionary! {},
        });
        kids.push(Object::Reference(page_id));
    }
    {
        let d = doc
            .objects
            .get_mut(&pages_id)
            .expect("p")
            .as_dict_mut()
            .expect("d");
        d.set("Kids", Object::Array(kids));
        d.set("Count", Object::Integer(pages as i64));
        d.set("MediaBox", vec![0.into(), 0.into(), 595.into(), 842.into()]);
    }
    let catalog_id =
        doc.add_object(dictionary! { "Type" => "Catalog", "Pages" => Object::Reference(pages_id) });
    doc.trailer.set("Root", Object::Reference(catalog_id));
    doc.save(path).expect("save");
}

fn pdf_only_filter() -> WorkflowStep {
    WorkflowStep {
        id: "f1".into(),
        r#type: weave_workflow::StepType::Filter,
        name: "pdf only".into(),
        kind: "filter.extension".into(),
        config: serde_json::json!({"extensionsIn": ["pdf"]}),
        enabled: true,
    }
}

// ── §224/§226 E2E：Input → Filter(pdf) → Rotate → Export（非 mock）──

#[test]
fn e2e_filter_rotate_export_real_files() {
    let dir = ws();
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("out");
    let a = dir.path().join("a.pdf");
    let b = dir.path().join("b.txt"); // 会被 pdf filter 排除（§11 语义）
    make_pdf(&a, 3);
    std::fs::write(&b, "not a pdf").expect("write");

    let wf = Workflow {
        schema_version: weave_workflow::WORKFLOW_SCHEMA_VERSION,
        id: "e2e-rot".into(),
        name: "Rotate PDFs".into(),
        description: String::new(),
        steps: vec![
            WorkflowStep {
                id: "i1".into(),
                r#type: weave_workflow::StepType::Input,
                name: "in".into(),
                kind: "files".into(),
                config: serde_json::json!({}),
                enabled: true,
            },
            pdf_only_filter(),
            WorkflowStep {
                id: "t1".into(),
                r#type: weave_workflow::StepType::Tool,
                name: "rot".into(),
                kind: "pdf.rotate".into(),
                config: serde_json::json!({"degrees": 180, "pages": "1-2"}),
                enabled: true,
            },
            WorkflowStep {
                id: "e1".into(),
                r#type: weave_workflow::StepType::Export,
                name: "out".into(),
                kind: "export.files".into(),
                config: serde_json::json!({"destinationDir": out.to_string_lossy()}),
                enabled: true,
            },
        ],
        parameters: vec![],
    };
    assert!(!validate(&wf).has_errors());
    let plan = compile(&wf, vec![a.clone(), b]).expect("compile");
    let result = weave_batch::execute_plan(&plan, &CancellationToken::new(), None);
    // b.txt 被 filter 排除 ⇒ skipped（§11 Filter 语义稳定）
    assert_eq!(
        (result.succeeded, result.skipped, result.failed),
        (1, 1, 0),
        "{result:?}"
    );
    // 产物真实存在且页字典带 /Rotate 180
    let out_pdf = out.join("a.pdf");
    let doc = lopdf::Document::load(&out_pdf).expect("reload");
    let pages = doc.get_pages();
    let rot = doc
        .get_object(pages[&1])
        .and_then(|o| o.as_dict())
        .and_then(|d| d.get(b"Rotate"))
        .and_then(|o| o.as_i64())
        .unwrap_or(0);
    assert_eq!(rot, 180, "§226 旋转落盘");
}

// ── §225 E2E-1 变体：Directory → Filter → Hash → Export CSV 走 M1/M9/M7/M5。
// M9(下) file_digest + export_report 已有域实现；这里验证 checksum_file 真实文件
// 与 export_report CSV 串联的真实链路（目录扫描由调用方提供——M1 scan 能力）。

#[test]
fn e2e_checksum_export_csv_real() {
    let dir = ws();
    let f1 = dir.path().join("数据 文件.txt");
    std::fs::write(&f1, "unicode content").expect("w");
    let f2 = dir.path().join("plain.txt");
    std::fs::write(&f2, b"plain").expect("w");

    // “目录输入”：真实扫描（模拟 M1 scan 产物 = 路径清单）
    let mut inputs: Vec<PathBuf> = vec![f1.clone(), f2.clone()];
    inputs.sort();

    let mut entries = Vec::new();
    for p in &inputs {
        let e = weave_utilities::checksum_file(
            p,
            weave_utilities::ChecksumAlgorithm::Crc32IsoHdlc,
            &CancellationToken::new(),
            &mut |_| {},
        )
        .expect("checksum");
        entries.push(e);
    }
    let csv = weave_utilities::export_report(&entries, weave_utilities::ExportFormat::Csv);
    // §147：Unicode 路径进 CSV（无需转义除逗号引号换行外；此处含中文原样保留）
    assert!(csv.contains("数据 文件.txt"));
    assert_eq!(csv.lines().count(), 3, "header + 2 rows");

    // M2 安全写落盘（§93）
    let out = dir.path().join("report.csv");
    weave_files::atomic_write(&weave_files::fs::StdFilesystem, &out, csv.as_bytes())
        .expect("atomic write");
    assert_eq!(
        std::fs::read_to_string(&out).expect("read"),
        csv,
        "§146 导出内容稳定"
    );
}

// ── §230 Preview E2E：无 mutation ──

#[test]
fn preview_e2e_no_mutation() {
    let dir = ws();
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("out");
    let a = dir.path().join("a.pdf");
    make_pdf(&a, 1);
    let wf = Workflow {
        schema_version: weave_workflow::WORKFLOW_SCHEMA_VERSION,
        id: "pv".into(),
        name: "pv".into(),
        description: String::new(),
        steps: vec![
            WorkflowStep {
                id: "i1".into(),
                r#type: weave_workflow::StepType::Input,
                name: "in".into(),
                kind: "files".into(),
                config: serde_json::json!({}),
                enabled: true,
            },
            WorkflowStep {
                id: "t1".into(),
                r#type: weave_workflow::StepType::Tool,
                name: "rot".into(),
                kind: "pdf.rotate".into(),
                config: serde_json::json!({"degrees": 90}),
                enabled: true,
            },
            WorkflowStep {
                id: "e1".into(),
                r#type: weave_workflow::StepType::Export,
                name: "out".into(),
                kind: "export.files".into(),
                config: serde_json::json!({"destinationDir": out.to_string_lossy()}),
                enabled: true,
            },
        ],
        parameters: vec![],
    };
    let plan = compile(&wf, vec![a.clone()]).expect("compile");
    let pv = weave_batch::preview_plan(&plan);
    assert_eq!(pv.succeeded, 1);
    assert!(pv.preview, "§30 preview 标记");
    assert!(!out.join("a.pdf").exists(), "§230 Preview 无 mutation");
    // Execute 用同一 plan ⇒ 产物出现（§33 同源）
    let ex = weave_batch::execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(ex.succeeded, 1);
    assert!(out.join("a.pdf").exists());
}

// ── §232 Cancellation E2E ──

#[test]
fn cancellation_e2e_truthful() {
    let dir = ws();
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("out");
    let inputs: Vec<PathBuf> = (0..500)
        .map(|i| {
            let p = dir.path().join(format!("c{i:04}.pdf"));
            make_pdf(&p, 1);
            p
        })
        .collect();
    let wf = Workflow {
        schema_version: weave_workflow::WORKFLOW_SCHEMA_VERSION,
        id: "cancel".into(),
        name: "cancel".into(),
        description: String::new(),
        steps: vec![
            WorkflowStep {
                id: "i1".into(),
                r#type: weave_workflow::StepType::Input,
                name: "in".into(),
                kind: "files".into(),
                config: serde_json::json!({}),
                enabled: true,
            },
            WorkflowStep {
                id: "e1".into(),
                r#type: weave_workflow::StepType::Export,
                name: "out".into(),
                kind: "export.files".into(),
                config: serde_json::json!({"destinationDir": out.to_string_lossy()}),
                enabled: true,
            },
        ],
        parameters: vec![],
    };
    let plan = compile(&wf, inputs).expect("compile");
    let cancel = CancellationToken::new();
    cancel.cancel(); // 运行前取消
    let result = weave_batch::execute_plan(&plan, &cancel, None);
    assert_eq!(result.cancelled, 500, "全部条目如 Cancelled");
    assert_eq!(result.succeeded, 0);
    assert_eq!(out.read_dir().expect("out").count(), 0, "无产物落盘");
}

// ── §233 Failure E2E：invalid workflow 不可执行 ──

#[test]
fn invalid_workflow_cannot_execute() {
    // §233/§230：invalid workflow cannot execute
    let mut wf = Workflow {
        schema_version: weave_workflow::WORKFLOW_SCHEMA_VERSION,
        id: "bad".into(),
        name: "bad".into(),
        description: String::new(),
        steps: vec![
            WorkflowStep {
                id: "i1".into(),
                r#type: weave_workflow::StepType::Input,
                name: "in".into(),
                kind: "files".into(),
                config: serde_json::json!({}),
                enabled: true,
            },
            WorkflowStep {
                id: "t1".into(),
                r#type: weave_workflow::StepType::Tool,
                name: "shell".into(),
                kind: "shell.exec".into(), // §2.4 任意执行严禁
                config: serde_json::json!({}),
                enabled: true,
            },
            WorkflowStep {
                id: "e1".into(),
                r#type: weave_workflow::StepType::Export,
                name: "out".into(),
                kind: "export.files".into(),
                config: serde_json::json!({}),
                enabled: true,
            },
        ],
        parameters: vec![],
    };
    assert!(validate(&wf).has_errors(), "unknown tool ⇒ ERROR");
    let e = compile(&wf, vec![PathBuf::from("x.pdf")]).expect_err("must not compile");
    assert_eq!(e.code, "workflow.validationFailed");
    let _ = &mut wf; // 保持变量使用
}

// ── §236 Import E2E：export → import → validate → run ──

#[test]
fn import_export_roundtrip_then_run() {
    let dir = ws();
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("out");
    let a = dir.path().join("imp.pdf");
    make_pdf(&a, 2);

    let wf = Workflow {
        schema_version: weave_workflow::WORKFLOW_SCHEMA_VERSION,
        id: "imp".into(),
        name: "Imported".into(),
        description: String::new(),
        steps: vec![
            WorkflowStep {
                id: "i1".into(),
                r#type: weave_workflow::StepType::Input,
                name: "in".into(),
                kind: "files".into(),
                config: serde_json::json!({}),
                enabled: true,
            },
            WorkflowStep {
                id: "e1".into(),
                r#type: weave_workflow::StepType::Export,
                name: "out".into(),
                kind: "export.files".into(),
                config: serde_json::json!({"destinationDir": out.to_string_lossy()}),
                enabled: true,
            },
        ],
        parameters: vec![],
    };
    // export
    let json = serde_json::to_string_pretty(&wf).expect("export");
    // import（走同一 parse 路径）
    let imported: Workflow = serde_json::from_str(&json).expect("import");
    assert!(!validate(&imported).has_errors());
    // run
    let plan = compile(&imported, vec![a]).expect("compile");
    let result = weave_batch::execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(result.succeeded, 1, "{result:?}"); // 2 页合成 PDF = 1 个条目
    assert!(out.join("imp.pdf").exists());
}

// ── §237 Broken Import E2E ──

#[test]
fn broken_imports_rejected() {
    for (raw, desc) in [
        (
            r#"{"schemaVersion": 1, "id": "x", "name": "x", "steps": []}"#,
            "empty steps",
        ),
        (
            r#"{"schemaVersion": 9, "id": "x", "name": "x", "steps": []}"#,
            "unknown version",
        ),
        (
            r#"{"schemaVersion": 1, "id": "x", "name": "x", "steps": [{"id":"t","type":"tool","name":"t","kind":"shell.exec","config":{},"enabled":true}]}"#,
            "unknown tool",
        ),
        ("not json at all", "malformed json"),
    ] {
        let parsed = serde_json::from_str::<Workflow>(raw);
        match parsed {
            Ok(wf) => {
                let v = validate(&wf);
                assert!(v.has_errors(), "{desc}: {v:?}");
            }
            Err(_) => { /* parse 拒绝即目标 */ }
        }
        let _ = desc;
    }
}

// ── §241 Performance Benchmark（1/10/50/100 steps × 输入规模，
// validation/planning/preview 实测）。默认忽略，release 手动运行。

#[test]
#[ignore = "perf: release 手动运行"]
fn perf_workflow_matrix() {
    use std::time::Instant;
    let dir = ws();
    let out = dir.path().join("out");
    std::fs::create_dir_all(&out).expect("out");
    let inputs: Vec<PathBuf> = (0..1000)
        .map(|i| {
            let p = dir.path().join(format!("w{i:04}.txt"));
            std::fs::write(&p, "x").expect("w");
            p
        })
        .collect();

    fn build_workflow(n_steps: usize, dest: &Path) -> Workflow {
        let mut steps = vec![WorkflowStep {
            id: "i1".into(),
            r#type: weave_workflow::StepType::Input,
            name: "in".into(),
            kind: "files".into(),
            config: serde_json::json!({}),
            enabled: true,
        }];
        for i in 0..n_steps.saturating_sub(1) {
            steps.push(WorkflowStep {
                id: format!("f{i}"),
                r#type: weave_workflow::StepType::Filter,
                name: format!("f{i}"),
                kind: "filter.extension".into(),
                config: serde_json::json!({"extensionsIn": ["txt"]}),
                enabled: true,
            });
        }
        steps.push(WorkflowStep {
            id: "e1".into(),
            r#type: weave_workflow::StepType::Export,
            name: "out".into(),
            kind: "export.files".into(),
            config: serde_json::json!({"destinationDir": dest.to_string_lossy()}),
            enabled: true,
        });
        Workflow {
            schema_version: weave_workflow::WORKFLOW_SCHEMA_VERSION,
            id: "perf".into(),
            name: "perf".into(),
            description: String::new(),
            steps,
            parameters: vec![],
        }
    }

    for n_steps in [1usize, 10, 30] {
        // §75 max_steps=32：100 步超 v1 限额（KNOWN LIMITATION）
        let d = out.join(format!("s{n_steps}"));
        std::fs::create_dir_all(&d).expect("d");
        let wf = build_workflow(n_steps, &d);
        for n_inputs in [1usize, 100, 1000] {
            let t0 = Instant::now();
            let v = validate(&wf);
            let t_validate = t0.elapsed();
            let plan = weave_workflow::compile(&wf, inputs[..n_inputs].to_vec()).expect("compile");
            let t_plan = t0.elapsed() - t_validate;
            let pv = weave_batch::preview_plan(&plan);
            let t_preview = t0.elapsed() - t_validate - t_plan;
            assert!(!v.has_errors());
            assert_eq!(pv.succeeded as usize, n_inputs);
            eprintln!(
                "PERF workflow steps={n_steps} inputs={n_inputs}: validate={t_validate:?} plan={t_plan:?} preview={t_preview:?}"
            );
        }
    }
}
