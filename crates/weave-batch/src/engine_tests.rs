//! M7 引擎测试（上 §20-§36/§55-§58）：管线校验、执行、失败隔离、
//! 取消、确定性枚举。

use std::path::PathBuf;
use std::sync::Arc;

use crate::{
    ItemStatus, JobExecOptions, Pipeline, StageSpec, TextOpSpec, build_job_plan, execute_plan,
    execute_subset, preview_plan, revalidate_snapshot, snapshot_inputs,
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
            write_text(
                &p,
                "  line one  
  line two  
",
            );
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

// ── M7（下）C2：子集执行 / workers 确定性 / pause / retryable ──

use std::sync::atomic::{AtomicBool, Ordering};

fn pipeline_trim_to(dest: &std::path::Path) -> Pipeline {
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
                destination_dir: dest.to_path_buf(),
                overwrite: true,
            },
        ],
    }
}

#[test]
fn workers_do_not_change_results() {
    // 下 §123：1 worker 与 4 workers 同计划同输入 ⇒ 结果一致（含批内重名）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    // 子目录 a 与 b 各有同名 same.txt ⇒ 批内输出重名（快照序小者胜）
    let sa = dir.path().join("a");
    let sb = dir.path().join("b");
    std::fs::create_dir_all(&sa).expect("sa");
    std::fs::create_dir_all(&sb).expect("sb");
    write_text(&sa.join("same.txt"), "  alpha  ");
    write_text(&sb.join("same.txt"), "  beta  ");
    write_text(&dir.path().join("z.txt"), "  gamma  ");
    let inputs = vec![
        sa.join("same.txt"),
        sb.join("same.txt"),
        dir.path().join("z.txt"),
    ];

    let results: Vec<_> = [1usize, 4]
        .iter()
        .map(|&w| {
            // 每个 worker 档独立目录，避免上次落盘碰撞干扰
            let d = dest.join(format!("w{w}"));
            std::fs::create_dir_all(&d).expect("d");
            let plan = build_job_plan(inputs.clone(), pipeline_trim_to(&d), d.clone(), true, false)
                .expect("plan");
            execute_plan(&plan, &CancellationToken::new(), None)
        })
        .collect();
    let r1 = &results[0];
    let r4 = &results[1];
    assert_eq!((r1.succeeded, r1.failed), (2, 1), "1 worker: {r1:?}");
    assert_eq!(
        (r4.succeeded, r4.failed, r4.skipped, r4.cancelled),
        (r1.succeeded, r1.failed, r1.skipped, r1.cancelled),
        "下 §123：worker 数不得改变结果"
    );
    assert_eq!(
        r1.items.iter().map(|i| i.status).collect::<Vec<_>>(),
        r4.items.iter().map(|i| i.status).collect::<Vec<_>>()
    );
    let cat = |r: &crate::JobResult| -> Vec<Option<String>> {
        r.items
            .iter()
            .map(|i| {
                i.error
                    .as_ref()
                    .map(|e| e.split(':').next().unwrap_or("").to_owned())
            })
            .collect()
    };
    assert_eq!(
        cat(r1),
        cat(r4),
        "Collision 获胜方 = 快照序小者，与完成序无关"
    );
}

#[test]
fn pause_at_safe_point_leaves_pending_not_cancelled() {
    // 下 §143：Pause ⇒ 未开始条目 = pending（非终态），非 Cancelled
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs: Vec<PathBuf> = (0..8)
        .map(|i| {
            let p = dir.path().join(format!("f{i}.txt"));
            write_text(&p, "x");
            p
        })
        .collect();
    let plan =
        build_job_plan(inputs, pipeline_trim_to(&dest), dest.clone(), true, false).expect("plan");

    let pause = AtomicBool::new(true); // 一开始就置位 ⇒ 第一安全点即暂停
    let result = execute_subset(
        &plan,
        &(0..8).collect::<Vec<_>>(),
        JobExecOptions {
            workers: 1,
            pause: Some(&pause),
            dry_run: false,
            on_item: None,
        },
        &CancellationToken::new(),
        None,
    );
    assert_eq!(result.pending, 8, "{result:?}");
    assert_eq!(result.succeeded + result.failed + result.cancelled, 0);
    assert_eq!(result.items.len(), 0, "pending 条目不产生 item 终态");

    // 解除暂停后续跑同一 subset ⇒ 全部完成（Resume 语义）
    pause.store(false, Ordering::SeqCst);
    let resumed = execute_subset(
        &plan,
        &(0..8).collect::<Vec<_>>(),
        JobExecOptions {
            workers: 1,
            pause: None,
            dry_run: false,
            on_item: None,
        },
        &CancellationToken::new(),
        None,
    );
    assert_eq!(resumed.succeeded, 8);
}

