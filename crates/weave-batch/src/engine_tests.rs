//! M7 引擎测试（上 §20-§36/§55-§58）：管线校验、执行、失败隔离、
//! 取消、确定性枚举。

use std::path::PathBuf;

use crate::{
    ItemStatus, Pipeline, StageSpec, TextOpSpec, build_job_plan, execute_plan, preview_plan,
    revalidate_snapshot, snapshot_inputs,
};

use weave_core::prelude::CancellationToken;

fn ws() -> tempfile::TempDir {
    tempfile::tempdir().expect("ws")
}

fn write_text(path: &std::path::Path, content: &str) {
    std::fs::write(path, content).expect("write");
}

fn pipeline_text() -> Pipeline {
    Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::TextTransform {
                operations: vec![TextOpSpec::TrimLines],
            },
            StageSpec::Encode {
                format: "txt".into(),
                quality: None,
            },
            StageSpec::Export {
                destination_dir: PathBuf::new(), // 测试中替换
                overwrite: false,
            },
        ],
    }
}

#[test]
fn pipeline_validation_rejects_bad_shapes() {
    // §56：无 Source / 无 Export / 多 Export / 类型不兼容
    let p = Pipeline { stages: vec![] };
    assert!(p.validate().is_err());

    let p = Pipeline {
        stages: vec![StageSpec::Export {
            destination_dir: PathBuf::new(),
            overwrite: false,
        }],
    };
    assert!(p.validate().is_err(), "必须以 Source 开始");

    let p = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Encode {
                format: "txt".into(),
                quality: None,
            },
        ],
    };
    assert!(p.validate().is_err(), "必须以 Export 结束");

    // 类型不兼容：TextTransform 后接 ImageResize（无 encode 中转）
    let p = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::TextTransform { operations: vec![] },
            StageSpec::ImageResize {
                width: 10,
                height: 10,
                mode: "fit".into(),
                prevent_upscale: true,
            },
            StageSpec::Export {
                destination_dir: PathBuf::new(),
                overwrite: false,
            },
        ],
    };
    assert!(p.validate().is_err(), "Text→Image 不兼容（§57/§58）");
}

#[test]
fn text_pipeline_executes_end_to_end() {
    let dir = ws();
    let src = dir.path().join("in.txt");
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    write_text(&src, "  hello  \n  world  \n");

    let mut pipeline = pipeline_text();
    if let Some(StageSpec::Export {
        destination_dir, ..
    }) = pipeline.stages.last_mut()
    {
        *destination_dir = dest.clone();
    }
    let plan =
        build_job_plan(vec![src.clone()], pipeline, dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);

    assert_eq!((result.succeeded, result.failed), (1, 0), "{result:?}");
    let out_file = dest.join("in.txt");
    assert_eq!(
        std::fs::read_to_string(&out_file).expect("read"),
        "hello\nworld\n",
        "trim 后写盘"
    );
}

#[test]
fn filter_rejection_is_skipped_not_failed() {
    // §14：Rejected ⇒ Skipped（非 Failed）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.log");
    write_text(&a, "a");
    write_text(&b, "b");

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
    assert_eq!((result.succeeded, result.skipped), (1, 1));
}

#[test]
fn per_item_failure_isolation() {
    // §25/§49：单条失败不放弃其余。文件在快照后、执行前消失 ⇒ 该条
    // Source Read 失败，其余照常。
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let good = dir.path().join("good.txt");
    let doomed = dir.path().join("doomed.txt");
    write_text(&good, "ok");
    write_text(&doomed, "bye");

    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(
        vec![good, doomed.clone()],
        pipeline,
        dest.clone(),
        true,
        false,
    )
    .expect("plan");
    std::fs::remove_file(&doomed).expect("remove");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (1, 1));
    let failed_item = result
        .items
        .iter()
        .find(|i| i.status == ItemStatus::Failed)
        .expect("failed item");
    assert!(
        failed_item
            .error
            .as_deref()
            .unwrap_or("")
            .starts_with("Read:")
    );
}

#[test]
fn cancellation_marks_unstarted_items() {
    // §33：取消安全点——未开始条目 = Cancelled
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs: Vec<PathBuf> = (0..3)
        .map(|i| {
            let p = dir.path().join(format!("f{i}.txt"));
            write_text(&p, "x");
            p
        })
        .collect();
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(inputs, pipeline, dest, true, false).expect("plan");
    let cancel = CancellationToken::new();
    cancel.cancel();
    let result = execute_plan(&plan, &cancel, None);
    assert_eq!(result.cancelled, 3);
    assert_eq!(result.succeeded, 0);
}

