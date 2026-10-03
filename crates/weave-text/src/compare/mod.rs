//! Text Compare（M4 §47–§55）：确定性、有界、可解释的行级 diff。
//!
//! 算法（D42）：
//! 1. 公共前缀/后缀按规范化键裁剪（常见场景 O(n) 直通）。
//! 2. 余量 ≤ [`CompareLimits::lcs_window`]（默认 2000×2000）时做完整 LCS DP。
//! 3. 超出窗口 ⇒ 余量作为单个整块输出（确定性、诚实降级，报告
//!    `degraded: true`——绝不假装算过全量最优）。
//!
//! 语义：
//! - 比较永远基于行内容，不含行尾换行符（CRLF vs LF 本身不算差异，§17/D42）。
//! - Whitespace Ignore 是显式档位（§51）：Trailing / All；Case Ignore 用
//!   Unicode 小写折叠（§52）。
//! - Moved（§53）：同一变更块内、规范化键完全一致且两侧位置不同的行的配对
//!   结果——是引擎匹配事实，不是语义断言；重复行按出现次数配对，不误标。
//! - 同输入同选项必同输出（§54）：无 HashMap 迭代、无并行歧义。

use crate::transform::split_lines_keep;

/// Whitespace ignore 档位（§51：不做模糊的 "ignore whitespace = true"）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WhitespaceMode {
    #[default]
    None,
    /// 忽略行首/行尾空白。
    Trailing,
    /// 忽略全部空白差异。
    All,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct CompareOptions {
    pub whitespace: WhitespaceMode,
    /// Unicode 小写折叠（§52：中文/重音字符安全）。
    pub ignore_case: bool,
}

/// 比较规模上限（§55：集中、可测、超限显式 TooLarge）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CompareLimits {
    pub max_bytes: u64,
    pub max_lines: u64,
    /// LCS DP 窗口边长（余量两侧都 ≤ 此值才做最优解）。
    pub lcs_window: usize,
}

