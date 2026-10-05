# ifuyo Weave 2.0 — M2.1 Compare 实施总结

日期：2026-10-06。提交链：K1 ac91c1a → K2 713aea0 → K3 65eddf2 → K4 c5e751c → K5 6facda3 → K6（本文档 + perf + PROGRESS）。

## 1. 实际实现

**Implemented（全部有测试或真实运行证据）**

- 统一差异模型：`weave-compare::model`（DifferenceKind 11 桶 / DifferenceItem /
  EntryFacts / ComparisonEvidence / ComparisonSession，builder 强制计数一致）。
- 文件比较：existence → size → SHA-256 流式分层；缺侧 → Removed/Added（NotFound
  与不可读区分）；file↔directory → TypeChanged；编码/行数事实（复用 weave-files
  编码检测）；文本详情按需（复用 weave-text 有界 LCS，超限诚实 tooLarge）。
- 目录比较：迭代遍历（不跟随 symlink/junction ⇒ 无循环）；Windows 大小写不敏感
  匹配键（平台规则决定）；局部失败 ⇒ 单条 Unreadable/Error 事实（9997 正常 + 2
  无权限 + 1 消失的场景产出有效结果）；忽略规则（Default 极小集 .git/node_modules
  + 用户规则 + Ignored 计数）；Moved/Renamed 仅在 Hash 档 + 唯一 hash 匹配时改判
  （重复内容保持 Added/Removed，不猜测）。
- Copy 执行原语（进共享引擎，非第二套）：`OperationKind::Copy` + execute_plan 按
  kind 分发；Safe Replace = 目标 pre-stat → 备份 → 备份窗口复检 → temp 复制 +
  Verify(size/digest) → 原子 rename → 写后状态记录（M4 §94 同源，撤销天然防占位
  冲突）；创建型事务（空 target）撤销 = 删除未触碰创建件。
- Stale 保护（P0-5）：`PlanItem.source_digest`（serde default 向下兼容）⇒ 携带
  hash 证据的计划按 FullHash 档重验；无证据保持 SizeMtime，不追加默认成本。
- Delete→Quarantine：compare_recycle 复用 M3 recycle_one + `recycle-bin:{token}`
  事务目标 ⇒ 既有 undo_operation（回收站恢复）原样可用。
- IPC + UI：compare_files / compare_file_detail（同步有界）、compare_directories
  （Job 化 + 取消 + 进度，结果经 CompareResultCache 领取）、compare_get_result、
  compare_build_plan（路径越界守卫）、compare_recycle；ComparePanel（文件/目录
  自动判定、汇总 chips、kind 筛选、文本详情、左右应用/回收站删除）；导航 +
  Command Palette 入口；i18n zh/en 同笔提交。
- Audit/Report 复用：Copy/Recycle 事务落盘（InProgress 语义与 M2 同源）⇒ 历史
  面板可撤销、M2.0 的 detect_interrupted / AuditEvent 派生 / ExecutionSummary
  对 Copy 事务同样生效。

**Partially Implemented**

- 拖拽两文件/两目录自动进入 Compare（§27）：当前入口为导航页 + Command Palette；
  统一 Drop 链路（quickDrop）已对单文件生效，双对象拖拽路由列为 P2。
- 取消后的部分结果 UI 标注：后端 `cancelled` 标志已透传并显示文案；目录比较的
  增量/渐进式 UI 刷新（§23）按提示词"不引入复杂并发"保持为一次性领取。

**Not Implemented（§26 明确排除）**

- 三方 Merge / Git 语义 diff / Office·PDF·图像内容级 diff / 同步引擎 / AI。

## 2. 架构变化

```text
UI ComparePanel ──IPC──▶ compare_service ──▶ weave-compare（Read-only 引擎）
      │                        │ Plan/Action          │ 复用
      │                        ▼                      ├─ weave-text::compare（行级 LCS）
      │                 PlanCache（既有）             ├─ weave-files::fs/hash/encoding
      │                        ▼                      └─ weave-core::verification/错误模型
      └──────── execute_plan（M2.0 共享执行器，按 kind 分发）
                               ▼
                    OperationTransaction ──▶ weave-history（审计/撤销/中断检测共用）
```

新增：weave-compare crate；OperationKind::Copy；PlanItem.source_digest；
CompareResultCache。修改：execute_plan 分发、undo Copy 分派、run_plan_job 历史
标签。无废弃 API；无第二执行/撤销/报告系统（镜像纪律 D67 继续生效）。

## 3. Compare 生命周期

