//! M4 Text 文件级集成测试（下 §131–§135）：真实文件系统 + 真实历史存储。
//!
//! 覆盖：Round-trip（§131）、TOCTOU 三例（§132）、History 重启存活（§133）、
//! Undo 冲突（§134）、权限故障（§135，Windows readonly 语义）。

use weave_app_lib::text_service::{TextWriteCache, build_text_write_plan, run_text_write_job};
use weave_core::prelude::CancellationToken;
use weave_history::HistoryStore;
use weave_testkit::TempWorkspace;

fn plan_cache() -> weave_app_lib::rename_service::PlanCache {
    weave_app_lib::rename_service::PlanCache::new()
}

fn tmp_history(ws: &TempWorkspace) -> std::path::PathBuf {
    let dir = ws.path().join("history");
    std::fs::create_dir_all(&dir).expect("mkdir");
    dir
}

/// §131：create → load → format → plan → apply → read back == formatted。
#[test]
fn round_trip_file_integration() {
    let ws = TempWorkspace::new("m4-roundtrip").expect("ws");
    let path = ws.path().join("cfg.json");
    std::fs::write(&path, "{\"b\":2,\"a\":1}").expect("seed");

    let doc =
        weave_app_lib::text_service::load_text_document(path.to_string_lossy().as_ref(), None)
            .expect("load");
    let formatted =
        weave_app_lib::text_service::format_text(&doc.format, "format", &doc.content, 2.0, true)
            .expect("format");
    let new_content = formatted.content.expect("content");

    let plans = plan_cache();
    let cache = TextWriteCache::new();
    let plan_dto = build_text_write_plan(
        &cache,
        &plans,
        path.to_string_lossy().as_ref(),
        &new_content,
        &doc.encoding,
        &doc.bom,
        doc.byte_size,
        doc.modified_ms,
        false,
    )
    .expect("plan");
    let plan = plans.take(&plan_dto.operation_id).expect("cached plan");
    let entry = cache.take(&plan_dto.operation_id).expect("cached bytes");
    let hist = tmp_history(&ws);
    let (report, _tx) =
        run_text_write_job(&hist, &plan, entry, &CancellationToken::new(), &mut |_| {})
            .expect("write ok");
    assert!(!report.failed);
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        new_content,
        "磁盘内容 == 格式化输出"
    );
}

/// §132-1/§132-2：外部修改 ⇒ changedSincePreview；外部删除 ⇒ fileMissing。
#[test]
fn toctou_modify_and_delete_are_rejected() {
    let ws = TempWorkspace::new("m4-toctou").expect("ws");
    let plans = plan_cache();
    let cache = TextWriteCache::new();
    let hist = tmp_history(&ws);

    // — modify —
    let path = ws.path().join("a.txt");
    std::fs::write(&path, "original").expect("seed");
    let plan_dto = build_text_write_plan(
        &cache,
        &plans,
        path.to_string_lossy().as_ref(),
        "updated",
        "utf8",
        "none",
        9.0, // 加载时快照
        None,
        false,
    )
    .expect("plan");
    std::fs::write(&path, "externally modified!").expect("external write");
    let plan = plans.take(&plan_dto.operation_id).expect("plan");
    let entry = cache.take(&plan_dto.operation_id).expect("entry");
    let err = run_text_write_job(&hist, &plan, entry, &CancellationToken::new(), &mut |_| {})
        .expect_err("must reject");
    assert_eq!(err.1.code, "text.fileChangedSincePreview");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "externally modified!",
        "外部内容不被覆盖"
    );

    // — delete —
    let path2 = ws.path().join("b.txt");
    std::fs::write(&path2, "gone soon").expect("seed");
    let plan_dto = build_text_write_plan(
        &cache,
        &plans,
        path2.to_string_lossy().as_ref(),
        "updated",
        "utf8",
        "none",
        9.0,
        None,
        false,
    )
    .expect("plan");
    std::fs::remove_file(&path2).expect("external delete");
    let plan = plans.take(&plan_dto.operation_id).expect("plan");
    let entry = cache.take(&plan_dto.operation_id).expect("entry");
    let err = run_text_write_job(&hist, &plan, entry, &CancellationToken::new(), &mut |_| {})
        .expect_err("must reject");
    assert_eq!(err.1.code, "text.fileMissing");
    assert!(!path2.exists(), "删除后不得静默重建");
}

