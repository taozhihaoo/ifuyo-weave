# Weave Development Progress

## M0 Foundation

Status: IN PROGRESS

### Infrastructure

- [x] Repository baseline（.gitignore / .gitattributes / 双许可 / brand.json / 规划文档入库）
- [x] Cargo workspace（10 crate 骨架，weave-core / weave-testkit / weave-app 有内容）
- [x] weave-core 领域契约（ToolId/OperationId/JobId、五字段错误模型、Tool trait、Registry、路径安全）
- [x] weave-testkit（TempWorkspace / DetRandom / FaultInjector）
- [x] Tauri 2 shell（窗口 / 最小 capabilities / CSP）
- [x] 类型化 IPC（tauri-specta 生成绑定，5 个命令）
- [x] brand.json 单一事实源（编译期嵌入）
- [x] Design tokens（8 类，裸色值自动扫描）
- [x] i18n 基线（zh-CN / en，资源一致性测试）
- [x] Path safety 契约（盘符 / UNC / 保留名 / 穿越 / 长路径 / 尾部点空格）
- [x] Logging（tracing rolling → %APPDATA%/ifuyo/Weave/logs）
- [x] Config（versioned + validated JSON，language/theme）
- [x] CI（windows-latest，rust-cache + npm cache，8 道门）
- [x] cargo-deny + 前端 license 审计
- [x] Documentation baseline（本文档套件）
- [ ] 全套质量门一次全绿（fmt / clippy / cargo test / typecheck / lint / vitest / tauri build / license audit）
- [ ] Runtime smoke（真实启动：窗口 / 品牌 / i18n / IPC ping / app info / path probe / drop）

### Quality Gates

- [ ] fmt
- [ ] clippy
- [ ] cargo test
- [ ] typecheck
- [ ] lint
- [ ] frontend test
- [ ] tauri build
- [ ] license audit

### M1+ 待办（不属 M0）

- M1 File Core：weave-files、Filesystem Adapter、完整 Windows case 契约、GB18030 编码检测
- M2 Rename/Organizer：OperationTransaction 字段级落地（D11 推迟决定）

---

（后续里程碑按 M0 → M1 → … 追加章节）
