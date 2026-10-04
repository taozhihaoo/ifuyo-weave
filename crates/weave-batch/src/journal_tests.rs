//! Job Journal 测试（M7 下 §126/§183/§196/§197）。

use std::path::PathBuf;

use crate::journal::{
    self, JOURNAL_SCHEMA_VERSION, JournalFinish, JournalHeader, JournalItemRecord,
    JournalItemStatus, JournalLine,
};
use crate::{Pipeline, StageSpec, build_job_plan};

fn header_for(job_id: &str, inputs: Vec<PathBuf>, dest: &std::path::Path) -> JournalHeader {
    let pipeline = Pipeline {
        stages: vec![
            StageSpec::Source,
            StageSpec::Export {
                destination_dir: dest.to_path_buf(),
                overwrite: false,
            },
        ],
    };
    let plan = build_job_plan(inputs, pipeline, dest.to_path_buf(), true, false).expect("plan");
    JournalHeader {
        schema_version: JOURNAL_SCHEMA_VERSION,
        job_id: job_id.to_string(),
        created_ms: 1_700_000_000_000,
        plan,
        total_items: 3,
    }
}

fn fixture_inputs(n: usize) -> (tempfile::TempDir, Vec<PathBuf>) {
    let dir = tempfile::tempdir().expect("ws");
    let mut inputs = Vec::new();
    for i in 0..n {
        let p = dir.path().join(format!("f{i}.txt"));
        std::fs::write(&p, "x").expect("write");
        inputs.push(p);
    }
    (dir, inputs)
}

#[test]
fn create_append_load_roundtrip() {
    let (_ws, inputs) = fixture_inputs(3);
    let dest = _ws.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let jdir = _ws.path().join("jobs");
    let h = header_for("job_a", inputs, &dest);
    let path = journal::create(&jdir, &h).expect("create");
    for i in 0..2u64 {
        journal::append(
            &path,
            &JournalLine::Item(JournalItemRecord {
                item_index: i as usize,
                status: JournalItemStatus::Success,
                output_path: Some(
                    dest.join(format!("f{i}.txt"))
                        .to_string_lossy()
                        .into_owned(),
                ),
                output_size: Some(1),
                error: None,
            }),
        )
        .expect("append");
    }
    journal::append(
        &path,
        &JournalLine::Finish(JournalFinish {
            state: "completed".into(),
        }),
    )
    .expect("append finish");

    let load = journal::load(&path).expect("load");
    assert_eq!(load.header.as_ref().expect("header").job_id, "job_a");
    assert_eq!(load.items.len(), 2);
    assert_eq!(load.finish.as_ref().expect("finish").state, "completed");
    assert_eq!(load.corrupt_tail_lines, 0);
    assert_eq!(load.completed_indices(), vec![0, 1]);
    assert_eq!(load.settled_indices(), vec![0, 1]);
}

#[test]
fn duplicate_create_rejected() {
    // §201/§202：同 job 重复创建 journal ⇒ 拒绝
    let (_ws, inputs) = fixture_inputs(1);
    let dest = _ws.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let jdir = _ws.path().join("jobs");
    let h = header_for("job_dup", inputs, &dest);
    journal::create(&jdir, &h).expect("first");
    assert!(
        journal::create(&jdir, &h).is_err(),
        "second create must fail"
    );
}

#[test]
fn crash_truncated_tail_is_isolated_not_fatal() {
    // §197：崩溃留下半行 ⇒ 完好前缀 + corrupt 计数；不 panic、不丢记录
    let (_ws, inputs) = fixture_inputs(3);
    let dest = _ws.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let jdir = _ws.path().join("jobs");
    let h = header_for("job_crash", inputs, &dest);
    let path = journal::create(&jdir, &h).expect("create");
    journal::append(
        &path,
        &JournalLine::Item(JournalItemRecord {
            item_index: 0,
            status: JournalItemStatus::Success,
            output_path: None,
            output_size: None,
            error: None,
        }),
    )
    .expect("append");
    // 模拟崩溃：追加半行 JSON
    use std::io::Write;
    let mut f = std::fs::OpenOptions::new()
        .append(true)
        .open(&path)
        .expect("open");
    write!(f, "{{\"type\":\"item\",\"itemInd").expect("half line");
    drop(f);

    let load = journal::load(&path).expect("load never panics");
    assert_eq!(load.items.len(), 1, "完好前缀保留");
    assert_eq!(load.completed_indices(), vec![0]);
    assert_eq!(load.corrupt_tail_lines, 1, "损坏行被如实报告");
    assert!(load.finish.is_none(), "无终态行 ⇒ Interrupted");

    // resume 前隔离：原件保留为 .corrupt
    let copy = journal::quarantine_corrupt(&path).expect("quarantine");
    assert!(copy.exists());
    assert!(path.exists(), "原路径继续用于追加（完好前缀）");
}

#[test]
fn unsupported_schema_version_detected() {
    // §196：加载后由调用方检查版本；这里验证旧版本字段可读出
    let (_ws, inputs) = fixture_inputs(1);
    let dest = _ws.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let jdir = _ws.path().join("jobs");
    let mut h = header_for("job_ver", inputs, &dest);
    h.schema_version = 999;
    let path = journal::create(&jdir, &h).expect("create");
    let load = journal::load(&path).expect("load");
    assert_eq!(load.header.expect("header").schema_version, 999);
}

#[test]
fn list_reports_states_across_restart() {
    // §183：重启后能区分 Completed 与 Interrupted(Recoverable)
    let (_ws, inputs) = fixture_inputs(2);
    let dest = _ws.path().join("out");
    std::fs::create_dir_all(&dest).expect("dest");
    let jdir = _ws.path().join("jobs");
    let done = header_for("job_done", vec![inputs[0].clone()], &dest);
    let p1 = journal::create(&jdir, &done).expect("create1");
    journal::append(
        &p1,
        &JournalLine::Finish(JournalFinish {
            state: "completed".into(),
        }),
    )
    .expect("finish");
    let stuck = header_for("job_stuck", vec![inputs[1].clone()], &dest);
    journal::create(&jdir, &stuck).expect("create2"); // 无 finish ⇒ Interrupted

    let all = journal::list(&jdir);
    assert_eq!(all.len(), 2);
    let done = all.iter().find(|s| s.job_id == "job_done").expect("done");
    let stuck = all.iter().find(|s| s.job_id == "job_stuck").expect("stuck");
    assert!(done.load.finish.is_some());
    assert!(stuck.load.finish.is_none(), "无 finish ⇒ 可恢复");
}
