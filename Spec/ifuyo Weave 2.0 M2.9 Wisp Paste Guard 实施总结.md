# ifuyo Weave 2.0 M2.9 Wisp / Paste Guard 实施总结

日期：2026-10-07
状态：**P0 完成（Detection Capability = Limited 如实声明，见 §8）**；门禁全绿。

---

## 1. 实际实现范围

- **敏感性检测**（`src-tauri/src/wisp/{model,rules}.rs`）：确定性规则引擎（ruleset v1，17 条内置规则），四类可配置开关（凭证/本地路径/个人信息/secret 配置 §32 Detection）。每条发现携带 `rule_id / category / confidence / severity / evidence / masked_preview / 字节偏移`（§6 可解释，非裸 bool）。
  - 规则形态：具名凭证前缀（AKIA/ghp_/xox/sk-/PEM 私钥/DB 连接串/JWT/Bearer ⇒ Critical-High）；secret 赋值形态（password=… 等，**placeholder 值降置信 Low、高熵长值升 High** §5E/§39）；本地路径/UNC/unix home（仅 Potentially §5C）；邮箱/IPv4（保守 Low §5D）。
  - 单规则命中上限 20（防风暴 §34）；分析上限 64K 字符（超限截断如实标注 §35）。
- **脱敏引擎**（`wisp/mask.rs`）：Full/Partial/Structured 三形态——`sk-••••3456`（前缀+尾 4 保留）、邮箱本地名 `a••••f@`、整体打码；`mask_text` 按偏移替换（重叠/越界保守跳过）。**不宣称对所有格式可靠（§40）**。
- **风险引擎**（`wisp/risk.rs`）：**敏感性与粘贴风险严格分离（§7）**——同一 Medium 内容，编辑器目标 = Medium、浏览器/聊天/未知目标升级 High；Critical 命名凭证内容恒 Critical。决策对象 `PasteDecision { action, risk, reasons[], requires_confirmation, target, rule_ids[] }`（§16），reasons 全部可解释。
- **目标识别**（§8/§22）：按**可执行文件名**分类（browser/terminal/editor/chat/office 表驱动，内部小写化防御），标题仅作辅助证据不参与身份；前台窗口推断 = **Likely** 置信（前台 ≠ 必然粘贴目标），用户显式声明 = Confirmed，取不到 = Unknown（**Unknown ≠ Safe：风险只升不降**）。
- **策略**（§9/§10/§21）：每应用规则（exe 身份 + mode + enabled，经 AppConfig 持久化）优先；全局默认保守——Normal→Allow、Potentially→Warn（仅 Low 置信占位符/单邮箱 → Allow 不打扰）、High/Critical→Warn 需确认、**Block 仅显式应用规则或 block_critical 显式开启**。
- **保护会话**（`wisp/session.rs` + service）：`ClipboardProtectionSession` 八态状态机（Prepared/Masked/Pasted/Restoring/Restored/Expired/Cancelled/Failed §12）；单活动会话（防多会话 race §34）；**惰性 TTL**（默认 300s，无常驻定时器——复用既有生命周期 §29）；**恢复指纹守卫（§19）**：当前剪贴板 ≠ masked 指纹 ⇒ 用户已复制新内容 ⇒ 跳过恢复并如实呈现原因，**绝不覆盖用户新内容**；崩溃恢复（§20/§43）：启动加载残留快照但不自动恢复，恢复仍走全部守卫；原文落盘仅在 `retain_sensitive_payload` 显式开启（默认关——**不伪装保险库 §25**，关时崩溃后不可恢复如实报 `wisp.restoreUnavailable`）。
- **Clipboard 自环**（§13/§58）：Wisp 全部剪贴板写入（masked/原文）**写前**登记 M2.3 `suppress` 指纹（规范 SHA-256 hex，与监听端同构）——消自环且消 M2.3 1s 轮询竞态；Wisp 后台分析挂在 **M2.3 监听线程同一轮询通路**（`note_clipboard_text` 钩子），无第二监听器/第二存储（§27/§44/§58 验收 1/2/20 通过）。
- **经 Weave 的粘贴意图防线**：`wisp_recopy_guarded(id, confirmed)`——M2.3 recopy 前评估（Block ⇒ 拒绝并说明；Warn ⇒ 返回决策等 UI 确认；Allow ⇒ 委托 `clipboard_recopy` 复用不重建）。
- **事件与通知**（§45/§48/§49）：有界安全事件（200 条，仅 Timestamp/Kind/Level/Risk/Action/Target-exe/RuleIds **无任何内容**）；通知复用 M2.6 NotificationCenter（60s 聚合，只报规则数量）；tracing 只记元数据。
- **UI**（`WispPanel.tsx` §44 轻量）：状态卡（Enabled/Paused + **"检测能力：有限"如实声明** §31）；粘贴前评估卡（风险/目标+置信/决策/理由/发现表含 masked 预览 + Mask&Protect）；保护会话卡（状态/TTL 倒计时/恢复原文/放弃 + 跳过原因呈现）；检测自测文本框（§38 误报自查）；最近事件表 + 清空；**设置区（§32 UI：启用/四类检测开关/block_critical/TTL/retain 附如实说明/每应用规则增删改启）**；palette 命令（Open Wisp / Toggle Pause / Recent Events §31）；i18n zh/en 同笔 key 对等（既有测试守门）。
- **设置持久化缺陷修复**：`set_app_config` 是整体替换——`persistTheme/persistFavorites/persistLocale` 及 App 内联保存原先只带 language/theme/favorites，**clipboard 与 wisp 段会被 serde default 静默重置**（主题/语言切换即触发）。修复 = `persistAppConfig(mutate)` 全量 fetch-modify-set round-trip，四处调用点全部收敛。

