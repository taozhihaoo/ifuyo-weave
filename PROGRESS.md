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

- M7（Spec/M7（上）（下）；General Batch Engine）

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

## M5 Data

Status: COMPLETE

Implemented:

- weave-data crate（M0 骨架填充）：DataLimits（§88 分档：输入 32 MiB/
  内存 500k 行/页 1000/preview 50/unique 100k/诊断 1000/profiling 50k）；
  CSV/TSV 解析（csv crate §9/D45：引号/多行/转义/BOM 剥除/incremental；
  头语义 §11-§13：诚实启发式+confidence、重复头 name_N+稳定 col_N、
  空 header fallback；ragged §14 补空/保留+诊断）；JSON profiling
  （§49-§51/§92-§94：root 语义、点路径、potential-type 集合含 Mixed、
  presence/null 率、诚实采样）；JSONL（§59-§62 流式+FailFast/Collect
  双模式+计数）；Inspector（§43-§47：potential 类型观测、null 率、
  exact unique 超限 Unavailable、前导零 Identifier 标注）
- 转换器（§52-§62）：JSON↔表（root 数组强制、点路径展平、JSON cell、
  null→空默认）、表→JSON（默认字符串/typed 可选/前导零绝不推断/超 i64
  保文本）、表→CSV/TSV（csv Writer，行尾可控，round-trip 无损）
- DataSession（§17-§24）：ephemeral 视图（AND 过滤/稳定排序 null 恒
  最后/分页）；DataTransformPlan（§78-§81 可序列化规则：Trim/大小写/
  FillEmpty w/ NullPolicy/FindReplace/Split/Merge/Rename/Delete/Dedupe/
  DateNormalizeIso（歧义⇒Anomaly）/NumericNormalize（显式分隔符）；
  校验收集结构化错误；列存在性对演化表逐规则检查）
- IPC（§83-§87）：data_open/page/set_view/inspect_profiles/preview_
  transform/apply_transform/export——会话句柄+分页上限；导出/覆盖
  复用 TextTransform 管线（TOCTOU/备份/原子替换/历史/Undo）
- UI：Data 页四标签（表格分页/检查器/清洗 JSON 计划/转换导出）；
  nav；i18n 43 键 zh-CN/en
- Undo 补强：创建型写回（无备份）撤销 = 删除已创建文件（§94 保护：
  用户改过 ⇒ 冲突，§74 Undo 对实际文件变更生效）；2 域测试

Quality:

- Rust gates: PASS（clippy workspace -D warnings；cargo deny ok；
  workspace 28 套全绿：weave-data 40 / files 131 / core 47 / testkit 11 /
  src-tauri 18+5 集成）
- Frontend gates: PASS（typecheck/lint/vitest 23）
- tauri build: PASS（3.18 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口）: PASS — ragged CSV 打开（3 行×2 列，
  多余单元格保留）→ contains 过滤（AND 视图）→ 检查器（潜在类型/exact
  唯一值）→ 清洗计划预览 → JSONL 导出新文件 → History 撤销 = 删除已
  创建文件（§74）
- 性能：transform 100k 行 2-61 ms、URL 提取 1 ms、JSON format 1 MiB
  17 ms（docs/PERF.md M4 节同机制；M5 专项 perf 列入（下））

Known Limitations:

- 1M 行真实场景未实测（fixture 与 100k 同机制；M7 批处理补测）
- OR 过滤组、日期 locale 归一（非 ISO 输入）v1 不做（显式 NOT SUPPORTED）
- 预览为同步有界计算（限额使最坏延迟小，D44）
- （下）范围：§115+ 完整测试矩阵、Viewer 虚拟滚动、Anomaly 扩展

### M5（下）补强轮（2026-10-04，提交 9aa0d10/f0a11f9/faf35db/9038b56/299abd3+）

- §116-§118 巨限守卫：max_columns 4096 / max_cell 8 MiB ⇒ 结构化
  `data.tooManyColumns` / `data.cellTooLarge`（fail safely）
- §119 取消：`parse_csv_cancellable` 每 record 检查 token ⇒
  `data.cancelled`（真取消，非假按钮）
