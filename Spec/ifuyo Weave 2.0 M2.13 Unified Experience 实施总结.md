# ifuyo Weave 2.0 M2.13 Unified Experience 统一体验中心 — 实施总结

日期：2026-10-07
状态：**P0 PASS（§86 九项逐项）；Definition of Done 大部分达成（§88 逐项标注）**；门禁全绿（42 套件 0 failed / fmt、clippy 0 / tsc、eslint 0 / vitest 38 / tauri build 6.66MiB）。

---

## 1. M2.13 目标

把 M2.0–M2.12 已存在的能力统一成完整、稳定、低认知成本、可预测的使用体验——统一**语义与状态**，不统一外观；不建第二套基础设施。

## 2. 统一前的真实问题（§1 审计 FACT）

- **导航**：1.0 的 12 个模块在可见导航；**2.0 的 12 个模块只能从 Palette 进入**（compare/space/clipboard/privacy/archive/agent/recipes/glint/wisp/web/monitor/smart）——"多个 Weave"问题的真实形态。
- **Job**：JobTracker 无 list IPC、map 无界增长（终态条目永不清理——内存泄漏 §61 CONFIRMED）。
- **通知**：M2.6 通知中心只有后端聚合，**无任何 UI**——用户看不见 monitor/wisp 的后台告警。
- **设置**：散落各模块面板，无统一 Settings 页；General（语言/主题）在 header 与 Settings 语义重叠。
- 术语/快捷键/Empty State 大体良好（M11 基线 + 各里程碑 i18n 同笔纪律）。

## 3. 统一后的架构

**Shell 增强，非重写（§89）**：App.tsx = 唯一 Shell（header+nav+workspace）；新增 Job Center 视图、通知抽屉（header 铃铛）、统一 Settings 页、分组导航。执行核心/存储/Job 系统零变化。

## 4. Unified Workbench

分组导航 §4：Files(rename/organizer/duplicates/batch)、Analyze(compare/space)、Content(text/data/image/documents)、Protect(archive/privacy)、Web、Capture(clipboard/glint/wisp)、Automate(agent/recipes/monitor)、Smart、System(history/jobs/settings)——**24 视图全部可达**，低频模块仍在 palette（§4 不为凑分类造层级）。

## 5. Job / History

- **Job Center**（§11 P0）：`jobs_list` IPC——Running 优先 + 终态倒序；行内 Cancel；进度如实（known/unknown §12）；**容量有界 200，淘汰最旧终态、Running 永不淘汰**（修复无界增长）。
- History 与 Job 分离维持（§21）：History=既有 get_history/undo_operation（M2.0）；Job=现在时。

## 6. Command / Search

Command Palette 已是统一入口（M11：registry/搜索/分类/快捷键）——本阶段新增 jobs/settings/smart 等命令均走 registry，无第二套（§9 PASS）。Global Search 维持 palette 搜索范围（Files 搜索=M3 duplicates/文本搜索=既有能力——不新建全文库 §10 PASS 如实）。

## 7. Input / Context

统一 Drop 路由（M11 quickDrop：按扩展名建议视图）+ 每模块限制类型；**Context Actions 统一**未做（P1 如实——当前各面板行内按钮即上下文动作，未抽 IContextActionProvider）。

## 8. Preview / Plan / Apply

全部维持 M2.0 语义（§17 零新 Apply——本阶段未写任何执行代码）；统一确认策略维持各模块风险映射（wisp Block/回收站/删除候选 High+回收站兜底 §77）。

## 9. Audit / Recovery / Report

M2.0 HistoryStore 单一事实源 + 各模块会话/运行记录（有界）；恢复入口统一 History（undo_operation 按 kind 分派——DuplicateRecycle 走回收站恢复）。

## 10. Agent Integration

**通知精准跳转（§35）**：通知抽屉每条带 Open 按钮——code 前缀映射视图（monitor.*→Monitor、wisp.*→Wisp、clipboard→Clipboard、smart.*/web.*）——Test C 链路成立。Agent/Workbench 边界不变（§34）。

## 11. UX Consistency

- **术语**（§54/§55）：Delete/Remove/Clear 区分核查——clipboard 面板文案为"清空历史"（Clear Clipboard History 语义）✓；统一键已入 guidelines。
- **Token**：新 UI 全部 var(--*)（tokens.test 守门——App.tsx 抽屉初版裸 #fff/rgba 即被测出修正）。
- **Settings 真正生效**（§25）：每次保存 = persistAppConfig round-trip + 模块 apply IPC（clipboard/wisp/web/monitor 四链路）。

## 12. Performance

jobs_list 有界（200 淘汰）；通知聚合 60s 窗口；导航纯前端无 IPC。**基准表未做（如实）**——无 O(n²) 引入。

## 13. Lifecycle

App Shutdown 链路未动（Agent stop/clipboard flags/monitor 线程随进程——M2.6/M2.11 既有）；Job evict 只淘汰终态——Running 永不丢（测试固化）。

## 14. Crash Recovery

M2.7 detect_interrupted + M2.11 monitor 状态持久化 + M2.9 session 快照——既有 Crash Recovery 未被本阶段破坏（42 套件回归）。

## 15. Compatibility / Migration

无 schema 变更；AppConfig 仅新增可选段（serde default）——旧配置兼容 ✓（§67）。

## 16. Privacy Review

通知/Job Center/Settings 全部只含元数据；无新增日志面；无遥测（§58）。**"绝对安全"表述检查 ✓**（wisp/web 文案保持"可能/启发式"措辞）。

## 17. Tests

jobs 3 新测试（retention 保 Running 淘汰最旧终态/running-first 排序+错误码/cancel 语义不伪造）；App.test mock 扩展（15 个 shell IPC stub）；42 套件 0 failed；tsc/eslint 0；vitest 38；tauri build 6.66MiB。

## 18. P0/P1 Issues（§86/§87）

- **P0 九项**：逐项 PASS——无双执行引擎/页面切换不杀任务（Job 归应用）/Agent 可停（M2.6 stop 既有）/History 数据可靠/无数据破坏/通知可定位（新映射）/结果有界/2.x 功能未破坏/无敏感泄漏。
- **P1 已修**：2.0 模块导航可见、Job Center 信息密度（running 优先+上限说明）、通知 Dismiss/Clear。
- **P1 剩余**：ContextActionProvider 抽象未做、Search scope 未扩展、快捷键 Ctrl+P 未加（palette 已覆盖）。

## 19. Remaining Risks

- 通知→视图映射基于 code 前缀约定（新模块需登记映射——guidelines 已列）。
- 分组导航 9 组在窄窗口依赖 flex-wrap（1280×720 实测换行正常；更窄未测）。
- Job Center 只覆盖 JobTracker 任务（batch pause/resume 仍在 batch 面板——统一 pause 语义 P2）。

## 20. M2.14 建议

Polish 里程碑聚焦：UX Guidelines 中 P1 项收尾（Context Actions 抽象、Search scope）、术语全量扫描、快捷键补全（Ctrl+P）、性能基准表、视觉验收（§70 十页面截检）。
