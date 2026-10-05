//! 空间分析（M2.2）：扫描聚合 → 统计 → 清理候选 → 快照增长。
//!
//! - **自底向上聚合**（§7/§8）：目录大小由实际文件汇总；子目录聚合完成后
//!   折入父目录（按发现序逆序折叠，parent 必先于 child 发现 ⇒ 逆序即后序）。
//! - **内存控制**（§18/§48/§49）：不保留全量文件对象——Top-N 用有界堆，
//!   目录聚合按目录数计（100k 文件 ≈ 万级目录，可接受）；文件列表即弃。
//! - **一致性诚实**（§21）：扫描期间文件系统在变 ⇒ `consistency = bestEffort`，
//!   绝不声称瞬时快照。
//! - **容错**（§22/§23）：单目录不可读/单文件消失 ⇒ 计数事实，不升级会话失败。
//! - **符号链接**（§24）：目录型链接不跟随（M1 同源策略），无循环扫描。
//! - Physical Size（§6）：平台 API 未接 ⇒ 全程 Logical Size，不伪造物理占用。

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use weave_core::prelude::{CancellationToken, OperationId, WeaveError};

use crate::classify::{FileCategory, classify_by_extension};
use crate::fs::Filesystem;

/// 候选规则（§30：可调阈值；集中管理，不散落 UI）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateRules {
    /// Large File 阈值（默认 1 GiB）。
    pub large_file_bytes: u64,
    /// Old File 阈值（默认 180 天未修改）。
    pub old_file_age: Duration,
    /// Temporary File 扩展名（小写；可靠规则识别，§30 不声称全部可安全删）。
    pub temp_extensions: Vec<String>,
}

impl Default for CandidateRules {
    fn default() -> Self {
        Self {
            large_file_bytes: 1024 * 1024 * 1024,
            old_file_age: Duration::from_secs(180 * 24 * 3600),
            temp_extensions: ["tmp", "temp", "cache", "bak", "old"]
                .iter()
                .map(|s| s.to_string())
                .collect(),
        }
    }
}

/// 扫描选项。
#[derive(Debug, Clone)]
pub struct SpaceScanOptions {
    pub top_n: usize,
    pub rules: CandidateRules,
    /// 候选评估开关（纯分析场景可关）。
    pub candidates: bool,
}

impl Default for SpaceScanOptions {
    fn default() -> Self {
        Self {
            top_n: 100,
            rules: CandidateRules::default(),
            candidates: true,
        }
    }
}

/// 目录聚合节点（相对键 + 显示路径；父子关系用于钻取与折叠）。
/// logical_size / file_count / dir_count 均为**递归总量**（含全部后代），
/// 与目录大小"由实际文件汇总"的语义一致（§7/§8）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirNode {
    /// 相对显示路径（根 = ""）；分隔符 `/`。
    pub rel: String,
    /// 父目录 rel（根为 None）。
    pub parent: Option<String>,
    pub logical_size: u64,
    pub file_count: u64,
    pub dir_count: u64,
    /// 不可读子目录数 + 消失/不可读文件数（错误不吞，§22/§23）。
    pub unreadable: u64,
}

/// 最大文件条目（有界 Top-N 堆产出）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LargestFile {
    pub path: String,
    pub name: String,
    pub size: u64,
    pub extension: Option<String>,
    pub category: FileCategory,
    pub modified: Option<SystemTime>,
}

/// 文件类型统计（§10/§11：分类集中管理，复用 M1 classify）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypeStat {
    /// 小写扩展名（无扩展名 = ""，§12 不强行归类）。
    pub extension: String,
    pub category: FileCategory,
    pub count: u64,
    pub bytes: u64,
}