- 回归套件（§126-§138）：CJK/emoji 往返、escaping 往返（语义等价
  §129/§130）、数值永不强转（§131）、行列边界、会话确定性/稳定性
  （§121/§124）、JSONL 错误预算（§118）——weave-data 7 integration
- §176/§179 导出范围：all | view（view = 过滤/排序后所见即所得）+
  UI "Exporting X of Y rows" 明示；data_close 会话关闭（§146 LRU+last-
  used，§147 隔离）
- 性能证据（§157-§161）：CSV scan 100k 17ms / filter 10ms / sort 16ms；
  JSONL 100k 48ms；csv→json 83ms；PERF.md M5 节
- docs/DATA_FORMATS.md（§192 每格式语义/限额/边界）
- 已知边界（如实）：大文件 open 仍同步（< 限额最坏延迟有界，D44）；
  OR 过滤组 / 非 ISO 日期归一 / 真实 1M 行未做（显式 NOT SUPPORTED）

## M6 Image

Status: COMPLETE

Implemented:

- weave-media crate（M0 骨架填充）：magic-bytes 格式检测（六格式，
  mismatch 报告不强制 §10）；ImageLimits（dimension 16384 / pixels 80M /
  decoded 256 MiB / metadata 4 MiB / preview 2048——§5/§7/§118-§120）；
  header 守卫先于解码（§8 Decode Policy，checked 算术 §6）；Capability
  Matrix 由真实编解码测试固化（§11——WebP 有损编码 NOT SUPPORTED，纯
  Rust 生态 spike 结论 §109/D50；动画 v1 Static Only §59）
- Inspector 事实（§33/§98 Fact only）：格式/尺寸/alpha/色彩/位深/帧数/
  EXIF（make/model/datetime/orientation，GPS presence 经 GPSInfoIFDPointer
  §23）/扩展名 mismatch
- Resize（§35-§44）：Fit/Fill/Exact/Scale + 防放大 + 四过滤器；确定性
  round-half-up（§40）；Alpha 合成防 halo（§15）
- Convert/Compress（§45-§58）：encode_image 按格式分 quality 语义
  （JPEG 0-100/WebP lossless only）；PNG→JPEG 白底合成；animated GIF→PNG
  拒绝动画（事实记录帧数）
- Metadata（§20-§27/§60-§62）：kamadak-exif 读取（make/model/datetime/
  orientation/GPS presence）；strip = 重编码不写 EXIF（§61 remove-
  removable 语义）+ orientation 物理归一（§25/§27 显示方向不破坏）
- IPC：image_open（有界预览 data URI §65）/image_preview（同引擎 §64，
  warnings：lossy/re-encode/animation-dropped/alpha-composited）/
  image_execute（新文件 dest-exists 拒绝 §75；覆盖源走 TextTransform
  管线 ⇒ TOCTOU/备份/原子/历史/Undo §74/§145；**创建型写回撤销 = 删除
  已创建文件 w/ §94 stat 保护**）/image_cancel 占位
- UI：Image 页（打开+事实表+GPS 隐私高亮+预览图+操作选择+质量滑条+
  resize 参数+strip checkbox+执行）；nav；i18n 35 键 zh-CN/en

Quality:

- Rust gates: PASS（clippy workspace -D warnings；cargo deny ok；
  workspace 29 套全绿：weave-media 21 / weave-data 41+7 / files 133 /
  core 47 / testkit 11 / src-tauri 18+5）
- Frontend gates: PASS（typecheck/lint/vitest 23）
- tauri build: PASS（4.07 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口）: PASS — CDP canvas 生成 PNG →
  打开（64×32 facts）→ resize 32×16 预览（同引擎 §64）→ 执行写新文件
  （PNG magic 验证）→ History 撤销 = 删除已创建文件（§74）

Known Limitations:

- WebP 有损编码 NOT SUPPORTED（纯 Rust 生态限制，§109 spike，D49）；
  解码/无损编码支持
