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
  InProgress 事务被 Undo 明确拒绝并提示恢复）（D31）
- 跨卷 Move 明确拒绝（D30）；Organizer 非递归
- 统一规则列表无法表达逐条目不同规则 ⇒ uniform 规则下真环仅 case-only 场景，
  执行器对真环的支持由手工 Plan 测试覆盖
- **D27 沿袭**：Open File/Folder 按钮依赖的对话框插件在本机挂起（P1
  BLOCKED），Rename/Organizer 面板的文件选择按钮同样不可用；路径输入框
  与 Drop 手势不受影响。待 D27 排查后自动恢复

## M0 Foundation

Status: COMPLETE（见 git 历史）

## 下一步 / Next

- M5（Spec/M5（上）（下）；JSON as data / CSV / TSV——必须复用 M4 文本
  基座 Encoding/TextDocument/Diagnostics，§243 Handoff）

## M3 Duplicate Finder

Status: COMPLETE

Implemented:

- weave-files duplicates：三级管线（recursive scan 复用 M1 策略 → size 分组
  count>=2 → partial hash（head+tail 4 KiB，单一常量 PARTIAL_HASH_BYTES）→
  full streaming SHA-256 前后一致性检查 → 分组）；GroupId = "grp_{hash16}"
  内容派生；wasted_size=(n-1)*size；确定性排序（组 wasted DESC / 文件路径
  case-insensitive 升序）；取消 ⇒ partial_result 明确标记；扩展名差异仅事实展示
- 回收站适配器（recycle.rs）：trash 5.2（Windows IFileOperation，进程内调用）；
  Shell API 唯一出现处；RecycleOutcome 三态（不假装删除）；回收后反查平台
  token（original_path + ±5s 时间窗，唯一才记录）
- build_recycle_plan：选择 → Plan（kind=DuplicateRecycle）；校验矩阵（组存在/
  文件属于组/每组至少保留一份 keepAtLeastOne/不重复选择）；条目快照
  size+modified 供 Revalidate；target 留空（目标由适配器决定）
- execute_recycle_plan：逐项 Revalidate（sourceNotFound / changedSinceScan 拒绝）
  → 适配器回收 → 事务记账（target=recycle-bin:{token}）；取消在条目间安全点
  生效——取消后未处理条目 NotExecuted 且绝不被回收；失败隔离；Reversibility
  如实计算
- Undo：undo_recycle_transaction 从事务重建回执 → token/时间窗匹配回收站 →
  restore_all 批量恢复 → 逐条原位校验；原位被占 ⇒ UndoConflict 不覆盖（§67）；
  回收站被清空 ⇒ Missing 如实报告；NotExecuted/畸形 target ⇒ NotUndoable
- IPC：scan_duplicates / build_recycle_plan / execute_recycle_plan 命令 +
  ScanCache（scan_id 服务端缓存，快照不过 IPC）；JobOutcome::DuplicateScan /
  RecycleExecuted；undo_operation 按 kind 分派（DuplicateRecycle → 回收站恢复）
- UI：Duplicates 面板（目录输入 + min size、组视图逐文件勾选、保留首个预选、
  选择摘要、回收计划 Preview、确认对话框「移入回收站（可撤销）」、结果 +
  撤销按钮）；nav 第 4 项；i18n zh-CN/en（修复 M2 遗留的 App 面板双渲染）
- 文档：docs/file-core.md M3 章节、docs/PERF.md（Scenario A–D）、
  DECISIONS D33–D36、ARCHITECTURE M3 边界、README/CHANGELOG/
  THIRD_PARTY 更新

Quality:

- Rust gates: PASS（clippy --workspace --all-targets -D warnings；cargo deny
  licenses ok；workspace 24 套全绿：core 47 / weave-files 126 / testkit 11 /
  src-tauri 18）

（下）补强轮（2026-10-04，提交 de02dfe/7189ee9/ff7674f/10ce6c8）：

- §91 fault 注入矩阵：RecycleAdapter trait 缝隙（§94 平台隔离边界）+
  不可用/逐条失败/中段取消（适配器可证未被再调用）/Undo 冲突与 Missing
