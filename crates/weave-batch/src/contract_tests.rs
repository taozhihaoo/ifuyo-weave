//! M7（下）JSON contract / 状态机 fuzz / 版本兼容（§195-§196/§199）。

use crate::journal::{JOURNAL_SCHEMA_VERSION, JournalHeader};
use crate::{JobResult, Pipeline, StageSpec, TextOpSpec, build_job_plan, execute_plan};
use weave_core::prelude::CancellationToken;

fn ws() -> tempfile::TempDir {
    tempfile::tempdir().expect("ws")
}

// ── §195 JSON Contract ──

#[test]
fn job_result_json_roundtrip_and_fields() {
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let a = dir.path().join("a.txt");
    std::fs::write(&a, "x").expect("w");
    let b = dir.path().join("b.log");
    std::fs::write(&b, "y").expect("w");
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Filter {
                extensions_in: vec!["txt".into()],
                max_bytes: None,
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(vec![a, b], pipeline, dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);

    let json = serde_json::to_string(&result).expect("serialize");
    let back: JobResult = SerdeRoundtrip::from_json(&json);
    assert_eq!(back, result, "结果 roundtrip 无损（§195）");
    let v: serde_json::Value = serde_json::from_str(&json).expect("json value");
    // 前端依赖的字段必须稳定存在（§195 fragile fields 防护）
    for field in [
        "items",
        "total",
        "succeeded",
        "failed",
        "skipped",
        "cancelled",
        "pending",
        "inputBytes",
        "outputBytes",
        "preview",
    ] {
        assert!(v.get(field).is_some(), "JobResult.{field} must exist");
    }
    let items = v
        .get("items")
        .and_then(|i| i.as_array())
        .expect("items array");
    let first = &items[0];
    for field in [
        "itemId",
        "source",
        "output",
        "status",
        "error",
        "retryable",
        "stageResults",
        "inputBytes",
        "outputBytes",
    ] {
        assert!(first.get(field).is_some(), "ItemResult.{field} must exist");
    }
}

// serde_json 直接 roundtrip（上面的 helper 不能存在——真实 roundtrip）
trait SerdeRoundtrip: serde::de::DeserializeOwned {
    fn from_json(s: &str) -> Self {
        serde_json::from_str(s).expect("deserialize")
    }
}
impl<T: serde::de::DeserializeOwned> SerdeRoundtrip for T {}

#[test]
fn job_plan_and_stage_spec_json_roundtrip() {
    // §19/§196：可序列化计划 = 可复现；字段名稳定
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let a = dir.path().join("a.txt");
    std::fs::write(&a, "  x  ").expect("w");
    // 文本管线（合法形态）roundtrip
    let text_pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Filter {
                extensions_in: vec!["txt".into()],
                max_bytes: Some(1024),
            },
            StageSpec::TextTransform {
                operations: vec![
                    TextOpSpec::TrimLines,
                    TextOpSpec::Replace {
                        find: "x".into(),
                        replace_with: "y".into(),
                        case_sensitive: true,
                    },
                ],
            },
            StageSpec::Encode {
                format: "txt".into(),
                quality: None,
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(
        vec![a.clone()],
        text_pipeline.clone(),
        dest.clone(),
        true,
        false,
    )
    .expect("plan");
    let json = serde_json::to_string(&plan).expect("serialize");
    let back: crate::JobPlan = SerdeRoundtrip::from_json(&json);
    assert_eq!(back, plan, "计划 roundtrip 必须无损（§19）");

    // 图像管线（合法形态）roundtrip
    let png = dir.path().join("i.png");
    image::DynamicImage::new_rgba8(4, 4)
        .save_with_format(&png, image::ImageFormat::Png)
        .expect("png");
    let image_pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::ImageResize {
                width: 16,
                height: 16,
                mode: "fit".into(),
                prevent_upscale: true,
            },
            StageSpec::Encode {
                format: "png".into(),
                quality: Some(85),
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan2 = build_job_plan(vec![png], image_pipeline, dest, true, false).expect("plan2");
    let json2 = serde_json::to_string(&plan2).expect("serialize2");
    let back2: crate::JobPlan = SerdeRoundtrip::from_json(&json2);
    assert_eq!(back2, plan2);

    // StageSpec tagged 形态稳定（前端契约）
    let stage_json = serde_json::to_string(&StageSpec::Encode {
        format: "png".into(),
        quality: Some(90),
    })
    .expect("stage");
    let v: serde_json::Value = serde_json::from_str(&stage_json).expect("v");
    assert_eq!(v.get("type").and_then(|t| t.as_str()), Some("encode"));
}

// ── §196 Version Compatibility ──

#[test]
fn journal_header_carries_schema_version() {
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let a = dir.path().join("a.txt");
    std::fs::write(&a, "x").expect("w");
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(vec![a], pipeline, dest.clone(), true, false).expect("plan");
    let header = JournalHeader {
        schema_version: JOURNAL_SCHEMA_VERSION,
        job_id: "j".into(),
        created_ms: 0,
        plan,
        total_items: 1,
    };
    let json = serde_json::to_string(&header).expect("serialize");
    let v: serde_json::Value = serde_json::from_str(&json).expect("v");
    assert_eq!(
        v.get("schemaVersion").and_then(|s| s.as_u64()),
        Some(JOURNAL_SCHEMA_VERSION as u64)
    );
}

// ── §199 State Machine Fuzz（确定性种子，可复现 §117）──

#[test]
fn state_machine_fuzz_never_enters_illegal_state() {
    use crate::state::JobState;
    // xorshift64 确定性种子——失败可复现（§117 Deterministic Retry 同源）
    let mut seed: u64 = 0x5EED_7A70_2026;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    for iteration in 0..2000 {
        let mut states: Vec<JobState> = vec![JobState::Created];
        for _ in 0..24 {
            let pick = next() % 6;
            let from = *states.last().expect("non-empty");
            let target = match pick {
                0 => JobState::Planned,
                1 => JobState::Ready,
                2 => JobState::Running,
                3 => JobState::Paused,
                4 => JobState::Cancelling,
                _ => JobState::Completed,
            };
            if from.transition(target).is_ok() {
                states.push(target);
            }
        }
        // 不变量：最后必为合法状态；Created 只能出现在开头
        for (i, s) in states.iter().enumerate() {
            if i > 0 {
                assert!(
                    !matches!(s, JobState::Created),
                    "iteration {iteration}: illegal re-entry to Created"
                );
            }
        }
        // Cancelled/Completed 后不得再 Running（引擎终态纪律）
        for w in states.windows(2) {
            if matches!(w[0], JobState::Cancelled | JobState::Completed) {
                assert!(
                    !matches!(w[1], JobState::Running),
                    "iteration {iteration}: terminal → Running"
                );
            }
        }
    }
}
