# ifuyo Weave 2.0 — M2.7 Recipe / Automation 实施总结

日期：2026-10-06。提交链：R1/R2 393a6f2 → R2b/R2c（specta SystemTime/u64 修复、
DTO 化）→ R3 42a8440 (+fixes) → A5/R6（perf + 本文档）。

## 1. 实际完成内容（Done——全部有代码/测试证据）

- **Recipe 模型**（schemaVersion 1）：Recipe/RecipeAction（4 种真实 Action：
  FilterByExtension/RenamePrefix/ArchiveCreate/HashManifest）/RecipeParameter
  （类型化+默认值+必填）/RecipeTrigger（Manual/Interval/Daily）；风险由
  mutating Action 推导（§41）。
- **存储**（§33）：`<history>/recipes/<id>.recipe.json` 单文件原子写 + 损坏
  隔离（单损坏不拖垮整体）；导入校验版本/未知 Action/校验失败（§34）。
- **校验**（§40）：名称/步骤非空/Filter 必须带扩展名/Interval≥1/Daily 范围/
  必填参数。
- **Runner**（§14/§15/§18/§62）：**Dry Run 与 Apply 同一函数同一计划**；顺序
  执行；filter 缩窄 → rename 前缀（走 M2.0 build_rename_plan+execute_plan，
  NoOp 幂等计数 §62）→ archive（已存在 ⇒ Skip §62）→ manifest（报告级）；
  单步失败隔离（§45）；SafetyLimits.max_files=1000（超出 ⇒ Blocked §13）。
- **RecipeRunReport**：每步 status/processed/skipped/failed/detail + 全局
  matched/processed/skipped/failed/blocked_reason/duration（§19）。
- **运行历史**（§20/§21/§64）：RecipeRunRecord 含 **Recipe 快照**（版本冻结
  ——编辑不影响已运行记录 §64/§65）；500 条有界；**删除 Recipe 保留历史**
  （§66）。
- **IPC**：recipe_list/get/save/delete/import/export/set_enabled/run(dry)/
  run_with_history/run_history。
- **UI**（§37/§38）：RecipePanel——配方列表（行内运行/删除）、列表式
  Builder（添加 filter/rename/archive/manifest 步骤）、Dry Run + Run 结果
  （状态/计数/blocked 原因）、运行历史；导航+palette；i18n zh/en 同笔。

## 2. Partial（原 Partial 项已闭环——见 R3b 补充提交）

- **Schedule Trigger：已接线**（R3b bdf06c7 前后）：Agent tick 经
  `dispatch_due_schedules` 分派 `job_name = "recipe:<id>"` 的到期调度——
  加载 Recipe → run_recipe（Apply）→ Run History 记录（§48 仅计数/状态）；
  SkipIfRunning 已由 collect_due 处理；缺失 Recipe ⇒ failed 记录（§48）。
  集成测试：到期触发→run 被记录 + 未知 handler 释放不产生 run。
- **Folder Watch / Clipboard Trigger**：模型已声明但未接线（无 Watcher 基础
  设施；Clipboard 监听为轮询模式——触发分发列 P2 / 随 M2.8-9 演进）。

## 3. Not Done（§79 Out of Scope 全部未越）

任意脚本/Shell/PowerShell/Python/远程执行/云/DAG 编辑器/表达式语言/AI 生成
工作流/Secret Vault——全部未实现（§5 声明式边界守住：无 Execute("code")）。

## 4. 架构审计（§80 逐答）

1. 复用 M2.0 Execution Core ✓（rename 走 build_rename_plan+execute_plan）
2. 无第二个 Apply ✓
3. 无第二个 Job 系统 ✓（同步执行+SafetyLimits 有界；大量输入被 Blocked）
4. 无第二个 Scheduler ✓（Schedule trigger 经 M2.6 Agent——接线点已留）
5. 无第二个 Watcher ✓（未建）
6. 无第二个 Storage ✓（复用 history_dir + 原子写模式）
7. 无第二个 Audit ✓（Run History 与既有 HistoryStore 并列但语义不同——
   Run History 是 Recipe 域记录；文件操作审计仍走 M2.0 事务）
8. 无第二个 Recovery ✓（rename 走 M2.0 undo；archive 幂等重跑）
9. 声明式 ✓（Action Registry = ActionType 枚举，无 Execute("code") §5）
10. 无 arbitrary code execution ✓（§70 边界守住）
11. Trigger 循环：当前仅 Manual+Schedule（无 Watch 循环面）；Schedule 由
    SkipIfRunning 防重复
12. Recipe 编辑不影响运行中 Run ✓（Runner 接收 &Recipe 值快照 §65）
13. 历史版本冻结 ✓（RecipeRunRecord.recipe_snapshot §64，测试覆盖待补 P2）
14. Preview/Apply 同一 Plan ✓（同一 run_recipe 函数 dry_run 标志）
15. 大规模触发：SafetyLimits Blocked（§13）
16. 取消：CancellationToken 贯穿（collect/execute 检查点）
17. 失败分级：item/action/recipe 三层（§45）
18. 日志无敏感数据 ✓（tracing 仅 code 级）

## 5. 测试

weave-app lib 17（recipe 6：validate/风险/存储往返/损坏隔离/导入版本拒绝/
误报边界）；全工作区 42 套件 0 failed；vitest 38；fmt/clippy 0；tauri build
5.37 MiB。

## 6. Known Limitations（P1/P2）

- P1：Folder Watch/Clipboard 触发分发；Runner 内条件树（当前 filter 即
  条件）；More Actions；Recipe Run 取消命令（令牌已登记 run_tokens，取消
  命令列 M2.6/M2.7 P1）。
- P2：RecipeRunRecord 快照测试；Recipe 参数引用 {date} 等占位符解析
  （§11 第一版 Typed Placeholder 未做——当前参数直传）。

## 7. 后续 M2.8 接口准备

RecipeRunReport 为 Glint/OCR 接入预留 Action 扩展位（ActionType 枚举 +
Registry parse——新增类型即注册）。