## 2. Architecture Audit（§1 事实结论）

- **FACT（复用）**：M2.3 `weave-clipboard` 共享敏感原语（`looks_like_jwt/secret_assignment/bearer`、`has_long_hex_run`——M2.3 §53 明示"M2.4/M2.9 复用防漂移"）；M2.3 监听线程/suppress 指纹机制；M2.6 NotificationCenter；M2.8 x-win 前台窗口封装（`ForegroundWindow` 增补 `process_name` 字段——加法扩展）；AppConfig 设置系统；IpcError/路径安全。
- **GAP（本阶段补齐）**：可解释 findings 模型、规则集 v1、目标分类、风险矩阵、保护会话、事件记录、UI。
- **RISK（规避）**：第二监听器诱惑 ⇒ 同通路钩子；键盘钩子不可用 ⇒ Limited 如实降级。
- **DO NOT TOUCH**：clipboard_service 监听核心/存储、agent runtime、recipe、glint、weave-privacy——均未重构。

## 3. 新增模块

`src-tauri/src/wisp/{mod,model,rules,mask,risk,session}.rs`、`src-tauri/src/wisp_service.rs`（11 IPC 命令）、`ui/src/features/wisp/WispPanel.tsx`。

## 4. 复用模块

M2.3（监听通路/抑制/store/敏感原语）、M2.6（通知中心/生命周期纪律）、M2.8（x-win 前台窗口）、AppConfig/Settings、IpcError。

## 5-7. Sensitivity / Rule / Risk

见 §1；规则集 `RULESET_VERSION=1`（§37 数据驱动，未来外置 v2 不改代码）；缓存键 = 内容指纹 + ruleset_version + analyzer_version（§36，monitor 钩子以 last hash 去重——规则变更后重启/重载即失效）。

## 8. Paste Detection —— **Detection Capability = Limited（核心如实声明）**

- **不可用能力**：全局 Ctrl+V 拦截需要 `SetWindowsHookEx(WH_KEYBOARD_LL)` 等 Win32 FFI——workspace `unsafe_code="forbid"` 下**不可能**。**NOT SUPPORTED**：拦截/修改任意应用的原生粘贴流。
- **可用防线（全部真实生效）**：① M2.3 通路持续分类新剪贴板内容（事件+通知元数据）；② Weave 内粘贴意图（guarded recopy）Block/Warn 确认；③ 保护会话（Mask→用户手动粘贴→TTL 内指纹守卫恢复）。
- UI/文档/`wisp_status.detectionCapability="limited"` 全链路如实，无一处宣称"100% Paste Detection"（§14 验收 8 通过）。

## 9. Mask / Restore

见 §1；Demo 4/5 链路（Mask→Paste→Restore；Mask→用户复制 B→Restore skipped）由 `session.rs` 单测固化（`restore_outcome_fingerprint_guard`）。

## 10. Application Policy

AppConfig `WispConfig`（§32 全字段：启用/四类检测/block_critical/TTL/retain/`app_rules`（exe + mode + enabled））；WispPanel 设置区直接编辑，`wisp_apply_config` 立即生效 + 全量 round-trip 持久化。

## 11. Privacy / Security（§26/§52/§53/§54/§55）

- **无任何完整 secret 入日志/事件/通知/决策对象**（`decision_carries_rule_ids_not_content` 等测试固化）。
- **无键盘记录**：Wisp 不注册任何输入钩子（也不可能——unsafe 禁令）；不保存按键/输入历史。
- **无网络**：全部本地规则，零新增依赖面。
- 敏感原文唯一新增落盘点 = 保护会话快照（默认关；开时明文 JSON 与 M2.3 payload 存储同级暴露面，如实文档化）；恢复/取消/过期即清。
- 多格式（§50）：文本路径全支持；HTML 按 M2.3 既有 (kind,hash) 身份不重复判断；Image/File 无文本语义 ⇒ `Sensitivity::Unknown` 语义（不分析不伪造）。