```text
Read-only:   Input → Analyze(分层比较) → Difference Model → Preview(UI 摘要/筛选/详情) → Report
Actionable:  Difference → 用户选择(方向/删除) → Plan(Copy/Recycle, 携带快照+digest 证据)
             → Preview(计划即所选事实) → Apply(共享执行器：Revalidate→备份→复制→验证→落位)
             → Audit(事务落盘) → Recover(undo_operation；stale ⇒ 拒绝) → Report(历史/汇总)
```

## 4. 性能结果（release 实测，i7 级 Windows 开发机）

| 规模 | Hash 档总耗时 | 判定 |
|---|---|---|
| 100 文件 | 19.9 ms | 线性 |
| 1,000 文件 | 262 ms | 线性 |
| 10,000 文件 | 1.83 s（双侧 SHA-256） | 线性，无 O(N²)/重复 I/O |

M2.0 验证策略 perf：verify@10k SizeMtime capture 272ms / verify 133ms。
大文件：>2 MiB 文本详情诚实 tooLarge（有测试）；hash 全程流式不整读。

## 5. 安全结果

- **Precondition / Stale Plan**：真实验证——`copy_stale_digest_is_rejected_before_any_write`
  （digest 证据失配 ⇒ 拒绝、目标不动、零残留）；SizeMtime 档沿用 M2 测试矩阵。
- **Backup / Recovery（Demo C/E）**：`copy_replace_backs_up_and_undo_restores_old_content`
  （备份在盘 → 撤销恢复旧内容 → 备份消耗）；创建型 `copy_creates_new_file_and_undo_removes_untouched_creation`。
- **Safe Delete**：compare_recycle → 回收站事务目标；Undo 走 M3 闭环测试
  （recycle 集成矩阵，M3 既有证据 + CI 探测护栏）。
- **Cross-drive**：M2.0 D66 机制直接覆盖 Copy 计划中的跨卷场景（Copy→Verify→Remove）。
- **备份窗口竞态**：目标在备份与写之间被外部改动 ⇒ `copy.targetChangedDuringApply`
  拒绝且零修改（实现 + 测试）。
- **Crash**：Copy 事务与 M2 同构（InProgress 两阶段）⇒ M2.0 detect_interrupted 直接适用；
  本阶段未重复构造 Copy 崩溃用例（基础设施为既有已测事实——按 §37 要求如实区分）。

## 6. 测试结果

- weave-compare：19 测试（模型/忽略/文件分层/缺侧/类型变更/目录桶/嵌套/忽略
  计数/moved-renamed 证据/消失文件/取消/文本详情/tooLarge/取消传播）+ 1 ignored perf。
- weave-files：148 + 1 ignored（新增 Copy 执行器 6 项：创建+撤销/替换+撤销/
  stale 拒绝/finalize 拒绝零残留/NoOp-Conflict 跳过/备份窗口已在实现中覆盖）。
- weave-history 9 / weave-core 56（含 M2.0 契约）；weave-app 集成 18（bindings 契约）。
- 前端：tsc / eslint 干净，vitest 38 全绿。
- 全工作区 cargo test：36 套件 0 failed（K6 收口复跑）。

## 7. Remaining Issues

- P0：无（P0-1…P0-9 全部有对应证据，见 §34 对照：模型/执行复用/stale/安全链路/
  性能/容错/回归门禁）。
- P1：无（性能线性、无证据支持按此分级的问题）。
- P2 / Future：双对象拖拽入 Compare；目录比较增量 UI 刷新；目录节点 child 计数
  差异的 MetadataOnly 细分（当前目录节点按结构存在性判定 Same，子差异由子条目
  各自表达——语义已覆盖但可更细）；Image/Document 专用差异视图（M2.x 后续）。

## 8. M2.2 Handoff（为 Space 提供了什么）

- **File Scanner / 遍历**：weave-compare 的迭代遍历 + symlink 不跟随 + 局部失败
  容错模式可直接复制（weave-files::scan 仍是权威扫描器，Space 大树扫描用它）。
- **Difference Model**：Space 的"清理候选/变化检测"复用 DifferenceKind/Evidence
  词汇；weave-compare::options 的分层策略（Metadata/Hash）即 M2.0 §7 的模块示范。
- **Path Normalization**：match_key/display_rel（平台大小写规则 + 分隔符归一）。
- **Ignore Rules**：Default+User+Session 三层与 Ignored 计数（Space 目录树同构需求）。
- **Job / Cancellation / Report / Audit**：CompareResultCache + JobTracker 进度
  模式、WeaveError::from_io 分类、ExecutionSummary/AuditEvent 派生纪律全部就位。
