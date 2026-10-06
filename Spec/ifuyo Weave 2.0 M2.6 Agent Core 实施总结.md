# ifuyo Weave 2.0 — M2.6 Agent Core 实施总结

日期：2026-10-06。提交链：AG1 (runtime/queue/scheduler core) → AG2 (lifecycle
wiring + IPC) → AG2b (Arc flag sharing) → A-UI (AgentPanel) → lint fixes。

## 1. 实际完成内容（全部有代码/测试证据）

- **Agent Runtime**（src-tauri/src/agent/）：AgentState 状态机（Stopped/
  Starting/Running/Pausing/Paused/Stopping/Faulted + Faulted→Starting 恢复
  通道，迁移显式校验 §4）；Start/Stop/Pause/Resume 幂等（Mutex 状态机 §8）。
- **Scheduler**（§12/§13/§14/§36/§37）：Interval/Daily，NextRunAt 基于
  last+interval（无漂移）；Daily 本地时区日窗重算；MissedRun Skip/RunOnce
  策略（防补跑风暴）；ConcurrencyPolicy SkipIfRunning（防重复执行）；
  ScheduleDef 持久化 agent-schedules.json（temp+rename，§35/§36）。
- **BoundedJobQueue**（§15/§16）：容量有界、DropOldest/Reject 溢出策略、
  Interactive 优先；**ConcurrencyGate**（§17）：MaxConcurrentJobs 有界。
- **Clipboard 接入**（§27）：Agent ClipboardComponent 与 M2.3 捕获线程
  **共享同一 Arc<AtomicBool> 标志**（单一主人——agent pause == capture pause）；
  注册/健康归 Agent；自环抑制（M2.3 指纹机制）原样保留（§28）。
- **通知**（§22/§23）：统一 NotificationCenter，60s 窗口同 code 聚合计数。
- **生命周期**（§9/§10）：App setup（配置加载→存储就绪）→ agent start；
  App Exit → agent stop + clipboard stop（RunEvent::Exit 钩子）。
- **健康**（§20）：agent_status IPC——状态/组件/队列/并发/丢弃计数/最近错误。
- **UI**（§40）：AgentPanel——●/○ 状态、组件列表、队列/并发统计、调度清单、
  通知聚合、暂停/恢复；导航+palette；i18n zh/en 同笔。

## 2. 实际架构（与既有系统的关系——§73 审计问答）

- **无第二套 Job System**：Agent 不自建执行——后台组件经共享标志位/既有
  JobTracker；调度 tick 只登记事实（M2.7 Recipe 作为 Job 接入点已留 §52）。
- **无第二套 Notification/Storage**：NotificationCenter 是统一出口（此前各
  service 不自行通知）；调度持久化复用 history_dir 约定。
- **单一主人**（§7）：AgentRuntime 是 clipboard paused/enabled 标志的共享方
  ——UI 页面不创建监听（M2.3 起即在 app setup 启动，页面关闭不影响 §27）。
- **Exit**：RunEvent::Exit → agent.stop() + clipboard.stop()（幂等）。

## 3. 性能

Agent tick 1s 轮询（sleep 非忙等）；队列有界 256/并发 4；无独立 CPU 路径。
未做 1h/8h soak（P1 §55——基础设施为轮询+有界队列，无累积增长结构）。

## 4. 测试

10 单元测试（状态机迁移矩阵/Interval 无漂移/Daily 过窗跳天/Interval=0 拒绝/
MissedRun Skip-RunOnce-SkipIfRunning-disabled 四态/队列有界 DropOldest/
Reject/Interactive 优先/并发闸门）；全工作区 42 套件 0 failed；vitest 38。

## 5. Known Issues / P1 / P2

- P0：§66 全项有证据。
- P1（§67 延期）：System Tray、Windows Login Startup、Component 自动恢复、
  1h/8h soak、高级 Health Dashboard、Folder Watcher（当前无 Watcher 基础设施
  ——M2.6 未引入 notify 依赖，Recipe/M2.7 需要时再建）。
- P2：调度任务当前无内置业务 Job（handler 注册表为空——M2.7 接入）；
  missed-run 的 tick 判定依据"上次 tick < next"由 collect_due 内部时间比较
  实现。

## 6. Out of Scope（§68）全部未触碰

Recipe Engine/Workflow Editor/Cloud Sync/Windows Service/Plugin/任意 Shell/
DLP/AI 均未实现。