- §92 集成闭环（spec 定义的"最关键测试组"）：真实 fs 扫描→分组→选择→
  计划→真实回收站→磁盘验证→HistoryStore 落盘→重开→Undo→内容级还原
  验证；Unicode（中/日文件名/emoji/括号）+ 嵌套目录；§138 变异不变量
- §146 真实故障：scan 与 execute 之间删除 source ⇒ sourceNotFound 结构化
  失败，其余照常回收+还原
- §88/95/96/137：大小写不污染内容身份、100 文件组不变量、唯一 size 永不
  进 full hash
- **P0 修复**：DuplicateFileEntry.path 曾泄漏小写 normalized 路径——
  Windows 靠大小写不敏感侥幸工作，Linux 上回收/还原会失配；现条目携带
  原始路径，normalized 仅限内部去重/排序
- §106/108 入口 UX：Duplicates 页复用全局 Drop 填 roots + 会话内最近根
  chips（charter #37 不入全局配置）；App.test 里程碑断言改资源驱动
- §112/129/130/132/134 文档：file-core 崩溃语义、PRIVACY M3 审计表、
  README 真实使用流程、DECISIONS D37（硬链接/无哈希缓存/顺序管线/排序）
- Frontend gates: PASS（typecheck / lint / vitest 23）
- tauri build: PASS（2.85 MiB NSIS）
- Real UI smoke（§144–§146 闭环，CDP 驱动真实窗口）: PASS — 导航到重复文件页
  → 填入真实目录（3 重复 + 1 独立文件）→ 扫描（1 组 3 文件，保留首个预选 2）
  → 生成回收计划 → 确认对话框 → 真实回收站执行（a.txt 保留、b/c.txt 移入
  回收站、unique.txt 不受影响）→ 撤销 → b/c.txt 从回收站真实还原
- 性能（docs/PERF.md）：Scenario A 50k 唯一 size 扫描 2.16s（0 内容读）；
  B 同 size 异内容 10k×64 KiB partial 裁剪 100%（full hash 0 次）；
  C 10k 重复分组精确（10 组 = 10 种内容）；D 32×32 MiB 流式 1.56 GiB/s

Known Limitations:

- ScanCache/PlanCache 内存态：应用重启后需重扫/重建 Plan（结构化错误）
- 回收站 Undo 依赖平台可枚举回收站；用户清空回收站后 Undo = Missing（如实）
- token 为平台 id 的 Debug 串（不持久化解析，仅同会话比对）；
  跨会话 Undo 退化为 original_path + 时间窗匹配
- 逐条 recycle_paths 适配器调用为逐条 IFileOperation（Windows 批量上限
  未压测；万级条目场景 M7 批处理引擎统一评估）
- Open Folder 对话框 D27 挂起（沿袭 P1 BLOCKED）；Duplicates 入口为
  Drop + 粘贴路径 + 会话最近根（§106 已按现有入口能力适配）
- 「最近根」为会话态；组列表无虚拟化（基准未证明瓶颈，M11 Polish 复测）
- macOS / Linux 运行时未验证（CI 为 windows-latest；路径大小写 P0 已修，
  回收行为按平台适配器隔离，跨平台回收语义未实测——如实标注）

## M4 Text

Status: COMPLETE

Implemented:

- weave-text crate（M0 骨架填充）：TextDocument 统一模型（source/content/
  encoding/bom/line_ending/format/path/byte_size/ends_newline）；
  编码解码级联（BOM 严格 → UTF-8 严格 → GB18030 严格 → Latin-1 无损兜底，
  全程零 U+FFFD——D22 欠账 GB18030/GBK/Latin-1 在本里程碑补齐）；
  Offset 契约（byte 域内 / line + UTF-16 列 at IPC，§18 高风险边界）；
  TextRange/LineIndex；结构化 Diagnostics；保守格式检测（扩展名 + 有界
  嗅探，Unknown 即要求手选）
- Formatter 七格式能力矩阵（§46 诚实原则）：JSON（真解析、preserve_order
  键序保持、递归键排序绝不排序数组）、XML（quick-xml 事件流、well-formed
  校验、结构保持格式化）、YAML（yaml-rust2 真解析、Sort 带"丢注释"警告）、
  SQL（词法级、字符串/引号标识符/注释保真）、JS（Minify 安全子集；
  Format/Validate 诚实 Unsupported）、CSS、Markdown（围栏不可侵犯）；
  Sort/Normalize 等不可靠能力一律 Unsupported + 原因