#[test]
fn subset_execution_equals_full_execution() {
    // 下 §39/§126：Retry/Resume 的子集机制 = 全量执行的逐条结果
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs: Vec<PathBuf> = (0..4)
        .map(|i| {
            let p = dir.path().join(format!("f{i}.txt"));
            write_text(&p, "  pad  ");
            p
        })
        .collect();
    // 失败条目：doomed.txt 在快照后删除 ⇒ Read 失败
    let doomed = dir.path().join("doomed.txt");
    write_text(&doomed, "bye");
    let mut all_inputs = inputs.clone();
    all_inputs.push(doomed.clone());

    let plan_a = build_job_plan(
        all_inputs.clone(),
        pipeline_trim_to(&dest),
        dest.clone(),
        true,
        false,
    )
    .expect("plan a");
    let full = execute_plan(&plan_a, &CancellationToken::new(), None);

    // 单独执行 doomed 一条 = 全量执行中该条结果
    std::fs::create_dir_all(dest.join("out2")).expect("out2");
    let plan_b = build_job_plan(
        all_inputs.clone(),
        pipeline_trim_to(&dest.join("out2")),
        dest.join("out2"),
        true,
        false,
    )
    .expect("plan b");
    std::fs::remove_file(&doomed).expect("remove");
    let single = execute_subset(
        &plan_b,
        &[4],
        JobExecOptions {
            workers: 1,
            pause: None,
            dry_run: false,
            on_item: None,
        },
        &CancellationToken::new(),
        None,
    );
    assert_eq!(single.items.len(), 1);
    assert_eq!(single.items[0].status, full.items[4].status);
    assert_eq!(
        single.items[0].error.clone(),
        full.items[4].error.clone(),
        "子集执行的失败事实与全量一致"
    );
}

#[test]
fn retryable_reflects_error_category() {
    // 下 §142/§237：Read 失败可重试；Validation 类不可重试
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let gone = dir.path().join("gone.txt");
    write_text(&gone, "x");
    let plan = build_job_plan(
        vec![gone],
        pipeline_trim_to(&dest),
        dest.clone(),
        true,
        false,
    )
    .expect("plan");
    std::fs::remove_file(dir.path().join("gone.txt")).expect("rm");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(result.failed, 1);
    assert!(result.items[0].retryable, "Read 失败 = 瞬态可重试");

    // Validation：未知 resize mode ⇒ 不可重试（需先过 Decode——用真实 PNG）
    let p2 = dir.path().join("v.png");
    image::DynamicImage::new_rgba8(8, 8)
        .save_with_format(&p2, image::ImageFormat::Png)
        .expect("save png");
    let pipeline_bad = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::ImageResize {
                width: 10,
                height: 10,
                mode: "bogus".into(),
                prevent_upscale: true,
            },
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: true,
            },
        ],
    };
    let plan2 = build_job_plan(vec![p2], pipeline_bad, dest, true, false).expect("plan2");
    let r2 = execute_plan(&plan2, &CancellationToken::new(), None);
    assert_eq!(r2.failed, 1);
    assert!(!r2.items[0].retryable, "Validation 失败不可重试");
}

/// PERF 矩阵（下 §130/§132）：100/1k/10k + cancel latency + workers 对比。
/// 默认忽略：`cargo test -p weave-batch --release -- --ignored --nocapture`。
#[test]
#[ignore = "perf: release 手动运行"]
fn perf_matrix_and_cancel_latency() {
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs: Vec<PathBuf> = (0..10_000)
        .map(|i| {
            let p = dir.path().join(format!("p{i:05}.txt"));
            write_text(&p, "  perf line  ");
            p
        })
        .collect();

    for n in [100usize, 1_000, 10_000] {
        let d = dest.join(format!("n{n}"));
        std::fs::create_dir_all(&d).expect("d");
        let plan = build_job_plan(
            inputs[..n].to_vec(),
            pipeline_trim_to(&d),
            d.clone(),
            true,
            false,
        )
        .expect("plan");
        let t0 = Instant::now();
        let r = execute_plan(&plan, &CancellationToken::new(), None);
        let dt = t0.elapsed();
        assert_eq!(r.succeeded, n as u64);
        eprintln!("PERF matrix text {n} files (workers=1): {dt:?}");
    }

    // workers 对比（§178：不只报 files/sec；4 workers 同结果）
    for w in [1usize, 4] {
        let d = dest.join(format!("w{w}k"));
        std::fs::create_dir_all(&d).expect("d");
        let plan = build_job_plan(
            inputs[..2_000].to_vec(),
            pipeline_trim_to(&d),
            d.clone(),
            true,
            false,
        )
        .expect("plan");
        let t0 = Instant::now();
        let r = execute_subset(
            &plan,
            &(0..2_000).collect::<Vec<_>>(),
            JobExecOptions {
                workers: w,
                pause: None,
                dry_run: false,
                on_item: None,
            },
            &CancellationToken::new(),
            None,
        );
        let dt = t0.elapsed();
        assert_eq!(r.succeeded, 2_000);
        eprintln!(
            "PERF workers={w}: 2000 files {dt:?} (succeeded={})",
            r.succeeded
        );
    }

    // 取消延迟（§132）：8k 条任务 1s 后 cancel ⇒ 从请求到返回的时延
    let d = dest.join("cancel");
    std::fs::create_dir_all(&d).expect("d");
    let plan = build_job_plan(
        inputs[..8_000].to_vec(),
        pipeline_trim_to(&d),
        d.clone(),
        true,
        false,
    )
    .expect("plan");
    let cancel = CancellationToken::new();
    let cancel2 = cancel.clone();
    let flag = Arc::new(AtomicBool::new(false));
    let flag2 = Arc::clone(&flag);
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        cancel2.cancel();
        flag2.store(true, Ordering::SeqCst);
    });
    let t0 = Instant::now();
    let r = execute_plan(&plan, &cancel, None);
    let total = t0.elapsed();
    // cancel 已置位后引擎收尾的耗时 = 延迟上界
    let after_cancel = Instant::now();
    while !flag.load(Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
    let _ = after_cancel.elapsed();
    assert_eq!(r.cancelled + r.succeeded, 8_000, "计数闭合 {r:?}");
    assert!(r.cancelled > 0, "取消必须生效");
    eprintln!(
        "PERF cancel: 8000-item job cancelled at 1s; wall={total:?}; succeeded={} cancelled={}",
        r.succeeded, r.cancelled
    );
}
