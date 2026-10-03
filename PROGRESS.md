# Weave Development Progress

## M1 File Core

Status: COMPLETE

Implemented:

- weave-files：Filesystem 抽象（lstat 语义、最小四操作面）+ StdFilesystem +
  FaultFilesystem（操作粒度故障注入：StatDenied/StatVanished/ReadDirDenied/
  OpenLocked/ReadFailure-after-partial）
- 路径安全集成：一切访问先过 weave_core::path；新增 `paths_conflict`
  大小写不敏感冲突契约（case-only rename 测试，M2 碰撞检测地基）
- 流式 SHA-256（sha2，256 KiB chunk）：协作取消 → Cancelled 结果；
  changed-during-hash → Unstable + best-effort digest（绝不静默）
- 有界类型分类（扩展名证据 + ≤8 KiB magic 嗅探补位，evidence 链）与
  轻量编码检测（BOM/UTF 系；GB18030 → M4，已记录）
- File Inspector（files.inspect）：stat 优先、嗅探降级 Partial + 警告、
  symlink 不跟随策略、错误/警告分离
- Directory Analyzer（files.analyze_directory）：栈式 DFS、增量统计、
  有界 Top-20 榜、确定性排序、空目录/深度/大小语义固化、失败聚合、
  资源守卫（limited + 原因）、协作取消
- IPC：独立 DTO 契约（f64 数值/epoch-ms 时间）；inspect_file 同步快速路径、
  hash_file/analyze_directory 后台任务、get_job/cancel_job 协作取消；
  JobTracker（任务管道，非 M7 引擎）；list_tools + 三个真实 Tool 注册
- UI：Drop → 判定 → Inspector/Analyzer 分发；InspectorPanel（显式哈希按钮）、
  ScanPanel（进度/取消/分布条/Top 榜/空目录）；i18n 70 键 zh-CN/en
- 文档：docs/file-core.md、docs/PERF.md、DECISIONS D15–D24、README/
  ARCHITECTURE/CHANGELOG/THIRD_PARTY 更新

Quality:

- cargo fmt --check: PASS
- cargo clippy --workspace --all-targets -- -D warnings: PASS
- cargo test --workspace: PASS（Rust 测试 24 套全绿：core 45 / testkit 11 /
  weave-files 54 / src-tauri 18 = 128 tests）
- frontend typecheck / lint / test: PASS（23 tests）
- tauri build: PASS（NSIS 安装包产出 Weave_0.1.0_x64-setup.exe 2.35 MiB）
- Runtime smoke: PASS — M1 二进制启动、日志按 charter 路径落盘、
  IPC 往返（app info 请求）真实打通
- Real UI smoke（§47，CDP 驱动真实窗口 + dev dispatch 钩子）: PASS —
  ① 文件：Inspector 渲染全部事实（17 B/文本/ASCII/时间戳），点击计算
  SHA-256 → 摘要 `146b041f…d44a8` 出现且与 Python hashlib 独立计算逐字一致；
  ② 目录（2000 文件）：扫描 50ms 完成，统计/分布条/Top-20 全部真实；
  ③ WinSxS 长扫描：点击取消扫描 → 10.2s 安全点收尾，状态=已取消，
  部分事实诚实呈现（80,993 文件 / 9.4 GB / 深度 10 / 权限错误聚合）。
  每步均有窗口截图。
- license audit: PASS（cargo deny + 前端审计）
- 性能实测（release，docs/PERF.md）：scan 100/1k/10k = 2/40/235 ms；
  取消 11 ms 生效；100 MiB hash 66 ms（O(chunk) 内存）
- 反模式扫描: 干净；架构审计: weave-core/weave-files 0 tauri 依赖、
  UI 无直接 fs、无 >500 行文件

Known Limitations:

- GB18030/GBK/Latin-1 编码检测未实现（M4，DECISIONS D22）
- 扫描分类仅按扩展名（内容嗅探只在单文件 Inspector 路径）
- 取消延迟无量化门槛（M7 引擎统一）；任务进度为 400ms 轮询（M7 改推送）
- OS 输入层（拖拽手势、原生文件对话框）未经自动化验证：对话框在本机挂起
  （P1/BLOCKED，DECISIONS D27——invoke 被接受但窗口永不出现，与前端代码无关；
  JS 侧已可见化）；Drop 事件桥本身仍是人工验证项
- Open File/Folder 按钮（§20）依赖上述对话框插件，本机不可用（同 D27）

## M2 Rename / Organizer

Status: COMPLETE

Implemented:

- weave-core：OperationKind/PlanItemStatus/CollisionKind/ItemOutcome/Plan/PlanItem
  （charter #10 结构化 Plan；Preview 与 Execute 共用）
- weave-files：rename 规则引擎（Prefix/Suffix/Replace/Regex/Counter/Date/Case/
  Extension/Template + 模板最后重组，未知占位符=Plan Error）、Plan 构建器
  （确定性排序 + 四类碰撞 + 环判定排除 NoOp）、两阶段执行器（TOCTOU
  Revalidate/逐项失败隔离/取消安全点/事务按实际顺序）、organizer（六类条件 +
  first-match-wins + root 边界 + 目标目录创建）、undo（LIFO + swap 占位暂存）
- weave-history：事务（InProgress/Completed 两阶段落盘）+ HistoryEntry、
  版本化 JSON 存储（原子写/损坏隔离/容量 500）、Undo 拒绝外部变更
- IPC：build_rename_plan/build_organizer_plan/execute_plan/undo_operation/
  get_history/get_operation；Plan 服务端缓存（§11 同一 Plan）
- UI：导航页签 + Rename 面板（规则块/模板/预览表/两步确认）+ Organizer 面板
  + History 面板（Undo 按钮）；i18n 57 新键

Quality:

- Rust gates: PASS（clippy -D warnings；workspace 24 套全绿，新增 rename/
  organize/execute/history 测试：golden、碰撞矩阵、环两阶段+undo、case-only、
  TOCTOU 三连、取消中段、权限部分成功、跨卷拒绝、损坏历史隔离）
- Frontend gates: PASS（typecheck/lint/test 23）
- tauri build: PASS（2.77 MiB NSIS）
- Real UI smoke（§99，CDP 驱动真实窗口）: PASS — 输入 4 路径 → 模板
  vacation-{counter}.{ext} → 预览（3 就绪 + 1 NoOp→Ready…实际 4 Ready）→
  两步确认执行 → 真实磁盘 4 文件重命名成功 → History 显示 "Rename 4 files" →
  点击撤销 → 真实磁盘 4 文件全部还原
- 性能（docs/PERF.md）：rename 10k = 2.5s 执行 + 0.5s plan，undo 10k =
  2.4s（0 conflicts）；100 MiB rename <1ms（元数据操作）

Known Limitations:

- Plan 缓存为内存态：应用重启后需重新 Preview（事务本身已落盘，crash 后
  InProgress 事务被 Undo 明确拒绝并提示恢复）
- 跨卷 Move 明确拒绝（D30）；Organizer 非递归
- 统一规则列表无法表达逐条目不同规则 ⇒ uniform 规则下真环仅 case-only 场景，
  执行器对真环的支持由手工 Plan 测试覆盖

## M0 Foundation

Status: COMPLETE（见 git 历史）

## M2 待办（下一步）

- Rename / Organizer：OperationTransaction 字段级落地、两阶段重命名、
  Preview/Collision/Undo/History（Spec/M2 提示词及其补丁条款）

---

（后续里程碑按 M2 → M3 → … 追加章节）
