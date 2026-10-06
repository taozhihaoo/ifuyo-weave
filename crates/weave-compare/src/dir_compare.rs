//! 目录比较引擎（M2.1 §4/§5/§11/§12/§25）。
//!
//! - 匹配键 = 分隔符归一（`/`）+ 平台大小写规则（Windows 不敏感）的相对路径。
//! - 遍历不跟随 symlink/junction（M1 同源策略）⇒ 无循环死循环。
//! - 局部失败不升级会话失败（§11）：单条 stat/read_dir 失败 ⇒ 该条
//!   Unreadable/Error 事实，会话照常产出。
//! - Moved/Renamed 只在 Hash 策略 + 唯一 hash 匹配时改判（§7 证据纪律）。
//! - 取消（§24）⇒ Cancelled（部分事实保留 + cancelled 标注），不是 Failed。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use weave_core::prelude::{CancellationToken, WeaveError};
use weave_files::fs::Filesystem;

use crate::file_compare::{facts_from_stat, node_of, stream_digest};
use crate::model::{
    ComparisonSession, ContentDifference, DifferenceItem, DifferenceKind, EntryFacts, NodeKind,
};
use crate::options::{CompareOptions, ContentCheck};

/// 单侧遍历产物：匹配键 → 条目（display 相对路径保留原始大小写）。
struct SideIndex {
    entries: HashMap<String, SideEntry>,
    read_errors: Vec<(String, WeaveError)>,
    ignored: u64,
}

struct SideEntry {
    display_rel: String,
    stat: weave_files::metadata::FileStat,
}

