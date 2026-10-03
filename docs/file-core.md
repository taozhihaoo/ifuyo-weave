# File Core（M1）

weave-files 的架构与语义参考。变更任何语义前先改本文档 + DECISIONS.md。

## 分层

```text
Application (src-tauri commands / files_dto)
   ↓
weave-files: inspector / scan / hash  ← 领域服务
   ↓
weave-files::fs::Filesystem           ← 唯一的文件系统边界
   ↓
StdFilesystem（生产）/ FaultFilesystem（测试故障注入）
```

weave-core 提供稳定契约（FileKind / HashResult / ScanStatus / TextEncoding /
WeaveError / CancellationToken / Progress）与路径校验（weave_core::path）。

## Filesystem 抽象

四个操作：exists / stat(lstat) / read_dir / open_read。lstat 语义（不跟随
symlink）。测试通过 FaultFilesystem 在操作粒度注入故障
（StatDenied / StatVanished / ReadDirDenied / OpenLocked / ReadFailure-after-partial）。

## 路径安全

一切输入先过 `weave_core::path::validate_absolute_path`：
盘符 / UNC / `\?\` 长路径前缀 / 保留设备名 / 非法字符 / 尾部点空格 /
`..` 穿越（相对路径可保留前导 `..`，绝对路径一律拒绝）。
大小写不敏感冲突契约：`paths_conflict`（M2 碰撞检测地基）。

## Hash

SHA-256（sha2），256 KiB chunk 流式，O(chunk) 内存。协作取消 ⇒
`Cancelled` 结果（digest=None）。前后 stat 对比 ⇒ `Unstable` + best-effort
digest（绝不静默）。只接受 RegularFile。

## File Inspector（files.inspect）

stat 优先立即返回；仅 RegularFile 且有内容时做 ≤8 KiB 嗅探。
分类带证据链（Extension / MagicBytes / None），编码仅类文本场景给出
（BOM 命中或 Text 类别），非 UTF 系如实 Unknown（GB18030 → M4）。
嗅探失败 ⇒ Partial + warning，元数据仍完整可用。

## Directory Analyzer（files.analyze_directory）

栈式 DFS，逐目录名称 case-insensitive 排序（确定性）。增量统计：
count / total size（仅普通文件）/ 类型分布（扩展名证据）/ Top-20 有界榜
（size DESC、time ASC/DESC，并列 path ASC）。空目录 = 无直接文件且无直接
子目录；root 深度 = 0。单条目失败聚合为错误记录（100 条封顶 + truncated），
root 打不开整体 Failed。资源守卫：max_depth（默认 64，硬顶 512）、
max_entries（默认 200k，硬顶 5M）——触发即 `limited` + 原因。
symlink/junction 计入 other_entries，永不跟随。

## 并发与取消

扫描/哈希在 spawn_blocking 任务中执行；进度经 JobTracker 轮询
（400ms，total 未知时不产出百分比）；取消是协作令牌，安全点检查。

## 已知限制

- GB18030/GBK/Latin-1 编码检测未实现（M4）
- 扫描分类不做内容嗅探（扩展名错误的大文件会归 Unknown——符合事实优先）
- 取消延迟无硬指标（M7 引擎统一）

## Rename / Organizer 语义（M2）

### Plan
Preview 与 Execute 共用同一结构化 Plan（operationId/kind/items）。Plan 是
快照：Execute 前逐项 Revalidate（source 存在 + size/mtime 匹配），变化 ⇒
`rename.sourceChanged` 安全失败，不智能纠正。

### 规则管线（顺序固定）
规则按用户列表顺序逐条应用（前条输出=后条输入）；模板（若提供）最后
重组完整最终名并无条件消费扩展名。Counter 整体替换 base；Date 整体替换
base；Prefix/Suffix/Replace/Regex 仅作用于 base（扩展名保护，§17）。
排序：完整路径 case-insensitive ASC（§20），序号基于该顺序。

### 碰撞（四类）
- ExistingTarget：目标已在盘上 ⇒ Conflict（默认 No Overwrite）
- InternalTarget：批内多条目同目标 ⇒ 全部 Conflict
- CaseOnly：仅大小写不同 ⇒ Ready（两阶段执行）
- Cycle：target 是批内另一条目的 source（该条目非 NoOp）⇒ Ready（两阶段）
- source==target ⇒ NoOp

### 执行
- 直通条目：Revalidate → rename → verify
- 两阶段：先全部让位到 `.weave-tmp-{op}-{n}` 唯一临时名，再落到最终名
- 取消在条目间安全点生效；取消后剩余项 outcome=Cancelled/NotExecuted
- 事务按**实际应用顺序**记录（含 temp 步），支撑 LIFO Undo

### Undo
LIFO 逆序 + 占位暂存：原位被同事务待撤销条目占据（swap/cycle）时，
占据者先暂存到 `.weave-undo-tmp` 并重定向其还原起点。安全检查：
目标缺失/外部修改（size+mtime）⇒ Missing/UndoConflict，绝不覆盖。
`leaked_temps` 不变量：全部条目处理后暂存必须清零。

### History
`%APPDATA%/ifuyo/Weave/history`：entries.json（版本化 schema，容量 500，
最新在前）+ transactions/{op}.json（InProgress→Completed 两阶段落盘）。
原子写（temp+rename）；损坏 ⇒ 隔离 *.corrupt-* 并安全降级，绝不阻塞启动。
