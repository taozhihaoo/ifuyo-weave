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