/// §132-3：Save-As 目标在计划后被外部创建 ⇒ destinationExists。
#[test]
fn save_as_destination_collision_is_rejected() {
    let ws = TempWorkspace::new("m4-saveas").expect("ws");
    let dest = ws.path().join("new.json");
    let plans = plan_cache();
    let cache = TextWriteCache::new();
    let hist = tmp_history(&ws);

    // 计划期即拒绝（目标已存在）
    std::fs::write(&dest, "already here").expect("external seed");
    let err = build_text_write_plan(
        &cache,
        &plans,
        dest.to_string_lossy().as_ref(),
        "{}",
        "utf8",
        "none",
        0.0,
        None,
        true,
    )
    .expect_err("destination exists at build");
    assert_eq!(err.code, "text.destinationExists");

    // 执行期复查：计划时目标不存在，计划后外部创建
    std::fs::remove_file(&dest).expect("clear dest");
    let plan_dto = build_text_write_plan(
        &cache,
        &plans,
        dest.to_string_lossy().as_ref(),
        "{}",
        "utf8",
        "none",
        0.0,
        None,
        true,
    )
    .expect("plan ok (dest absent)");
    std::fs::write(&dest, "external").expect("external create");
    let plan = plans.take(&plan_dto.operation_id).expect("plan");
    let entry = cache.take(&plan_dto.operation_id).expect("entry");
    let err = run_text_write_job(&hist, &plan, entry, &CancellationToken::new(), &mut |_| {})
        .expect_err("must reject at execute");
    assert_eq!(err.1.code, "text.destinationExists");
    assert_eq!(std::fs::read_to_string(&dest).expect("read"), "external");
}

/// §133 + §134：写回 → 历史落盘 → 重开存储 → undo 还原；
/// 用户在写回后手动编辑 ⇒ undo 拒绝覆盖且文件完好。
#[test]
fn history_survives_restart_and_undo_conflicts_on_user_edit() {
    let ws = TempWorkspace::new("m4-history").expect("ws");
    let path = ws.path().join("doc.txt");
    std::fs::write(&path, "v1").expect("seed");
    let hist = tmp_history(&ws);

    let plans = plan_cache();
    let cache = TextWriteCache::new();
    let plan_dto = build_text_write_plan(
        &cache,
        &plans,
        path.to_string_lossy().as_ref(),
        "v2 formatted",
        "utf8",
        "none",
        2.0,
        None,
        false,
    )
    .expect("plan");
    let plan = plans.take(&plan_dto.operation_id).expect("plan");
    let entry = cache.take(&plan_dto.operation_id).expect("entry");
    let (_report, tx) =
        run_text_write_job(&hist, &plan, entry, &CancellationToken::new(), &mut |_| {})
            .expect("write");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "v2 formatted"
    );

    // 重启模拟：重开存储
    let store = HistoryStore::open(&hist).expect("reopen");
    let loaded = store
        .load_transaction(&tx.operation_id)
        .expect("load")
        .expect("tx persisted");
    drop(store);

    // §134：写回后用户手动编辑 ⇒ undo 拒绝覆盖
    std::fs::write(&path, "user edit after write").expect("user edit");
    let report = weave_files::undo_transaction(
        &weave_files::fs::StdFilesystem,
        &loaded,
        &CancellationToken::new(),
        &mut |_| {},
    );
    assert_eq!(report.conflicts, 1, "{report:?}");
    assert_eq!(
        std::fs::read_to_string(&path).expect("read"),
        "user edit after write",
        "绝不覆盖用户编辑"
    );
}

/// §135：写权限故障（Windows readonly 语义：原子替换到只读目标失败）。
#[test]
fn readonly_target_write_fails_structured() {
    let ws = TempWorkspace::new("m4-readonly").expect("ws");
    let path = ws.path().join("ro.txt");
    std::fs::write(&path, "locked").expect("seed");
    let meta = std::fs::metadata(&path).expect("stat");
    let mut perms = meta.permissions();
    perms.set_readonly(true);
    std::fs::set_permissions(&path, perms).expect("chmod");

    let plans = plan_cache();
    let cache = TextWriteCache::new();
    let plan_dto = build_text_write_plan(
        &cache,
        &plans,
        path.to_string_lossy().as_ref(),
        "should fail",
        "utf8",
        "none",
        6.0,
        None,
        false,
    )
    .expect("plan");
    let plan = plans.take(&plan_dto.operation_id).expect("plan");
    let entry = cache.take(&plan_dto.operation_id).expect("entry");
    let hist = tmp_history(&ws);
    let err = run_text_write_job(&hist, &plan, entry, &CancellationToken::new(), &mut |_| {});
    // 还原权限以便清理
    let mut perms = meta.permissions();
    perms.set_readonly(!perms.readonly());
    std::fs::set_permissions(&path, perms).expect("chmod back");
    match err {
        Err((report, e)) => {
            assert!(report.failed);
            assert!(
                e.code.starts_with("text.") || e.code.starts_with("atomic."),
                "structured error, got {}",
                e.code
            );
            assert_eq!(
                std::fs::read_to_string(&path).expect("read"),
                "locked",
                "只读目标内容不变"
            );
        }
        Ok(_) => panic!("readonly target must fail the write"),
    }
}