/// 清理候选（§29-§32：候选 ≠ 垃圾；必须携带可解释 Evidence）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CleanupCandidate {
    pub path: String,
    pub kind: CandidateKind,
    pub size: u64,
    pub modified: Option<SystemTime>,
    /// 为什么推荐（事实清单，§31）：如 "size = 8.4 GB" / "未修改 214 天"。
    pub reasons: Vec<String>,
    pub risk: Risk,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CandidateKind {
    LargeFile,
    OldFile,
    TempFile,
    EmptyDir,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Risk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confidence {
    Low,
    Medium,
    High,
}

/// 空间扫描会话（Read-only 事实集合；模型与 UI 解耦，§50）。
#[derive(Debug, Clone, PartialEq)]
pub struct SpaceScanSession {
    pub operation_id: OperationId,
    pub root: String,
    pub started_at: SystemTime,
    pub duration_ms: u64,
    pub cancelled: bool,
    /// §21：扫描期间文件系统在变 ⇒ bestEffort（不声称瞬时快照）。
    pub consistency: &'static str,
    pub logical_bytes: u64,
    pub file_count: u64,
    pub dir_count: u64,
    pub unreadable_dirs: u64,
    pub unreadable_files: u64,
    pub disappeared: u64,
    /// 全部目录聚合（含根；rels 可直接做钻取/子目录排行）。
    pub directories: Vec<DirNode>,
    pub largest_files: Vec<LargestFile>,
    pub type_stats: Vec<TypeStat>,
    pub candidates: Vec<CleanupCandidate>,
}

/// 有界 Top-N（§48：BinaryHeap 反向小顶堆，超界弹最小；禁止全量排序）。
struct TopN {
    cap: usize,
    heap: std::collections::BinaryHeap<std::cmp::Reverse<(u64, u64)>>,
    seq: u64,
    items: HashMap<u64, LargestFile>,
}

impl TopN {
    fn new(cap: usize) -> Self {
        Self {
            cap: cap.max(1),
            heap: std::collections::BinaryHeap::new(),
            seq: 0,
            items: HashMap::new(),
        }
    }

    fn push(&mut self, file: LargestFile, size: u64) {
        let id = self.seq;
        self.seq += 1;
        self.items.insert(id, file);
        self.heap.push(std::cmp::Reverse((size, id)));
        if self.heap.len() > self.cap {
            if let Some(std::cmp::Reverse((_, evicted))) = self.heap.pop() {
                self.items.remove(&evicted);
            }
        }
    }

    fn into_sorted(mut self) -> Vec<(u64, LargestFile)> {
        let mut pairs: Vec<(u64, u64)> = self.heap.into_iter().map(|r| r.0).collect();
        pairs.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        pairs
            .into_iter()
            .filter_map(|(size, id)| self.items.remove(&id).map(|f| (size, f)))
            .collect()
    }
}

/// 空间扫描（Read-only；取消在目录间与条目批次安全点生效）。
pub fn scan_space(
    fs: &dyn Filesystem,
    root: &Path,
    options: &SpaceScanOptions,
    cancel: &CancellationToken,
    progress: &mut dyn FnMut(u64, Option<u64>),
) -> SpaceScanSession {
    let started = SystemTime::now();
    let root_display = root.to_string_lossy().into_owned();

    // 聚合状态：发现序 = parent 先于 child（DFS 栈保证）
    let mut order: Vec<String> = vec![String::new()];
    let mut aggs: HashMap<String, DirAgg> = HashMap::new();
    aggs.insert(
        String::new(),
        DirAgg {
            rel: String::new(),
            parent: None,
            own_size: 0,
            own_files: 0,
            child_dirs: 0,
            unreadable: 0,
            display: root_display.clone(),
            empty: true,
        },
    );

    let mut top = TopN::new(options.top_n);
    let mut types: HashMap<(String, FileCategory), (u64, u64)> = HashMap::new();
    let mut candidates: Vec<CleanupCandidate> = Vec::new();
    let (mut file_count, mut unreadable_dirs, mut unreadable_files, mut disappeared) =
        (0u64, 0u64, 0u64, 0u64);
    let mut processed = 0u64;

    // DFS 栈：绝对路径 + rel 显示路径
    let mut stack: Vec<(PathBuf, String)> = vec![(root.to_path_buf(), String::new())];
    while let Some((dir_abs, dir_rel)) = stack.pop() {
        if cancel.is_cancelled() {
            break;
        }
        let names = match fs.read_dir(&dir_abs) {
            Ok(names) => names,
            Err(_) => {
                // 根不可读 ⇒ 整个会话无数据（诚实空结果 + unreadable 计数）
                if dir_rel.is_empty() {
                    unreadable_dirs += 1;
                } else {
                    let agg = aggs.get_mut(&dir_rel).expect("dir discovered");
                    agg.unreadable += 1;
                }
                continue;
            }
        };
        for name in names {
            if cancel.is_cancelled() {
                break;
            }
            processed += 1;
            if processed % 512 == 0 {
                progress(processed, None);
            }
            let child_abs = dir_abs.join(&name);
            let stat = match fs.stat(&child_abs) {
                Ok(s) => s,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    // §23：扫描中消失 ⇒ Disappeared 事实
                    disappeared += 1;
                    continue;
                }
                Err(_) => {
                    // §22：权限/占用等 ⇒ Unreadable 事实，挂到所在目录
                    unreadable_files += 1;
                    if let Some(agg) = aggs.get_mut(&dir_rel) {
                        agg.unreadable += 1;
                    }
                    continue;
                }
            };
            let child_rel = if dir_rel.is_empty() {
                name.clone()
            } else {
                format!("{dir_rel}/{name}")
            };
            match stat.kind {
                weave_core::prelude::FileKind::Directory => {
                    // 目录型链接不是 FileKind::Directory（lstat 语义）⇒ 不跟随，无循环
                    aggs.insert(
                        child_rel.clone(),
                        DirAgg {
                            rel: child_rel.clone(),
                            parent: Some(dir_rel.clone()),
                            own_size: 0,
                            own_files: 0,
                            child_dirs: 0,
                            unreadable: 0,
                            display: child_abs.to_string_lossy().into_owned(),
                            empty: true,
                        },
                    );
                    if let Some(agg) = aggs.get_mut(&dir_rel) {
                        agg.child_dirs += 1;
                        agg.empty = false;
                    }
                    order.push(child_rel.clone());
                    stack.push((child_abs, child_rel));
                }
                _ => {
                    // 普通文件/symlink 条目：按自身 stat 计入逻辑大小
                    let parent = aggs.get_mut(&dir_rel).expect("dir discovered");
                    parent.own_size += stat.size;
                    parent.own_files += 1;
                    parent.empty = false;
                    file_count += 1;
                    // 类型统计（§10-§12：无扩展名 = ""，不强行归类）
                    let ext = extension_of(&name);
                    let category = classify_by_extension(ext.as_deref()).category;
                    let entry = types
                        .entry((ext.unwrap_or_default(), category))
                        .or_insert((0, 0));
                    entry.0 += 1;
                    entry.1 += stat.size;
                    // Top-N（有界堆）
                    top.push(
                        LargestFile {
                            path: child_abs.to_string_lossy().into_owned(),
                            name,
                            size: stat.size,
                            extension: extension_of(&child_rel),
                            category,
                            modified: stat.modified,
                        },
                        stat.size,
                    );
                    // 候选评估（§29-§32）
                    if options.candidates {
                        if let Some(c) = evaluate_file_candidate(
                            &child_abs,
                            stat.size,
                            stat.modified,
                            &options.rules,
                        ) {
                            candidates.push(c);
                        }
                    }
                }
            }
        }
    }

    // ── 自底向上折叠：发现序逆序 = child 先于 parent（O(n)，无 O(n²) 排序）──
    for rel in order.iter().rev() {
        let (parent, own) = {
            let agg = &aggs[rel];
            (agg.parent.clone(), agg.clone())
        };
        if let Some(parent_rel) = parent
            && let Some(p) = aggs.get_mut(&parent_rel)
        {
            p.own_size += own.own_size;
            p.own_files += own.own_files;
            p.child_dirs += own.child_dirs;
            p.unreadable += own.unreadable;
            if !own.empty {
                p.empty = false;
            }
        }
    }
    // 空目录候选（折叠后仍 empty 的目录节点；跳过根）
    if options.candidates && !cancel.is_cancelled() {
        for rel in order.iter().skip(1) {
            let agg = &aggs[rel];
            if agg.empty {
                candidates.push(CleanupCandidate {
                    path: agg.display.clone(),
                    kind: CandidateKind::EmptyDir,
                    size: 0,
                    modified: None,
                    reasons: vec!["目录为空（无文件且无子目录）".to_string()],
                    risk: Risk::Low,
                    confidence: Confidence::High,
                });
            }
        }
    }

    let finished = SystemTime::now();
    let duration_ms = finished
        .duration_since(started)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let cancelled = cancel.is_cancelled();

    // 目录节点展开（含聚合后总数）
    let mut directories: Vec<DirNode> = aggs
        .into_iter()
        .map(|(_, agg)| DirNode {
            rel: agg.rel,
            parent: agg.parent,
            logical_size: agg.own_size,
            file_count: agg.own_files,
            dir_count: agg.child_dirs,
            unreadable: agg.unreadable,
        })
        .collect();
    directories.sort_by(|a, b| a.rel.cmp(&b.rel));
    let dir_count = directories.len().saturating_sub(1) as u64;
    let logical_bytes = directories
        .iter()
        .find(|d| d.rel.is_empty())
        .map(|d| d.logical_size)
        .unwrap_or(0);

    let mut type_stats: Vec<TypeStat> = types
        .into_iter()
        .map(|((extension, category), (count, bytes))| TypeStat {
            extension,
            category,
            count,
            bytes,
        })
        .collect();
    type_stats.sort_by(|a, b| b.bytes.cmp(&a.bytes).then(a.extension.cmp(&b.extension)));

    candidates.sort_by(|a, b| b.size.cmp(&a.size));
    candidates.truncate(options.top_n.max(100));

    progress(processed, Some(processed));

    SpaceScanSession {
        operation_id: OperationId::generate(),
        root: root_display,
        started_at: started,
        duration_ms,
        cancelled,
        consistency: "bestEffort",
        logical_bytes,
        file_count,
        dir_count,
        unreadable_dirs,
        unreadable_files,
        disappeared,
        directories,
        largest_files: top.into_sorted().into_iter().map(|(_, f)| f).collect(),
        type_stats,
        candidates,
    }
}

