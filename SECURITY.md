# SECURITY

Weave 的安全基线（M0 §15 / §50 / §51）。M0 建立边界与机制，不声称完整安全系统。

## 已建立 / Established in M0

### Capability 最小化

`src-tauri/capabilities/default.json` 仅授予 `core:default`（事件/窗口/应用元数据）。
无 filesystem、shell、process、http 权限。任何新权限必须在本文件登记用途。

### CSP

`default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self' ipc: http://ipc.localhost`
——无远程 JS / CSS / 字体。`style-src 'unsafe-inline'` 仅为 design tokens 运行时注入所需。

### IPC 校验

- 全部命令 typed（tauri-specta 生成），无 `map[string, any]`。
- 文件路径输入一律先过 `weave_core::path`（拒绝穿越 / 保留名 / 非法字符 / 尾部点空格 / 相对路径）。
- 结构化错误（五字段）跨 IPC 返回，不吞异常。

### 禁止事项（对本仓库所有贡献者与 AI agent 生效）

- 禁止 UI 侧执行 shell / 任意进程
- 禁止引入遥测、崩溃上传、analytics
- 禁止在仓库或日志中嵌入 secret / token / 密码
- 禁止静默覆盖用户文件（Safe Write 由 M2 建立，此前不写用户文件）
- 禁止未经本文件登记而扩大 capabilities

### 安全测试基线（M0 §50）

路径穿越拒绝、非法参数、malformed 输入由 weave-core 路径契约与命令层测试覆盖；
不允许 panic（workspace 级 `unsafe_code = "forbid"` + clippy `-D warnings`）。

## 漏洞处理

发现安全问题：按 M14 提示词的 P0/P1/P2 分级 → Reproduce → Contain → Patch →
Regression Test → Document。不删除测试换取绿色，不在 release note 泄漏可利用细节。

## M4 Text 专项（下 §167–§176 / §229）

| 项 | 状态 | 证据 |
| --- | --- | --- |
| Path Traversal / Root Escape | ✅ PASS | 打开/写回先过 `validate_absolute_path`（text_service）；复用 M1 path safety，未另造规则 |
| Symlink / Junction | ✅ PASS（复用 M1 策略） | 未跟随语义沿用 M1；未新造（§169） |
| Regex Safety | ✅ PASS | Rust regex 线性时间引擎（设计性质，§140）；pattern/replacement 视为不可信输入，无 shell 插值（§171） |
| Parser Resource Limits | ✅ PASS | JSON 递归限深（200/1000/5000 层拒绝测试）；深 XML 事件流无递归；TextLimits 输入上限（§182/§183） |
| Large Input Handling | ✅ PASS | TextLimits 分档 + `text.tooLarge`；Compare 降级有界（docs/PERF.md M4 节，§104/§141） |
| Formatter Output Explosion | ✅ PASS | 输出/输入线性比断言（malformed_inputs::json_output_explosion_is_bounded，§184） |
| Result Size Limits | ✅ PASS | 提取结果 10k 条上限 + 截断标注（§185） |
| No Shell | ✅ PASS | 无 Command/shell 调用做核心功能；全进程内（§170） |
| No Network | ✅ PASS | capabilities 无网络；无 remote 格式化/上传（§172/§173） |
| Secret Logging / 剪贴板 | ✅ PASS | 日志仅 id/计数；剪贴板仅本地（§174–§176） |
| TOCTOU Protection | ✅ PASS | text.fileChangedSincePreview / text.fileMissing（src-tauri/tests/text_integration.rs，§132） |
| Atomic Write | ✅ PASS | weave-files::atomic_write temp+flush+rename + 3 故障测试（§91/§92） |
| Undo 不覆盖用户编辑 | ✅ PASS | undo.rs TextTransform 分支 + 双层测试（§94/§134） |
| Save-As Collision | ✅ PASS | text.destinationExists 计划期+执行期双检查（§132） |
| 敏感文本 | ✅ PASS | 粘贴内容仅驻内存/显式写回目标，不入日志（§175） |

### M4 已知边界

- Windows readonly 写回 ⇒ 结构化失败（text_integration::readonly_target_write_fails_structured）
- Latin-1 兜底 + 控制字符密度守卫为两道二进制防线（text_service::detect_binary）
- macOS/Linux 权限与 symlink 行为未实测（CI windows-latest，如实标注）
