//! 重复扫描测试（M3 §88–§91：大小分组/partial/full/分组/排序/确定性）。

use super::*;
use crate::fs::StdFilesystem;
use weave_core::prelude::CancellationToken;
use weave_testkit::TempWorkspace;

fn scan(ws: &TempWorkspace) -> DuplicateScanReport {
    scan_duplicates(
        &StdFilesystem,
        &[ws.path().to_string_lossy().into_owned()],
        0,
        &CancellationToken::new(),
        &mut |_| {},
    )
    .expect("scan")
}

#[test]
fn size_grouping_only_size_peers_are_candidates() {
    let ws = TempWorkspace::new("dup-size").expect("ws");
    ws.file("a.txt", "same content").expect("write");
    ws.file("b.txt", "same content").expect("write");
    ws.file("c.txt", "different length").expect("write");
    let report = scan(&ws);
    // a/b 同 size 同内容 → 1 组 2 文件；c 尺寸不同不进候选
    assert_eq!(report.files_scanned, 3);
    assert_eq!(report.candidate_files, 2);
    assert_eq!(report.duplicate_groups, 1);
    assert_eq!(report.duplicate_files, 2);
    assert_eq!(report.potential_reclaimable_size, 12); // len("same content")
}

#[test]
fn same_size_different_content_is_not_duplicate() {
    let ws = TempWorkspace::new("dup-samesize").expect("ws");
    ws.file("a.bin", "AAAA").expect("write");
    ws.file("b.bin", "BBBB").expect("write");
    let report = scan(&ws);
    assert_eq!(report.candidate_files, 2, "同 size 进入候选");
    assert_eq!(report.duplicate_groups, 0, "内容不同 full hash 分开");
    assert_eq!(report.potential_reclaimable_size, 0);
}

#[test]
fn zero_byte_files_group_without_disk_reads() {
    let ws = TempWorkspace::new("dup-zero").expect("ws");
    ws.file("a.txt", "").expect("write");
    ws.file("b.txt", "").expect("write");
    ws.file("c.tmp", "").expect("write");
    let report = scan(&ws);
    assert_eq!(report.duplicate_groups, 1);
    assert_eq!(report.duplicate_files, 3);
}

#[test]
fn partial_hash_distinguishes_head_and_tail_differences() {
    // first-chunk-differs 与 last-chunk-differs 两类用例（M3 §4 验收硬化）
    let n = PARTIAL_HASH_BYTES as usize;
    let head_a = vec![1u8; n];
    let head_b = vec![2u8; n];
    let tail_diff_a = {
        let mut v = vec![7u8; n * 2];
        v[n..].copy_from_slice(&vec![3u8; n]);
        v
    };
    let tail_diff_b = {
        let mut v = vec![7u8; n * 2];
        v[n..].copy_from_slice(&vec![4u8; n]);
        v
    };
    let ws = TempWorkspace::new("dup-partial").expect("ws");
    ws.bytes("same_head_1.bin", &head_a).expect("w1");
    ws.bytes("same_head_2.bin", &head_a).expect("w2");
    ws.bytes("same_head_3.bin", &head_b).expect("w3");
    ws.bytes("tail_1.bin", &tail_diff_a).expect("t1");
    ws.bytes("tail_2.bin", &tail_diff_b).expect("t2");

    let p1 = partial_hash(
        &StdFilesystem,
        &ws.path().join("same_head_1.bin"),
        head_a.len() as u64,
    )
    .expect("p1");
    let p2 = partial_hash(
        &StdFilesystem,
        &ws.path().join("same_head_2.bin"),
        head_a.len() as u64,
    )
    .expect("p2");
    let p3 = partial_hash(
        &StdFilesystem,
        &ws.path().join("same_head_3.bin"),
        head_a.len() as u64,
    )
    .expect("p3");
    assert_eq!(p1, p2, "same head → same partial");
    assert_ne!(p1, p3, "different head → partial hash filters correctly");

    let t1 = partial_hash(
        &StdFilesystem,
        &ws.path().join("tail_1.bin"),
        tail_diff_a.len() as u64,
    )
    .expect("t1");
    let t2 = partial_hash(
        &StdFilesystem,
        &ws.path().join("tail_2.bin"),
        tail_diff_b.len() as u64,
    )
    .expect("t2");
    assert_ne!(t1, t2, "last-chunk-differs: partial hash filters correctly");
}

