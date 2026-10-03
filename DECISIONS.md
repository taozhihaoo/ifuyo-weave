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

## D37 — M3 语义定案：硬链接 / 无哈希缓存 / 顺序管线 / 结果排序

- **Exact Duplicate 定义**：size 相同且完整 SHA-256 相同（M3 §2）。工程判定
  基于密码学哈希，不宣称数学绝对证明；文件名/mtime/扩展名不参与判定，
  路径大小写不污染内容身份（§95，A.TXT 与 a.txt 内容相同即重复——有测试）。
- **硬链接语义**：同一内容的两条硬链接路径 size/hash 一致 ⇒ 如实报告为
  同组重复。回收其中一条不影响另一条（回收站按路径操作）；不检测
  硬链接关系、不做去重豁免——诚实呈现事实，避免智能猜测。
- **符号链接语义**：复用 M1 策略——不跟随 symlink/junction（other_entries
  计数），访问集防环（§139 scope 不逃逸）。
- **无哈希缓存**：M3 不建持久 hash 缓存/索引（§99/§104 防 premature
  optimization）。每次 Scan 重新验证文件系统（§100 Repeat Scan 语义）；
  ScanCache 只是单次扫描报告的会话内快照，不是内容缓存。
- **顺序管线**：三级管线单线程顺序执行。Scenario A–D 实测证明当前
  瓶颈是 I/O 而非 CPU（D 场景 1.56 GiB/s）；并行化（rayon 等）留待
  实测证明需要时再评估（§104）。
- **结果排序**：组按 wasted_size DESC（展示策略）；组内文件按规范化
  路径 case-insensitive 升序；GroupId = "grp_{hash16}" 内容派生——
  三者共同保证结果确定性（不依赖 OS 枚举顺序，有 r1/r2 测试）。

## D38 — M4 编码解码级联与 Offset 契约（§15/§16/§18）

- **解码级联**（weave-text::encoding）：BOM 命中按 BOM 严格解码（截断/非法
  surrogate ⇒ 明确错误，不静默丢字节）→ 无 BOM 时 UTF-8 严格 → GB18030 严格
  → Latin-1（字节→U+00xx 无损兜底，必须如实标注）。全程**零 U+FFFD**：
  encoding_rs 的 had_errors 不通过即降级/报错。GB 编码族保守归类 Gb18030
  （超集），GBK 仅在用户显式指定时使用；UI 提供编码手动覆盖。
- **依赖**：encoding_rs 0.8（Mozilla 维护、Firefox 同源、MIT/Apache-2.0、
  无传递重依赖）。
- **Offset 契约（§18 高风险边界）**：域内 = byte offset；IPC 额外携带
  line（1-based）+ column（**UTF-16 code units**，与 JS string index 对齐，
  消解边界风险）；UI 禁止把 byte offset 当 str 索引。测试覆盖 ASCII/é/汉字/
  emoji（surrogate 对）/组合字符/CR-only。
- **BOM/换行策略（§16/§17）**：Preserve 默认——BOM 按文档事实回写；行级
  操作按行携带各自原始结尾（split_inclusive），Mixed 是事实不是错误；
  无换行文档写出基线 LF；final newline 的有无是事实，操作不静默增删。

## D39 — 文本写回：备份 + 原子替换 + 复用 M2 Undo（§89–§95）

- 写回 = Plan（TextTransform）→ 任务内 Revalidate（加载快照 size ⇒ 外部
  改动 `text.fileChangedSincePreview` 拒绝，§90）→ 复制备份
  （`.{name}.weave-text-bak-{op}`）→ `weave-files::atomic_write`
  （temp+flush+rename，§91/§92，共享基础设施——文本工具不自造 write，§10）。
- **事务 target = 备份路径、original_size/modified = 写后状态**：M2 undo 的
  占用检查由此天然实现 §94——"写回后用户又改过 ⇒ stat 不符 ⇒ UndoConflict
  拒绝覆盖"。Undo 按 kind 分派：TextTransform 专用还原（备份存在 + 原位
  stat 匹配 ⇒ rename 覆盖原位；原位消失 ⇒ 仍可还原），其余走 M2 Move 语义。
  这是 §115 意义上的最小通用改进（M2 占位检查对"覆盖写"失效是 P0），
  不存在平行的 TextUndoRecord（§93）。
