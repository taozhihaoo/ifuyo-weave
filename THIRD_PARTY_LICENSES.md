# THIRD_PARTY_LICENSES

策略（M0 §26.2）：本清单由 CI **自动生成/校验**，不手工猜测。

- Rust 依赖：`cargo deny check licenses`（策略见 `deny.toml`）——每次 CI 运行校验实际依赖图。
- 前端依赖：`node scripts/check-frontend-licenses.mjs`（策略内嵌，SPDX OR/AND 表达式感知）——遍历
  `node_modules` 全量校验，含传递依赖。

## 当前策略摘要

| 范围 | 允许 | 显式拒绝 |
| --- | --- | --- |
| Rust | MIT / Apache-2.0 / (Apache-2.0 WITH LLVM-exception) / BSD-2 / BSD-3 / Zlib / MPL-2.0 / Unicode-3.0 / CC0-1.0 | GPL / AGPL / LGPL |
| 前端 | 上述 + ISC / 0BSD / Python-2.0 / CC-BY-4.0 / MIT-0 / OFL / CDLA-Permissive-2.0 | 同上 |

白名单相对 charter #40 的每一项扩展均带内联理由（见 deny.toml 与检查脚本注释）。

## 主要直接依赖（M0）

| 依赖 | 用途 | 许可 |
| --- | --- | --- |
| tauri / tauri-build / @tauri-apps/api / @tauri-apps/cli | 桌面框架 | Apache-2.0 OR MIT |
| tauri-specta / specta / specta-typescript | IPC 类型生成 | MIT OR Apache-2.0 |
| serde / serde_json | 序列化 | MIT OR Apache-2.0 |
| tracing / tracing-subscriber / tracing-appender | 日志 | MIT |
| sha2 (RustCrypto) | SHA-256 流式哈希 | MIT OR Apache-2.0 |
| react / react-dom | UI | MIT |
| zustand | 状态 | MIT |
| vite / typescript / vitest / eslint 系 / prettier | 工具链 | MIT 系（详见审计） |

发布版完整清单（含版本与法律文本指针）将在 M12 Release 阶段生成归档。
