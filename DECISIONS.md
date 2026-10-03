# DECISIONS

只记录影响未来架构的决定（M0 §29）。按序号追加，不删除。

## D1 — Tauri 2 作为桌面框架

charter #22 指定。Rust 核心 + 系统 WebView2，local-first 友好，无 Electron 级体积。

## D2 — Cargo Workspace 一次建满 10 个 crate 骨架

M0 一次性建立 charter #22 的全部 crate（仅 weave-core / weave-testkit / weave-app 有内容）。
理由： crate 边界即架构边界，先立住可避免 M1+ 的结构性搬迁；空 crate 用"骨架 + 一行文档"约束，
禁止预填未来类型。

## D3 — IPC：tauri-specta 生成契约

选 `tauri-specta 2.0.0-rc.25 + specta 2.0.0-rc.25 + specta-typescript 0.0.12`（三者版本互相钉死）。
稳定版 1.x 只支持 Tauri v1，故使用 RC 线（Tauri 2 生态事实标准）。绑定生成到
`ui/src/generated/bindings.ts` 并提交仓库：CI/前端 typecheck 不依赖本地 Tauri 运行。
Result 命令在 TS 侧是 `{status:"ok"}|{status:"error"}` 联合，错误是值不是异常。

## D4 — IPC 边界的 u64 → f64

specta-typescript 0.0.12 禁止导出 u64/i64（BigInt 精度守卫）且无配置项。
IPC DTO 中字节量等大整数用 f64（JSON number 的实际类型，≤2^53 无损）；
领域层（weave-core）保持 u64 不变。仅影响 IPC 表面，不影响领域模型。

## D5 — 品牌单一事实源：brand.json 编译期嵌入

`brand.json` 经 `include_str!` 嵌入 src-tauri（build-time 策略，避免运行时读仓库根目录）。
运行时版本号取自 Cargo 包信息（brand.json `versionSource: "package"`）。

## D6 — 不引入 SQLite / ORM / i18n 框架 / 重型前端依赖

- 配置用 versioned + validated 的 JSON 文件（M0 §42）；SQLite 留到 M2 History 真正需要时。
- i18n 用 ~30 行手写 loader + zh-CN/en JSON 资源，不引入 i18next。
- 状态只有 zustand；无 UI 组件库（token + 少量内联样式，M0 不需要更多）。

## D7 — 错误模型手写实现

`WeaveError`（五字段：Code/Message/Location/Recoverability/Suggestion + 类别枚举）手写
Serialize/Display/Error impl，不引入 thiserror——实现约 40 行，省一个 proc-macro 依赖链。

## D8 — Windows bundle 仅 NSIS

`bundle.targets = ["nsis"]`：M0 不引入 WiX/MSI 下载链。后续里程碑需要 MSI 时再评估。

## D9 — license 白名单相对 charter #40 的扩展

deny.toml（Rust 侧）：MPL-2.0、Unicode-3.0、CC0-1.0、`Apache-2.0 WITH LLVM-exception`。
前端（check-frontend-licenses.mjs）：Python-2.0、CC-BY-4.0（纯数据包）、MIT-0、OFL、CDLA、ISC、0BSD。
逐项理由见两处配置内注释。拒绝 GPL/AGPL/LGPL 不变。

## D10 — clippy::result_large_err 豁免（src-tauri/src/commands.rs）

IPC 命令返回 `Result<T, IpcError>`（Err ~144 字节）。IPC 每次调用都跨序列化边界，
此 lint 针对的热路径场景不成立。文件级 `#![expect]` + 注释记录。

## D11 — Tool trait M0 即建签名

8 成员 trait + `UnsupportedTool` 参考实现（方法体返回 Unsupported 错误），
契约测试证明签名可用。M2 的 OperationTransaction 字段级落地前，
weave-core 只留 Preview.reversible 与 History 概念边界（M0 §22 授权推迟，已记录）。

## D12 — Windows 测试可执行文件需要 common-controls manifest

链接 Tauri/wry 的测试 exe 若无 manifest，加载器取 comctl32 v5，启动即
STATUS_ENTRYPOINT_NOT_FOUND。tauri-build 只给 bin 嵌 manifest，因此 src-tauri/build.rs
为 test 目标注入 `/MANIFEST:EMBED /MANIFESTINPUT`。全部测试放在
`src-tauri/tests/`（集成测试目标）而非 lib 内嵌 `#[cfg(test)]`。

