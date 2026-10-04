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
| trash | 回收站适配（Windows IFileOperation，MIT，M3/D34） | MIT |
| encoding_rs | 文本解码级联（Mozilla/Firefox 同源，M4/D38） | MIT OR Apache-2.0 |
| quick-xml | XML 事件流解析（MIT/Apache-2.0，M4/D43） | MIT OR Apache-2.0 |
| yaml-rust2 | YAML 真解析（MIT/Apache-2.0，维护中替代 archived serde_yaml，M4/D43） | MIT OR Apache-2.0 |
| lopdf 0.45.0 | PDF parse/操作（inspect/merge/split/reorder/rotate，M8/D59） | MIT |
| zip 8.6.0（deflate） | OOXML 容器读取 + §87 炸弹守卫（MIT，M8/D59） | MIT |
| md-5 / sha1 / sha2 0.10 | Hash 工具摘要算法（RustCrypto digest 统一 API，M9/D62） | MIT OR Apache-2.0 |
| blake3 1.x | BLAKE3 摘要（M9/D62） | CC0-1.0 OR Apache-2.0 OR MIT |
| crc 3.x | CRC-32/ISO-HDLC + CRC-32C（catalog 标准参数，M9/D62） | MIT OR Apache-2.0 |
| uuid 1.x（v4+v7） | UUID 生成（getrandom CSPRNG §46/§47，M9/D62） | MIT OR Apache-2.0 |
| time 0.3 | Timestamp RFC3339/偏移（M9/D62） | MIT OR Apache-2.0 |
| url 2 / percent-encoding 2 | URL 解析/百分号编码（WHATWG/RFC 3986，M9/D62） | MIT OR Apache-2.0 |
| regex 1.x | Regex Tester 唯一引擎（与 M4 同源线性引擎 §66/§73，M9/D62） | MIT OR Apache-2.0 |


| regex | 重命名规则正则（M2/D28） | MIT OR Apache-2.0 |
| chrono | 日期重命名规则（M2/D28） | MIT OR Apache-2.0 |
| react / react-dom | UI | MIT |
| zustand | 状态 | MIT |
| vite / typescript / vitest / eslint 系 / prettier | 工具链 | MIT 系（详见审计） |

发布版完整清单（含版本与法律文本指针）将在 M12 Release 阶段生成归档。
