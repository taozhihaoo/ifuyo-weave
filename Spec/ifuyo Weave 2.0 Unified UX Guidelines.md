# ifuyo Weave 2.0 Unified UX Guidelines（M2.13 内部规范）

适用范围：M2.13 及以后的所有 Weave 模块（含 M2.14/M2.15 与未来 3.0）。
原则：**统一语义与状态，不统一外观**；Advisor 不越权；执行永远走 Weave 核心。

---

## 1. Navigation（导航）

- 导航按**用户任务**分组：文件 / 分析 / 内容 / 保护 / 网络 / 采集 / 自动化 / 智能 / 系统。
- 低频模块不占一级导航——经 Command Palette（Ctrl+K）可达。
- 每个视图必须在 `App.tsx` 的 `view` 联合类型 + 分组表中登记。
- 新视图三件套：视图组件、`nav.<id>` i18n 键（zh/en 同笔）、palette 命令。

## 2. Input（输入）

- 统一入口：Drag & Drop / Picker / Clipboard / URL / Recent / Palette。
- 模块可限制输入类型，但**拒绝必须可解释**：说明支持什么、当前是什么、可以去哪里处理。
- 输入上限必须显式（如 smart ≤200 路径、web batch ≤500 URL）。

## 3. Selection（选择）

- 列表选择支持：单选/多选/全选/清除；键盘可达。
- 全选只默认作用于**高置信/安全子集**时必须显式标注（参考 SmartPanel §21）。

## 4. Job（任务）

- 长任务一律走 `JobTracker.register()`——禁止自建线程/任务系统。
- Job Center 是唯一任务列表；行内可取消；Running 永不被容量淘汰。
- 任务生命周期归应用，不归页面：切页不终止任务。

## 5. Progress（进度）

- 已知进度报 `{n} items processed`；未知进度如实显示 unknown。
- **禁止伪进度**。

## 6. Error（错误）

- 一律 `WeaveError` → `IpcError`（code/message/suggestion）；禁止裸字符串。
- Partial Failure 是一等状态（完成但有失败 ≠ Failed）。

## 7-9. Preview / Plan / Apply

- 可修改操作：Preview（只读）→ Plan（构建期冲突/环/STALE 标记）→ Apply（M2.0 执行核心）。
- **禁止任何模块直接 File.Move/Delete**——执行只经 execute_plan / execute_recycle_plan / 专用既有 IPC。

## 10. Audit（审计）

- 统一词汇：Input / Planned / Applied / Skipped / Failed / Recovered。
- 日志只记元数据；正文/secret/URL query 值绝不入日志。

## 11. Recovery（恢复）

- 恢复入口统一在 History / 完成摘要（Undo available）；删除候选一律回收站（可恢复）。

## 12. History（历史）

- History 回答"过去发生过什么"，Job 回答"现在正在发生什么"——不混同。
- 会话/历史持久化必须有界（数量/时间）。

## 13. Notification（通知）

- 经 M2.6 NotificationCenter（聚合 + dismiss/clear）；code 前缀映射目标视图实现精准跳转。
- 后台自动任务默认安静；通知只含元数据。

## 14. Command（命令）

- 一切命令经 CommandRegistry（id/labelKey/category/keywords）；模块不得自建搜索。
- 新命令必须 zh/en keywords 同笔。

## 15. Settings（设置）

- 一切持久化经 AppConfig（`persistAppConfig` 全量 round-trip——**整体替换语义**）+ 模块 apply IPC 立即生效。
- 统一 Settings 页只放开关/默认值；模块高级项留在模块面板。

## 16. Terminology（术语）

- Delete=删除对象 / Remove=移出集合 / Clear=清空记录——严格区分。
- Analyze/Scan/Inspect/Preview/Plan/Apply/Export/Recover/Undo 全局同义。

## 17. Risk（风险）

- Low=无确认；Medium=预览+破坏性确认；High=显式确认；Critical=强确认+后果说明。
- 删除候选恒 High risk（回收站兜底）；外部上传恒需显式同意。

## 18. UI Tokens

- 禁止裸色值/裸阴影（tokens.test 守门）——一律 `var(--*)`。
- 新 token 进 `design/tokens.ts`（zh/en 无关）。
