//! M7（下）真实文件系统 / 故障注入 / golden / 不变量测试（§121-§122/§128/
//! §193-§194）。

use std::path::PathBuf;

#[cfg(windows)]
use std::os::windows::fs::OpenOptionsExt;

use crate::{
    JobExecOptions, Pipeline, StageSpec, TextOpSpec, build_job_plan, execute_plan, execute_subset,
    preview_plan,
};
use weave_core::prelude::CancellationToken;

fn ws() -> tempfile::TempDir {
    tempfile::tempdir().expect("ws")
}

fn write_text(path: &std::path::Path, content: &str) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("mkdir");
    }
    std::fs::write(path, content).expect("write");
}

fn trim_pipeline(dest: &std::path::Path) -> Pipeline {
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
                overwrite: false,
            },
        ],
    }
}

// ── §121 Real Filesystem ──

#[test]
fn real_fs_unicode_and_nested_paths() {
    let dir = ws();
    let dest = dir.path().join("输出 目录");
    std::fs::create_dir_all(&dest).expect("dest");
    // Unicode 目录 + Unicode 文件名 + 嵌套层级
    let a = dir.path().join("资料/中文 文件.txt");
    let b = dir.path().join("deep/nested/path/émoji-✓.txt");
    write_text(&a, "  alpha  ");
    write_text(&b, "  beta  ");
    let plan =
        build_job_plan(vec![a, b], trim_pipeline(&dest), dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (2, 0), "{result:?}");
    let out_a = dest.join("中文 文件.txt");
    assert_eq!(
        std::fs::read_to_string(&out_a).expect("read unicode output"),
        "alpha"
    );
}

#[test]
fn real_fs_long_file_name() {
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    // Windows MAX_PATH 边界：180 字符文件名（stem）在 temp 下安全
    let stem = "l".repeat(180);
    let p = dir.path().join(format!("{stem}.txt"));
    write_text(&p, "  long  ");
    let plan =
        build_job_plan(vec![p], trim_pipeline(&dest), dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(result.succeeded, 1, "{result:?}");
    assert!(dest.join(format!("{stem}.txt")).exists());
}

#[test]
fn collision_with_preexisting_output_fails_item() {
    // §122 DestinationExists + §176：Preview 后目标出现 ⇒ Execute 拒绝
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let a = dir.path().join("a.txt");
    write_text(&a, "  content  ");
    let plan =
        build_job_plan(vec![a], trim_pipeline(&dest), dest.clone(), true, false).expect("plan");
    // 模拟 Preview 后外部创建目标
    write_text(&dest.join("a.txt"), "pre-existing");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(result.failed, 1);
    assert!(
        result.items[0]
            .error
            .as_deref()
            .unwrap_or("")
            .starts_with("Collision:")
    );
    // 原目标未被覆盖（§212 Dangerous Operations：Never overwrite 默认）
    assert_eq!(
        std::fs::read_to_string(dest.join("a.txt")).expect("read"),
        "pre-existing"
    );
}

#[cfg(windows)]
#[test]
fn locked_file_fails_item_isolation() {
    // §122 LockedFile（Windows：独占句柄）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let locked = dir.path().join("locked.txt");
    write_text(&locked, "held");
    let free = dir.path().join("free.txt");
    write_text(&free, "  ok  ");
    let file = std::fs::OpenOptions::new()
        .read(true)
        .share_mode(0) // 排他：任何其他访问被拒
        .open(&locked)
        .expect("lock handle");
    let plan = build_job_plan(
        vec![locked.clone(), free],
        trim_pipeline(&dest),
        dest.clone(),
        true,
        false,
    )
    .expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (1, 1));
    assert!(
        result.items.iter().any(|i| {
            i.source == locked && i.error.as_deref().unwrap_or("").starts_with("Read:")
        })
    );
    drop(file);
}

#[test]
fn destination_write_failure_when_target_is_directory() {
    // §122 DestinationWriteFailed（可移植形态：输出路径已是目录 ⇒ 写失败）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let a = dir.path().join("a.txt");
    write_text(&a, "x");
    // 目标名被目录占用；overwrite=true 跳过碰撞检查直抵写失败
    std::fs::create_dir_all(dest.join("a.txt")).expect("occupy");
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Export {
                destination_dir: dest.clone(),
                overwrite: true,
            },
        ],
    };
    let plan = build_job_plan(vec![a], pipeline, dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(result.failed, 1, "{result:?}");
    assert!(
        result.items[0]
            .error
            .as_deref()
            .unwrap_or("")
            .starts_with("Write:")
    );
}