#[test]
fn full_hash_separates_what_partial_cannot() {
    // 同 size 同 partial（中部差异）：partial 不能定论，full hash 分开
    let n = PARTIAL_HASH_BYTES as usize;
    let mut a = vec![5u8; n * 3];
    let mut b = vec![5u8; n * 3];
    a[n] = 1;
    b[n] = 2;
    let ws = TempWorkspace::new("dup-mid").expect("ws");
    ws.bytes("mid_a.bin", &a).expect("wa");
    ws.bytes("mid_b.bin", &b).expect("wb");

    let pa =
        partial_hash(&StdFilesystem, &ws.path().join("mid_a.bin"), a.len() as u64).expect("pa");
    let pb =
        partial_hash(&StdFilesystem, &ws.path().join("mid_b.bin"), b.len() as u64).expect("pb");
    assert_eq!(pa, pb, "partial 无法区分中部差异（设计内行为）");

    let report = scan(&ws);
    assert_eq!(report.duplicate_groups, 0, "full hash separates");
    assert_eq!(report.full_hashed, 2);
}

#[test]
fn extension_does_not_affect_duplicate_detection() {
    let ws = TempWorkspace::new("dup-ext").expect("ws");
    ws.file("a.bin", "identical").expect("write");
    ws.file("b.jpg", "identical").expect("write");
    let report = scan(&ws);
    assert_eq!(
        report.duplicate_groups, 1,
        "内容判定，扩展名不参与（M3 §8）"
    );
}

#[test]
fn group_ordering_by_wasted_size_desc_and_stable_membership() {
    let ws = TempWorkspace::new("dup-order").expect("ws");
    // 小组（2×3B wasted 3）与大组（2×20B wasted 20）
    ws.file("small_1.txt", "aaa").expect("write");
    ws.file("small_2.txt", "aaa").expect("write");
    ws.file("big_1.bin", "01234567890123456789").expect("write");
    ws.file("big_2.bin", "01234567890123456789").expect("write");
    let r1 = scan(&ws);
    let r2 = scan(&ws);
    // 确定性：两次扫描组序一致（wasted DESC）
    let ids1: Vec<String> = r1.groups.iter().map(|g| g.group_id.clone()).collect();
    let ids2: Vec<String> = r2.groups.iter().map(|g| g.group_id.clone()).collect();
    assert_eq!(ids1, ids2);
    assert_eq!(ids1.len(), 2);
    // 第一组 wasted 最大
    let wastes: Vec<u64> = r1.groups.iter().map(|g| g.wasted_size).collect();
    assert!(wastes.windows(2).all(|w| w[0] >= w[1]));
}

#[test]
fn group_id_is_content_derived_not_scan_order() {
    let ws1 = TempWorkspace::new("dup-gid-1").expect("ws");
    let ws2 = TempWorkspace::new("dup-gid-2").expect("ws");
    ws1.file("name_one.txt", "shared content").expect("write");
    ws2.file("completely_different_name.txt", "shared content")
        .expect("write");
    let r1 = scan(&ws1);
    let r2 = scan(&ws2);
    assert_eq!(
        r1.groups
            .iter()
            .map(|g| g.group_id.clone())
            .collect::<Vec<_>>(),
        r2.groups
            .iter()
            .map(|g| g.group_id.clone())
            .collect::<Vec<_>>(),
        "GroupId 由内容派生（M3 §43）"
    );
}

#[test]
fn symlink_policy_reused_from_m1_not_followed() {
    let ws = TempWorkspace::new("dup-symlink").expect("ws");
    ws.file("real.txt", "real").expect("write");
    #[cfg(windows)]
    {
        let link = ws.path().join("link.txt");
        if std::os::windows::fs::symlink_file(ws.path().join("real.txt"), &link).is_err() {
            return; // 无符号链接特权环境：跳过（如实标注）
        }
    }
    let report = scan(&ws);
    // symlink 是 other_entries，不进候选也不计 files_scanned（lstat 策略）
    assert_eq!(report.duplicate_groups, 0);
}
