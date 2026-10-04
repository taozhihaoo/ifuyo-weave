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

### M5 Data 专项（下 §167–§176 同源；M5 docs 轮补记）

| 项 | 状态 | 证据 |
| --- | --- | --- |
| Path Traversal / Root Escape | ✅ PASS | data_open/export 先过 `validate_absolute_path`（text_service 同源） |
| Regex Safety | ✅ PASS | Rust regex 线性引擎；用户 pattern 不可信输入处理 |
| Large Input / Result | ✅ PASS | DataLimits 分档（输入 32 MiB/页 1000/结果 10k 截断标注） |
| No Shell / No Network | ✅ PASS | 全进程内（weave-data） |
| Session Not Persisted | ✅ PASS | DataSessions ephemeral ≤8 LRU（§18 不是数据库） |
| 敏感数据 | ✅ PASS | 历史/日志不记数据集内容（§174/§175） |

### M6 Image 专项（上 §116–§176）

| 项 | 状态 | 证据 |
| --- | --- | --- |
| Decompression Bomb | ✅ PASS | 解码前 header 守卫：dimension 16384 / pixels 80M / decoded 256 MiB 预算（limits::tests + inspect crafted-header 测试，§7/§118/§119） |
| Huge Dimension / Columns | ✅ PASS | `data.tooManyColumns` / 维度拒绝（§117/§118） |
| Huge Metadata | ✅ PARTIAL | max_metadata_bytes 已定义；解析侧完整上限 M6（下）补 |
| Output Collision | ✅ PASS | 新文件模式 dest-exists ⇒ `image.destinationExists`（§75） |
| TOCTOU | ✅ PASS | 覆盖源走 TextTransform 管线（§145）；新文件 must_not_exist 双检查 |
| Atomic Write | ✅ PASS | 复用 weave-files::atomic_write（§146） |
| Output Verification | ✅ PASS | 导出后 PNG magic/尺寸 roundtrip 测试（§147/§148） |
| GPS Privacy | ✅ PASS | GPS presence 显式标注 + strip 剥离（§23/§62） |
| No Shell / No Network | ✅ PASS | 纯 Rust image 管线，无 CLI/网络（§111–§113） |
| 整数溢出 | ✅ PASS | checked 算术测试（§6 limits::checked_arithmetic_no_overflow） |

### M6 已知边界

- WebP 有损编码 NOT SUPPORTED（纯 Rust 生态，§109 spike，D49）
- 动画 GIF/WebP Static Only（帧数为事实、转换拒绝保留动画，§59）
- macOS/Linux 解码行为未实测（CI windows-latest）

## M7 Batch Engine 专项（下 §189）

- Path Safety：输入/目标目录经 `validate_absolute_path`；Export 输出名 =
  dest/{source_stem}.{ext}——stem 来自源文件名、ext 来自固定格式串，
  无用户可控相对路径成分；无 `..` 拼接面。
- Symlink：导出目标存在性检查按 lstat 语义（exists()）；快照/重校验基于
  stat（跟随链接的目标事实）——与 M1 既有边界一致。
- Overwrite：默认 Never（§211）；覆盖需显式 `overwrite_existing`，且覆盖
  产物在 History 中标记为不可自动撤销（undo 守卫必冲突，D58）。
- Resource Limits：workers ≤ 8（IPC clamp）；单 item 图像解码受
  weave-media ImageLimits 守卫（§113 Resource Bomb 同源）；journal 逐行
  append，无内存放大。
- No Shell / No Arbitrary Code（下 §153/§154）：Pipeline 定义只有六种
  Stage 枚举（serde tag=type）——无脚本节点、无命令执行面；未知 stage
  类型在反序列化即拒绝。
- Journal Corruption：不 panic；隔离保留原件（§197），恢复仅基于完好
  前缀——无"损坏即重放"面。

## M8 Documents 专项（§162-§163）

- Path Safety：输入/输出经 validate_absolute_path；输出命名 = 固定后缀
  模板（{stem}.{merged|rotated|extract}.pdf / part-NNN），无用户可控路径
  成分；temp = 同目录 `.name.weave-tmp-<pid>`（受控、可清理 §127）。
- ZIP Safety（§87）：条目数/解压总量/单条目上限在打开时全包校验；
  失败 = 结构化拒绝。
- XML Safety（§88）：quick-xml 无 DTD/实体展开引擎——XXE/实体扩张面
  不存在；外部关系只计数不访问（§89 无网络）。
- PDF Parsing（§162/§163）：lopdf 纯 Rust 解析；malformed corpus（截断/
  垃圾字节/坏 XML）测试无 panic；限额（pages/file size）防 CPU/内存
  耗尽；加密 PDF 结构化拒绝（§84）。
- 输出碰撞：默认不覆盖（output-first）；Merge TOCTOU 重校验（§141）。
- 崩溃/半成品：temp+promote 保证最终名下永远是完整产物（§179）。
