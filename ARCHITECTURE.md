# ARCHITECTURE

Weave 的架构边界。任何偏离本文档的代码都是 bug——要么改代码，要么先改本文档并记录 DECISIONS.md。

## 分层 / Layers

```text
┌─────────────────────┐
│     React / TS      │  ui/ — 只能通过 IPC 与 Rust 通信
└──────────┬──────────┘
           │  typed IPC（tauri-specta 生成 ui/src/generated/bindings.ts）
┌──────────▼──────────┐
│    Tauri / App      │  src-tauri — 桌面集成、IPC 命令注册
└──────────┬──────────┘
┌──────────▼──────────┐
│    Application      │  src-tauri/src/* — 校验输入、调用 core、结构化结果
└──────────┬──────────┘
┌──────────▼──────────┐
│    weave-core       │  领域契约：Tool trait / Error / Path Safety / Registry
└──────────┬──────────┘
┌──────────▼──────────┐
│     Adapters        │  文件系统 / OS / 编解码（M1 起逐步物化）
└──────────┬──────────┘
┌──────────▼──────────┐
│   OS / File System  │
└─────────────────────┘
```

## 硬性规则 / Invariants

1. **UI 不直接操作文件系统**——React 组件禁止 import 任何 fs/path 能力，一切经由 IPC。
2. **weave-core 不依赖 Tauri/React/GUI**——纯领域逻辑 + serde；`std::fs` 只允许出现在测试中。
3. **IPC 契约是生成物**——`ui/src/generated/bindings.ts` 由 `npm run generate:bindings` 生成、提交进仓库；禁止手改。
4. **错误统一翻译**——Domain Error（weave-core `WeaveError`，五字段）→ Application 层转 `IpcError` DTO → UI 呈现。UI 文案不进领域层。
5. **路径先验证后使用**——一切文件操作走 `weave_core::path`（Normalize → Validate → Resolve → Operate）。
6. **无循环依赖**——crate 级由 cargo 保证；模块级靠 code review + 审计。

## Cargo Workspace / 依赖方向

```text
weave-core
    ↑
weave-files / weave-text / weave-data / weave-documents /
weave-media / weave-batch / weave-history / weave-search
    ↑
weave-app (src-tauri)
```

| crate | 职责 | M0 状态 |
| --- | --- | --- |
| weave-core | ID / 错误模型 / Tool trait / Registry / 路径安全契约 / Progress / Cancellation / Preview / OperationResult | ✅ 已建立 |
| weave-testkit | TempWorkspace / DetRandom / FaultInjector | ✅ 已建立 |
| weave-files | 文件与目录工具（M1 File Core / M2 / M3） | 骨架 |
| weave-text | 文本工具（M4） | 骨架 |
| weave-data | CSV / JSON / TSV（M5） | 骨架 |
| weave-media | 图片（M6） | 骨架 |
| weave-batch | 统一批处理引擎（M7）：JobPlan/Preview/Execute | ✅ M7（上） |
| weave-documents | PDF / Office（M8） | 骨架 |
| weave-history | 操作历史（M2+） | 骨架 |
| weave-search | 工具/命令搜索（M11） | 骨架 |
| weave-app | Tauri 壳 + Application 层 + IPC 契约 | ✅ 已建立 |

## File Core 边界（M1）

一切文件系统访问经 `weave_files::fs::Filesystem`（lstat 语义、可注入故障）；
一切路径先过 `weave_core::path`。扫描/哈希跑在 spawn_blocking 任务里，
进度/取消经 JobTracker（IPC 管道，非 M7 引擎）。详见 docs/file-core.md。

## M2 Rename/Organizer 边界

```text
m2_commands (IPC) → rename_service (编排 + PlanCache) → weave-files
   ├── rename/: rule.rs（9 类规则纯函数管线）+ plan.rs（快照 + 四类碰撞）
   ├── execute.rs: 逐项 Revalidate → 直通 / 两阶段（CaseOnly+Cycle 临时让位）
   ├── organize.rs: 非递归 root 快照 + first-match-wins + root 边界
   └── undo.rs: LIFO + swap 占位暂存（leaked_temps==0 不变量）
weave-history: 事务 InProgress→Completed 两阶段落盘 + entries.json（版本化）
```

