# Batch Engine（M7）— 引擎 / 管线 / 恢复 / 错误模型 / 限额

> 覆盖规格（下）§186 要求的 BATCH_ENGINE / PIPELINE_MODEL / RECOVERY /
> ERROR_MODEL / RESOURCE_LIMITS 五个主题（按仓库文档规范合并为一篇）。
> 权威来源：`Spec/ifuyo Weave — M7 Batch Engine 完整开发提示词（上/下）.md` +
> `DECISIONS.md` D51-D58。漂移以实际代码为准（weave-batch / src-tauri/batch_service）。

## 1. 架构边界（§162/§163/§218）

```text
Tool 层（weave-text / weave-media 域逻辑）
        ↓  Batch Adapter（engine 内按 StageSpec 投影调用，无第二套实现）
weave-batch（Job 状态机 / Pipeline / 子集执行 / Journal / Preview / Execute）
        ↑
weave-app（src-tauri/batch_service：IPC DTO / 任务化 / 目的地锁 / History 事务）
```

- 依赖方向：weave-batch → weave-{core,text,media}；不反向、不感知 React/Tauri
  （§165 headless：全部引擎能力可经 cargo test 直接驱动）。
- §218 审计：weave-batch 不含 `if ext == jpg` 类域逻辑——格式能力全部来自
  weave-media（ImageFormat::from_extension / encode_image / ImageLimits），
  文本变换全部来自 weave-text TransformKind。

## 2. Pipeline 模型（§11-§16/§56-§58）

Linear Only：`Source + 0..N Filter + 0..N Transform + 恰一个末置 Export`。
DAG / 分支 / 循环 / 子工作流**显式不实现**（§12/§0.2；M10 才是 Workflow）。

负载三态 `ItemPayload = Bytes | Text | Image`（§57 精简集）；`Pipeline::validate`
做结构 + 类型静态校验：TextTransform 要求 Text/Bytes（Bytes ⇒ 隐含 UTF-8
解码）；图像中转统一 PNG 无损；最终格式由 Encode 决定。适配层只投影不隐式
转换（§58）。

Stage 一览：

| Stage | 语义 | 失败类别 |
| --- | --- | --- |
| Source | 读输入文件字节 | Read |
| Filter | 扩展名白名单 / max_bytes；拒绝 ⇒ **Skipped**（非 Failed，§14） | — |
| TextTransform | 按序应用 weave_text 算子 | Decode / Transform / Validation |
| ImageResize | weave-media inspect(Limits)+resize，PNG 无损中转 | Decode / Validation / Unsupported |
| Encode | 目标格式编码（quality 按格式语义） | Encode / Unsupported |
| Export | 写 dest/{stem}.{ext}；批内重名预claim（快照序小者胜 §124）；已存在未开覆盖 ⇒ Collision | Collision / Write |

## 3. Preview / Execute 同引擎（§20-§23）

`preview_plan` 与 `execute_plan` 共享 `run_job`，唯一差异 = Export 阶段
`dry_run` 旗标：跳过 `fs::write`，但**碰撞检查照常**（= §22 Potential
Failures）。产物字节数 Preview 取内存负载、Execute 取落盘 stat。
JobResult.preview 标记来源；UI 不自行推断。

## 4. 执行与并发（§24-§36/下 §123）

- item 级失败隔离：单条失败不放弃其余（continue_on_error 语义）。
- 取消：item 起点 + 阶段间安全点；未开始条目 ⇒ Cancelled（终态）。
- Pause：安全点检查旗标；未开始条目 ⇒ `pending`（非终态，Resume 续跑）。
- workers：有界并发（1-8；1 = 顺序）。**结果按快照序还原，与 worker 数
  无关**（下 §123/§124；批内输出名冲突在计划期预claim——快照序小者胜，
  获胜方与完成时序无关）。进度回调只在调用线程触发（worker 事件经
  channel 汇聚），诚实 per-item 计数（§30）。