/// 目录聚合中间态（折叠前持有 own 事实）。
#[derive(Debug, Clone)]
struct DirAgg {
    rel: String,
    parent: Option<String>,
    own_size: u64,
    own_files: u64,
    child_dirs: u64,
    unreadable: u64,
    display: String,
    empty: bool,
}

fn extension_of(name: &str) -> Option<String> {
    let base = name.rsplit(['/', '\\']).next().unwrap_or(name);
    let ext = base.rsplit_once('.')?.1.to_lowercase();
    if ext.is_empty() { None } else { Some(ext) }
}

/// 文件候选评估（§31：reasons = 事实清单，绝不黑盒 "recommended"）。
fn evaluate_file_candidate(
    path: &Path,
    size: u64,
    modified: Option<SystemTime>,
    rules: &CandidateRules,
) -> Option<CleanupCandidate> {
    let now = SystemTime::now();
    let age = modified.map(|m| now.duration_since(m).unwrap_or(Duration::ZERO));
    let mut reasons: Vec<String> = Vec::new();
    let display = path.to_string_lossy().into_owned();
    let ext = extension_of(&display);

    if size >= rules.large_file_bytes {
        reasons.push(format!("size = {} bytes（≥ large 阈值）", size));
    }
    if let Some(age) = age {
        if age >= rules.old_file_age {
            reasons.push(format!("未修改 {} 天（≥ old 阈值）", age.as_secs() / 86400));
        }
    }
    let is_temp = matches!(&ext, Some(e) if rules.temp_extensions.iter().any(|t| t == e));
    if is_temp {
        reasons.push(format!(
            "扩展名 .{} 属临时文件表（不保证可安全删除）",
            ext.as_ref().expect("checked")
        ));
    }
    if reasons.is_empty() {
        return None;
    }
    // 风险（§32）：绝不按扩展名断言安全——temp 系 Medium；大且旧 Medium；单一大 High 保守
    let risk = if is_temp || (reasons.len() >= 2) {
        Risk::Medium
    } else {
        Risk::High
    };
    // 置信度：多事实重叠 ⇒ Medium（第一版不给 High——不假装智能）
    let confidence = if reasons.len() >= 2 {
        Confidence::Medium
    } else {
        Confidence::Low
    };
    let kind = if is_temp {
        CandidateKind::TempFile
    } else if reasons.len() >= 2 {
        CandidateKind::LargeFile
    } else if reasons[0].starts_with("size") {
        CandidateKind::LargeFile
    } else {
        CandidateKind::OldFile
    };
    Some(CleanupCandidate {
        path: display,
        kind,
        size,
        modified,
        reasons,
        risk,
        confidence,
    })
}