#[test]
fn decode_failure_isolated_in_image_pipeline() {
    // §122 DecodeFailure：伪 PNG 进图像管线 ⇒ 该条 Failed，其余继续
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let garbage = dir.path().join("garbage.png");
    std::fs::write(&garbage, b"not a png at all").expect("write garbage");
    let good = dir.path().join("good.png");
    image::DynamicImage::new_rgba8(8, 8)
        .save_with_format(&good, image::ImageFormat::Png)
        .expect("save");
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::ImageResize {
                width: 4,
                height: 4,
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
    let plan =
        build_job_plan(vec![garbage, good], pipeline, dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (1, 1), "{result:?}");
    assert!(
        result.items[0]
            .error
            .as_deref()
            .unwrap_or("")
            .starts_with("Decode:")
    );
}

// ── §193/§194 Golden ──

#[test]
fn golden_ten_files_two_stage() {
    // §193：10 输入 × trim → 10 成功 / 0 失败 / 10 产物（确定性命名）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs: Vec<PathBuf> = (0..10)
        .map(|i| {
            let p = dir.path().join(format!("g{i:02}.txt"));
            write_text(&p, &format!("  golden {i}  \n"));
            p
        })
        .collect();
    let plan =
        build_job_plan(inputs, trim_pipeline(&dest), dest.clone(), true, false).expect("plan");
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!(
        (result.succeeded, result.failed, result.skipped),
        (10, 0, 0)
    );
    assert_eq!(
        result.items.iter().filter(|i| i.output.is_some()).count(),
        10
    );
    for i in 0..10 {
        let out = dest.join(format!("g{i:02}.txt"));
        assert_eq!(
            std::fs::read_to_string(&out).expect("golden output"),
            format!(
                "golden {i}
"
            ),
            "TrimLines 保留文档行尾（TextDocument 语义）"
        );
    }
}

#[test]
fn failure_golden_three_inaccessible() {
    // §194：10 输入中 3 个快照后消失 ⇒ 7 成功 / 3 失败（稳定）
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let mut inputs: Vec<PathBuf> = Vec::new();
    for i in 0..10 {
        let p = dir.path().join(format!("f{i}.txt"));
        write_text(&p, "x");
        inputs.push(p);
    }
    let plan = build_job_plan(
        inputs.clone(),
        trim_pipeline(&dest),
        dest.clone(),
        true,
        false,
    )
    .expect("plan");
    for p in &inputs[7..] {
        std::fs::remove_file(p).expect("remove");
    }
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    assert_eq!((result.succeeded, result.failed), (7, 3), "{result:?}");
}

// ── §128 Invariants ──

#[test]
fn invariants_counts_and_output_truthfulness() {
    // §128：Success+Failed+Skipped+Cancelled = Total；
    // Failed 条目不得有 output；全部终态合法。
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let mut inputs: Vec<PathBuf> = Vec::new();
    for i in 0..6 {
        let p = dir.path().join(format!("m{i}.txt"));
        write_text(&p, "x");
        inputs.push(p);
    }
    // .log 会被过滤（Skipped），后 2 个删除（Failed）
    let log = dir.path().join("extra.log");
    write_text(&log, "skip me");
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
    let mut all = inputs.clone();
    all.push(log);
    let plan = build_job_plan(all, pipeline, dest.clone(), true, false).expect("plan");
    for p in &inputs[4..] {
        std::fs::remove_file(p).expect("remove");
    }
    let result = execute_plan(&plan, &CancellationToken::new(), None);
    let counted = result.succeeded + result.failed + result.skipped + result.cancelled;
    assert_eq!(counted, result.total, "§128 计数闭合");
    assert_eq!((result.succeeded, result.failed, result.skipped), (4, 2, 1));
    for item in &result.items {
        if item.status == crate::ItemStatus::Failed {
            assert!(item.output.is_none(), "Failed 不得声称产物");
        }
        assert!(
            matches!(
                item.status,
                crate::ItemStatus::Success
                    | crate::ItemStatus::Failed
                    | crate::ItemStatus::Skipped
                    | crate::ItemStatus::Cancelled
            ),
            "终态合法集"
        );
    }
}

#[test]
fn preview_never_mutates_under_faults() {
    // §21/§234：即便全部条目注定失败，Preview 也不产生任何文件
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let gone = dir.path().join("gone.txt");
    write_text(&gone, "x");
    let plan =
        build_job_plan(vec![gone], trim_pipeline(&dest), dest.clone(), true, false).expect("plan");
    std::fs::remove_file(dir.path().join("gone.txt")).expect("rm");
    let pv = preview_plan(&plan);
    assert_eq!(pv.failed, 1);
    assert!(pv.preview);
    assert_eq!(std::fs::read_dir(&dest).expect("dest").count(), 0);
}

// ── §126 Journal-driven resume 集成（领域侧）──

#[test]
fn journal_records_drive_subset_resume() {
    use crate::journal::{
        self, JOURNAL_SCHEMA_VERSION, JournalHeader, JournalItemStatus, JournalLine,
    };
    let dir = ws();
    let dest = dir.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let inputs: Vec<PathBuf> = (0..5)
        .map(|i| {
            let p = dir.path().join(format!("r{i}.txt"));
            write_text(&p, "  x  ");
            p
        })
        .collect();
    let plan = build_job_plan(
        inputs.clone(),
        trim_pipeline(&dest),
        dest.clone(),
        true,
        false,
    )
    .expect("plan");
    // 模拟部分完成：只跑前 2 条 + on_item 写 journal
    let jdir = dir.path().join("jobs");
    let header = JournalHeader {
        schema_version: JOURNAL_SCHEMA_VERSION,
        job_id: "job_resume".into(),
        created_ms: 1,
        plan: plan.clone(),
        total_items: 5,
    };
    let jpath = journal::create(&jdir, &header).expect("create");
    let part = execute_subset(
        &plan,
        &[0, 1],
        JobExecOptions {
            workers: 1,
            pause: None,
            dry_run: false,
            on_item: Some(&|item| {
                let status = match item.status {
                    crate::ItemStatus::Success => JournalItemStatus::Success,
                    _ => JournalItemStatus::Failed,
                };
                let _ = journal::append(
                    &jpath,
                    &JournalLine::Item(crate::journal::JournalItemRecord {
                        item_index: item
                            .item_id
                            .trim_start_matches("item_")
                            .parse()
                            .unwrap_or(usize::MAX),
                        status,
                        output_path: item
                            .output
                            .as_ref()
                            .map(|o| o.to_string_lossy().into_owned()),
                        output_size: None,
                        error: item.error.clone(),
                    }),
                );
            }),
        },
        &CancellationToken::new(),
        None,
    );
    assert_eq!(part.succeeded, 2);
    let load = journal::load(&jpath).expect("load");
    assert_eq!(load.completed_indices(), vec![0, 1]);
    // Resume = 子集 {2,3,4}（§39：已完成不重跑）
    let settled = load.settled_indices();
    let rest: Vec<usize> = (0..5).filter(|i| !settled.contains(i)).collect();
    assert_eq!(rest, vec![2, 3, 4]);
    let jpath2 = jpath.clone();
    let resumed = execute_subset(
        &plan,
        &rest,
        JobExecOptions {
            workers: 1,
            pause: None,
            dry_run: false,
            on_item: Some(&|item| {
                let status = match item.status {
                    crate::ItemStatus::Success => JournalItemStatus::Success,
                    _ => JournalItemStatus::Failed,
                };
                let _ = journal::append(
                    &jpath2,
                    &JournalLine::Item(crate::journal::JournalItemRecord {
                        item_index: item
                            .item_id
                            .trim_start_matches("item_")
                            .parse()
                            .unwrap_or(usize::MAX),
                        status,
                        output_path: item
                            .output
                            .as_ref()
                            .map(|o| o.to_string_lossy().into_owned()),
                        output_size: None,
                        error: item.error.clone(),
                    }),
                );
            }),
        },
        &CancellationToken::new(),
        None,
    );
    assert_eq!(resumed.succeeded, 3);
    // 合并事实 = 全量执行
    assert!(dest.join("r4.txt").metadata().expect("r4").len() > 0);
    // 已完成条目未被重跑（overwrite=false ⇒ 若重跑会 Collision/Failed）
    let load2 = journal::load(&jpath).expect("load2");
    assert_eq!(load2.completed_indices(), vec![0, 1, 2, 3, 4]);
}