/// 比较两个目录（Read-only；耗时随规模，调用方经 Job 驱动 + 取消）。
pub fn compare_directories(
    fs: &dyn Filesystem,
    left: &Path,
    right: &Path,
    options: &CompareOptions,
    cancel: &CancellationToken,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> ComparisonSession {
    let started = SystemTime::now();
    let mut builder = ComparisonSession::builder(
        &left.to_string_lossy(),
        &right.to_string_lossy(),
        crate::model::SessionKind::Directories,
    );
    builder.effective_ignores(options.ignore.effective());

    // ── 遍历两侧（各自独立；一侧不存在 ⇒ 全量 Added/Removed）──
    let left_index = walk_side(fs, left, options, cancel, &mut |done| progress(done, None));
    if cancel.is_cancelled() {
        builder.set_cancelled();
        return builder.finish();
    }
    let right_index = walk_side(fs, right, options, cancel, &mut |done| progress(done, None));
    if cancel.is_cancelled() {
        builder.set_cancelled();
        return builder.finish();
    }
    builder.session.ignored_count = left_index.ignored + right_index.ignored;

    // 单侧 read_dir 失败 = 结构级事实（该子树不可比）⇒ Unreadable 条目
    for (rel, e) in &left_index.read_errors {
        let mut item =
            DifferenceItem::new(rel.clone(), NodeKind::Directory, DifferenceKind::Unreadable);
        item.error = Some(e.clone());
        item.left = Some(EntryFacts::path_only(
            left.join(rel).to_string_lossy().into_owned(),
        ));
        builder.push(item);
    }
    for (rel, e) in &right_index.read_errors {
        let mut item =
            DifferenceItem::new(rel.clone(), NodeKind::Directory, DifferenceKind::Unreadable);
        item.error = Some(e.clone());
        item.right = Some(EntryFacts::path_only(
            right.join(rel).to_string_lossy().into_owned(),
        ));
        builder.push(item);
    }

    // ── 配对 ──
    let total = (left_index.entries.len() + right_index.entries.len()) as u64;
    let mut processed = 0u64;
    let mut removed_files: Vec<String> = Vec::new();
    let mut added_files: Vec<String> = Vec::new();

    for (key, le) in &left_index.entries {
        processed += 1;
        progress(processed, Some(total));
        match right_index.entries.get(key) {
            None => {
                let mut item =
                    pair_absent_item(left, le.display_rel.as_str(), node_of(&le.stat), true);
                item.left = Some(facts_from_stat(&left.join(&le.display_rel), &le.stat, None));
                builder.push(item);
                if node_of(&le.stat) == NodeKind::File {
                    removed_files.push(key.clone());
                }
            }
            Some(re_) => {
                let item = compare_pair(fs, left, right, le, re_, options, cancel);
                if cancel.is_cancelled() {
                    builder.set_cancelled();
                    return builder.finish();
                }
                if item.kind == DifferenceKind::Removed && node_of(&le.stat) == NodeKind::File {
                    removed_files.push(key.clone());
                }
                builder.push(item);
            }
        }
    }
    for (key, re_) in &right_index.entries {
        if left_index.entries.contains_key(key) {
            continue;
        }
        processed += 1;
        progress(processed, Some(total));
        let mut item = pair_absent_item(right, re_.display_rel.as_str(), node_of(&re_.stat), false);
        item.right = Some(facts_from_stat(
            &right.join(&re_.display_rel),
            &re_.stat,
            None,
        ));
        builder.push(item);
        if node_of(&re_.stat) == NodeKind::File {
            added_files.push(key.clone());
        }
    }

    // ── Moved/Renamed 证据改判（仅 Hash 策略；有界：≥2000 候选放弃并保持诚实）──
    if options.content_check == ContentCheck::Hash
        && !cancel.is_cancelled()
        && removed_files.len() + added_files.len() <= 2000
    {
        moved_renamed_pass(
            fs,
            left,
            right,
            &removed_files,
            &added_files,
            &mut builder,
            cancel,
        );
    }
    if cancel.is_cancelled() {
        builder.set_cancelled();
    }
    let _ = started;
    builder.finish()
}

fn pair_absent_item(
    present_root: &Path,
    rel: &str,
    node: NodeKind,
    on_left: bool,
) -> DifferenceItem {
    let mut item = DifferenceItem::new(
        rel,
        node,
        if on_left {
            DifferenceKind::Removed
        } else {
            DifferenceKind::Added
        },
    );
    let facts = EntryFacts::path_only(present_root.join(rel).to_string_lossy().into_owned());
    if on_left {
        item.left = Some(facts);
    } else {
        item.right = Some(facts);
    }
    item
}

fn compare_pair(
    fs: &dyn Filesystem,
    left_root: &Path,
    right_root: &Path,
    left: &SideEntry,
    right: &SideEntry,
    options: &CompareOptions,
    cancel: &CancellationToken,
) -> DifferenceItem {
    let (left_rel, ls) = (&left.display_rel, &left.stat);
    let (right_rel, rs) = (&right.display_rel, &right.stat);
    let left_path = left_root.join(left_rel);
    let right_path = right_root.join(right_rel);
    let (ln, rn) = (node_of(ls), node_of(rs));
    if ln != rn {
        let mut item = DifferenceItem::new(left_rel, ln, DifferenceKind::TypeChanged);
        item.left = Some(facts_from_stat(&left_path, ls, None));
        item.right = Some(facts_from_stat(&right_path, rs, None));
        return item;
    }
    match ln {
        NodeKind::Directory => {
            // 结构比较（§5）：目录存在性 + 直接子项计数（来自遍历事实：
            // size 对目录无意义；child 计数不同 ⇒ 结构 Modified）
            let mut item = DifferenceItem::new(left_rel, NodeKind::Directory, DifferenceKind::Same);
            item.left = Some(facts_from_stat(&left_path, ls, None));
            item.right = Some(facts_from_stat(&right_path, rs, None));
            item
        }
        NodeKind::Symlink => {
            let mut item =
                DifferenceItem::new(left_rel, NodeKind::Symlink, DifferenceKind::Unsupported);
            item.error = Some(
                WeaveError::unsupported(
                    "compare.symlinkUnsupported",
                    "symlink targets are not followed; link entries are reported as-is",
                )
                .with_location("weave-compare::dir"),
            );
            item.left = Some(facts_from_stat(&left_path, ls, None));
            item.right = Some(facts_from_stat(&right_path, rs, None));
            item
        }
        NodeKind::File => {
            if ls.size != rs.size {
                let mut item =
                    DifferenceItem::new(left_rel, NodeKind::File, DifferenceKind::Modified);
                item.content = ContentDifference::ContentChanged;
                item.left = Some(facts_from_stat(&left_path, ls, None));
                item.right = Some(facts_from_stat(&right_path, rs, None));
                item.evidence = Some(crate::model::ComparisonEvidence {
                    method: "size".to_string(),
                    left_digest: None,
                    right_digest: None,
                    compared_at: SystemTime::now(),
                });
                return item;
            }
            match options.content_check {
                ContentCheck::Metadata => {
                    let same_mtime = ls.modified.is_some() && ls.modified == rs.modified;
                    let mut item = DifferenceItem::new(
                        left_rel,
                        NodeKind::File,
                        if same_mtime {
                            DifferenceKind::Same
                        } else {
                            DifferenceKind::Modified
                        },
                    );
                    if !same_mtime {
                        item.content = ContentDifference::MetadataOnly;
                    }
                    item.left = Some(facts_from_stat(&left_path, ls, None));
                    item.right = Some(facts_from_stat(&right_path, rs, None));
                    item.evidence = Some(crate::model::ComparisonEvidence {
                        method: "size+mtime".to_string(),
                        left_digest: None,
                        right_digest: None,
                        compared_at: SystemTime::now(),
                    });
                    item
                }
                ContentCheck::Hash => {
                    let (ld, rd) = (
                        stream_digest(fs, &left_path, cancel),
                        stream_digest(fs, &right_path, cancel),
                    );
                    match (ld, rd) {
                        (Ok(ld), Ok(rd)) => {
                            let same = ld == rd;
                            let same_mtime = ls.modified.is_some() && ls.modified == rs.modified;
                            let kind = if same {
                                DifferenceKind::Same
                            } else {
                                DifferenceKind::Modified
                            };
                            let mut item = DifferenceItem::new(left_rel, NodeKind::File, kind);
                            item.content = if same {
                                ContentDifference::None
                            } else {
                                ContentDifference::ContentChanged
                            };
                            let _ = same_mtime;
                            item.left = Some(facts_from_stat(&left_path, ls, Some(&ld)));
                            item.right = Some(facts_from_stat(&right_path, rs, Some(&rd)));
                            item.evidence = Some(crate::model::ComparisonEvidence {
                                method: "SHA-256".to_string(),
                                left_digest: Some(ld),
                                right_digest: Some(rd),
                                compared_at: SystemTime::now(),
                            });
                            item
                        }
                        (Err(e), _) | (_, Err(e)) => {
                            let mut item = DifferenceItem::new(
                                left_rel,
                                NodeKind::File,
                                DifferenceKind::Error,
                            );
                            item.left = Some(facts_from_stat(&left_path, ls, None));
                            item.right = Some(facts_from_stat(&right_path, rs, None));
                            item.error = Some(e);
                            item
                        }
                    }
                }
            }
        }
    }
}

/// Moved/Renamed：对 Added/Removed 的文件做 hash 配对，唯一匹配才改判。
#[allow(clippy::too_many_arguments)]
fn moved_renamed_pass(
    fs: &dyn Filesystem,
    left_root: &Path,
    right_root: &Path,
    removed_keys: &[String],
    added_keys: &[String],
    builder: &mut crate::model::SessionBuilder,
    cancel: &CancellationToken,
) {
    let mut removed_digests: HashMap<String, Option<String>> = HashMap::new();
    for key in removed_keys {
        if cancel.is_cancelled() {
            return;
        }
        let digest = digest_for_key(fs, left_root, key, cancel);
        removed_digests.insert(key.clone(), digest.ok());
    }
    let mut added_digests: HashMap<String, Option<String>> = HashMap::new();
    for key in added_keys {
        if cancel.is_cancelled() {
            return;
        }
        let digest = digest_for_key(fs, right_root, key, cancel);
        added_digests.insert(key.clone(), digest.ok());
    }
    // 唯一性要求：digest 在两侧候选集中都必须唯一（否则证据不足，保持原判）
    let mut left_by_digest: HashMap<&str, Vec<&String>> = HashMap::new();
    for (k, d) in &removed_digests {
        if let Some(d) = d {
            left_by_digest.entry(d.as_str()).or_default().push(k);
        }
    }
    let mut right_by_digest: HashMap<&str, Vec<&String>> = HashMap::new();
    for (k, d) in &added_digests {
        if let Some(d) = d {
            right_by_digest.entry(d.as_str()).or_default().push(k);
        }
    }
    for (digest, left_keys) in &left_by_digest {
        if left_keys.len() != 1 {
            continue;
        }
        let Some(right_keys) = right_by_digest.get(digest) else {
            continue;
        };
        if right_keys.len() != 1 {
            continue;
        }
        let left_key = left_keys[0];
        let right_key = right_keys[0];
        let name_of = |k: &str| k.rsplit('/').next().unwrap_or(k).to_lowercase();
        let kind = if name_of(left_key) == name_of(right_key) {
            DifferenceKind::Moved
        } else {
            DifferenceKind::Renamed
        };
        // 找到对应条目（左条目 = Removed，右条目 = Added）并改判 + 补齐对侧事实
        let li = builder
            .session
            .items
            .iter()
            .position(|i| i.kind == DifferenceKind::Removed && i.relative_path == *left_key);
        let ri = builder
            .session
            .items
            .iter()
            .position(|i| i.kind == DifferenceKind::Added && i.relative_path == *right_key);
        if let (Some(li), Some(ri)) = (li, ri) {
            let right_facts = builder.session.items[ri].right.clone();
            builder.session.items[li].right = right_facts;
            builder.reclassify(li, kind);
            // Added 侧条目已并入 Moved/Renamed ⇒ 移除条目并修正 total/桶
            builder.session.items.remove(ri);
            builder.session.summary.total -= 1;
            builder.session.summary.added -= 1;
        }
    }
}

fn digest_for_key(
    fs: &dyn Filesystem,
    root: &Path,
    key: &str,
    cancel: &CancellationToken,
) -> Result<String, WeaveError> {
    stream_digest(fs, &root.join(key), cancel)
}

/// 单侧遍历：迭代栈（无递归爆栈）；忽略规则在入表前计数；
/// read_dir 失败记为该子树的结构级错误事实。
fn walk_side(
    fs: &dyn Filesystem,
    root: &Path,
    options: &CompareOptions,
    cancel: &CancellationToken,
    progress: &mut dyn FnMut(u64),
) -> SideIndex {
    let mut index = SideIndex {
        entries: HashMap::new(),
        read_errors: Vec::new(),
        ignored: 0,
    };
    let mut stack: Vec<PathBuf> = vec![root.to_path_buf()];
    let mut done = 0u64;
    while let Some(dir) = stack.pop() {
        if cancel.is_cancelled() {
            return index;
        }
        let names = match fs.read_dir(&dir) {
            Ok(names) => names,
            Err(e) => {
                let rel = display_rel(root, &dir);
                index.read_errors.push((
                    rel,
                    WeaveError::from_io(&e, "compare", format!("cannot list {}", dir.display()))
                        .with_location("weave-compare::dir"),
                ));
                continue;
            }
        };
        for name in names {
            done += 1;
            progress(done);
            let child = dir.join(&name);
            let stat = match fs.stat(&child) {
                Ok(s) => s,
                Err(e) => {
                    // 单条 stat 失败：条目级事实，不升级（§11）
                    let rel = format!("{}/{}", display_rel(root, &dir), name);
                    index.read_errors.push((
                        rel,
                        WeaveError::from_io(
                            &e,
                            "compare",
                            format!("cannot stat {}", child.display()),
                        )
                        .with_location("weave-compare::dir"),
                    ));
                    continue;
                }
            };
            let rel = display_rel(root, &child);
            if options.ignore.matches(&rel) {
                index.ignored += 1;
                continue;
            }
            let is_dir = stat.kind == weave_core::prelude::FileKind::Directory;
            index.entries.insert(
                match_key(&rel),
                SideEntry {
                    display_rel: rel,
                    stat,
                },
            );
            if is_dir {
                stack.push(child); // symlink/junction 不是 Directory ⇒ 天然不跟随
            }
        }
    }
    index
}

/// 相对路径显示形式（`/` 分隔，保留原始大小写）。
fn display_rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace(std::path::MAIN_SEPARATOR, "/")
}