## 12. Agent Integration

无自建循环（§29/Step9）：后台分析在 M2.3 监听线程；TTL 惰性判定；通知走 M2.6 NotificationCenter；暂停状态由 `wisp_set_paused` + AppConfig 驱动。

## 13. Clipboard Integration（验收 1/2）

M2.3 仍是唯一 Clipboard 数据源与唯一监听器；Wisp 写入全部经 M2.3 suppress 机制（写前登记）。

## 14. Tests

- **Wisp 域 29 测试**：误报集（Normal 文本/普通路径/单邮箱不升级）、漏报集（8 类具名凭证全检出）、模糊集（placeholder 降置信/高熵升置信/本地路径仅 Potentially）、开关失效、截断、per-rule 上限、masked 预览不含原文、分类按 exe、§7 同内容不同目标、Unknown≠Safe 升级、默认 Warn 不 Block、显式 Block/Allow、决策无内容、会话四守卫（状态/TTL/指纹/损坏快照）。
- 门禁：全工作区 **42 套件 0 failed**；fmt/clippy **0**；tsc 0；eslint 0；**vitest 38 绿**；tauri build 通过。

## 15. Performance

规则引擎正则一次编译（OnceLock）；64K 截断；同指纹去重（重复分析消除）；事件/会话均有界；编码/分析在 M2.3 监听线程与 IPC 线程（不阻塞 UI）。**未做**：1k/10k 事件压测基准表（逻辑上有界：监听 1s 节流 + 同 hash 去重 + 事件 200 上限——P2 补基准数据）。

## 16. Known Limitations（如实）

- **LIMITATION**：无全局粘贴钩子（unsafe 禁令）——外部应用的原生 Ctrl+V 不可拦截，只有"内容已在剪贴板"与"Weave 内复制动作"两处设防；palette 命令标签为静态（状态在面板如实呈现）；多显示器混合 DPI 环境未实测前台窗口矩形（不影响目标识别——只用进程名）。
- **RISK**：规则为确定性启发式——**漏检可能**（规则覆盖不了的内容不会被发现）；Mask 是字面替换，不能保证对抗性还原。
- **NOT SUPPORTED**：密码管理器/TOTP/Vault/DLP/键盘监听/浏览器注入/网页内容识别/云分析（§59 全清单未越界）。

## 17. P0/P1/P2

- **P0 全部交付**（§63 清单逐项：检测/规则/置信/风险 ✓；Guard 三防线+会话 ✓；安全六项（不泄日志/不覆盖新内容/race 守卫/TTL/崩溃处理/Unknown≠Safe+Degraded）✓；集成五项 ✓；UX 四项 ✓；性能有界 ✓；测试矩阵 ✓）。
- P1：更丰富规则包、事件高级过滤。（应用规则编辑器已在 R4 交付）
- P2：1k/10k 压测基准表、palette 动态状态标签。

## 18. 未实现能力

录屏式实时粘贴流拦截（平台不可能）；M2.10 Web / M2.11 Monitor 未提前实现（§1 禁令遵守）。

## 19. 后续阶段接口

`wisp_analyze_text` / `wisp_evaluate_current` 为稳定服务边界——M2.12 Smart Processing 可直接调用；规则集版本化字段（`ruleset_version`）为 v2 外置规则包预留。

## 20. 最终验收（§65 25 问逐项）

1. M2.3 唯一数据源 ✓　2. 无第二监听器 ✓　3. 无第二 Agent ✓　4. 无第二通知 ✓　5. 可解释（rule_id/evidence/masked preview）✓　6. 四概念分离（Sensitivity/Severity/Confidence/Risk 独立字段）✓　7. Unknown 处理为 Warn 非 Safe ✓　8. Limited 如实降级 ✓　9. 不会（指纹守卫 + 写前 suppress）✓　10. 是 ✓　11. 不会（无锁机制；会话惰性 TTL + 启动加载不自动恢复）✓　12. 无（测试固化）✓　13. 无（无任何钩子）✓　14. 无（零网络）✓　15. 否（占位符/单邮箱 → Allow）✓　16. 是（决策卡+理由+masked 预览）✓　17. 是（同 hash 去重 + 单会话 + 事件上限）✓　18. 无（事件 200/会话 1/分析 64K 截断）✓　19. 逻辑有界（监听节流）——压测表 P2 ✓*　20. 无（复用审计见 §2）✓　21. 无（仅 exe 级轻量防护，UI 明示 Limited）✓　22. 否（§59 边界未越）✓　23. 无（单会话 + 指纹守卫 + 写前登记）✓　24. 是（正常/敏感/模糊/异常四集测试）✓　25. 是（零网络）✓