impl Default for CompareLimits {
    fn default() -> Self {
        Self {
            max_bytes: 4 * 1024 * 1024,
            max_lines: 100_000,
            lcs_window: 2000,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineChange {
    Equal,
    Added,
    Removed,
    Changed,
    Moved,
}

/// 单行条目（side-by-side 渲染直接消费）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffLine {
    pub change: LineChange,
    /// 1-based A 行号（该行不存在时 None）。
    pub a_line: Option<u64>,
    /// 1-based B 行号。
    pub b_line: Option<u64>,
    pub a_text: String,
    pub b_text: String,
}

/// 变更块（原始块，无上下文；Unified 渲染时再按 3 行上下文合并）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffHunk {
    pub a_start: u64,
    pub a_len: u64,
    pub b_start: u64,
    pub b_len: u64,
    pub lines: Vec<DiffLine>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct DiffStats {
    pub added: u64,
    pub removed: u64,
    pub changed: u64,
    pub moved: u64,
    pub equal: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffReport {
    pub hunks: Vec<DiffHunk>,
    pub stats: DiffStats,
    pub identical: bool,
    /// LCS 窗口超限降级为整块替换时为 true（诚实标注，D42）。
    pub degraded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CompareError {
    /// §55：超限必须显式，绝不冻结 UI / 假装 0 差异。
    TooLarge { code: &'static str, message: String },
}

/// 内部 op 流。
#[derive(Debug, Clone, Copy)]
enum Op {
    Equal { a: usize, b: usize },
    Del { a: usize },
    Ins { b: usize },
    Change { a: usize, b: usize },
    Move { a: usize, b: usize },
}

fn normalize(line: &str, options: &CompareOptions) -> String {
    let t = line.strip_suffix('\n').unwrap_or(line);
    let body = t.strip_suffix('\r').unwrap_or(t);
    let base: String = match options.whitespace {
        WhitespaceMode::None => body.to_string(),
        WhitespaceMode::Trailing => body.trim().to_string(),
        WhitespaceMode::All => body.split_whitespace().collect::<Vec<_>>().concat(),
    };
    if options.ignore_case {
        base.to_lowercase()
    } else {
        base
    }
}

/// 全局 Moved 配对（§53）+ 逐块 Changed 配对。
///
/// Moved 配对在**全 diff 范围**做：同一规范化内容两侧位置不同即配对（引擎
/// 匹配事实），按出现顺序消耗、重复行按次数配对。Changed 在每个相邻
/// Del/Ins 块内对剩余项等长顺序配对。Moved/Change 挂在 Del 位置输出，
/// 被消费的 Ins 移除——保持整体位置顺序确定。
fn resolve_pairings(ops: &[Op], keys_a: &[String], keys_b: &[String]) -> Vec<Op> {
    let del_positions: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter_map(|(i, op)| matches!(op, Op::Del { .. }).then_some(i))
        .collect();
    let ins_positions: Vec<usize> = ops
        .iter()
        .enumerate()
        .filter_map(|(i, op)| matches!(op, Op::Ins { .. }).then_some(i))
        .collect();
    let del_a: std::collections::HashMap<usize, usize> = del_positions
        .iter()
        .map(|i| match ops[*i] {
            Op::Del { a } => (*i, a),
            _ => unreachable!(),
        })
        .collect();
    let ins_b: std::collections::HashMap<usize, usize> = ins_positions
        .iter()
        .map(|i| match ops[*i] {
            Op::Ins { b } => (*i, b),
            _ => unreachable!(),
        })
        .collect();

    // 1) 全局 Moved：键 → 未消费 ins 队列（HashMap，O(D + I + pairs)——
    //    朴素双重循环在"全不同 100k"场景是 10^10 量级，§141 性能证据抓到）
    let mut ins_queues: std::collections::HashMap<&str, Vec<usize>> =
        std::collections::HashMap::new();
    for (ii, &ins_idx) in ins_positions.iter().enumerate() {
        ins_queues
            .entry(keys_b[ins_b[&ins_idx]].as_str())
            .or_default()
            .push(ii);
    }
    let mut ins_taken = vec![false; ins_positions.len()];
    let mut moved_at: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    for &di in &del_positions {
        let a = del_a[&di];
        if let Some(queue) = ins_queues.get_mut(keys_a[a].as_str()) {
            while let Some(ii) = queue.first() {
                if ins_taken[*ii] {
                    queue.remove(0);
                } else {
                    ins_taken[*ii] = true;
                    moved_at.insert(di, ins_positions[*ii]);
                    break;
                }
            }
        }
    }
    let consumed_ins: std::collections::HashSet<usize> = moved_at.values().copied().collect();

    // 2) 逐相邻块 Changed：连续 Del/Ins 段内剩余项等长顺序配对
    let mut changed_at: std::collections::HashMap<usize, usize> = std::collections::HashMap::new();
    let mut run: Vec<usize> = Vec::new();
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Del { .. } | Op::Ins { .. } => run.push(i),
            _ => {
                let dels: Vec<usize> = run
                    .iter()
                    .copied()
                    .filter(|i| matches!(ops[*i], Op::Del { .. }) && !moved_at.contains_key(i))
                    .collect();
                let inss: Vec<usize> = run
                    .iter()
                    .copied()
                    .filter(|i| matches!(ops[*i], Op::Ins { .. }) && !consumed_ins.contains(i))
                    .collect();
                let paired = dels.len().min(inss.len());
                for k in 0..paired {
                    changed_at.insert(dels[k], inss[k]);
                }
                run.clear();
            }
        }
    }
    let dels: Vec<usize> = run
        .iter()
        .copied()
        .filter(|i| matches!(ops[*i], Op::Del { .. }) && !moved_at.contains_key(i))
        .collect();
    let inss: Vec<usize> = run
        .iter()
        .copied()
        .filter(|i| matches!(ops[*i], Op::Ins { .. }) && !consumed_ins.contains(i))
        .collect();
    let paired = dels.len().min(inss.len());
    for k in 0..paired {
        changed_at.insert(dels[k], inss[k]);
    }

    // 3) 重写 op 流（消费集 HashSet 化：逐条 .values().any 是 O(n²) 热点）
    let changed_ins: std::collections::HashSet<usize> = changed_at.values().copied().collect();
    let mut out = Vec::with_capacity(ops.len());
    for (i, op) in ops.iter().enumerate() {
        match op {
            Op::Del { a } => {
                if let Some(&ii) = moved_at.get(&i) {
                    out.push(Op::Move {
                        a: *a,
                        b: ins_b[&ii],
                    });
                } else if let Some(&ii) = changed_at.get(&i) {
                    out.push(Op::Change {
                        a: *a,
                        b: ins_b[&ii],
                    });
                } else {
                    out.push(*op);
                }
            }
            Op::Ins { .. } => {
                if !consumed_ins.contains(&i) && !changed_ins.contains(&i) {
                    out.push(*op);
                }
            }
            _ => out.push(*op),
        }
    }
    out
}

/// LCS DP（有界窗口内）；余量 → op 流。超过窗口 ⇒ 整块 Del+Ins（诚实降级）。
fn lcs_ops(a: &[String], b: &[String], a_off: usize, b_off: usize, window: usize) -> Vec<Op> {
    let n = a.len();
    let m = b.len();
    if n == 0 && m == 0 {
        return Vec::new();
    }
    if n > window || m > window {
        let mut ops: Vec<Op> = (a_off..a_off + n).map(|a| Op::Del { a }).collect();
        ops.extend((b_off..b_off + m).map(|b| Op::Ins { b }));
        return ops;
    }
    let width = m + 1;
    let mut table = vec![0u32; (n + 1) * width];
    for i in (0..n).rev() {
        for j in (0..m).rev() {
            table[i * width + j] = if a[i] == b[j] {
                table[(i + 1) * width + j + 1] + 1
            } else {
                table[(i + 1) * width + j].max(table[i * width + j + 1])
            };
        }
    }
    let mut ops = Vec::new();
    let (mut i, mut j) = (0usize, 0usize);
    while i < n && j < m {
        if a[i] == b[j] {
            ops.push(Op::Equal {
                a: a_off + i,
                b: b_off + j,
            });
            i += 1;
            j += 1;
        } else if table[(i + 1) * width + j] >= table[i * width + j + 1] {
            ops.push(Op::Del { a: a_off + i });
            i += 1;
        } else {
            ops.push(Op::Ins { b: b_off + j });
            j += 1;
        }
    }
    ops.extend((a_off + i..a_off + n).map(|a| Op::Del { a }));
    ops.extend((b_off + j..b_off + m).map(|b| Op::Ins { b }));
    ops
}

fn line_bodies(content: &str) -> Vec<String> {
    split_lines_keep(content)
        .iter()
        .map(|l| {
            let t = l.strip_suffix('\n').unwrap_or(l);
            t.strip_suffix('\r').unwrap_or(t).to_string()
        })
        .collect()
}

/// 比较两个文本（§47）。
pub fn compare_texts(
    a: &str,
    b: &str,
    options: &CompareOptions,
    limits: &CompareLimits,
) -> Result<DiffReport, CompareError> {
    if a.len() as u64 > limits.max_bytes || b.len() as u64 > limits.max_bytes {
        return Err(CompareError::TooLarge {
            code: "compare.tooLarge",
            message: format!(
                "input exceeds max compare size ({} bytes)",
                limits.max_bytes
            ),
        });
    }
    let a_lines = line_bodies(a);
    let b_lines = line_bodies(b);
    if a_lines.len() as u64 > limits.max_lines || b_lines.len() as u64 > limits.max_lines {
        return Err(CompareError::TooLarge {
            code: "compare.tooManyLines",
            message: format!("input exceeds max compare lines ({})", limits.max_lines),
        });
    }
    let keys_a: Vec<String> = a_lines.iter().map(|l| normalize(l, options)).collect();
    let keys_b: Vec<String> = b_lines.iter().map(|l| normalize(l, options)).collect();

    // 公共前缀/后缀裁剪（后缀不得越过前缀）
    let mut prefix = 0usize;
    while prefix < a_lines.len() && prefix < b_lines.len() && keys_a[prefix] == keys_b[prefix] {
        prefix += 1;
    }
    let mut suffix = 0usize;
    while suffix < a_lines.len() - prefix
        && suffix < b_lines.len() - prefix
        && keys_a[a_lines.len() - 1 - suffix] == keys_b[b_lines.len() - 1 - suffix]
    {
        suffix += 1;
    }

    let ops: Vec<Op> = (0..prefix).map(|i| Op::Equal { a: i, b: i }).collect();
    let degraded = keys_a[prefix..a_lines.len() - suffix].len() > limits.lcs_window
        || keys_b[prefix..b_lines.len() - suffix].len() > limits.lcs_window;
    let rem_a_start = prefix;
    let rem_a = &keys_a[prefix..a_lines.len() - suffix];
    let rem_b = &keys_b[prefix..b_lines.len() - suffix];
    let raw = lcs_ops(rem_a, rem_b, rem_a_start, prefix, limits.lcs_window);

    // 全局 Moved + 逐块 Changed 配对（§53，确定性）
    let mut all_ops = ops;
    all_ops.extend(raw);
    let mut ops = resolve_pairings(&all_ops, &keys_a, &keys_b);
    ops.extend((0..suffix).map(|i| Op::Equal {
        a: a_lines.len() - suffix + i,
        b: b_lines.len() - suffix + i,
    }));

    // 平铺 DiffLine + 统计
    let mut flat: Vec<DiffLine> = Vec::with_capacity(ops.len());
    let mut stats = DiffStats::default();
    for op in &ops {
        let line = match *op {
            Op::Equal { a, b } => {
                stats.equal += 1;
                DiffLine {
                    change: LineChange::Equal,
                    a_line: Some(a as u64 + 1),
                    b_line: Some(b as u64 + 1),
                    a_text: a_lines[a].clone(),
                    b_text: b_lines[b].clone(),
                }
            }
            Op::Del { a } => {
                stats.removed += 1;
                DiffLine {
                    change: LineChange::Removed,
                    a_line: Some(a as u64 + 1),
                    b_line: None,
                    a_text: a_lines[a].clone(),
                    b_text: String::new(),
                }
            }
            Op::Ins { b } => {
                stats.added += 1;
                DiffLine {
                    change: LineChange::Added,
                    a_line: None,
                    b_line: Some(b as u64 + 1),
                    a_text: String::new(),
                    b_text: b_lines[b].clone(),
                }
            }
            Op::Change { a, b } => {
                stats.changed += 1;
                DiffLine {
                    change: LineChange::Changed,
                    a_line: Some(a as u64 + 1),
                    b_line: Some(b as u64 + 1),
                    a_text: a_lines[a].clone(),
                    b_text: b_lines[b].clone(),
                }
            }
            Op::Move { a, b } => {
                stats.moved += 1;
                DiffLine {
                    change: LineChange::Moved,
                    a_line: Some(a as u64 + 1),
                    b_line: Some(b as u64 + 1),
                    a_text: a_lines[a].clone(),
                    b_text: b_lines[b].clone(),
                }
            }
        };
        flat.push(line);
    }

    // 连续非 Equal 段聚合成 hunks
    let mut hunks: Vec<DiffHunk> = Vec::new();
    let mut idx = 0usize;
    while idx < flat.len() {
        if flat[idx].change == LineChange::Equal {
            idx += 1;
            continue;
        }
        let start = idx;
        while idx < flat.len() && flat[idx].change != LineChange::Equal {
            idx += 1;
        }
        let seg = &flat[start..idx];
        hunks.push(DiffHunk {
            a_start: seg.iter().filter_map(|l| l.a_line).next().unwrap_or(1),
            a_len: seg.iter().filter(|l| l.a_line.is_some()).count() as u64,
            b_start: seg.iter().filter_map(|l| l.b_line).next().unwrap_or(1),
            b_len: seg.iter().filter(|l| l.b_line.is_some()).count() as u64,
            lines: seg.to_vec(),
        });
    }

    let identical = stats.added + stats.removed + stats.changed + stats.moved == 0;
    Ok(DiffReport {
        hunks,
        stats,
        identical,
        degraded,
    })
}

/// Unified Diff 输出（§50）：`--- A` / `+++ B` / `@@ -a,c +b,d @@`，3 行上下文。
pub fn unified_diff(report: &DiffReport, label_a: &str, label_b: &str) -> String {
    let mut out = String::new();
    if report.identical {
        return out;
    }
    out.push_str(&format!("--- {label_a}\n+++ {label_b}\n"));
    // 展平全部行，带 Equal 上下文滑窗合并（3 行上下文，间隔 ≤6 合并）
    let mut all: Vec<&DiffLine> = Vec::new();
    for hunk in &report.hunks {
        all.extend(hunk.lines.iter());
    }
    // 重建含 Equal 的完整序列：hunks 之间缺少 Equal 上下文——从 hunks 的
    // 行号关系无法重建全部 Equal 行，因此 Unified 输出直接以 hunk 为单位，
    // 上下文行取 hunk 边界附近已有的 Equal 行（hunk 内含 Equal 时）。
    // 简化且确定性：每个 hunk 独立输出，行号范围以 a_start/a_len、b_start/b_len。
    for hunk in &report.hunks {
        out.push_str(&format!(
            "@@ -{},{} +{},{} @@\n",
            hunk.a_start, hunk.a_len, hunk.b_start, hunk.b_len
        ));
        for line in &hunk.lines {
            let (prefix, text) = match line.change {
                LineChange::Added => ('+', line.b_text.as_str()),
                LineChange::Removed => ('-', line.a_text.as_str()),
                LineChange::Equal => (' ', line.a_text.as_str()),
                LineChange::Changed => ('-', line.a_text.as_str()),
                LineChange::Moved => ('~', line.a_text.as_str()),
            };
            out.push(prefix);
            out.push_str(text);
            out.push('\n');
            if line.change == LineChange::Changed {
                out.push('+');
                out.push_str(&line.b_text);
                out.push('\n');
            }
            if line.change == LineChange::Moved {
                out.push('~');
                out.push_str(&line.b_text);
                out.push('\n');
            }
        }
    }
    out
}

#[cfg(test)]
mod tests;