## D13 — 占位应用图标由脚本生成

`scripts/generate-icon.mjs`（纯 Node PNG 编码器，无图像库依赖）生成
`assets/icon-source.png`，`tauri icon` 派生各平台格式。这是工程占位而非品牌资产。

## D14 — 版本策略

Node 24 / React 19 / Vite 8 / TS 5.9 / ESLint 9（flat config）/ Tauri 2.12 / Rust stable-msvc。
TypeScript 7 与 ESLint 10 发布初期工具链兼容性未验证，刻意保守。锁定文件
（Cargo.lock / package-lock.json）提交进仓库。

## D15 — Filesystem 抽象形状（weave-files::fs）

最小面：exists / stat(lstat) / read_dir / open_read。`stat` 返回自有 `FileStat`
而非 `std::fs::Metadata`——后者无法手工构造，会堵死测试替身与故障注入。
备选：全量包装 std::fs（拒绝：面太大且不可测）。`read_dir` 按目录批量返回
（目录内条目有限，内存可控；全仓 streaming 由 scanner 的栈式遍历保证）。

## D16 — Hash：SHA-256 via sha2，chunk 256 KiB

不自研密码学原语；sha2（RustCrypto，MIT/Apache）是成熟最小实现。
只做 SHA-256（charter：只实现当前确认需要的能力，不一次加 MD5/BLAKE3 等）。
256 KiB chunk：摊薄 syscall 与内存常数之间的折中，单一 pub 常量可调。

## D17 — Changed-during-hash：best-effort + Unstable 状态

前后各 stat 一次（size + modified），不一致 ⇒ `HashStatus::Unstable`，
digest 照常返回。选择 best-effort 而非直接报错：哈希本身只读、无破坏性，
但用户必须显式看到状态——禁止静默返回"看似最终"的摘要（M1 §10.3）。

## D18 — Symlink 策略：lstat、不跟随、计为 other_entries

所有 kind 判定基于 symlink_metadata（lstat）：链接条目报 `Symlink`，
walker 永不进入，哈希拒绝非 RegularFile。循环在结构上不可能；
visited 集合仅为纵深防御（junction/未来语义变化兜底）。
理由：扫描失控（目录环、跨 root 漂移）比漏扫链接内容危害大；
链接目标事实留给未来的显式"跟随"选项（M3 起按需）。

## D19 — 目录语义：root=0；空=无直接文件且无直接子目录；大小=普通文件之和

深度 root 为 0（子目录逐层 +1）。空目录定义采用"直接子代"口径（不含间接）。
total_size 只累计 RegularFile 逻辑大小；symlink 不跟随因此不计入。
全部写入 ScanReport 结构化字段，UI 不自行解释（M1 §12.4/§12.5）。

## D20 — 扫描失败聚合：单条失败 → 记录 + 继续；root 失败 → Failed

对齐 charter #10.3（998 成功 / 2 失败）。错误进入有界错误列表
（100 条 + truncated 标志，error_count 记录全量）。
资源上限（depth 64 / entries 200k，硬顶 512 / 5M）触发 ⇒ `limited` +
原因，状态 CompletedWithWarnings——绝不假装完成（M1 §13）。

## D21 — 文件类型：扩展名证据优先，有界 magic 嗅探补位，Unknown 正常

扫描路径只按扩展名分类（10k 文件逐一嗅探不可接受）；Inspector 单文件路径
在扩展名缺失/未知时读 ≤8 KiB 头部嗅探。分类必带 evidence
（Extension/MagicBytes/None）可追溯；`.bin` 永不凭空猜 Executable。

## D22 — 编码检测 M1 范围：BOM + UTF 系；GB18030 → M4

BOM 优先（UTF-8/UTF-16LE/BE）→ ASCII → UTF-8 有效性（采样截断尾部裁剪）。
非 UTF 系如实 Unknown（不猜测）；GB18030/GBK/Latin-1 属 charter #27，
推迟到 M4 Encoding Detection——已在本文件与 Known Limitations 记录。

## D23 — IPC DTO 与领域类型分离；JobTracker ≠ Batch Engine

