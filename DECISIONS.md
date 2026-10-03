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