- **Plan 是事实快照**：Preview 与 Execute 共用同一 Plan（服务端 PlanCache
  以 operation_id 缓存，源快照不过 IPC）；Execute 前逐项 Revalidate，
  外部变化 ⇒ `rename.sourceChanged` 安全失败，绝不智能纠正。
- **No Overwrite**：执行时目标重现 ⇒ 该项 Failed；默认无 Overwrite 策略。
- **事务按实际顺序**：两阶段 temp 步亦入事务，支撑 swap/cycle 的 LIFO
  Undo（占位暂存）；崩溃遗留 InProgress 事务被 Undo 明确拒绝。
- 语义全文：docs/file-core.md（M2 章节）。

## M3 Duplicate Finder 边界

```text
m3_commands (IPC) → duplicates_service (编排 + ScanCache) → weave-files
   ├── duplicates/pipeline.rs: 三级管线（size → partial → full SHA-256）
   ├── duplicates/recycle_exec.rs: 逐项 Revalidate → 适配器回收 → 事务记账
   ├── recycle.rs: 平台适配器（Shell API 唯一出现处；Move to Recycle Bin）
   └── undo: undo_recycle_transaction（token/时间窗匹配恢复，不覆盖）
weave-history: 复用 M2 事务/历史（kind = duplicateRecycle）
```

- **快照不过 IPC**：ScanCache（scan_id）/ PlanCache（operation_id）
  服务端内存缓存，同 M2 纪律；重启即失效（结构化错误）。
- **回收站是唯一删除动作**：domain 只见 `RecycleOutcome`；Undo 匹配
  token，原位被占 ⇒ UndoConflict 不覆盖（M3 §67）。
- 语义全文：docs/file-core.md（M3 章节）。

## M4 Text 边界

```text
m4_commands (IPC) → text_service (编排 + TextWriteCache) → weave-text
   ├── encoding: 解码级联（BOM/UTF-8/GB18030/Latin-1，零 U+FFFD）
   ├── model/offset/diagnostics/detection: TextDocument/UTF-16 列契约
   ├── format/*: 七格式能力矩阵（诚实 Unsupported，§46）
   ├── transform / extract / compare: 纯函数域引擎
   └── （写回）weave-files::atomic_write + M2 Plan/Transaction/History/Undo
```

- **Offset 契约**：域内 byte；IPC = byte + line + UTF-16 列（§18）。
- **写回**：快照 TOCTOU（§90）→ 备份 → 原子替换；Undo 按 kind 分派
  （TextTransform 备份还原 + §94 用户改动拒覆盖），无平行 undo 系统。
- 语义与边界：DECISIONS D38–D43。

## M5 Data 边界

```text
data_service (IPC) → weave-data → weave-text / weave-files(atomic_write)
   ├── csv_parse: csv crate 事件解析 + 头语义/ragged/诊断（D45）
   ├── json_doc/jsonl: 观测 schema profiling + 流式 JSONL（D46）
   ├── inspect/convert: potential 类型/唯一值 + 四向转换（D46）
   ├── session: 非破坏视图 filter/sort/paging（D47）
   └── transform: DataTransformPlan 有序清洗规则（D46/§79-§81）
AppState: DataSessions（ephemeral ≤8；§18 不是数据库）
```

- **复用 M4**：解码级联/诊断/TextLimits 同源纪律/atomic_write/
  TextTransform 导出管线（§243：无第二套文本 I/O）。
- **导出只写新文件**：目标存在 ⇒ 拒绝；覆盖源走 M4 管线（§72）。

## Tool Contract（charter #24）