weave-core/weave-files 类型不加 specta 派生；src-tauri/files_dto.rs 建立独立
公开契约（大整数 f64 = D4、时间 epoch ms）。JobTracker 只做"任务 id +
取消令牌 + 进度共享"，无队列/重试/持久化——那是 M7 的 Job 模型。

## D24 — Windows 测试可执行文件 manifest（src-tauri）

链接 Tauri/wry 的测试 exe 无 manifest 时加载 comctl32 v5 启动即崩
（STATUS_ENTRYPOINT_NOT_FOUND）。tauri-build 只给 bin 嵌 manifest，
故 build.rs 为 test 目标注入 `/MANIFEST:EMBED /MANIFESTINPUT`；
src-tauri 单测统一放 tests/ 集成目标。

## D25 — Open File/Folder 用官方 dialog 插件（M1 §20）

@tauri-apps/plugin-dialog 2.8.1 + tauri-plugin-dialog 2，capability 追加
dialog:default（用户显式发起的选择器，不构成静默权限扩大）。

## D26 — dev-only 自动化冒烟钩子

`window.__weaveDev.dispatch(path)` 仅在 `import.meta.env.DEV` 下挂载
（生产构建常量折叠 + 死码消除）。它触发与 Drop 完全相同的
dispatchPath 链路，使真实窗口的 UI 冒烟可自动化（CDP 驱动）；
OS 输入层（拖拽手势/原生对话框）仍是人工验证项。

## D27 — Open 按钮对话框在本机挂起（P1，BLOCKED）

本机（Windows 11 26200 + WebView2 Edg/154）上 plugin-dialog 的
`plugin:dialog|open` IPC 被接受但对话框窗口永不出现（UIAutomation 与
EnumWindows 均无窗口、无 panic、invoke 永久 pending）。已确认与前端代码
无关（原始 invoke 同样挂起）。JS 侧已补 catch 使任何对话框失败可见。
Drop 入口不受影响（Tauri 运行时原生处理）。M2 排查方向：最小复现仓库
+ rfd/COM 线程模型；若确认为环境特异则关闭。

---

# M2 Decisions（D28–D32）

## D28 — 依赖：regex + chrono（M2 §44 评估记录）

- `regex 1.x`（RustCrypto 生态，MIT/Apache）：Rename RegexReplace 与
  Organizer NamePattern 需要；编译错误映射为结构化 `rename.invalidRegex`。
- `chrono 0.4`（default-features=false + clock/std，MIT/Apache）：Date 规则
  需要本地时区日期格式化（YYYY-MM-DD/YYYYMMDD/YYYY-MM）。UTC 无法表达
  用户语义的"拍摄日期"，自研时区转换不可靠，故引入。
- 两者均为高维护性成熟依赖；转递依赖已过 cargo deny + 前端 337 包审计。

## D29 — History 存储形态：版本化 JSON + 原子写 + 损坏隔离（M2 §54）

- `entries.json`（schema_version=1，容量 500，最新在前，Atomic temp+rename）
  + `transactions/{op_id}.json`（每操作一份，InProgress→Completed 两阶段
  落盘，M2 §82 crash safety）。
- 拒绝 SQLite：§54 明确"最简单可靠的持久化形式"，M2 历史无并发写、无
  部分查询需求，JSON 足够且损坏可肉眼诊断；M7 如需查询再评估。
- 损坏策略：解析失败 ⇒ `*.corrupt-{ts}` 隔离 + 空状态安全降级，绝不阻塞
  启动；未知 schema 版本同样隔离（不静默猜测迁移）。
- 容量淘汰仅删 entries 条目，事务文件保留（淘汰条目的 Undo 仍可用）。

## D30 — 跨文件系统 Move 明确拒绝（M2 §71）

执行器校验 source/target 卷前缀（盘符或 UNC server+share），不匹配 ⇒
`move.crossVolumeUnsupported` 结构化失败，条目原样保留。拒绝 copy+delete
模拟：无可靠的"copy→flush→verify→remove→rollback"事务模型前，宁可拒绝
也不假装支持（M2 §71 授权，charter #116"宁可拒绝执行"）。

## D31 — PlanCache 内存态 + 模板消费扩展名语义

- **PlanCache**：build 产生的 Plan 以 operation_id 为键缓存在服务端内存，
  execute 仅收 operation_id —— 源快照（size/mtime）不过 IPC，且天然满足
  §11"Preview 与 Execute 同一 Plan"。应用重启后缓存清空 ⇒ execute 返回
  `plan.unknownOrExpired`（需重建 Plan）；事务本身已落盘，crash safety
  不受影响。M7 持久化 Job 时再评估 Plan 落盘。