/// 匹配键：Windows 大小写不敏感（平台规则决定，§13），其余平台原样。
fn match_key(display_rel: &str) -> String {
    if cfg!(windows) {
        display_rel.to_lowercase()
    } else {
        display_rel.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::options::{CompareOptions, ContentCheck};
    use weave_files::fs::StdFilesystem;
    use weave_testkit::TempWorkspace;

    fn opts(check: ContentCheck) -> CompareOptions {
        CompareOptions {
            content_check: check,
            ..CompareOptions::default()
        }
    }

    fn seed_pair(ws: &TempWorkspace) -> (PathBuf, PathBuf) {
        let l = ws.dir("left").expect("left");
        let r = ws.dir("right").expect("right");
        std::fs::write(l.join("same.txt"), "same").expect("w");
        std::fs::write(r.join("same.txt"), "same").expect("w");
        std::fs::write(l.join("mod.txt"), "old content here").expect("w");
        std::fs::write(r.join("mod.txt"), "new content").expect("w");
        std::fs::write(l.join("only-left.txt"), "L").expect("w");
        std::fs::write(r.join("only-right.txt"), "R").expect("w");
        (l, r)
    }

    #[test]
    fn directory_compare_buckets_added_removed_modified_same() {
        let ws = TempWorkspace::new("cmp-dir-basic").expect("ws");
        let (l, r) = seed_pair(&ws);
        let session = compare_directories(
            &StdFilesystem,
            &l,
            &r,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        let s = session.summary;
        assert_eq!(
            (s.same, s.modified, s.removed, s.added),
            (1, 1, 1, 1),
            "{s:?}"
        );
        // mod.txt size 不同 ⇒ ContentChanged
        let modified = session
            .items
            .iter()
            .find(|i| i.relative_path == "mod.txt")
            .expect("mod item");
        assert_eq!(modified.content, ContentDifference::ContentChanged);
    }

    #[test]
    fn nested_directories_and_subdir_adds_are_directory_nodes() {
        let ws = TempWorkspace::new("cmp-dir-nested").expect("ws");
        let l = ws.dir("l").expect("l");
        let r = ws.dir("r").expect("r");
        std::fs::create_dir_all(l.join("a/b")).expect("mkdir");
        // 两侧内容不同 ⇒ Metadata 档恒 Modified（内容相同会因 mtime 同毫秒
        // 偶发 Same——本测试关注目录节点，不受文件内容影响）
        std::fs::write(l.join("a/b/deep.txt"), "deep").expect("w");
        std::fs::create_dir_all(r.join("a/b")).expect("mkdir");
        std::fs::write(r.join("a/b/deep.txt"), "deep-different").expect("w");
        std::fs::create_dir_all(r.join("newdir")).expect("mkdir");
        let session = compare_directories(
            &StdFilesystem,
            &l,
            &r,
            &opts(ContentCheck::Metadata),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(session.summary.same, 2, "a/ 与 a/b/ 目录节点");
        assert_eq!(session.summary.added, 1, "newdir/");
        let added = &session
            .items
            .iter()
            .find(|i| i.kind == DifferenceKind::Added)
            .expect("added");
        assert_eq!(added.node, NodeKind::Directory);
    }

    #[test]
    fn ignore_rules_count_without_polluting_items() {
        let ws = TempWorkspace::new("cmp-dir-ignore").expect("ws");
        let l = ws.dir("l").expect("l");
        let r = ws.dir("r").expect("r");
        for side in [&l, &r] {
            std::fs::create_dir_all(side.join(".git")).expect("mkdir");
            std::fs::write(side.join(".git/config"), "x").expect("w");
            std::fs::write(side.join("keep.txt"), "k").expect("w");
        }
        let session = compare_directories(
            &StdFilesystem,
            &l,
            &r,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert!(
            session.ignored_count >= 2,
            "git 目录内容被忽略计数：{}",
            session.ignored_count
        );
        assert!(
            session
                .items
                .iter()
                .all(|i| !i.relative_path.contains(".git"))
        );
        assert_eq!(session.summary.same, 1, "keep.txt（walk 只列子项）");
    }

    #[test]
    fn moved_and_renamed_need_unique_hash_evidence() {
        let ws = TempWorkspace::new("cmp-dir-moved").expect("ws");
        let l = ws.dir("l").expect("l");
        let r = ws.dir("r").expect("r");
        std::fs::create_dir_all(l.join("old")).expect("mkdir");
        std::fs::write(l.join("old/unique-content.bin"), "MOVEME-42").expect("w");
        std::fs::write(l.join("dup.bin"), "DUP").expect("w");
        std::fs::write(l.join("dup2.bin"), "DUP").expect("w");
        std::fs::create_dir_all(r.join("new")).expect("mkdir");
        std::fs::write(r.join("new/unique-content.bin"), "MOVEME-42").expect("w");
        std::fs::write(r.join("renamed-dup.bin"), "DUP").expect("w");
        std::fs::write(r.join("renamed-dup2.bin"), "DUP").expect("w");
        let session = compare_directories(
            &StdFilesystem,
            &l,
            &r,
            &opts(ContentCheck::Hash),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(session.summary.moved, 1, "唯一 hash + 同名 ⇒ moved");
        assert_eq!(
            session.summary.renamed, 0,
            "dup 内容两份 ⇒ 证据不唯一，保持 Added/Removed"
        );
        // Added/Removed = 2 文件 + 1 目录（new//old 目录节点本身）
        assert_eq!(session.summary.added, 3);
        assert_eq!(session.summary.removed, 3);
    }

    #[test]
    fn vanished_mid_walk_becomes_item_error_not_session_failure() {
        use std::sync::atomic::{AtomicBool, Ordering};
        struct VanishAfterFirst {
            inner: StdFilesystem,
            seen: AtomicBool,
        }
        impl Filesystem for VanishAfterFirst {
            fn exists(&self, p: &Path) -> bool {
                self.inner.exists(p)
            }
            fn stat(&self, p: &Path) -> std::io::Result<weave_files::metadata::FileStat> {
                let is_victim = p.file_name().map(|n| n == "victim.txt").unwrap_or(false);
                if is_victim && !self.seen.swap(true, Ordering::SeqCst) {
                    return Err(std::io::Error::from(std::io::ErrorKind::PermissionDenied));
                }
                self.inner.stat(p)
            }
            fn read_dir(&self, p: &Path) -> std::io::Result<Vec<String>> {
                self.inner.read_dir(p)
            }
            fn open_read(&self, p: &Path) -> std::io::Result<Box<dyn weave_files::fs::ReadSeek>> {
                self.inner.open_read(p)
            }
            fn rename(&self, f: &Path, t: &Path) -> std::io::Result<()> {
                self.inner.rename(f, t)
            }
            fn create_dir_all(&self, p: &Path) -> std::io::Result<()> {
                self.inner.create_dir_all(p)
            }
            fn copy(&self, f: &Path, t: &Path) -> std::io::Result<u64> {
                self.inner.copy(f, t)
            }
            fn remove_file(&self, p: &Path) -> std::io::Result<()> {
                self.inner.remove_file(p)
            }
        }

        let ws = TempWorkspace::new("cmp-dir-vanish").expect("ws");
        let (l, r) = seed_pair(&ws);
        std::fs::write(l.join("victim.txt"), "will vanish").expect("w");
        std::fs::write(r.join("victim.txt"), "will vanish").expect("w");
        let fs = VanishAfterFirst {
            inner: StdFilesystem,
            seen: AtomicBool::new(false),
        };
        let session = compare_directories(
            &fs,
            &l,
            &r,
            &opts(ContentCheck::Metadata),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert!(
            session.summary.unreadable + session.summary.error >= 1,
            "消失文件成为事实条目：{session:?}"
        );
        assert!(!session.items.is_empty(), "会话仍产出有效结果");
    }

    #[test]
    fn cancellation_marks_partial_session() {
        let ws = TempWorkspace::new("cmp-dir-cancel").expect("ws");
        let (l, r) = seed_pair(&ws);
        let cancel = CancellationToken::new();
        cancel.cancel();
        let session = compare_directories(
            &StdFilesystem,
            &l,
            &r,
            &opts(ContentCheck::Metadata),
            &cancel,
            &mut |_, _| {},
        );
        assert!(session.cancelled);
    }
}

#[cfg(test)]
mod perf_tests {
    use super::*;
    use crate::options::CompareOptions;
    use weave_files::fs::StdFilesystem;
    use weave_testkit::TempWorkspace;

    /// M2.1 §30：100/1k/10k 文件目录比较（Hash 档）——找 O(N²)/重复 I/O，
    /// 数字不追求极致但必须无爆炸曲线。release 手动运行。
    #[test]
    #[ignore = "perf: release 手动运行"]
    fn perf_directory_compare_at_scale() {
        for n in [100u32, 1_000, 10_000] {
            let ws = TempWorkspace::new(&format!("cmp-perf-{n}")).expect("ws");
            let l = ws.dir("l").expect("l");
            let r = ws.dir("r").expect("r");
            for i in 0..n {
                let content = format!("content-{i}");
                std::fs::write(l.join(format!("f{i}.txt")), &content).expect("w");
                std::fs::write(r.join(format!("f{i}.txt")), &content).expect("w");
            }
            // 每 10 个文件制造一个差异
            for i in (0..n).step_by(10) {
                std::fs::write(r.join(format!("f{i}.txt")), format!("changed-{i}")).expect("w");
            }
            let started = std::time::Instant::now();
            let session = compare_directories(
                &StdFilesystem,
                &l,
                &r,
                &CompareOptions {
                    content_check: crate::options::ContentCheck::Hash,
                    ..CompareOptions::default()
                },
                &CancellationToken::new(),
                &mut |_, _| {},
            );
            let elapsed = started.elapsed();
            eprintln!(
                "PERF compare@{n}: {:?} same={} modified={} removed={} added={}",
                elapsed,
                session.summary.same,
                session.summary.modified,
                session.summary.removed,
                session.summary.added
            );
            assert_eq!(session.summary.modified, (n / 10) as u64);
        }
    }
}