```text
Tool
├── id() -> ToolId            // "<category>.<name>"，稳定机器可读
├── category() -> ToolCategory
├── input_types() -> Vec<InputKind>
├── options() -> Vec<ToolOptionSpec>
├── preview(input) -> Result<Preview, WeaveError>       // 无副作用
├── execute(input, cancel, report) -> Result<OperationResult, WeaveError>
├── can_undo() -> bool                                  // 默认 false，如实声明
└── undo(operation) -> Result<(), WeaveError>
```

M0 只建立 Rust 类型签名（`UnsupportedTool` 为参考实现）；真实工具从 M1 起按
`Specification → Test Fixture → Implementation → Integration → UI → Audit` 加入。

## IPC 契约

- 命令集中在 `src-tauri/src/lib.rs::ipc_builder()` 注册。
- 类型经 tauri-specta 生成；Result 命令在 TS 侧呈现为
  `{ status: "ok", data } | { status: "error", error: IpcError }` 联合类型。
- Tauri capabilities 最小化：当前仅 `core:default`（无 fs / shell / process / network）。

## 测试架构

```text
Unit（weave-core / weave-app tests/）
  → Integration（fixture 驱动，weave-testkit）
  → Contract（IPC 绑定生成测试 + 前端契约测试）
  → Smoke（真实启动，M0 Gate G）
```

前端测试：vitest + @testing-library/react（jsdom）；IPC 以 mock 边界做单元测试，
真实链路由运行时冒烟验证——mock 只用于测试隔离，不冒充成功（M0 §9.2）。

## Batch Engine 边界（M7 上）

领域逻辑全部在 weave-batch：Job 状态机（显式合法迁移表）、输入快照
（path/size/mtime + ChangedSincePreview 重校验）、Linear Pipeline 校验
（结构 + 类型跟踪）、同引擎 Preview（dry-run Export）/Execute、item 级
失败隔离与协作取消。src-tauri 只做 DTO 投影（batch_service）与任务化
（JobTracker + get_job/cancel_job 复用，无第二套任务管道）。变换实现
复用既有适配器：文本 = weave_text TransformKind，图像 = weave-media
inspect/resize/encode（含 ImageLimits 守卫）——不复制第二份 I/O 或
编解码层。持久化（Journal）与 Resume/Retry 语义属 M7（下）。

## Documents 边界（M8）

领域 = weave-documents（detect/facts/page_range/pdf/office）；适配 = lopdf
（PDF）/ zip+quick-xml（OOXML），crate-specific API 不出适配层（§182）。
批量经 M7 StageSpec::DocumentInspect/PdfRotate（§58/§202 无第二套批量）；
单文档 mutation = output-first（新文件，temp+校验+promote §178）+ 共享
record_creation_transaction 入 History（创建型撤销 = stat 守卫删除）。
UI 不解析文档（§3/§200 parser leakage 禁止）。资源限额集中
DocumentResourceLimits（§100）。隐私：无网络/无内容快照/日志只记
operation id 与计数（§164-§166）。

## Utilities 边界（M9）

域 = weave-utilities（hash/checksum/base64/uuid_tool/timestamp/url_tool/
regex_tool/color —— 每工具一模块，无 God Manager §198）。分层 =
UI → utilities_service（IPC 映射 + UtilityError→IpcError）→ weave-utilities
→ 既有底层（M1 文件哈希流式、M4 编码层、regex engine）（§17）。无
History/Undo（纯转换 §91）；无文件输出（本版）；批量经 M7 既有能力
（§95）。React 无核心语义（§18）。隐私：全部本地计算，复制内容不进
日志/遥测（§90）。

## Workflow 边界（M10）

weave-workflow = 组合层（模型/Registry/Validation/Compiler），不承载
执行（§2.2 执行归 M7）、Safe Write（§2.3 归 M2）、脚本（§2.4 禁止）。
Compiler 产物 = weave_batch::JobPlan——Preview（dry-run）与 Execute
共用同一 plan（§33）。持久化 = workflows/<id>.json（atomic_write，
schemaVersion=1）。UI = WorkflowPanel（Library + 垂直 step builder，
v1 无节点画布 §52）。无遥测/云（§2.5/§99）。
