# Weave Development Progress

## M0 Foundation

Status: COMPLETE

Implemented:

- Cargo workspace（10 crate 骨架；weave-core / weave-testkit / weave-app 有内容）
- weave-core 领域契约：ToolId / OperationId / JobId、五字段错误模型、
  Tool trait（charter #24 八成员）、ToolRegistry、Windows 路径安全契约、
  Progress / Cancellation / Preview / OperationResult（charter #25 七字段）
- weave-testkit：TempWorkspace / DetRandom / FaultInjector 框架
- Tauri 2 shell：窗口、最小 capabilities（core:default）、严格 CSP
- 类型化 IPC：tauri-specta 生成 `ui/src/generated/bindings.ts`；
  ping / get_app_info / inspect_path / get_app_config / set_app_config
- brand.json 单一事实源（编译期嵌入）；版本取自包信息
- Design tokens（8 类，裸色值自动扫描测试）
- i18n 基线（zh-CN / en，资源一致性测试）
- Logging：tracing rolling → `%APPDATA%/ifuyo/Weave/logs`（已验证落盘）
- Config：versioned + validated JSON（language / theme）
- CI：windows-latest + rust-cache + npm cache，8 道门
- cargo-deny（Rust）+ SPDX 表达式感知的前端 license 审计（336 包）
- 文档基线：README / ARCHITECTURE / DECISIONS(14) / PROGRESS / PRIVACY /
  SECURITY / BRAND / CHANGELOG / THIRD_PARTY_LICENSES / docs/
- 占位应用图标（脚本生成，见 BRAND.md）

Quality:

- cargo fmt --check: PASS
- cargo clippy --workspace --all-targets -- -D warnings: PASS
- cargo test --workspace: PASS（63 tests：weave-core 41 / testkit 12 / weave-app 10）
- frontend typecheck: PASS
- frontend lint: PASS（含 jsx-a11y 基线 + no-floating-promises + no-explicit-any）
- frontend test: PASS（19 tests：i18n / registry / bootstrap / App shell / token 纪律）
- tauri build: PASS → `target/release/bundle/nsis/Weave_0.1.0_x64-setup.exe` (2.27 MiB)
- license audit: PASS（cargo deny + 前端 336 包）
- Runtime smoke: PASS — weave.exe 启动，日志按 charter 路径落盘，
  `app info requested` 证明 UI → IPC → Rust → UI 全链路真实打通
- 反模式审计: 干净（无 TODO/unwrap/panic/unsafe/console.log 于生产路径；
  豁免项见 DECISIONS D10，且均有记录）
- 架构审计: weave-core 依赖树 0 tauri 命中；无 >500 行文件；无循环依赖

Commit（代码收口）: 69e4394（`M0: fix production build — vite 8 default minifier`）
收口说明: 本 PROGRESS 终态由随后一笔 `M0: close milestone` 提交记录。

## M1 待办（下一步）

- M1 File Core：weave-files、Filesystem Adapter、完整 Windows 大小写契约、
  GB18030/GBK 编码检测（M1 提示词 #6/#9 及其补丁条款）

---

（后续里程碑按 M1 → M2 → … 追加章节）