- Compare：前/后缀裁剪 + 有界 LCS（2000²）+ 超窗诚实降级（degraded 标注）；
  Changed 等长配对 + Moved 全局同键配对；Whitespace 三档 + Unicode 折叠；
  TooLarge 显式限额；Unified 输出
- Extractor 八类 + 用户 Regex（Rust regex 线性引擎 ReDoS 安全）：统一
  ExtractMatch（byte + line + UTF-16 列），source order + 确定性次序，
  出现次数/唯一值双视图；JSON 平衡扫描 + 语法终验；IPv4/IPv6 实校验
- Transformer 九操作（纯函数、确定性、行结尾随行携带）；Find/Replace
  带匹配计数；Rust regex $1 替换契约
- 安全写回（复用 M2 全套基建，§93/§114）：Plan(TextTransform) → 任务内
  TOCTOU Revalidate（§90）→ 备份 → weave-files::atomic_write（§91/§92
  共享写设施）→ 事务/历史；Undo 按 kind 分派——TextTransform 校验写后
  状态实现 §94（用户改动拒覆盖）；无平行 TextUndoRecord
- IPC：7 命令（load/format/transform/extract/compare/build_text_write_plan/
  execute_text_plan）+ TextWriteCache + JobOutcome::TextExecuted；
  2 MiB 输入上限 + NUL 二进制守卫
- UI：Text 页四工具面板（格式化/变换/提取/比较），Preview 截断 20k 字符
  （仅显示，Apply 基于完整内容 §85），写回确认对话框，复制/复制全部；
  i18n 60 键 zh-CN/en

Quality:

- Rust gates: PASS（clippy --workspace --all-targets -D warnings；cargo deny
  licenses ok；weave-files 129 / weave-text 64 / core 47 / testkit 11 /
  src-tauri 18 全绿）
- Frontend gates: PASS（typecheck / lint / vitest 23）
- tauri build: PASS（3.17 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口）: PASS — 加载真实 JSON 文件（ascii 13B）
  → Format 预览（键序保持 + 2 空格）→ 写回确认 → 磁盘 pretty 化（键序不变）
  → History 撤销 → 磁盘还原为原紧凑内容
- smoke 抓到并修复 P0：M2 undo 占位检查对"覆盖写"恒冲突（原位必然被占）
  ⇒ TextTransform 专用还原 + §94 测试×2（weave-files undo_text_tests）

Known Limitations:

- YAML Sort/重序列化丢注释与锚点展开（Warning 如实告知；D43）
- JS Format/Validate 需真 parser（v1 Unsupported）；JS Minify 的 regex
  字面量含 `//` 可能误判（D43 记录）
- SQL Validate 为词法级 + 括号平衡（非完整语法）；Compare 超窗口降级
  （degraded 标注）
- Open Folder/File 对话框 D27 挂起沿袭；Text 文件入口 = 路径输入
- macOS/Linux 运行时未验证（CI windows-latest）
- （下）补强轮（2026-10-04，提交 df7139d/ee1bb20/e396b8d/096893e + 本笔）：
  TextLimits 分档限额（§104/§166）；auto 检测移服务端（§112）；二进制守卫
  加固（§103）；Drop 文件直接加载（§102）；Compare 选项 UI + Empty State +
  提取计数/截断标注（§146/§160/§185）；Save-As + collision 双检（§88/§132）；
  golden 7 基线（审阅抓出 XML 括号缺失/SQL 缩进双写两个真 bug）+ 性质 9 +
  畸形 6（§126-129/§181-184）；文件级集成 5（§131-135）；**性能证据抓到
  Compare P1（100k 全不同 169.6s → 102ms，HashMap 配对修复）**；
  docs/text.md 工具文档 + SECURITY.md M4 专项（§209/§215）
- （下）遗留（如实）：预览为同步有界计算（限额使最坏延迟小，D44 决策）；
  写回任务取消经错误码映射为"已取消"（§108 等价实现）；1M 行实测与 100k
  同机制（fixture 复用），真实 1M 列入 M7 场景；macOS/Linux 未实测