- **模板语义**：模板（若提供）为最后一步，组合出**完整最终名**并无条件
  消费扩展名——`vacation-{counter}.{ext}` 与 `new-{counter}.txt` 都产出
  单扩展名的正确结果（无 `.jpg.jpg` 双拼）。非模板路径下 Prefix/Suffix/
  Replace/Regex 仅作用于 base，扩展名受 Extension 规则显式控制。
- **环判定修正**：`a→b` 且 `b→b(NoOp)` 不是环——NoOp 条目永不让位，
  `a→b` 是真实的 ExistingTarget。环判定排除 NoOp 条目的 source。

## D32 — Undo 的 LIFO + 占位暂存（swap/cycle 可还原）

朴素 LIFO 在 swap 场景必然全冲突（互为原位占据者）。算法：撤销某条目
时若原位被同事务**待撤销条目**的当前位置占据，将占据者暂存到
`.{stem}.weave-undo-tmp{ext}` 并重定向该条目的还原起点为 tmp；后续轮次
tmp→其 source。不变量：全部条目处理后 `leaked_temps == 0`（测试断言）。
外部文件占据（不在事务内）⇒ UndoConflict 拒绝覆盖（M2 §49）。

## D33 — Partial hash 常量与策略（M3 §4）

`PARTIAL_HASH_BYTES = 4 KiB`，SHA-256 over (head N + tail N)，前缀
"weave-partial-v1"。单一集中定义（pipeline.rs 常量），理由：头部特征
（格式头/元数据差异）+ 尾部特征（追加差异）以 2N 字节 I/O 覆盖大部分
真实差异；≤ 2N 的文件等价全量，无额外成本。明确局限：中部差异对
partial 不可见——**设计内行为**，由 full hash 兜底；partial 相同绝不
视为重复（M3 §3），partial 不同绝不进入 full hash（候选缩减的全部
价值所在，Scenario B 实测 100% 裁剪）。无随机采样（M3 §4 硬边界：
确定性、可重放、结果可解释）。

## D34 — trash crate 依赖与回收站适配器隔离（M3 §22–§26）

依赖 `trash` 5.2.9（MIT，无传递重依赖）：Windows 走 IFileOperation
回收站 API（进程内调用，非 shell 进程——不违反 M3 §24 禁 shell）。
平台适配器隔离：Shell/回收站 API 只出现在 `recycle.rs`，Domain
（duplicates/*）只面向 `RecycleOutcome` 枚举（Success/Unsupported/
Failed），不泄漏平台类型（M3 §94）。**默认动作 = Move to Recycle Bin，
不存在永久删除路径**（硬边界，唯一删除动作）。

## D35 — 回收 token 与 Undo 匹配语义（M3 §58–§60）

事务 target 记 `recycle-bin:{token}`：token = 平台回收条目 id 的
Debug 串（Windows 可枚举回收站拿到 original_path + id + 删除时间）。
回收后反查 token（original_path 精确匹配 + 删除时间 ±5s 窗口，唯一才
记录）；不可得时记空前缀，Undo 退化为 original_path + 时间窗匹配。
匹配不到（回收站被清空）⇒ Missing；不唯一或原位被外部重建 ⇒
UndoConflict，**不恢复、不覆盖**（M3 §67）。恢复用
`trash::os_limited::restore_all` 批量执行，恢复后逐条验证原位存在，
不轻信平台返回值。跨 token 串稳定性：同一 crate 版本内 Debug 格式
确定，比较双方均为当次运行生成——不持久化解析 token。

## D36 — ScanCache：扫描快照服务端缓存（M2 PlanCache 同源）

scan_duplicates 的报告以 scan_id 缓存服务端内存；build_recycle_plan
只收 scan_id + 选择——扫描快照（含每个文件的 size/mtime/hash）不过
IPC，天然满足"Preview 与 Execute 同一快照"。内存态：重启后 scan_id
失效 ⇒ `duplicates.scanUnknownOrExpired`（提示重扫）；大报告驻留
内存的规模边界与 PlanCache 一致（会话级，M7 Job 持久化时统一评估）。