- 文本输入上限 2 MiB（集中常量 MAX_TEXT_BYTES，下 §104 前置）；加载侧
  NUL 前 8 KiB ⇒ `text.binaryDetected`（§103 二进制守卫）。

## D40 — Transformer/Extract 语义定案（§72–§83 / §58–§70）

- Trim：line/doc 两义分开；Dedupe：Keep First/Last（Last 取最后出现位置、
  顺序保持）+ 空行参与/保留两档；Sort：稳定排序（同键保输入相对序）、
  键 = 原文/小写（无自然排序——a10 < a2 是定义行为）、空行首/尾/保位；
  Title = 空白分词首字母大写、Sentence = 每行首字母大写（简单模式，§78
  允许并如实声明）；行号 pad 按末号位数补零。
- Find/Replace：literal 或 regex（Rust regex 线性引擎，§68 ReDoS 安全）；
  替换语法 contract = **Rust regex replacement（$1/${name}）**，UI 如实
  标注（§82 单一契约）；空 pattern 拒绝；match count 必须展示（§80）。
- Extract（D41 预告位并入）：practical email（非 RFC 完整）；路径启发式
  （盘符/UNC/unix 绝对/显式相对 + 词边界守卫，裸 word/word 不算，尾部
  `,;)]}` 裁剪而 `.` 属版本号语义保留）；Number 不含 hex/octal/binary，
  currency 符号不并入匹配（$5 如实提取 5）；IPv4 octet 实校验、IPv6 用
  std::net 真解析器；JSON 平衡扫描（字符串/转义状态机）+ serde_json 终验、
  只报最外层；Markdown link 跳过围栏与行内代码；用户 Regex 零长度匹配跳过。

## D41 — Extractor 边界（§58–§70 补充）

见 D40 内嵌条目；补充：JSON 提取的候选由平衡扫描给出、**合法性由
serde_json 终验**（平衡 ≠ 合法）；Markdown autolink `<url>` v1 不提取
（如实降级为仅 `[text](url)`）。

## D42 — Compare 算法与限制（§47–§55）

公共前缀/后缀键裁剪 → 余量 ≤ 2000×2000 完整 LCS DP → 超窗**诚实降级**：
单块替换 + `degraded: true` 标注（绝不假装最优、绝不冻结）。Moved =
全 diff 范围同键配对（引擎匹配事实，重复行按次数配对）；Changed = 相邻
Del/Ins 段等长顺序配对。比较键不含行尾换行（CRLF vs LF 非差异）；
Whitespace Ignore 三档 None/Trailing/All；Case Ignore = Unicode 小写折叠。
限额：4 MiB / 100k 行，超限 `compare.tooLarge` 显式错误（§55）。

## D43 — Formatter 能力矩阵与依赖（§24–§46）

- JSON：serde_json **preserve_order**（Format 保持键序，仅空白变化；
  特性影响全局 serde_json——历史存储用 struct 序列化，无 Map 迭代依赖，
  已核实）。Sort = 递归对象键；数组绝不排序（§27）。Normalize ≡ Sort+Format
  规范序列化（§28 明确定义）。
- XML：quick-xml 真事件流；Format 保序重排缩进（文本/CDATA/注释/PI 原样）；
  Minify 仅去元素间纯空白；**Sort Unsupported**（§30 子元素顺序语义）。
- YAML：yaml-rust2 真解析；Validate 带 marker；Sort 重序列化**丢注释/
  锚点展开** ⇒ Warning 如实告知；Format/Minify/Normalize Unsupported
  （§31/§46 注释不可丢）。
- SQL：词法 tokenizer（字符串/引号标识符/注释保真 + byte 区间）；
  Validate = 词法结构 + 括号平衡（§36"基本 validation"，非完整语法——
  如实声明）；Format = 主句关键字布局；Minify = 去注释+空白压缩；
  Sort Unsupported。方言：保守 ANSI 通用子集，不宣称特定方言。
