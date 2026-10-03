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