- 动画 GIF/WebP：帧数为事实、转换显式拒绝保留动画（§59 Static Only）
- JPEG→JPEG metadata-preserving rewrite 未做（重编码，§55 如实警示）
- 16-bit PNG：由 image crate 解码后转 8-bit 处理（观测如实报位深）
- （下）范围：multi-image 批量/批量预览/per-file 结果、转换选项 UI 化、
  metadata cleaner UI、§142+ 矩阵——列入"下一步"

## M7 Batch Engine（上）

Status: COMPLETE

Implemented:

- weave-batch crate（M0 骨架填充）：Job 状态机（Created→…→Failed 显式
  合法迁移表 + 终态判定 §4）；输入快照 path/size/mtime（§10/§203）+
  revalidate（ChangedSincePreview/FileMissing §204/§75）；Linear
  Pipeline = Source + 0..N Filter + 0..N Transform + 末置唯一 Export
  （§11-§13；DAG/分支/循环显式不实现 §12/§0.2）；Pipeline::validate
  结构 + 类型跟踪（Bytes/Text/Image §57；TextTransform 后 PNG 无损中转
  D51）；JobPlan 可序列化（§19 serde）
- 执行引擎：同引擎 Preview/Execute（§20-§23；preview = dry_run 旗标，
  Export 不落盘但碰撞照报 = Potential Failures，D52）；item 级失败
  隔离（§25/§49 单条失败不放弃其余）；Filter 拒绝 ⇒ Skipped（§14
  Rejected ≠ Failed）；协作取消（§32-§34 未开始/阶段间安全点 ⇒
  Cancelled）；诚实进度 per-item 计数（§30）；稳定 ItemId = item_{n}
  （§18 快照序）；§26 Result Model 全计数
- 变换适配（不复制第二套实现 §243）：文本 = weave_text TransformKind
  映射（TrimLines/CaseConvert/FindReplace）；图像 = weave-media
  inspect_bytes(ImageLimits 守卫)/resize/encode_image（PNG 无损中转）
- IPC（§87-§90）：batch_preview（同步模拟）+ batch_execute（任务化，
  任务内 §75 快照重校验 ⇒ ChangedSincePreview Job Failed；进度经
  JobTracker sink；取消复用 cancel_job）；DTO 镜像 StageSpec/TextOpSpec
  （specta tag=type；适配层只投影不隐式转换 §58）
- UI：Batch 页（§86 不承载引擎逻辑）——三条预设 Linear 管线（文本
  trim / trim+lowercase / 图片 Fit1920→PNG）+ 输入清单 + 目标目录 +
  覆盖/继续开关 + Preview/Execute/Cancel + per-item 结果表（D53）；
  i18n zh-CN/en
- 决策：D51 负载模型/PNG 中转、D52 同引擎 dry-run Preview、D53 预设
  管线 UI、D54 （上）不新增持久化（§91；Journal/Resume/Pause/Retry
  全列（下））

Quality:

- Rust gates: PASS（clippy -D warnings；fmt；weave-batch 12 测试 +
  workspace 全绿）
- Frontend gates: PASS（typecheck/lint/vitest 23）
- PERF 实测：text 1000 files 288.7 ms；image 100 files 96.8 ms
  （docs/PERF.md M7 节）
- tauri build: PASS（3.97 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口）: PASS — 批量页预设管线（文本
  trim）× 2 输入（a.txt + b.log）：Preview 计数 2/成功1/跳过1 且盘侧
  零写入（§21）；Execute 后台任务 + 结果表逐条正确 + 产物字节级断言
  （"hello weave
batch engine
"）；取消冒烟 = 20000 条任务中途
  cancel ⇒ 成功 1627 + 取消 18373（计数闭合）；三阶段 console 零错误

Known Limitations:

- Job Journal/崩溃恢复/Resume/Retry/Pause IPC 未做（§40-§42/§90 部分
  ——M7（下）主范围，D54）
- 取消经 cancel_job 生效于未开始条目与阶段边界；单阶段内部（如大图
  decode）不可中断
- Preview 对图像执行真实解码+resize（同引擎 §23 的代价）；超大清单
  预览耗时线性（v1 无预览采样）
