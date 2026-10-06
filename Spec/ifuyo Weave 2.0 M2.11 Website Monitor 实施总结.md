# ifuyo Weave 2.0 M2.11 Website Monitor 网站监控中心 — 实施总结

日期：2026-10-07
状态：**P0 PASS（§84 逐项）；P1 PARTIAL（Selector UI 精简版 ✓、Robots/magnitude ✓、高级通知策略/结构分析 未实现——如实）**；门禁全绿（42 套件 0 failed / fmt、clippy 0 / tsc、eslint 0 / vitest 38 / tauri build）。

---

## 1. 实施内容（§84 P0 逐项）

**核心能力**：创建监控目标 ✓（monitor_create：名称/URL/模式/selector/间隔，URL scheme 白名单校验，间隔 ≥60s §18）；删除（§63 默认仅删 Target 保留历史）/暂停/恢复 ✓；手动 Check Now ✓（经 JobTracker，与调度共用 run_check §24）；自动定时检查 ✓（M2.6 Scheduler `monitor:<id>` 每目标注册）；HTTP 抓取 ✓（复用 M2.10 web::fetch）；Timeout ✓（默认 30s）；Retry ✓（连续失败退避 interval×2^min(n,4) §18，429 同路径受制）；Cancellation ✓（fetch 分块检查 + per-target 锁随 Agent 生命周期）；Redirect ✓（final_url 记录）；Snapshot ✓；Hash ✓（规范化文本 SHA-256 §55）；Change Detection ✓；History ✓（逐快照文件 + 有界事件）。

**内容能力**：WholePage ✓（plain_text 规范化）；至少一种稳定提取 ✓（MainContent 启发式提取 + Element CSS Selector——三种全真实可用）；Normalize ✓（行尾/行尾空白/空行折叠）；NoChange/Changed ✓；Added/Removed/Modified ✓（行级 diff 统计 + unified diff——删除被识别为真实变化 §3 场景 4）。

**集成**：M2.6 Agent ✓（scheduler job_name + agent_cancel 贯穿检查）；M2.7 Scheduler ✓（dispatch_due_schedules `monitor:` 分支）；M2.1 Compare ✓（text_service::compare_text / weave-text 有界 LCS——无第二 Diff §41）；Notification ✓（M2.6 NotificationCenter，on_change/on_error/on_recovery 三开关 §22）；Audit ✓（tracing 元数据 + creation-undo 报告导出）。

**稳定性**：并发限制 ✓（max_concurrent_checks=4 全局 + per-target inflight 锁 §48）；Rate limit ✓（间隔下限 60s + backoff）；最大响应大小 ✓（5MB 截断）；历史容量 ✓（每 target 50 快照 + 30 天，最近+基线受保护 §28）；单 Target 错误隔离 ✓（run_check catch_unwind + 逐 target 独立状态 §46）；程序重启恢复 ✓（targets/snapshots 持久化 + register_all_schedules）；Shutdown 安全 ✓（无自建常驻线程——检查线程随进程退终止，fetch 响应 cancel）；任务取消 ✓；无重复监控执行 ✓（inflight 锁：手动+调度同 target 只跑一个）；无资源泄漏 ✓（reader drop / 锁释放 / catch_unwind 兜底）。

**安全**：不泄露 Cookie/Token ✓（无凭据机制 §38）；网页正文不进日志 ✓（tracing 只记 id/result/trigger；报告 URL 经 mask_sensitive §62）；不绕过访问限制 ✓（403 ⇒ Blocked 如实 §19）；无反爬绕过 ✓。

## 2. 架构变化 / 复用 / 新增

- **复用**（§88 Q1）：Job=JobTracker；Scheduler=M2.6 Agent Scheduler；HTTP=M2.10 web::fetch（唯一网络出口）；Diff=weave-text（经 text_service）；Notification=M2.6 NotificationCenter；Persistence=原子 JSON 范式（recipe/clipboard 同构）；Hash=weave_files::hash。
- **新增**：`monitor/{model,normalize,store,check}.rs`（领域）+ `monitor_service.rs`（14 IPC）+ `MonitorPanel.tsx`。无第二套任何基础设施（Q2/Q3：否）。
- **调度接线方式（§32）**：dispatch_due_schedules 的 `monitor:` 分支经 **Handler 注册表（OnceLock 回调）** 执行——生产由 start_agent 注册（闭包持 AppHandle），dispatch 保持零 tauri 符号。此设计顺带修复了一个被 clean 重建暴露的构建问题（见 §19）。

## 3. 数据模型

`MonitorTarget`（id/name/url/enabled/paused/mode/selector/interval/notify 三开关/consecutive_failures/last_* /status）；`MonitorSnapshot`（seq/status_code/fingerprint/text_size/duration/result/payload_file——metadata 与正文分离 §29，NoChange metadata-only §46）；`MonitorChangeEvent`（result/magnitude/added/removed/previous/current/summary——确定性摘要 §13）。seq 为 f64（specta 纪律）。