## 5. Journal 与恢复（下 §42/§126/§183/§185/§197）

- 形态：每 Job 一个 append-only JSONL（`app-data/jobs/job-<id>.jsonl`）：
  首行 header（schema_version + 完整 JobPlan + total），每条 item 完成
  追加一行 record，终态追加 finish 行（completed/failed/cancelled/paused）。
  已落盘行 = 已发生事实；崩溃 ⇒ 无 finish 行 = Interrupted（Recoverable）。
- Resume：journal 驱动（对 in-session paused 与跨进程 interrupted 统一）——
  已有终态事实的条目不重跑（§39 副作用不重复）；剩余条目**逐一重校验**
  size+mtime，不符者排除并作为 conflicts 上报（§143 Review Conflicts，
  §185 No Magic Recovery）；返回新 run 的 job_id，journal 续写。
- 损坏处理（§197）：坏行/半行**不 panic、不静默删除**——基于完好前缀
  恢复，resume 前把原件整体隔离为 `.jsonl.corrupt` 并显式上报。
- 版本（§196）：schema_version 不符 ⇒ `batch.journalVersion` 显式拒绝。

## 6. Retry（下 §39/§142）

`batch_retry_failed`：失败条目子集 + **全新快照**（当前文件事实）+ 同管线
同选项 = 新 Job 新 Journal（Retry 是新的一次运行）。从 item 起点重跑，
不存在"stage 中途续跑"的副作用重复问题。`retryable` 分类：Validation /
Unsupported 类失败不可重试（UI 显式 "Retry unavailable"）。

## 7. 互斥与冲突（下 §201-§204）

- 同一 Job 不会双执行：tracker job 唯一 + journal create 幂等拒绝。
- 目的地锁：canonical dest dir 粒度，同目录并发第二 Job ⇒
  `batch.jobConflictOnDestination`（策略 = 拒绝，不排队）。
- 输入重叠检测由 §75 快照重校验 + §238 resume 重校验承担（首跑/re-run
  都基于最新 stat 事实，不依赖运气）。

## 8. History / Undo（下 §184/§205-§206）

执行成功产物写入 weave-history（kind=`BatchExecute`）：创建型产物撤销 =
**删除已创建文件**（stat 守卫：用户改过 ⇒ UndoConflict）；覆盖写产物无
备份 ⇒ 事务记录 original_modified=None，撤销时守卫**必报冲突**（拒绝
删除，绝不静默二次破坏 §185）。历史写失败 ≠ 操作失败（M2 §84）。

## 9. 错误模型（下 §207-§209）

每层只解释自己：weave-batch 产生 `StageError { category, message }`
（Read/Decode/Transform/Encode/Write/Collision/Validation/Unsupported）；
IPC 层映射为 IpcError（code 前缀 `batch.*`）；UI 显示本地化消息 + 保留
原文（technical details）。无 `let _ =` 吞错——journal/history/cleanup
失败全部显式记录（warn 日志 / 冲突上报）。

## 10. 资源限额（下 §211/§36 同源）

单 item 负载驻留（图像受 weave-media ImageLimits：dim 16384 / pixels 80M /
decoded 256MiB 守卫）；Job 级 O(1) 额外；workers ≤ 8（IPC clamp）；
结果表 UI 分页 100 行（下 §145 有界 DOM）；journal 单文件随条目线性增长
（每行一条记录，无放大）。

## 11. 已知限制（如实声明，§247/§254）

- DiskFull / PartialWrite / 进程级 TransactionFailure 注入未覆盖
  （Windows 无便携注入手段）；Write 失败路径由"目标为目录"故障覆盖。
- Engine 内单条大图 decode 阶段不可中断（取消生效于条目/阶段边界）。
- workers>1 时 `current item` 进度描述为尽力而为（多 worker 并发推进）。
- Resume 冲突条目被排除后不会自动重试——用户需 re-plan（显式设计，
  防盲跑）。
