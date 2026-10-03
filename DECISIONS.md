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
