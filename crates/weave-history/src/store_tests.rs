//! 历史存储测试（M2 §54–§55 / §78 restart / corrupt）。

use super::*;
use std::time::{Duration, SystemTime};
use weave_core::prelude::{OperationId, OperationKind};
use weave_testkit::TempWorkspace;

fn sample_entry(op: &OperationId, summary: &str) -> HistoryEntry {
    HistoryEntry {
        operation_id: op.clone(),
        kind: OperationKind::Rename,
        timestamp: SystemTime::UNIX_EPOCH,
        summary: summary.to_string(),
        item_count: 2,
        success_count: 2,
        failed_count: 0,
        skipped_count: 0,
        undoable: true,
        status: crate::model::OperationStatus::Completed,
        input_root: Some("C:\\w".to_string()),
        rule_summary: Some("vacation-{counter}.{ext}".to_string()),
    }
}

fn sample_transaction(op: &OperationId) -> OperationTransaction {
    OperationTransaction {
        operation_id: op.clone(),
        kind: OperationKind::Rename,
        status: crate::model::OperationStatus::Completed,
        timestamp: SystemTime::UNIX_EPOCH,
        reversible: crate::model::Reversibility::Full,
        items: vec![TransactionItem {
            item_id: "item_0000".to_string(),
            source_path: "C:\\w\\a.jpg".to_string(),
            target_path: "C:\\w\\b.jpg".to_string(),
            status: crate::model::TransactionItemStatus::Executed,
            timestamp: Some(SystemTime::UNIX_EPOCH + Duration::from_secs(1)),
            original_size: Some(10),
            original_modified: Some(SystemTime::UNIX_EPOCH),
            original_created: None,
        }],
    }
}

#[test]
fn upsert_then_reload_round_trips() {
    let ws = TempWorkspace::new("hist-round").expect("ws");
    let store = HistoryStore::open(ws.path()).expect("open");
    let op = OperationId::parse("op_aa0000000001").expect("id");
    store
        .upsert_entry(sample_entry(&op, "Rename 2 files"))
        .expect("upsert");
    store
        .save_transaction(&sample_transaction(&op))
        .expect("tx");

    // 新实例（模拟重启）读取
    let reopened = HistoryStore::open(ws.path()).expect("reopen");
    let entries = reopened.recent(10).expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].summary, "Rename 2 files");
    assert!(entries[0].undoable);

    let tx = reopened
        .load_transaction(&op)
        .expect("load")
        .expect("exists");
    assert_eq!(tx.items.len(), 1);
    assert_eq!(tx.items[0].source_path, "C:\\w\\a.jpg");
}

#[test]
fn upsert_same_operation_replaces_not_duplicates() {
    let ws = TempWorkspace::new("hist-upsert").expect("ws");
    let store = HistoryStore::open(ws.path()).expect("open");
    let op = OperationId::parse("op_bb0000000001").expect("id");
    store.upsert_entry(sample_entry(&op, "v1")).expect("v1");
    store.upsert_entry(sample_entry(&op, "v2")).expect("v2");
    let entries = store.recent(10).expect("entries");
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].summary, "v2");
}

#[test]
fn corrupted_entries_file_is_quarantined_and_app_survives() {
    let ws = TempWorkspace::new("hist-corrupt").expect("ws");
    let store = HistoryStore::open(ws.path()).expect("open");
    let op = OperationId::parse("op_cc0000000001").expect("id");
    store
        .upsert_entry(sample_entry(&op, "before corruption"))
        .expect("upsert");

    // 模拟部分写入 / 损坏
    let entries_path = ws.path().join("entries.json");
    std::fs::write(&entries_path, "{ not valid json").expect("corrupt");

    let outcome = store.load_entries().expect("safe degradation");
    assert!(
        outcome.entries.is_empty(),
        "corrupt history degrades to empty"
    );
    assert_eq!(outcome.quarantined.len(), 1);
    assert!(outcome.quarantined[0].starts_with("entries.json.corrupt-"));
    // 隔离后可以继续写入（应用存活）
    store
        .upsert_entry(sample_entry(&op, "after corruption"))
        .expect("write after");
    assert_eq!(store.recent(10).expect("r").len(), 1);
}

#[test]
fn corrupted_transaction_quarantines_and_reports_none() {
    let ws = TempWorkspace::new("hist-tx-corrupt").expect("ws");
    let store = HistoryStore::open(ws.path()).expect("open");
    let op = OperationId::parse("op_dd0000000001").expect("id");
    store
        .save_transaction(&sample_transaction(&op))
        .expect("tx");
    let tx_path = ws.path().join("transactions").join(format!("{op}.json"));
    std::fs::write(&tx_path, "truncated").expect("corrupt");

    let tx = store.load_transaction(&op).expect("safe degradation");
    assert!(
        tx.is_none(),
        "corrupt transaction reports None (undo unavailable)"
    );
}

#[test]
fn unknown_schema_version_is_quarantined_not_migrated_silently() {
    let ws = TempWorkspace::new("hist-schema").expect("ws");
    let store = HistoryStore::open(ws.path()).expect("open");
    let entries_path = ws.path().join("entries.json");
    std::fs::write(&entries_path, r#"{"schemaVersion": 99, "entries": []}"#)
        .expect("write future schema");

    let outcome = store.load_entries().expect("degrade");
    assert!(outcome.entries.is_empty());
    assert_eq!(outcome.quarantined.len(), 1);
}

#[test]
fn entry_capacity_is_bounded_oldest_evicted() {
    let ws = TempWorkspace::new("hist-cap").expect("ws");
    let store = HistoryStore::open(ws.path()).expect("open");
    for i in 0..=crate::MAX_ENTRIES {
        let op = OperationId::generate();
        store
            .upsert_entry(sample_entry(&op, &format!("op {i}")))
            .expect("upsert");
    }
    let entries = store.recent(usize::MAX).expect("entries");
    assert_eq!(entries.len(), crate::MAX_ENTRIES);
    assert!(
        entries[0]
            .summary
            .ends_with(&format!("{}", crate::MAX_ENTRIES))
    );
}
