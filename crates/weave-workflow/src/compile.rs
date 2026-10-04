//! Workflow Compiler（M10 上 §33-§35）：Workflow Definition → M7 JobPlan。
//!
//! §33 Preview/Execute 一致性：编译产物 = 同一个 `weave_batch::JobPlan`，
//! preview 走 `weave_batch::preview_plan`（同引擎 dry-run），execute 走
//! `weave_batch::execute_plan`——两个入口共享一份编译结果，无第二系统。

use weave_batch::{JobPlan, Pipeline, StageSpec};

use crate::model::{StepType, Workflow};
use crate::validation::validate;

#[derive(Debug, Clone, PartialEq)]
pub struct CompileError {
    pub code: &'static str,
    pub message: String,
}

/// §34：编译前必须已通过 Validation（ERROR 级阻止）。
pub fn compile(
    workflow: &Workflow,
    inputs: Vec<std::path::PathBuf>,
) -> Result<JobPlan, CompileError> {
    let validation = validate(workflow);
    if validation.has_errors() {
        let first = validation
            .errors()
            .next()
            .map(|i| format!("{}: {}", i.code, i.message))
            .unwrap_or_default();
        return Err(CompileError {
            code: "workflow.validationFailed",
            message: first,
        });
    }
    let mut stages: Vec<StageSpec> = Vec::new();
    let mut destination_dir = None;
    for step in &workflow.steps {
        if !step.enabled {
            // §55/§56：disabled = bypass，但 Preview 侧会标注；编译跳过
            continue;
        }
        match step.r#type {
            StepType::Input => { /* 输入来自运行时参数 inputs，编译为 Source */ }
            StepType::Filter => match step.kind.as_str() {
                "filter.extension" => {
                    let exts = step
                        .config
                        .get("extensionsIn")
                        .or_else(|| step.config.get("extensions_in"))
                        .and_then(|v| v.as_array())
                        .map(|a| {
                            a.iter()
                                .filter_map(|e| e.as_str().map(|s| s.to_ascii_lowercase()))
                                .collect()
                        })
                        .unwrap_or_default();
                    stages.push(StageSpec::Filter {
                        extensions_in: exts,
                        max_bytes: None,
                    });
                }
                other => {
                    return Err(CompileError {
                        code: "workflow.unknownFilter",
                        message: format!("unknown filter '{other}'"),
                    });
                }
            },
            StepType::Tool => match step.kind.as_str() {
                "document.inspect" => stages.push(StageSpec::DocumentInspect),
                "pdf.rotate" => stages.push(StageSpec::PdfRotate {
                    degrees: step
                        .config
                        .get("degrees")
                        .and_then(|d| d.as_f64())
                        .unwrap_or(90.0),
                    pages: step
                        .config
                        .get("pages")
                        .and_then(|p| p.as_str())
                        .unwrap_or("")
                        .to_owned(),
                }),
                "image.resize" => stages.push(StageSpec::ImageResize {
                    width: step
                        .config
                        .get("width")
                        .and_then(|w| w.as_f64())
                        .unwrap_or(1920.0) as u32,
                    height: step
                        .config
                        .get("height")
                        .and_then(|h| h.as_f64())
                        .unwrap_or(1080.0) as u32,
                    mode: step
                        .config
                        .get("mode")
                        .and_then(|m| m.as_str())
                        .unwrap_or("fit")
                        .to_owned(),
                    prevent_upscale: step
                        .config
                        .get("preventUpscale")
                        .and_then(|p| p.as_bool())
                        .unwrap_or(true),
                }),
                "image.encode" => stages.push(StageSpec::Encode {
                    format: step
                        .config
                        .get("format")
                        .and_then(|f| f.as_str())
                        .unwrap_or("png")
                        .to_owned(),
                    quality: step
                        .config
                        .get("quality")
                        .and_then(|q| q.as_f64())
                        .map(|q| q.clamp(1.0, 100.0) as u8),
                }),
                other => {
                    return Err(CompileError {
                        code: "workflow.unknownTool",
                        message: format!("unknown tool '{other}'"),
                    });
                }
            },
            StepType::Export => {
                // §18/§66：Export 只描述 What/Where；dest 目录来自运行时
                // 参数（portable workflow §62），不进持久化定义
                if let Some(dir) = step.config.get("destinationDir").and_then(|d| d.as_str()) {
                    destination_dir = Some(dir.to_owned());
                }
            }
        }
    }
    // M7 线性 pipeline 约束：Source 必须在首位、Export 在末位
    stages.insert(0, StageSpec::Source);
    let dest = std::path::PathBuf::from(destination_dir.as_deref().unwrap_or("."));
    stages.push(StageSpec::Export {
        destination_dir: dest.clone(),
        overwrite: workflow
            .steps
            .last()
            .and_then(|s| s.config.get("overwrite"))
            .and_then(|o| o.as_bool())
            .unwrap_or(false),
    });

    let pipeline = Pipeline { stages };
    weave_batch::build_job_plan(inputs, pipeline, dest, true, false).map_err(|e| CompileError {
        code: "workflow.compileFailed",
        message: e,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Workflow, WorkflowStep};

    fn step(id: &str, ty: StepType, kind: &str, config: serde_json::Value) -> WorkflowStep {
        WorkflowStep {
            id: id.into(),
            r#type: ty,
            name: id.into(),
            kind: kind.into(),
            config,
            enabled: true,
        }
    }

    fn ws() -> tempfile::TempDir {
        tempfile::tempdir().expect("ws")
    }

    fn sample_workflow() -> Workflow {
        Workflow {
            schema_version: 1,
            id: "wf".into(),
            name: "PDF Rotate Workflow".into(),
            description: String::new(),
            steps: vec![
                step("i1", StepType::Input, "files", serde_json::json!({})),
                step(
                    "f1",
                    StepType::Filter,
                    "filter.extension",
                    serde_json::json!({"extensionsIn": ["pdf"]}),
                ),
                step(
                    "t1",
                    StepType::Tool,
                    "pdf.rotate",
                    serde_json::json!({"degrees": 90}),
                ),
                step(
                    "e1",
                    StepType::Export,
                    "export.files",
                    serde_json::json!({"destinationDir": "OUT"}),
                ),
            ],
            parameters: vec![],
        }
    }

    #[test]
    fn compiles_to_m7_pipeline() {
        // §34/§35：Workflow → M7 JobPlan（Source/Filter/PdfRotate/Export）
        let dir = ws();
        let pdf = dir.path().join("a.pdf");
        lopdf_write(&pdf, 2);
        let wf = sample_workflow();
        let wf = wf_with_dest(&wf, dir.path().join("out").to_string_lossy().as_ref());
        std::fs::create_dir_all(dir.path().join("out")).expect("out");
        let plan = compile(&wf, vec![pdf.clone()]).expect("compile");
        let stages: Vec<String> = plan
            .pipeline
            .stages
            .iter()
            .map(|s| match s {
                weave_batch::StageSpec::Source => "source".into(),
                weave_batch::StageSpec::Filter { .. } => "filter".into(),
                weave_batch::StageSpec::DocumentInspect => "inspect".into(),
                weave_batch::StageSpec::PdfRotate { .. } => "rotate".into(),
                weave_batch::StageSpec::Export { .. } => "export".into(),
                other => format!("{other:?}"),
            })
            .collect();
        assert_eq!(
            stages,
            vec!["source", "filter", "rotate", "export"],
            "§35 编译为 M7 pipeline"
        );
        // 编译产物直接可执行（§33 同引擎）
        let result =
            weave_batch::execute_plan(&plan, &weave_core::prelude::CancellationToken::new(), None);
        assert_eq!(result.succeeded, 1, "{result:?}");
        assert!(dir.path().join("out").join("a.pdf").exists());
        let _ = lopdf_write; // 使用标记
    }

    #[test]
    fn disabled_step_is_bypassed() {
        // §55：disabled = bypass
        let dir = ws();
        let pdf = dir.path().join("a.pdf");
        lopdf_write(&pdf, 2);
        let mut wf = sample_workflow();
        wf.steps[2].enabled = false; // rotate 禁用
        let wf = wf_with_dest(&wf, dir.path().join("out").to_string_lossy().as_ref());
        std::fs::create_dir_all(dir.path().join("out")).expect("out");
        let plan = compile(&wf, vec![pdf]).expect("compile");
        let stages: Vec<String> = plan
            .pipeline
            .stages
            .iter()
            .map(|s| match s {
                weave_batch::StageSpec::PdfRotate { .. } => "rotate".into(),
                weave_batch::StageSpec::Filter { .. } => "filter".into(),
                _ => "other".into(),
            })
            .collect();
        assert!(!stages.contains(&"rotate".to_string()), "disabled = bypass");
    }

    #[test]
    fn compile_rejects_invalid_workflow() {
        let mut wf = sample_workflow();
        wf.steps[2].config = serde_json::json!({"degrees": 45});
        let e = compile(&wf, vec![]).expect_err("validation");
        assert_eq!(e.code, "workflow.validationFailed");
    }

    // helpers
    fn wf_with_dest(wf: &Workflow, dest: &str) -> Workflow {
        let mut w = wf.clone();
        if let Some(last) = w.steps.last_mut() {
            last.config = serde_json::json!({"destinationDir": dest});
        }
        w
    }

    fn lopdf_write(path: &std::path::Path, pages: usize) {
        use lopdf::{Document, Object, Stream, dictionary};
        let mut doc = Document::with_version("1.5");
        let pages_id = doc.add_object(dictionary! {});
        let mut kids = Vec::new();
        for _ in 0..pages {
            let content_id = doc.add_object(Stream::new(dictionary! {}, b"x".to_vec()));
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
        let catalog_id = doc.add_object(
            dictionary! { "Type" => "Catalog", "Pages" => Object::Reference(pages_id) },
        );
        doc.trailer.set("Root", Object::Reference(catalog_id));
        doc.save(path).expect("save");
    }
}