## 4. 监控模式 / Normalize / 指纹

WholePage=页面规范化文本；MainContent=启发式正文提取（复用 M2.10）；Element=CSS Selector 首个匹配（测试命令作用于最近一次 M2.10 抓取缓存 §34 最低要求）。Normalize=行尾统一+行尾空白剔除+空行折叠；**指纹基于规范化提取文本而非 raw body（§55）**——广告/时间戳变化在 MainContent 模式下不产生假变化（§58/§75 测试存在）。

## 5. HTTP 策略 / 失败处理

状态分类：2xx 正常；403 ⇒ Blocked；404/4xx ⇒ FetchFailed（Unavailable 语义，不宣布"内容删除" §16）；5xx/网络 ⇒ FetchFailed。**失败绝不写新快照、绝不覆盖 last_fingerprint（§15）**；失败后成功 ⇒ Recovered 事件（§73）。Backoff：连续失败 n ⇒ 实际间隔 interval×2^min(n,4)。

## 6. Snapshot / Diff / UI

快照 payload 仅 Initial/Changed 存全文；NoChange metadata-only。Diff：UI"对比最近两次内容快照"→ monitor_diff → compare_text → unified 视图 + 行统计（§56 Current vs Previous；历史 A vs B 由 seq 参数天然支持）。UI：列表（状态/间隔/上次检查/Check Now/暂停/删除）+ 创建卡（**scope 解释行 §33**）+ 详情（历史表/变化表/diff）。

## 7. Scheduler / Notification / Storage

§23 集成：每 target 一个 Interval Schedule（SkipIfRunning）；dispatch spawn 线程异步检查（tick 不被网络拖住）；启动恢复注册。通知三策略 + NotificationCenter 60s 聚合（§22 无轰炸）。存储：`history_dir/website-monitor/{targets,events}.json + snapshots/<tid>/<seq>.json`——损坏隔离（.corrupt）、事件有界 1000、快照 50/target（可配）。

## 8. 测试结果

- monitor 域 12 测试（删除即变化/损坏隔离/retention 保护基线/事件有界/ID 防穿越/backoff/magnitude/规范化假变化防线）；agent_service 2 集成测试（无注册回调如实释放/recipe 通路回归守卫）；全工作区 **42 套件 0 failed**（--no-fail-fast）。
- UI：tsc 0 / eslint 0 / vitest 38。Rust：fmt/clippy **0**。tauri build 6.6MiB ✓。

## 9. 性能结果

结构性有界：并发 ≤4、响应 5MB、快照 50/target、事件 1000、批量分页（历史 latest-20 渲染）。**500/1000 targets 压测基准表未做**（§50/§76 如实 P2）——结构上每检查为独立 spawn 线程 + 全局上限，调度本身 O(targets) 扫描。

## 10. 已知限制（如实）

- **JS 渲染页面**：不支持（§36 如实——Unsupported 场景；无 Headless，M2.10 边界延续）。
- **登录/认证页面**：不支持（§37/§38 Public URL only）。
- Element 多匹配只取首个（多元素监控 P1）；robots.txt awareness 未实现（§20 允许；默认低频克制）。
- 压测基准表未做（§9）；diff 依赖两次有 payload 的快照（NoChange 不占存储的设计使然——有意的容量取舍 §46）。
- 手动 Check Now 与调度在同 target 并发时后者 skipped（§48 语义，UI 显示为列表刷新而非报错）。

## 11. Out of Scope（未越界确认）

§82 全清单未实现：无浏览器/Headless/登录/Cookie/OAuth/CAPTCHA/反爬/代理池/爬虫/RSS/云监控/同步/AI 功能。本地离线，无遥测（§61）。

## 12. §88 十四问逐答

Q1 全复用（见 §2）。Q2 无。Q3 无。Q4 不会（失败路径不写指纹/快照）。Q5 不会（inflight 锁）。Q6 能（agent_cancel 贯穿 fetch 分块；无常驻自建线程）。Q7 结构可接受（有界并发；基准表 P2）。Q8 不会（三重容量控制）。Q9 不会（MainContent 模式 + 规范化指纹；§75 测试存在）。Q10 能（创建卡 scope 解释 + UI 规范化说明）。Q11 无（元数据 only + mask）。Q12 无（§82 未越界）。Q13 是（Agent+Scheduler 真实消费者）。Q14 否。

## 13. 附带修复（本里程碑内）

- **构建回归修复**：clean 重建后 lib 单元测试二进制 STATUS_ENTRYPOINT_NOT_FOUND（`cargo:rustc-link-arg-tests` 只覆盖 tests/ 集成目标，不覆盖 lib 单元测试二进制——tao/wry 窗口栈对象一旦被测试代码引用即触发加载器失败）。修复 = §32 Handler 注册表解耦（dispatch 零 tauri 符号），非链接器 hack；tauri bin 的 manifest 注入不受影响（release 构建验证通过）。
- compare nested_directories flake 修复（M2.10 里程碑内完成，同分支）。
