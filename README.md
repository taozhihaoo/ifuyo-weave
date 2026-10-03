# ifuyo Weave

**Weave** 是一个 local-first 的通用桌面工具集：用一组高质量的小工具，解决文件、文本、数据、文档、媒体与批处理中的高频杂事。

> 把文件、文本、数据和各种琐碎工作，编织成简单、可靠、可控的本地工作流。

EN: Weave is a local-first desktop utility suite — a set of carefully crafted small tools for files, text, data, documents, media and batch work. Local-first, no account, no cloud, no telemetry.

## 核心哲学 / Philosophy

- **Local-first** — 文件本地处理，默认不联网，无账号无上传
- **Utility-first** — 首先是工具，不为视觉牺牲效率
- **Batch-first** — 1 个文件和 10,000 个文件同等对待
- **Preview-first** — Analyze → Preview → Confirm → Apply，先说明再动手
- **Reversible-first** — 能撤销必须撤销；不能撤销的明确警告

完整产品约束见《开发总纲领 / Project Charter v1.0》。

## 技术栈 / Tech Stack

| 层 | 选型 | 实际版本 |
| --- | --- | --- |
| 桌面框架 | Tauri 2 | 2.12.x |
| 后端 | Rust (stable, msvc) | 1.99 |
| 前端 | React + TypeScript + Vite | 19.3 / 5.9 / 8.3 |
| 状态 | Zustand | 5.x |
| IPC 类型 | tauri-specta（生成 `ui/src/generated/bindings.ts`） | 2.0.0-rc.25 |
| 日志 | tracing + rolling file | 0.1.x |

## 开发 / Development

```powershell
npm install          # 前端依赖
npm run dev          # 仅前端（Vite, :1420）
npm run tauri dev    # 完整桌面应用（首次 Rust 编译较久）
npm run tauri build  # 生产构建（NSIS 安装包）
```

质量门（charter #45，CI 强制）：

```text
cargo fmt --check / cargo clippy -D warnings / cargo test --workspace
npm run typecheck / npm run lint / npm test
npm run tauri build
cargo deny check licenses / node scripts/check-frontend-licenses.mjs
```

生成类命令：

```text
npm run generate:bindings   # 重新生成 ui/src/generated/bindings.ts（IPC 契约）
npm run generate:icon       # 重新生成占位应用图标（scripts/generate-icon.mjs）
```

## 仓库结构 / Repository Layout

```text
crates/           Rust workspace：weave-core（领域契约）+ 各领域 crate + weave-testkit
src-tauri/        Tauri 壳 + Application 层（IPC 命令、配置、日志）
ui/               React/TS 前端（app/ design/ i18n/ stores/ commands/ generated/）
scripts/          工具脚本（图标生成、license 审计）
docs/             项目文档（性能数据 PERF.md 等，按里程碑填充）
.github/          CI（windows-latest）
```

## 架构 / Architecture

```text
UI (React)
 ↓ IPC（typed, tauri-specta 生成）
Application (src-tauri)
 ↓
Core (weave-core: Tool trait / Error / Path Safety / Registry)
 ↓
Adapters → OS / File System
```

禁止 React 组件直接触碰文件系统。详见 [ARCHITECTURE.md](ARCHITECTURE.md)。

## 隐私 / Privacy

无遥测、无账号、无云端、无上传。详见 [PRIVACY.md](PRIVACY.md)。

## 许可 / License

MIT OR Apache-2.0（双许可，见 LICENSE-MIT / LICENSE-APACHE）。

## 当前里程碑 / Current Milestone

- **M0 — Foundation** / **M1 — File Core** / **M2 — Rename / Organizer**：COMPLETE
- **M0 — Foundation** / **M1 — File Core** / **M2 — Rename / Organizer** /
  **M3 — Duplicate Finder**：COMPLETE
- **M4 — Text**（进行中）：格式化（JSON/XML/YAML/SQL/JS/CSS/Markdown，
  能力矩阵诚实标注）、比较（确定性 diff）、提取（8 类 + 正则）、变换
  （9 操作）、安全写回（快照 TOCTOU + 备份 + 原子替换 + 历史/撤销）。
  进度见 [PROGRESS.md](PROGRESS.md)。

## 文本工具 / Text Tools（M4）

- **Formatter**：JSON / XML / YAML / SQL / JavaScript / CSS / Markdown——
  Validate / Format / Minify / Sort / Normalize 按格式真实能力提供
  （能力矩阵与边界见 [docs/text.md](docs/text.md)）
- **Compare**：A vs B 确定性 diff（Added/Removed/Changed/Moved + Unified）
- **Extractor**：URL / Email / 路径 / 数字 / IPv4 / IPv6 / JSON / MD 链接 / 正则
- **Transformer**：Trim / 去重 / 排序 / 前后缀 / 大小写 / 行号 / 查找替换

## Text Tools 使用流程 / M4 Flow

```text
粘贴文本 / 按路径加载文件
  → Format（Validate/Format/Minify/Sort/Normalize，按格式能力）
  → Transform（Trim/去重/排序/前后缀/大小写/行号/查找替换）
  → Extract（URL/Email/路径/数字/IPv4/IPv6/JSON/MD 链接/正则）
  → Compare（A vs B，Side-by-side 统计 + Unified）
  → Preview（预览无副作用；大文本截断仅影响显示）
  → 写回（显式确认 + 快照校验 + 备份 + 原子替换 + 历史/撤销）或复制/导出
```

## Duplicate Finder 使用流程 / M3 Flow

```text
拖入 / 粘贴目录 → Scan 扫描（size → partial hash → full SHA-256）
  → 按组查看重复文件 → 勾选要回收的文件（每组至少保留一份）
  → 生成回收计划（Preview）→ 确认（明确告知：移入系统回收站，可撤销）
  → Recycle 执行 → 结果统计 → History 历史 → Undo（从回收站还原）
```

Weave 不做"一键清理磁盘"式的承诺：扫描结果只是事实，回收什么由用户逐项
决定；每组必须保留至少一份；扫描后被改动的文件会被拒绝回收。

## 已知限制 / Known Limitations

- Text / Data / Image / PDF 等上层工具尚未实现——按 M4–M11 里程碑推进
- GB18030/GBK/Latin-1 编码检测未实现（charter #27 要求项，M4 补齐，已记录 DECISIONS D22）
- Directory Analyzer 按扩展名分类，不做内容嗅探（事实优先，见 docs/file-core.md）
- 应用图标为脚本生成的工程占位，非品牌资产（见 BRAND.md）
- Windows 测试可执行文件依赖 build.rs 注入的 common-controls manifest（见 DECISIONS.md #12）