// ── 快照与增长（§26-§28：本地历史快照，不过度设计）──

/// 空间快照（扫描后落一条；JSON 列表持久化由服务层负责）。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SpaceSnapshot {
    pub timestamp: SystemTime,
    pub root: String,
    pub logical_bytes: u64,
    pub file_count: u64,
}

/// 增长对比（§27：Previous = 0 ⇒ growth_pct = None，绝不 Infinity）。
#[derive(Debug, Clone, PartialEq)]
pub struct Growth {
    pub previous_bytes: u64,
    pub current_bytes: u64,
    pub delta_bytes: i64,
    /// 增长百分比（previous = 0 时 None）。
    pub growth_pct: Option<f64>,
}

pub fn growth(current: &SpaceSnapshot, previous: &SpaceSnapshot) -> Growth {
    let delta = current.logical_bytes as i64 - previous.logical_bytes as i64;
    let growth_pct = if previous.logical_bytes == 0 {
        None
    } else {
        Some(delta as f64 * 100.0 / previous.logical_bytes as f64)
    };
    Growth {
        previous_bytes: previous.logical_bytes,
        current_bytes: current.logical_bytes,
        delta_bytes: delta,
        growth_pct,
    }
}

/// 快照存储：JSON 列表 + temp+rename 原子写 + 容量有界（最旧淘汰）。
pub struct SnapshotStore {
    path: PathBuf,
}