- Export 命名 = 同名.新扩展名（§76 v1）；无冲突重命名策略（§77-§79
  列（下））

## M7 Batch Engine（下）

Status: COMPLETE

Implemented:

- weave-batch journal（下 §42/§92）：append-only JSONL（header=完整
  JobPlan+schema_version；item 记录逐条 flush；finish 行终态）——崩溃 ⇒
  Interrupted(Recoverable)（§183）；损坏尾部隔离 `.corrupt` 不 panic
  （§197）；版本不符显式拒绝（§196）
- 引擎扩展（下 §39/§123/§124/§143）：`execute_subset`（Retry/Resume 共同
  机制——结果按快照序还原）；workers 有界并发（1-8，进度回调 channel
  汇聚到调用线程 §30）；Pause ⇒ pending（非终态）；retryable 分类；
  批内输出名预claim（快照序小者胜——1/4 workers 结果一致测试固化）
- History/Undo 集成（下 §184/D58）：OperationKind::BatchExecute；创建型
  产物撤销 = stat 守卫删除（复用 weave-files creation-undo）；覆盖写产物
  撤销必报冲突（拒绝删除 §185）；JobResult.operationId 关联入口
- IPC（下 §90/§202-§204）：batch_pause / batch_resume（journal 驱动，
  逐条重校验 ⇒ conflicts 排除上报，新 run job_id）/ batch_retry_failed
  （失败子集 + 全新快照）/ batch_jobs_list（跨重启状态合并）；目的地
  互斥锁（canonical dest，同目录第二 Job 拒绝）
- UI（下 §137-§151）：状态过滤五档 + 结果内搜索 + 分页表（100 行/页，
  有界 DOM §145）；Retry Failed 按钮 + 不可重试显式说明（§142）；暂停
  按钮；任务清单面板（settled/pending 计数 + Resume，§143）；恢复冲突
  展示；workers 选择；进度 aria-live；i18n zh/en
- 测试矩阵（下 §118-§135/§193-§199）：34 活跃 + 3 ignored——真实 fs
  （Unicode/嵌套/180 字符长名/独占锁句柄）、故障注入（碰撞/解码/写失败/
  源消失/SourceChanged 经 revalidate）、golden（10 输入 2-stage）+
  failure golden（7/3 稳定）、不变量（计数闭合/Failed 无产物/终态合法
  集）、JSON contract（JobResult/JobPlan/StageSpec roundtrip + 字段稳定
  + schemaVersion）、状态机 fuzz（2000 确定性种子迭代，非法迁移恒拒绝）、
  journal-resume 集成、perf 矩阵（100/1k/10k + workers 对比 + 取消延迟）
- docs：docs/BATCH_ENGINE.md（引擎/管线/恢复/错误/限额合一）；D55-D58；
  SECURITY/PRIVACY M7 专项；README Batch Engine 行；PERF（下）矩阵

Quality:

- Rust gates: PASS（clippy workspace -D warnings；fmt；weave-batch 34+3）
- Frontend gates: PASS（typecheck/lint/vitest 23）
- PERF 实测：见 docs/PERF.md M7（下）——10k files 3.21s 线性；workers
  1→4 同结果 1.6×；取消延迟 ~3ms
- tauri build: PASS（4.07 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口，A–E 全链路）: PASS — A：批量页
  Preview 诚实计数（5 输入 ⇒ 3/1/1，持锁文件=潜在失败）且盘侧零写入
  （§21）→ Execute 失败隔离 3 产物落盘；B：Retry Failed ⇒ 新 job 1/1
  成功（释放锁后）；C：6000 条任务暂停（成功 82 + 待续 5918，§143 诚实
  pending）→ 任务清单恢复 ⇒ 5918 全成功、盘侧恰 6000；D：History 撤销
  批量产物（创建型删除 + stat 守卫）；E：修复后闭环复核（产物删、输入
  完好）；全程 console 零错误
- **冒烟抓出 P0 并修复**：批量 History 事务误把输入路径记为
  source_path——creation-undo 的删除分支指向源文件（stat 守卫意外拦下，
  未造成实际删除）；已改为记录产物路径（与 text_service 创建型契约一致）
  并经 phase E 复核