- JavaScript：Format/Validate Unsupported（无真 parser；§37/§39 禁止
  regex 冒充）；Minify = §38 明示的安全子集（字符串/模板原样、注释删除）；
  已知限制：regex 字面量含 `//` 可能被误判行注释（如实记录）。
- CSS：Format/Minify（字符串/url()/注释感知）；Sort/Normalize Unsupported
  （层叠语义）；Validate Unsupported（§42/§46 无完整语法校验）。
- Markdown：Normalize（标题空行/空行折叠/final newline；围栏内容不可侵犯
  §43；不改列表标记/链接/raw HTML）；Sort = H1/H2 顶层 section 按标题文本
  （§45 opt-in）；Format/Minify/Validate Unsupported（§46）。
- 默认缩进 2 空格 + final newline（§25，统一全局）。

## D45 — csv crate 选型（M5 §9）

CSV/TSV 解析采用 `csv` 1.4（BurntSushi，MIT/Apache-2.0，§9 指名候选）：
位置感知错误、引号/多行/转义语义经大规模验证。TSV 复用同一 parser
（delimiter='	'，§10）。`flexible(true)` 开启后 ragged 由 M5 语义处理
（§14 补空/保留+诊断），`UnequalLengths` 一旦出现即未知错位 ⇒ Stop。
incremental `BufRead`（§89）——无 read-all-then-parse。

## D46 — M5 转换语义定案（§51–§62）

- JSON→表：root 必须为对象数组（对象/原始 ⇒ `data.jsonRootObject` /
  `data.jsonRootNotRecords` 结构化拒绝，不静默包装）；嵌套按点路径展平
  （`$.profile.age` 同源 §94）；数组 = JSON cell（§54 不展开）；缺失 = 空、
  null = 空（§31 可配置，v1 默认空串）。
- 表→JSON：默认全字符串（§55）；Typed 模式推断 Integer/Decimal/Boolean/
  Null——**前导零绝不推断**（§57），超 i64/u64 整数保留文本（§7 精度
  优先），Decimal 经 f64（可能精度损失——仅 typed 模式、preview 可见）。
- 重复 header：解析层 name_2/name_3 可见后缀 + col_N 稳定身份（§58）。
- JSONL：BufRead 逐行流式（§59）；FailFast / CollectErrors 双模式，
  valid/invalid/empty 计数 + 行号诊断（§62）；空行跳过并计数。
- 类型观测：ISO 日期（yyyy-mm-dd）唯一无歧义识别；dd/mm vs mm/dd 不猜
  （§38）；大写 Decimal 科学计数法识别；前导零 ⇒ String + Identifier-like
  标注（§6/§57）。

## D47 — DataSession 视图语义（§15–§24）

过滤 = AND 组合的非破坏视图（§20/§21；OR 组 v1 不做——UI 不假装）；
排序 = 稳定（相等行保原相对顺序，§22）+ null/empty 恒最后（两个方向
一致，§23/D47 显式决策）；分页由视图切片（§85，页上限 1000 行）；
行身份 row_N 跟随数据移动（§15）。会话 ephemeral/上限 8 个（§17/§18
——不是数据库、无持久化、无索引引擎）。

## D48 — M5 限额与导出安全（§69/§88/§104 同源）

DataLimits：输入 32 MiB / 内存 500k 行 / 每页 1000 行 / preview 采样
50 行 / exact unique 基数 100k（超 ⇒ Unavailable + 原因，§47）/ JSON
深度 128 / 诊断 1000 条。Export（§69/§71）：只写**新文件**（目标存在
⇒ `data.destinationExists` 拒绝；覆盖源文件须走 M4 TextTransform 管线
——TOCTOU/备份/原子替换/历史/Undo 全套复用，§72-§75）； ridden via
TextTransform Plan ⇒ 历史记录与撤销零成本复用（§93/§114 反向同源）。