pub const MAX_SNAPSHOTS: usize = 500;

impl SnapshotStore {
    pub fn at(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Vec<SpaceSnapshot> {
        let Ok(raw) = std::fs::read_to_string(&self.path) else {
            return Vec::new();
        };
        serde_json::from_str(&raw).unwrap_or_default()
    }

    pub fn append(&self, snapshot: &SpaceSnapshot) -> Result<(), WeaveError> {
        let mut list = self.load();
        list.insert(0, snapshot.clone());
        list.truncate(MAX_SNAPSHOTS);
        let json = serde_json::to_string_pretty(&list)
            .map_err(|e| WeaveError::internal("space.serializeFailed", format!("{e}")))?;
        let tmp = self.path.with_extension("tmp");
        std::fs::write(&tmp, json)
            .and_then(|()| std::fs::rename(&tmp, &self.path))
            .map_err(|e| {
                WeaveError::io(
                    "space.snapshotWriteFailed",
                    format!("snapshot write failed: {e}"),
                )
            })?;
        Ok(())
    }

    /// 同一根的最新一条既有快照（增长对比的 previous；不含本次）。
    pub fn latest_for(&self, root: &str, before: SystemTime) -> Option<SpaceSnapshot> {
        self.load()
            .into_iter()
            .find(|s| s.root == root && s.timestamp < before)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::StdFilesystem;
    use weave_testkit::TempWorkspace;

    fn opts() -> SpaceScanOptions {
        SpaceScanOptions::default()
    }

    #[test]
    fn empty_dir_session_is_honest_zero() {
        let ws = TempWorkspace::new("space-empty").expect("ws");
        let session = scan_space(
            &StdFilesystem,
            ws.path(),
            &opts(),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(
            (session.logical_bytes, session.file_count, session.dir_count),
            (0, 0, 0)
        );
        assert_eq!(session.consistency, "bestEffort");
        // 根自身作为目录节点存在
        assert_eq!(session.directories.len(), 1);
    }

    #[test]
    fn single_file_aggregates_to_root() {
        let ws = TempWorkspace::new("space-one").expect("ws");
        std::fs::write(ws.path().join("a.bin"), vec![0u8; 1024]).expect("w");
        let session = scan_space(
            &StdFilesystem,
            ws.path(),
            &opts(),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(session.logical_bytes, 1024);
        assert_eq!(session.file_count, 1);
        assert_eq!(session.largest_files.len(), 1);
        assert_eq!(session.largest_files[0].size, 1024);
    }

    #[test]
    fn nested_tree_folds_bottom_up_without_double_counting() {
        let ws = TempWorkspace::new("space-nested").expect("ws");
        let sub = ws.dir("sub").expect("dir");
        std::fs::write(ws.path().join("top.txt"), vec![0u8; 100]).expect("w");
        std::fs::write(sub.join("inner.txt"), vec![0u8; 200]).expect("w");
        let deep = ws.dir("sub/deep").expect("dir");
        std::fs::write(deep.join("deep.txt"), vec![0u8; 400]).expect("w");

        let session = scan_space(
            &StdFilesystem,
            ws.path(),
            &opts(),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(session.logical_bytes, 700, "根 = 全部文件之和，无重复计数");
        let root = &session.directories[0];
        let sub_node = session
            .directories
            .iter()
            .find(|d| d.rel == "sub")
            .expect("sub node");
        let deep_node = session
            .directories
            .iter()
            .find(|d| d.rel == "sub/deep")
            .expect("deep node");
        assert_eq!(
            (
                root.logical_size,
                sub_node.logical_size,
                deep_node.logical_size
            ),
            (700, 600, 400)
        );
        // 递归总量语义：sub = 自有 1 + deep 1 = 2
        assert_eq!((sub_node.file_count, deep_node.file_count), (2, 1));
        assert_eq!(sub_node.parent.as_deref(), Some(""), "sub 的父 = 根");
        assert_eq!(deep_node.parent.as_deref(), Some("sub"));
    }

    #[test]
    fn top_n_is_bounded_not_full_sort() {
        let ws = TempWorkspace::new("space-topn").expect("ws");
        for i in 0..50u32 {
            std::fs::write(
                ws.path().join(format!("f{i:02}.txt")),
                vec![0u8; i as usize + 1],
            )
            .expect("w");
        }
        let o = SpaceScanOptions { top_n: 5, ..opts() };
        let session = scan_space(
            &StdFilesystem,
            ws.path(),
            &o,
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        assert_eq!(session.largest_files.len(), 5);
        // 降序
        for w in session.largest_files.windows(2) {
            assert!(w[0].size >= w[1].size);
        }
        assert_eq!(session.largest_files[0].size, 50);
    }

    #[test]
    fn type_stats_group_by_extension_with_unknown_bucket() {
        let ws = TempWorkspace::new("space-types").expect("ws");
        std::fs::write(ws.path().join("a.png"), vec![0u8; 10]).expect("w");
        std::fs::write(ws.path().join("b.PNG"), vec![0u8; 20]).expect("w");
        std::fs::write(ws.path().join("c.mp4"), vec![0u8; 30]).expect("w");
        std::fs::write(ws.path().join("noext"), vec![0u8; 40]).expect("w");
        std::fs::write(ws.path().join("weird.foo"), vec![0u8; 50]).expect("w");
        let session = scan_space(
            &StdFilesystem,
            ws.path(),
            &opts(),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        let png = session
            .type_stats
            .iter()
            .find(|t| t.extension == "png")
            .expect("png bucket");
        assert_eq!((png.count, png.bytes), (2, 30));
        assert_eq!(png.category, FileCategory::Image);
        let none = session
            .type_stats
            .iter()
            .find(|t| t.extension.is_empty())
            .expect("no-extension bucket");
        assert_eq!(none.count, 1);
        let unknown = session
            .type_stats
            .iter()
            .find(|t| t.extension == "foo")
            .expect("unknown ext kept honestly");
        assert_eq!(unknown.category, FileCategory::Unknown);
    }

    #[test]
    fn candidates_carry_reasons_not_verdicts() {
        let ws = TempWorkspace::new("space-cand").expect("ws");
        let old = std::time::SystemTime::now() - Duration::from_secs(200 * 24 * 3600);
        let big_old = ws.path().join("big-old.zip");
        std::fs::write(&big_old, vec![0u8; 1024]).expect("w");
        set_mtime(&big_old, old);
        // 小而新 ⇒ 非候选
        std::fs::write(ws.path().join("small-new.txt"), b"hi").expect("w");

        // 自定义阈值（默认 large = 1 GiB；测试用 512 B 触发双事实重叠）
        let o = SpaceScanOptions {
            candidates: true,
            rules: CandidateRules {
                large_file_bytes: 512,
                ..CandidateRules::default()
            },
            ..opts()
        };
        let session = scan_space(
            &StdFilesystem,
            ws.path(),
            &o,
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        let cand = session
            .candidates
            .iter()
            .find(|c| c.path.ends_with("big-old.zip"))
            .expect("candidate");
        // 事实 Evidence（§31）：size + age 双 reason；风险不因扩展名断言安全
        assert!(cand.reasons.len() >= 2, "{:?}", cand.reasons);
        assert!(cand.reasons.iter().any(|r| r.starts_with("size")));
        assert!(cand.reasons.iter().any(|r| r.contains("天")));
        assert_eq!(cand.confidence, Confidence::Medium);
        assert!(
            session
                .candidates
                .iter()
                .all(|c| !c.path.ends_with("small-new.txt"))
        );
    }

    #[test]
    fn empty_dirs_are_candidates_but_never_auto_deleted() {
        let ws = TempWorkspace::new("space-emptydir").expect("ws");
        ws.dir("empty-a").expect("dir");
        ws.dir("empty-a/nested-empty").expect("dir");
        std::fs::write(ws.path().join("x.txt"), b"x").expect("w");
        let session = scan_space(
            &StdFilesystem,
            ws.path(),
            &opts(),
            &CancellationToken::new(),
            &mut |_, _| {},
        );
        let empties: Vec<_> = session
            .candidates
            .iter()
            .filter(|c| c.kind == CandidateKind::EmptyDir)
            .collect();
        // 严格语义（无文件且无子目录）：nested-empty 是空目录；
        // empty-a 含一个子目录 ⇒ 不算空（含空子目录的目录是否级联 = P2 判断题）
        assert_eq!(empties.len(), 1, "{:?}", session.candidates);
        assert!(empties[0].path.ends_with("nested-empty"));
        assert!(
            empties
                .iter()
                .all(|c| c.risk == Risk::Low && c.confidence == Confidence::High)
        );
    }

    #[test]
    fn growth_handles_zero_previous_without_infinity() {
        let now = SystemTime::now();
        let prev = SpaceSnapshot {
            timestamp: now,
            root: "R".into(),
            logical_bytes: 0,
            file_count: 0,
        };
        let cur = SpaceSnapshot {
            timestamp: now,
            root: "R".into(),
            logical_bytes: 500,
            file_count: 1,
        };
        let g = growth(&cur, &prev);
        assert_eq!(g.delta_bytes, 500);
        assert_eq!(g.growth_pct, None, "prev=0 ⇒ 无百分比，绝不 Infinity");
        let g2 = growth(&prev, &cur);
        assert!(g2.growth_pct.unwrap() < 0.0);
    }

    #[test]
    fn snapshot_store_round_trips_and_caps() {
        let ws = TempWorkspace::new("space-snap").expect("ws");
        let store = SnapshotStore::at(ws.path().join("snapshots.json"));
        assert!(store.load().is_empty());
        for i in 0..12u64 {
            store
                .append(&SpaceSnapshot {
                    timestamp: SystemTime::UNIX_EPOCH + Duration::from_secs(i),
                    root: "R".into(),
                    logical_bytes: i * 100,
                    file_count: i,
                })
                .expect("append");
        }
        let list = store.load();
        assert_eq!(list.len(), 12);
        assert_eq!(list[0].logical_bytes, 1100, "最新在前");
        // latest_for：严格早于给定时刻
        let mid = SystemTime::UNIX_EPOCH + Duration::from_secs(6);
        let prev = store.latest_for("R", mid).expect("previous");
        assert_eq!(prev.logical_bytes, 500);
    }

    fn set_mtime(path: &Path, t: SystemTime) {
        // Windows 上 set_modified 需要写句柄（只读句柄 ⇒ 拒绝访问）
        let f = std::fs::OpenOptions::new()
            .write(true)
            .open(path)
            .expect("open for write");
        f.set_modified(t).expect("set_modified");
    }
}