Known Limitations:

- DiskFull/PartialWrite/进程级 TransactionFailure 故障注入未覆盖（Windows
  无便携注入手段；Write 失败由"目标为目录"故障覆盖）
- 目录只读属性在 Windows 不阻止写入（POSIX 语义差异）——写失败用可移植
  形态测试
- 单条大图 decode 阶段不可中断（取消生效于条目/阶段边界）
- M6 独立 image_batch_execute 保留（直接流）；§159 统一适配方向已记录，
  引擎侧无第二套批量逻辑
- 进程级 RSS 分档内存基准未做（payload 隔离 + workers 上限构成上界）——
  KNOWN LIMITATION 如实声明

## M8 Documents（上）

Status: COMPLETE

Implemented:

- weave-documents crate（M0 骨架填充）：DocumentDetector（§94 内容签名
  %PDF- → OOXML 容器 part 判别 → 文本启发 + 扩展名兜底；§95 mismatch
  警告）；DocumentIdentity/Facts（§9/§11 Field 四态 Known/Unknown/
  Unavailable/Estimated——解析失败 ≠ 0）；DocumentDiagnostic（§97）；
  capabilities(format) 能力矩阵按真实代码生成（§8/§61）；DocumentResource
  Limits 集中定义（§87/§100 保守初值）
- PDF 域（lopdf 0.45，D59）：inspect（页数/MediaBox 尺寸/Rotate/Info
  元数据/版本/加密识别 §16-§17）；PageRange 解析器（§21-§22 结构化错误：
  0/逆序/越界/坏 token）；Merge（§18-§20 对象重挂+Kids 拼接+首输入元数据
  策略 §80，**引用重写仅作用于新挂对象——全文档 traverse 因 id 空间重叠
  损坏 target，测试抓出后修复**）；Split every N / Extract（§23/§28
  part-NNN 确定性命名）；Reorder（§24 完整置换校验：缺页/重复/越界拒绝）；
  Rotate（§25 /Rotate 元数据语义累积 mod 360 + §26 如实说明）；输出校验
  （§27 重解析 + 页数一致）
- Office 检查（zip 8.6 + quick-xml，Inspect-only §29-§52）：DOCX（段落/
  标题 pStyle/表格/图片 media 计数/超链接/sectPr + core properties + 词数
  Estimated §34）；XLSX（sheets + visible/hidden/veryHidden §44、dimension
  与 populated cells 并列 §41、公式 presence 计数 §42、merged cells）；
  PPTX（slides + show="0" hidden、文本字符/形状/图片计数、notes presence）
  ；ZIP 炸弹守卫（§87 条目数/解压总量/单条目）；无 XXE 面（quick-xml 无
  实体处理 §88）
- M7 集成（§58-§60/§113-§116）：StageSpec::DocumentInspect（只读事实进
  stage 日志）+ PdfRotate（经 weave_documents::rotate_pdf_bytes 域函数，
  §218 无第二套 PDF 逻辑）+ Batch pdfRotate90 预设；§114 混入能力不符 ⇒
  Filter 拒绝 Skipped
- IPC（§109）：document_inspect（只读）；pdf_merge_preview/execute（§74
  单一逻辑操作）；pdf_extract/rotate/split_every_n_execute（output-first
  §66：只写新文件，目标存在拒绝；§67 Source Replace 不在（上））；产物
  入 History（§77/§78 what happened；共用 record_creation_transaction
  §205）
- UI：Documents 页（打开/路径 → 事实表 + sheets/slides 表 + 元数据 +
  诊断；PDF 操作区：范围提取/旋转/拆分 N/合并预览+执行）；i18n zh/en
- 决策：D59 依赖选型、D60 事实诚实化 + IPC f64 约定

Quality:

- Rust gates: PASS（clippy -D warnings；fmt；weave-documents 18 + weave-batch 38
  含 4 文档 stage 测试）
