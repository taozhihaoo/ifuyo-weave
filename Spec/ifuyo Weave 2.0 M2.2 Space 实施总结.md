# ifuyo Weave 2.0 — M2.2 Space 实施总结

日期：2026-10-06。提交链：S1 2167475 → lint c0494b0 → S2 151c7c2 → S3 4ffd1ec → S4 4c9eb79。

## 1. 实施范围

空间分析 → 定位占用 → 识别异常 → 清理候选 → 预览 → 安全执行 → 可恢复。
**Analysis-first, Action-second**（§67）：默认只分析，任何清理都走
Candidate → Review → Plan/重验 → Recycle（Quarantine）→ Audit → Recovery。

## 2. 实际完成内容（有代码/测试证据）

- **磁盘概览**（§4.1）：`space_list_volumes`（sysinfo 安全 API——workspace
  `unsafe_code = forbid`，不手写 FFI；容量 0 卷剔除；字节内部表示，UI 转换）。
- **空间扫描聚合**（§5/§7/§8）：`weave-files::space::scan_space`——迭代 DFS +
  **自底向上折叠**（发现序逆序，O(n) 无 O(n²)）；DirNode 递归总量
  （logical_size/file_count/dir_count 语义一致）；不保留全量文件对象。
- **Top-N**（§48）：有界 BinaryHeap（容量 = top_n，超界弹最小），非全量排序。
- **文件类型统计**（§10-§12）：复用 M1 `classify_by_extension`（集中分类表）；
  无扩展名 = 空桶、未知扩展名 = Unknown，不强行归类。
- **权限/消失容错**（§22/§23）：单目录不可读/单文件消失 ⇒ 计数事实
  （unreadable_dirs/files、disappeared），会话照常产出。
- **符号链接**（§24）：lstat 语义，目录型链接非 Directory ⇒ 不跟随，无循环。
- **一致性诚实**（§21）：`consistency = "bestEffort"`；硬链接逻辑大小口径
  如实（§25 不声称实际释放 = 逻辑大小）。
- **清理候选**（§29-§32）：LargeFile/OldFile/TempFile/EmptyDir 四规则；
  reasons = 事实清单（size/未修改天数/扩展名表）；risk 不按扩展名断言安全；
  confidence 第一版最高 Medium（不假装智能）。空目录严格语义（含空子目录
  不算空——级联判定列 P2）。
- **清理执行**（§33-§37）：`space_cleanup` = 逐项 SizeMtime 重验（失配 ⇒
  `stale_skipped`，**绝不静默删除新内容** §36）→ M3 `recycle_paths`
  （Quarantine，禁止永久删除 §34）→ `recycle-bin:{token}` 事务目标 ⇒ 既有
  `undo_operation` 恢复（Demo 7）；历史落盘（undoable/summary）。
- **释放空间双报告**（§37）：`expectedLogicalFreed`（回收项逻辑字节）与
  `observedFreeDeltaBytes`（同卷空闲前后差；跨卷/不可测 ⇒ None）分离。
- **快照/增长**（§26-§28）：每次扫描落一条 SpaceSnapshot
  （history_dir/space-snapshots.json，temp+rename 原子写，500 条有界）；
  growth 对比（previous=0 ⇒ 无百分比，绝不 Infinity §27）。
- **UI**（§40-§45）：SpacePanel——卷条形概览（pick）、扫描进度/取消、
  breadcrumb 钻取 + 子目录排行 + 占比条（proportional bars，无图形库 §15）、
  最大文件 Top20、类型统计条、候选 Evidence 列表 + 选择清理、cleanup 双
  释放报告；导航 + palette 入口；i18n zh/en 同笔。
- **Job 化**（§44）：扫描 = JobTracker（进度/取消），结果经 SpaceResultCache
  领取（与 M2.1 同模式）；取消 ⇒ cancelled 部分结果（非 Failed §46）。

## 3. 部分完成内容

- 增量/渐进式 UI 刷新（§16/§45）：后端单次产出 + job 进度；"扫描中部分结果"
  的实时面板未做（与 §23"不引入复杂并发"一致，列 P2）。
- 扫描缓存（§19/§20）：未做缓存层——Space 每次全量扫描（100k=1.4s 实测，
  缓存收益有限）；Fresh/PossiblyStale 状态机留给真实需求出现时。

## 4. 未实施内容（§55 边界全部未越）

系统垃圾清理/Registry/浏览器清理/启动项/碎片整理/SMART/云盘/NAS/自动删除/AI
决定删除/Everything 式索引——全部未实现（产品边界保持）。

## 5-10. 架构 / 生命周期 / 模型 / Cache / Candidate / Plan 集成

```text
Discover(space_scan job) → Analyze(聚合/TopN/类型/候选) → Understand(钻取/占比/增长)
→ Recommend(候选+Evidence) → Preview(UI 列表+原因) → Apply(逐项 stale 重验→recycle)
→ Audit(事务+历史) → Recover(undo_operation=回收站恢复)
```

Q1：Space walk 未直接调用 M1 `scan_directory`——M1 扫描器是"平铺列表 +
DEFAULT_MAX_ENTRIES 截断"形状，Space 需要全量树聚合（无截断）；复用的是
Filesystem trait、FileStat、classify、路径校验与取消契约。Q2：**无第二套
删除/清理系统**（recycle_paths + 既有 undo）。Q3：是——全部清理经
重验 → recycle → 事务 → undo。Q6：Logical/Physical（Physical 未接 ⇒ 不伪造）、
Expected/Observed 四概念分离。Q7：候选可解释（reasons 事实清单）。Q8：Space
仍是分析工具（候选需人工选择，无自动删除）。

## 11. 性能测试（release 实测）

1k = 14.2ms / 10k = 136ms / **100k = 1.40s**（线性；stat+聚合，无 Hash）。
取消响应：目录间安全点（§24 取消 ⇒ cancelled 部分事实）。

## 12. 错误与取消处理

Fatal（根不可读）/ Recoverable（单条 Unreadable/Disappeared）/ Cancellation
三态分离（§46）；全部有测试（empty/one/nested/topn/types/candidates/emptydir/
growth/snapshot 九项单元 + perf）。

## 13. 测试结果

weave-files 157+1 ignored（space 9 单元 + 1 perf）；全工作区 38 套件 0 failed；
tsc/eslint/vitest 38 绿；fmt/clippy workspace 0 警告；tauri build 通过。

## 14-15. 已知问题 / P0-P1

- P0：无（§63 P0 清单逐条有证据）。
- P1：无（无证据支持分级问题）。
- P2：含空子目录的目录不算空目录（级联判定）；增量 UI 刷新；扫描缓存层。

## 16. 与 M2.1 的关系

Space UI 的条目动作可跳转 Compare（架构上 service 分层清晰）；两者共享
JobTracker 模式、SpaceResultCache/CompareResultCache 同构、候选/差异 Evidence
理念同源（D67 派生纪律）。未实现"当前目录 vs 快照"的 Compare 直连（P2）。

## 17. 为 M2.3 Clipboard 提供的接口/能力

- Job 化长任务模式（register → spawn_blocking → cache → get_result）已成
  可复制范式（第三次使用）。
- SpaceSnapshot 的"本地历史 JSON + 原子写 + 有界淘汰"可复用为剪贴板历史
  持久化骨架（Clipboard History 同为本地有界列表）。
- 候选 Evidence 结构（事实清单 + risk/confidence）为 Wisp（M2.9）敏感数据
  报告提供了同源先例。