#[test]
fn snapshot_is_deterministically_sorted_and_revalidates() {
    // §55/§202/§204：确定性排序 + ChangedSincePreview
    let dir = ws();
    let a = dir.path().join("a.txt");
    let b = dir.path().join("b.txt");
    write_text(&a, "a");
    write_text(&b, "b");
    let snap = snapshot_inputs(vec![b.clone(), a.clone()]).expect("snap");
    assert_eq!(
        snap.iter().map(|e| e.path.clone()).collect::<Vec<_>>(),
        vec![a.clone(), b.clone()],
        "路径字典序"
    );
    // 外部修改 ⇒ ChangedSincePreview；文件消失 ⇒ FileMissing
    let snap2 = snapshot_inputs(vec![a.clone()]).expect("snap");
    std::fs::write(&a, "changed").expect("modify");
    let err = revalidate_snapshot(&snap2).expect_err("changed");
    assert!(err.contains("ChangedSincePreview"));
    std::fs::remove_file(&a).expect("remove");
    let err = revalidate_snapshot(&snap2).expect_err("missing");
    assert!(err.contains("FileMissing"));
}

#[test]
fn preview_runs_same_engine_without_writing() {
    // §20-§23：Preview 与 Execute 同引擎；§21 无副作用（不落盘）；
    // 碰撞仍上报（§22 Potential Failures）。
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let src = dir.path().join("in.txt");
    write_text(&src, "  hello  ");

    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::TextTransform {
                operations: vec![TextOpSpec::TrimLines],
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(vec![src], pipeline, dest.clone(), true, false).expect("plan");
    let pv = preview_plan(&plan);
    assert!(pv.preview);
    assert_eq!((pv.succeeded, pv.failed, pv.skipped), (1, 0, 0), "{pv:?}");
    assert_eq!(pv.items[0].output_bytes, 5, "hello 在内存中 5 字节");
    assert!(!dest.join("in.txt").exists(), "Preview 不得写盘（§21）");

    // Execute 用同一 plan 才真正落盘
    let ex = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(ex.succeeded, 1);
    assert_eq!(
        std::fs::read_to_string(dest.join("in.txt")).expect("read"),
        "hello"
    );

    // 碰撞：Preview 也报 Failed（Collision）
    let pv2 = preview_plan(&plan);
    assert_eq!(pv2.failed, 1);
    assert!(
        pv2.items[0]
            .error
            .as_deref()
            .unwrap_or("")
            .starts_with("Collision:")
    );
}

/// PERF（charter #68：真实测量）。默认忽略，`cargo test -p weave-batch
/// --release -- --ignored --nocapture` 运行；数字记入 docs/PERF.md。
#[test]
#[ignore = "perf: release 手动运行"]
fn perf_text_batch_1000() {
    use std::time::Instant;
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs: Vec<PathBuf> = (0..1000)
        .map(|i| {
            let p = dir.path().join(format!("f{i:04}.txt"));
            write_text(&p, "  line one  
  line two  
");
            p
        })
        .collect();
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::TextTransform {
                operations: vec![TextOpSpec::TrimLines],
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
    let plan = build_job_plan(inputs, pipeline, dest, true, false).expect("plan");
    let t0 = Instant::now();
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    let dt = t0.elapsed();
    assert_eq!(result.succeeded, 1000);
    eprintln!("PERF text batch 1000 files (read+trim+write): {dt:?}");
}

/// PERF：100 张 512×384 PNG → Fit 1920×1080（防放大⇒原尺寸）+ PNG 重编码。
#[test]
#[ignore = "perf: release 手动运行"]
fn perf_image_batch_100() {
    use std::time::Instant;
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let img = image::DynamicImage::new_rgba8(512, 384);
    let inputs: Vec<PathBuf> = (0..100)
        .map(|i| {
            let p = dir.path().join(format!("img{i:03}.png"));
            img.save_with_format(&p, image::ImageFormat::Png)
                .expect("save png");
            p
        })
        .collect();
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::ImageResize {
                width: 1920,
                height: 1080,
                mode: "fit".into(),
                prevent_upscale: true,
            },
            StageSpec::Encode {
                format: "png".into(),
                quality: None,
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(inputs, pipeline, dest, true, false).expect("plan");
    let t0 = Instant::now();
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    let dt = t0.elapsed();
    assert_eq!(result.succeeded, 100);
    eprintln!("PERF image batch 100 files (decode+fit-resize+png-encode): {dt:?}");
}