- Frontend gates: PASS（typecheck/lint/vitest 23）
- docs/PERF.md M8 节（synthetic 实测 + 限额初值）

Known Limitations:

- PDF 渲染预览 NOT SUPPORTED（§51/§53 capability 如实 false）
- Office → PDF NOT SUPPORTED（§92 保守：不依赖用户安装 Office）
- Encrypted PDF = 结构化拒绝（§84；密码输入 §85 列（下））
- 单文档 PDF 操作为同步命令（§105 取消：批量路径经 M7 已支持；单文档
  大文件取消列（下））
- Bookmarks/Page Labels/Annotations 保留行为未验证记录（§81/§82 列（下）
  专项测试）；大文档基线（§101/§102）列（下）
- tauri build: PASS（4.48 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口）: PASS — 文档页 inspect（PDF facts 含
  元数据标题/页数 Known(4)；DOCX facts 含 core properties/标题/表格计数）
  → rotate（output-first：二次执行 destinationExists 拒绝）→ extract 1-2
  → merge 预览（4+2=6）+ 执行 → History 撤销 = 删除 merged 产物、输入与
  其余产物保留；console 零错误
- **冒烟抓出 P1 并修复（fdf743f）**：record_creation_transaction 为
  HistoryEntry 生成了第二个 OperationId——按入口 id 撤销找不到事务
  （undo.unknownOperation）；入口现共享事务 id，经冒烟复核

## M8 Documents（下）

Status: COMPLETE

Implemented:

- Fixtures（§131-§135）：`crates/weave-documents/tests/fixtures/documents/`
  （valid×9 + malformed×4，gen_fixtures example 可复现产出；PDF 页含
  WeaveMarker 内容标记供顺序验证 §142；200 页大文档 §101 入集）
- TOCTOU（§141）：PdfMergePlan 携带输入快照（size+mtime），execute 前重
  校验 ⇒ `pdf.changedSincePlan` 拒绝盲执行（测试：篡改后拒绝且无产物）
- 原子输出（§178-§179）：PDF 写出 = 同目录 `.weave-tmp-<pid>` → 保存 →
  重解析校验 → rename promote；失败清理 temp（无半成品 final）
- Golden（§136-§137）：pdf/docx/xlsx/pptx inspect 期望值固定测试；
  merge 顺序 = 输入序（内容标记逐页断言 §142）；split 2-4,8-10（§143）；
  reorder 置换非排序（§144）；rotate 选定页/未选定页（§145）
- Property/Fuzz（§138-§139）：split 划分恰好覆盖一次（total×n 矩阵）；
  range 解析器 500 种子随机用例；300 随机字节样本 detect/inspect 无 panic
- 混合批量（§172-§173）：pdf/docx/xlsx/pptx 混合 inspect 聚合（stage
  note 承载格式事实）；zip/exe 混入 = unknown/unsupported 如实路由不 crash
- 批量吞吐基准（§168）：10/100/1000 docs ⇒ 2.5/1.58/1.43 ms/doc（线性）
- UI（§122-§124/§154）：merge 有序列表（上移/下移）、重复输入标记 +
  执行阻止、空状态
- docs：D61（原子化/TOCTOU/恢复语义/版本记录 §184）、ARCHITECTURE/SECURITY
  M8 边界、THIRD_PARTY_LICENSES（lopdf/zip 行 §161）、docs/DOCUMENTS.md
  （§190 能力矩阵 + §191 语言约定 + §192 已知限制）、PERF M8（下）、
  README（§188）

Quality:

- Rust gates: PASS（clippy -D warnings；fmt；weave-documents 18+15 /
  weave-batch 40+4）
- Frontend gates: PASS（typecheck/lint/vitest 23）
- 真窗口冒烟：见收口记录

Known Limitations:

- 大文档基线（§101/§102 500/1000 页、大型 Office）仅 200 页 fixture 入集，
  更大规模列 M9 复测
