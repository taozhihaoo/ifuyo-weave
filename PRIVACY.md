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

## M3 Duplicate Finder 实施状态 / M3 Implementation Status

| 项 | 状态 | 位置 |
| --- | --- | --- |
| 哈希全部在本机计算（无 remote hash、无内容出机） | ✅ | weave-files duplicates 管线 |
| 日志不记录文件内容；仅 operation_id / job_id / 计数 | ✅ | duplicates_service / m3_commands |
| 历史（transactions/entries）只存路径、大小、时间等元数据，不存内容 | ✅ | weave-history（M2 复用） |
| 删除唯一动作 = 系统回收站；无永久删除路径、无 fs::remove fallback | ✅ | crates/weave-files/src/recycle.rs（§126 P0 审计项） |
| ScanCache / PlanCache 为内存态，进程退出即消失，不落盘 | ✅ | src-tauri/src/duplicates_service.rs |
| 「最近扫描根目录」为纯会话态（appStore），不写入配置文件 | ✅ | ui/src/stores/appStore.ts |
| 网络能力：无（capabilities 未授予任何网络权限，trash 适配器无网络调用） | ✅ | src-tauri/capabilities/default.json |

## 验收（随里程碑复核）

- [x] 无默认遥测（M0–M3 复核）
- [x] 无强制联网（M0–M3 复核）
- [x] 文件默认本地处理（M0–M3 复核）
- [x] 日志不包含敏感内容（M3 复核：仅 id/计数；路径仅在错误消息中面向本机 UI 呈现，不入日志宏）
- [x] 不上传用户文件（M3 复核）
- [x] 路径经过校验（M3 复核：roots 过 validate_absolute_path；选择必须属于扫描结果）
- [x] 删除默认进入回收站（M3 达成：唯一删除动作）
- [x] 破坏性操作有 Preview / Confirm（M2 起；M3 回收前 Plan Preview + 确认对话框）
