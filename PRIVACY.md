# PRIVACY

Weave 的隐私边界（charter #19 / M0 §14）。这既是文档也是验收清单（charter #69）。

## 承诺 / Commitments

- **No Telemetry** — 不收集、不上报任何使用数据
- **No Account** — 无账号体系
- **No Cloud Requirement** — 所有功能离线可用
- **No File Upload** — 用户文件永不离开本机
- **Local-first** — 默认不联网；M0 的 capabilities 未授予任何网络权限

## M0 实施状态 / Implementation Status

| 项 | 状态 | 位置 |
| --- | --- | --- |
| capabilities 最小化（仅 core:default，无 fs/shell/network） | ✅ | src-tauri/capabilities/default.json |
| CSP：仅 self + IPC；无远程 JS/CSS/字体 | ✅ | src-tauri/tauri.conf.json |
| 日志写入本地 `%APPDATA%/ifuyo/Weave/logs` | ✅ | src-tauri/src/logging.rs |
| 日志禁止记录文件内容/密码/token | ✅（规约 + 评审，M0 无敏感路径写入） | logging.rs 文档注释 |
| 配置仅存本机（language/theme） | ✅ | src-tauri/src/config.rs |
| 崩溃上报 / analytics SDK | 不存在 | 依赖审计保证 |

## 验收（随里程碑复核）

- [ ] 无默认遥测
- [ ] 无强制联网
- [ ] 文件默认本地处理
- [ ] 日志不包含敏感内容
- [ ] 不上传用户文件
- [ ] 路径经过校验
- [ ] 删除默认进入回收站（M3 起）
- [ ] 破坏性操作有 Preview / Confirm（M2 起）