- 单文档同步命令的取消（批量经 M7 已支持）仍为已知限制
- M6 直接工具与 M8 均无交叉回归面（workspace 29 套全绿 = §195 PASS）
- tauri build: PASS（4.47 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口）: PASS — merge 预览（4+2=7 页口径）→
  预览后篡改输入 ⇒ 执行报 `pdf.changedSincePlan` 且盘侧零产物（§141
  TOCTOU，P0 修复验证）→ 重复输入警告 + 执行阻止（§124）→ 上移可用
  （§123）→ unicode 文件名 inspect（§175）；console 零错误
- **冒烟抓出 P0 并修复（0393e6a）**：merge execute 原实现从原始输入重建
  plan——预览快照被丢弃，§141 TOCTOU 形同虚设；现 preview 缓存
  PdfMergePlan（plan_id），execute 消费缓存 plan（失败放回允许重试），
  UI 传 planId

## M9 Utilities（上）

Status: COMPLETE

Implemented:

- weave-utilities crate（M9 §16 新建——workspace 无既有 utilities crate）：
  hash（RustCrypto MD5/SHA-1/SHA-224/256/384/512 + BLAKE3；按实际编码
  字节 §26-§27；弱算法 Warning §31；能力矩阵 §24）；checksum（§32-§36
  CRC-32/ISO-HDLC + CRC-32C 标准参数 + Adler-32 手写，"123456789"/
  "Wikipedia" golden 向量，zlib 交叉验证）；base64（§37-§43 Standard/
  URL-safe × Padded/Unpadded、显式编码 §39、strict/lenient 解码 §40、
  解码上限 §43）；uuid（§44-§50 v4/v7 CSPRNG、batch ≤10,000、四种格式、
  validate/parse 版本/variant）；timestamp（§51-§58/§100-§101 显式单位、
  auto-detect 标注、RFC3339→UTC 归一、Fixed Offset、Clock trait 注入）；
  url（§59-§64 Component/Query 语义分离、strict/lenient 解码、Full
  parse、UTF-8 percent 确定性）；regex（§65-§74 唯一 engine、能力矩阵
  如实——lookaround/backref Unsupported、match 含行/列/组、资源上限、
  线性引擎反回溯测试）；color（§75-§88 sRGB HEX/RGB/HSL/HSV/HWB strict
  解析、round-trip 容差、WCAG 对比度）
- 42 域测试（RFC 4648/RustCrypto/标准 CRC 向量 golden；round-trip；
  边界/错误路径/确定性 §98/§102-§104）
- IPC：utilities_service 17 命令（UtilityError→IpcError 映射 §22；DTO
  f64 约定）；UI：Utilities 统一单入口 8 工具 tabs（§120/§121）+ Copy/
  Copy All（§30/§89）+ 能力矩阵展示 + 空状态；i18n zh/en

Quality:

- Rust gates: PASS（clippy -D warnings；fmt；weave-utilities 42）
- Frontend gates: PASS（typecheck/lint/vitest 23）
- 真窗口冒烟：见收口记录

Known Limitations:

- 文件哈希 UI 沿用 M1 Inspector（hash_file SHA-256 流式任务）——M9 页
  只做文本哈希（§5/§28 复用，无第二文件引擎）
- Named Timezone/DST NOT SUPPORTED（§55-§56 如实；UTC/Fixed Offset/Local
  支持）；GB 族编码 Base64 输入指向 M4 管线（§39 复用 D38）
- 文件 Export（§93）不在本批（文本工具 v1 = 剪贴板复制）；批量经 M1
  hash job / M7 既有能力，M9 专用批量 UI 列（下）
- tauri build: PASS（4.58 MiB NSIS）
- Real UI smoke（CDP 驱动真实窗口，8 工具 tab 逐一）: PASS — Hash SHA-256
  golden "abc"；CRC-32 "123456789"=cbf43926；Base64 foobar↔Zm9vYmFy 往返；
  UUID 生成 5 条+校验 RFC 4122；时间戳 1720000000→2024-07-03T09:46:40Z +
  auto-detect 显示 Seconds；URL component %20 / query + 双语义；正则分组
  + 位置 [1:1] + 能力矩阵（lookaround=不支持 §67）；颜色 #ff8800→#FF8800
  归一 + 对比度；console 零错误
