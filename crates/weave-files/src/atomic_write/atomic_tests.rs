//! atomic_write 测试（M4 §92：write/rename 失败路径）。

use crate::atomic_write::atomic_write;
use crate::fs::StdFilesystem;
use weave_testkit::TempWorkspace;

#[test]
fn atomic_write_replaces_content_and_cleans_temp() {
    let ws = TempWorkspace::new("atomic-ok").expect("ws");
    let path = ws.path().join("doc.txt");
    std::fs::write(&path, "old").expect("seed");
    atomic_write(&StdFilesystem, &path, b"new content").expect("write");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "new content");
    // temp 已被 rename 消费
    let leftovers: Vec<_> = std::fs::read_dir(ws.path())
        .expect("dir")
        .flatten()
        .filter(|e| e.file_name().to_string_lossy().contains(".weave-tmp-"))
        .collect();
    assert!(leftovers.is_empty(), "{leftovers:?}");
}

#[test]
fn atomic_write_creates_new_file() {
    let ws = TempWorkspace::new("atomic-new").expect("ws");
    let path = ws.path().join("fresh.txt");
    atomic_write(&StdFilesystem, &path, b"fresh").expect("write");
    assert_eq!(std::fs::read_to_string(&path).expect("read"), "fresh");
}

#[test]
fn atomic_write_missing_parent_is_structured_error() {
    let ws = TempWorkspace::new("atomic-bad").expect("ws");
    let path = ws.path().join("no-such-dir").join("x.txt");
    let err = atomic_write(&StdFilesystem, &path, b"x").expect_err("no parent");
    assert_eq!(err.code, "atomic.createTemp");
}
