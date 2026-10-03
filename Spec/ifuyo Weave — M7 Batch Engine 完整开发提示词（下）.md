# ifuyo Weave — M7 Batch Engine 完整开发提示词（下）

> 本文件是原《ifuyo Weave — M7 Batch Engine 完整开发提示词》的 **第 2/2 部分**，
> 覆盖原第 118–258 节：Testing Strategy、单元/集成/属性/模糊测试与性能基准、UI Architecture 与全部 UI/UX 及 a11y/i18n、Tool Registry 与能力矩阵、架构目标与 Crate 边界、E2E 场景与审计、Documentation 与 Fixture/Golden/JSON Contract、AI Coding Workflow 执行流程与 P0/P1 规则、CI Gate 与 No Fake Tests、Final Audit A–P、验收标准与提交纪律、M8/M9/M10 Handoff 与最终完成标准。
> **必须与（上）一起阅读执行**：硬边界、设计原则、领域模型与后端实现规范均在（上）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---

# 118. Testing Strategy

M7 必须同时建立：

```text
Unit Tests
Integration Tests
Filesystem Tests
Fault Injection Tests
Property Tests
Concurrency Tests
Cancellation Tests
Persistence Tests
Recovery Tests
Performance Tests
UI Smoke Tests
```

不能只有：

```text
unit tests
```

---

# 119. Core Unit Tests

至少覆盖：

```text
Job State
Pipeline Validation
Type Compatibility
Filter Semantics
Result Aggregation
Retry Policy
Cancellation
Progress
Collision Policy
Deterministic Ordering
```

---

# 120. Integration Tests

真实执行：

```text
Rename
Organizer
Image
Text
Data
```

通过 Batch Engine。

重点验证：

> 前面 M2–M6 工具真正接入统一引擎，而不是只在测试里造 mock Tool。

---

# 121. Real Filesystem Tests

必须使用：

```text
temporary filesystem
```

真实创建：

```text
files
folders
nested directories
Unicode paths
long names
collisions
locked files where platform allows
```

验证：

```text
真实 mutation
真实 result
真实 error
```

---

# 122. Fault Injection

至少模拟：

```text
PermissionDenied
SourceMissing
SourceChanged
DestinationExists
DestinationWriteFailed
DiskFull
PartialWrite
LockedFile
ToolFailure
EncodeFailure
DecodeFailure
TransactionFailure
HistoryFailure
Cancellation
Process Interruption
```

---

# 123. Concurrency Tests

验证：

```text
1 worker
2 workers
4 workers
bounded max workers
```

结果必须一致。

即：

```text
same plan
same inputs
```

不能因为并发从：

```text
4 workers
```

变成：

```text
1 worker
```

就产生不同的最终结果。

---

# 124. Race Tests

重点：

```text
Preview
      ↓
外部程序修改
      ↓
Execute
```

以及：

```text
Worker A
Worker B
```

同时访问：

```text
same destination
```

必须正确处理：

```text
collision
conflict
serialization
```

---

# 125. Cancellation Tests

测试取消发生在：

```text
before start
during planning
during preview
during item
during stage
during write
after partial completion
```

验证：

```text
no corruption
state truthful
history consistent
temporary files cleaned
```

---

# 126. Resume Tests

至少：

```text
clean interruption
mid-stage interruption
post-write interruption
post-journal interruption
```

然后：

```text
resume
```

验证最终状态。

---

# 127. Property Tests

适合做：

```text
result counts
state transitions
pipeline order
filter invariants
deterministic sort
retry bounds
```

例如：

> 所有 Item 最终只能落在合法终态集合。

---

# 128. Invariant Tests

必须明确 Engine invariants。

例如：

```text
Success + Failed + Skipped + Cancelled = Total
```

在适用场景下必须成立。

并：

```text
Failed item cannot have Export Success
```

以及：

```text
Cancelled Job cannot claim all items Success
```

---

# 129. Result Integrity

验证：

```text
Engine counts
filesystem reality
history reality
```

三者一致。

不要出现：

```text
UI says 100 success
filesystem only 97 outputs
```

---

# 130. Performance Test Matrix

至少：

```text
100 items
1,000 items
10,000 items
```

并尽可能增加：

```text
50,000 items
```

测试 workload 至少包含：

```text
small files
medium files
large files
mixed success/failure
pipeline with 2 stages
pipeline with 4+ stages
```

---

# 131. Memory Benchmark

至少观察：

```text
peak RSS / peak process memory
```

并比较：

```text
1 worker
4 workers
8 workers
```

不要只看：

```text
elapsed time
```

因为：

> 快 20% 但内存爆炸，不属于有效优化。

---

# 132. Cancellation Latency

记录：

```text
Cancel requested
        ↓
Execution stopped
```

测量：

```text
latency
```

特别是：

```text
large image
large text
large CSV
large directory
```

---

# 133. Backpressure Benchmark

验证：

```text
fast producer
slow consumer
```

不会：

```text
unbounded memory growth
```

---

# 134. M2–M6 Regression

M7 完成后必须重新验证：

```text
M1
M2
M3
M4
M5
M6
```

至少：

```text
existing tests pass
```

并且：

> 旧工具不接入 M7 的场景仍然能独立正常工作。

---

# 135. Direct Tool vs Batch Engine

每个工具需要有两种能力边界：

```text
Direct Execution
Batch Execution
```

例如：

```text
Image Resize
```

用户：

```text
1 image
```

可以直接使用。

也可以：

```text
1000 images
```

通过：

```text
Batch Engine
```

二者：

> 必须共享 domain logic。

---

# 136. 禁止双重实现

不要：

```text
ImageResizeTool
ImageResizeBatchTool
```

两套逻辑。

应该：

```text
ImageResizeTool
      ↓
Batch Adapter
```

---

# 137. UI Architecture

至少建立：

```text
Batch Job View
Job Preview View
Execution View
Result View
Failed Items View
Job History View
```

但保持：

```text
Utility-first
```

不要设计成：

```text
企业任务调度中心
```

---

# 138. Batch UI

用户应能看见：

```text
What will happen
```

执行中：

```text
What is happening
```

完成后：

```text
What happened
```

也就是：

```text
Preview
Execution
Result
```

三个阶段必须清楚。

---

# 139. Preview UI

至少显示：

```text
Pipeline
Input count
Filtered count
Potential changes
Output location
Collision warnings
Warnings
```

对于代表性 item：

```text
Before → After
```

---

# 140. Execution UI

至少显示：

```text
Overall Progress
Current Item
Current Stage
Success
Failed
Skipped
Cancelled
```

提供：

```text
Cancel
```

而不是：

```text
Cancel Button 只是 UI 动画
```

必须真实连接 Engine cancellation。

---

# 141. Result UI

完成后：

```text
Success
Failed
Skipped
Cancelled
```

都可点击查看。

例如：

```text
Failed (3)
```

打开：

```text
item
stage
reason
retryable
```

---

# 142. Retry UI

支持：

```text
Retry Failed
```

如果某个 item：

```text
NonRetryable
```

必须显示：

```text
Retry unavailable
```

并解释原因。

---

# 143. Resume UI

恢复任务时，应显示：

```text
Interrupted Job
43 completed
12 pending
2 conflicts
```

然后：

```text
Resume
Review Conflicts
Cancel
```

不要：

```text
点击一下直接重放全部
```

---

# 144. Job Details

允许查看：

```text
Pipeline
Options
Inputs
Execution Time
Result
Failures
Retries
Outputs
Undo Status
```

但不要把：

```text
10000 个 item
```

一次全部渲染到 React DOM。

必须：

```text
virtualized
paged
bounded
```

---

# 145. Large Result UI

10k/50k item：

不能：

```text
render all rows
```

应：

```text
virtual list
paging
filter
sort
```

---

# 146. Result Search

M7 可以提供：

```text
Search within current Job Result
```

例如：

```text
failed
jpg
permission
```

但不要提前实现：

```text
Global Search
```

那是后续：

```text
M9 / M10
```

---

# 147. Job Filtering

可以支持：

```text
All
Success
Failed
Skipped
Cancelled
```

以及：

```text
Retryable only
```

但保持简单。

---

# 148. Accessibility

至少检查：

```text
keyboard navigation
focus
button semantics
progress announcements
error readability
```

特别是：

```text
long-running Job
```

不能依赖颜色区分：

```text
Success / Failure
```

---

# 149. Internationalization

所有 UI 文案必须走现有 i18n。

不要在组件中硬编码：

```text
Batch completed
3 failed
Retry
```

---

# 150. Error UX

用户看到的不是：

```text
Rust error:
Os { code: 5 ... }
```

而应该是：

```text
无法处理此文件

原因：
文件被其他程序占用

操作：
Image → Convert → Export
```

同时保留：

```text
technical details
```

供诊断使用。

---

# 151. No AI Requirement

整个 M7：

> **不需要 AI。**

不能依赖：

```text
LLM
network
cloud
API key
```

M10 范围以 M10 提示词为准（线性 Workflow）；Natural Language Batch 属 charter #48 [C] 加分项，不在既定范围。

---

# 152. Batch Engine API Safety

任何用户输入：

```text
path
tool options
pipeline
destination
```

必须经过：

```text
parse
validate
normalize
capability check
```

再进入：

```text
execution
```

---

# 153. No Arbitrary Code Execution

Pipeline Definition：

> 不得支持任意用户脚本直接执行。

例如不要实现：

```text
run shell command
run arbitrary PowerShell
run arbitrary JS
run arbitrary Rust
```

否则 Weave 会从：

```text
utility
```

变成：

```text
arbitrary code execution platform
```

---

# 154. No Generic Scripting Engine

M7 不实现：

```text
Script Node
Python Node
Shell Node
JavaScript Node
```

这不是 M7 的责任。

---

# 155. Tool Registry Integration

如果已有：

```text
Tool Registry
```

必须复用。

M7 应从 Registry 发现：

```text
available tools
capabilities
metadata
```

不要：

```text
hardcode every tool
```

---

# 156. Capability Matrix

建立清晰能力矩阵：

| Tool | Preview | Batch | Pipeline | Retry | Resume | Undo |
|---|---|---|---|---|---|---|
| Rename | Yes | Yes | Yes | Yes | Yes | Yes |
| Organizer | Yes | Yes | Yes | Yes | Yes | Yes |
| Duplicate Finder | Yes | Limited | No/Analyze | Limited | Yes | N/A |
| Text Transform | Yes | Yes | Yes | Yes | Yes | Yes/where supported |
| Data Cleaner | Yes | Dataset-based | Yes | Yes | Yes | Export undo |
| Image Resize | Yes | Yes | Yes | Yes | Yes | Depends on output mode |

注意：

> 表格必须基于实际实现更新，不能把“应该支持”写成“已经支持”。

---

# 157. Duplicate Finder 特殊边界

M3 的 Duplicate Finder：

> 不是普通 Item Transform。

它更像：

```text
Analyze
 ↓
Duplicate Groups
```

因此 M7 不得强行把它变成：

```text
File → File
```

可以让 Batch Engine 后期消费其：

```text
structured result
```

但第一版：

> 明确记录这种能力边界。

---

# 158. Data Tool 特殊边界

例如：

```text
CSV Cleaner
```

本质可能是：

```text
Dataset
 ↓
Transform
 ↓
Dataset
```

不是：

```text
File
 ↓
File
```

因此 Engine 必须支持：

> **logical item / document-level processing**

而不是死绑文件路径。

---

# 159. Image Tool 特殊边界

M6 已经支持：

```text
multi-image processing
```

M7 必须重构或接入为：

```text
one image operation
+
same options
+
many inputs
```

如果当前 M6 已经实现自己的批处理：

> 不要保留两套 engine。

应：

```text
M6 Image Multi-Processing
        ↓
M7 Batch Engine Adapter
```

逐步统一。

---

# 160. M6 Migration

首先审计 M6：

```text
multi-image queue
concurrency
progress
cancellation
result aggregation
```

判断哪些可以：

```text
move into M7
```

哪些继续属于：

```text
Image domain
```

---

# 161. M6 不得被破坏

M7 重构 M6 时必须保证：

```text
Image Resize direct
Image Convert direct
Image Metadata direct
Image Compress direct
```

继续工作。

---

# 162. M7 Architecture Target

最终目标：

```text
                    ┌───────────────┐
                    │   Tool Layer  │
                    └───────┬───────┘
                            ↓
                    ┌───────────────┐
                    │ Tool Adapter  │
                    └───────┬───────┘
                            ↓
                    ┌───────────────┐
                    │ Batch Engine  │
                    └───────┬───────┘
                            ↓
          ┌─────────────────┼─────────────────┐
          ↓                 ↓                 ↓
       Planner           Scheduler        Journal
          ↓                 ↓                 ↓
       Preview          Execution          Recovery
          └─────────────────┼─────────────────┘
                            ↓
                     Result / History
```

---

# 163. Crate Boundary

优先：

```text
weave-batch
```

负责：

```text
Job
Pipeline
Planner
Scheduler
Execution
Retry
Resume
Progress
Result
```

而：

```text
weave-files
```

负责：

```text
filesystem
path safety
safe write
```

```text
weave-history
```

负责：

```text
history
persistence
```

具体依赖方向服从 M0 已建立的 workspace 架构。

---

# 164. weave-core 不膨胀

不要把：

```text
Batch Engine
Scheduler
Retry
Persistence
```

全部塞进：

```text
weave-core
```

Core 只保持：

```text
stable contracts
primitive domain types
```

---

# 165. Batch Engine 不依赖 React

Rust Engine：

> 必须可脱离 UI 执行。

至少能通过：

```text
unit tests
integration tests
headless execution
```

证明。

---

# 166. Headless Testability

所有关键能力应该可以：

```text
create Job
preview
execute
cancel
retry
resume
```

不依赖：

```text
browser DOM
window
manual clicking
```

---

# 167. UI Smoke

真实 UI 至少跑通：

```text
Select files
 ↓
Create pipeline
 ↓
Preview
 ↓
Start
 ↓
Progress
 ↓
Failure
 ↓
Retry
 ↓
Result
```

以及：

```text
Cancel
```

---

# 168. End-to-End Scenarios

至少实现以下真实场景。

### Scenario A — Image Pipeline

```text
10 JPG
 ↓
Resize 1920
 ↓
WebP
 ↓
Strip Metadata
 ↓
Output
```

要求：

```text
Preview
Execute
Result
Failures
History
```

---

# 169. Scenario B — Rename + Organizer

```text
Input Folder
 ↓
Filter images
 ↓
Rename
 ↓
Move to output/images
```

验证：

```text
collision
path safety
undo
```

---

# 170. Scenario C — Text

```text
100 text files
 ↓
Trim Lines
 ↓
Deduplicate
 ↓
Export
```

验证：

```text
same output semantics
```

---

# 171. Scenario D — Data

```text
CSV
 ↓
Cleaner
 ↓
Export CSV
```

或者：

```text
CSV
 ↓
Convert JSON
 ↓
Export
```

验证：

```text
Data semantics
encoding
null
leading zero
```

---

# 172. Scenario E — Partial Failure

```text
100 files
```

其中：

```text
3 inaccessible
```

最终：

```text
97 success
3 failed
```

且：

```text
Retry 3
```

单独重新处理。

---

# 173. Scenario F — Cancellation

```text
1000 images
```

执行中：

```text
Cancel
```

最终：

```text
Completed
+
Cancelled
```

状态必须真实。

---

# 174. Scenario G — Crash Recovery

人为：

```text
terminate process
```

重新启动：

```text
open interrupted job
```

然后：

```text
Resume
```

验证。

---

# 175. Scenario H — External Mutation

执行前：

```text
modify source
```

必须：

```text
detect
```

而不是：

```text
blind execute
```

---

# 176. Scenario I — Destination Mutation

Preview 后：

```text
create conflicting destination
```

Execute：

> 必须阻止危险覆盖。

---

# 177. Scenario J — Multi-stage Failure

```text
Resize
 ↓
Convert
 ↓
Metadata Strip
 ↓
Export
```

让：

```text
Metadata Strip
```

对部分 items 失败。

必须：

```text
failure isolated
```

---

# 178. Benchmark Scenario

至少：

```text
1000 images
```

记录：

```text
1 worker
4 workers
8 workers
```

指标：

```text
elapsed
peak memory
success
failure
cancel latency
```

不要只报告：

```text
X files/sec
```

---

# 179. Performance Claims

禁止：

> “M7 比 M6 快 5 倍。”

除非有：

```text
baseline
hardware
dataset
configuration
method
```

全部可复现。

---

# 180. Resource Leak Audit

重点检查：

```text
file handles
temp files
threads/tasks
channels
memory
locks
journal handles
```

长时间运行：

```text
10000 items
```

不能：

```text
memory continuously grow
```

---

# 181. Deadlock Audit

并发系统必须检查：

```text
Mutex
RwLock
channel waits
worker shutdown
cancellation
journal write
history write
```

尤其是：

```text
Cancel
+
worker exits
+
UI waits
```

可能形成：

```text
deadlock
```

必须实际测试。

---

# 182. Shutdown Semantics

应用退出时：

如果 Job 正在运行：

必须明确：

```text
Allow exit
Wait
Cancel
Persist state
Recover later
```

M7 第一版选择最合理且最安全的实现。

不要出现：

> 用户关闭窗口后 Job 还在偷偷修改文件，却没有任何可观测状态。

---

# 183. App Restart

重启后：

```text
job list
```

应该能够区分：

```text
Completed
Failed
Interrupted
Recoverable
```

---

# 184. History Reuse

从 History：

```text
Open Again
```

得到：

```text
Job Template / Plan
```

然后重新：

```text
validate
preview
execute
```

不要：

```text
直接执行旧计划
```

---

# 185. No Magic Recovery

恢复时：

禁止：

```text
“看起来没问题，所以继续”
```

必须有：

```text
verification evidence
```

---

# 186. Documentation

至少新增或更新：

```text
docs/BATCH_ENGINE.md
docs/PIPELINE_MODEL.md
docs/RECOVERY.md
docs/ERROR_MODEL.md
docs/RESOURCE_LIMITS.md
```

具体文件名可按仓库规范调整。

---

# 187. Architecture Documentation

明确说明：

```text
Tool
Batch Adapter
Planner
Scheduler
Execution
Journal
History
Undo
```

以及：

```text
M7
vs
M10
```

边界。

---

# 188. Decision Record

至少记录：

```text
为什么先 Linear Pipeline
为什么不做 DAG
为什么 Job 不承诺 Atomic
为什么 Retry 从失败 Stage 开始
为什么 Resume 需要 Revalidation
为什么 Batch Engine 不允许 arbitrary scripts
```

写入：

```text
DECISIONS.md
```

或项目实际 ADR 体系。

---

# 189. Security Documentation

更新：

```text
SECURITY.md
```

明确：

```text
Path Safety
Symlink
Resource Limits
Temporary Files
No Shell
No Upload
No Arbitrary Code
```

---

# 190. Privacy Documentation

确认：

```text
Job Journal
History
Logs
```

全部符合：

```text
Local-first
No Telemetry
No Upload
No Cloud
```

---

# 191. README

M7 完成后 README 应能够描述：

```text
Batch Processing
Linear Pipeline
Preview
Progress
Retry
Recovery
History
Undo
```

但不要宣称：

```text
visual workflow builder
```

如果 M10 尚未完成。

---

# 192. Test Fixture Strategy

不要把：

```text
大型真实用户文件
```

提交到仓库。

建立：

```text
small deterministic fixtures
```

需要大型数据时：

```text
generate at test runtime
```

---

# 193. Golden Tests

至少建立：

```text
pipeline input
expected result
expected stats
expected output
```

例如：

```text
10 images
Resize + Convert

Expected:
10 success
0 failed
10 outputs
```

---

# 194. Failure Golden Tests

例如：

```text
3 inaccessible files
```

Golden：

```text
7 success
3 failed
```

必须稳定。

---

# 195. JSON Contract Tests

Job Result 如果暴露为 JSON：

必须测试：

```text
schema
status
counts
errors
items
```

避免前端依赖：

```text
fragile fields
```

---

# 196. Version Compatibility

如果：

```text
Job / Pipeline
```

持久化：

必须测试：

```text
current version
unsupported version
corrupt data
missing field
extra field
```

---

# 197. Corruption Handling

如果：

```text
job journal
```

损坏：

不能：

```text
panic
```

应该：

```text
detect corruption
preserve original
report recovery limitation
```

必要时：

```text
safe quarantine
```

但不要静默删除。

---

# 198. Fuzz Testing

至少针对：

```text
Pipeline JSON
Job state data
Tool options
Path inputs
Error serialization
```

进行 fuzz/property testing。

---

# 199. State Machine Fuzz

随机生成：

```text
Start
Cancel
Retry
Resume
Fail
Complete
```

检查：

> Engine 不进入非法状态。

---

# 200. Idempotency

对于纯：

```text
read-only
```

操作：

应该尽量：

```text
retry-safe
```

对于 mutation：

必须根据：

```text
transaction
output verification
```

判断能否重复。

禁止笼统声称：

> 所有 Pipeline 都幂等。

---

# 201. Duplicate Execution Detection

如果一个 Job 被：

```text
start_job
```

调用两次：

不能：

```text
two concurrent executions
```

除非明确设计允许。

必须防止：

```text
double start
```

---

# 202. Job Locking

同一个：

```text
JobId
```

只能有：

```text
one active execution
```

并发调用：

必须返回：

```text
AlreadyRunning
```

或等价错误。

---

# 203. Input Locking

如果可能：

```text
Job A
```

正在：

```text
modify A.txt
```

同时：

```text
Job B
```

也要：

```text
modify A.txt
```

系统必须有明确策略：

```text
serialize
reject
detect conflict
```

不要依赖运气。

---

# 204. Inter-Job Conflict

M7 第一版不需要构建：

```text
global distributed scheduler
```

但至少必须防止：

```text
同一资源
```

同时被多个 Weave Job 修改而没有任何检测。

---

# 205. Transaction Ownership

必须定义：

```text
谁拥有 Transaction
```

推荐：

```text
Tool creates operation transaction
Batch Engine aggregates
History persists summary
```

但必须根据现有 M2–M6 真实实现决定。

不要同时存在：

```text
Engine Transaction
Tool Transaction
Filesystem Transaction
```

三套互不兼容的机制。

---

# 206. Result Ownership

同理明确：

```text
Tool Result
Stage Result
Item Result
Job Result
```

之间的职责。

目标：

> 避免 5 层嵌套同样字段。

---

# 207. Error Ownership

明确：

```text
Tool error
Adapter error
Engine error
Persistence error
IPC error
UI error
```

每层：

> 只负责自己能解释的部分。

---

# 208. No Error Swallowing

禁止：

```rust
let _ = ...
```

掩盖：

```text
write
journal
history
cleanup
```

如果错误可忽略：

必须：

```text
explicitly documented
```

---

# 209. Cleanup Errors

如果：

```text
temp cleanup
```

失败：

不要覆盖原始：

```text
TransformFailed
```

应该：

```text
primary error
+
cleanup warning
```

保留两者。

---

# 210. User Intent Preservation

Batch Engine 不应修改用户的：

```text
path
options
filter
destination
overwrite policy
```

除非：

```text
explicit normalization
```

且 Preview 中可见。

---

# 211. Default Policies

默认建议：

```text
Preview = On
Overwrite = Never
Retry = Limited
Concurrency = Bounded
Source Replace = Off
Delete = Off
Telemetry = Off
Cloud = Off
```

如果现有产品设置已有不同默认值：

> 必须以真实项目决策为准，并在文档中明确。

---

# 212. Dangerous Operations

涉及：

```text
Delete
Permanent Delete
Source Replace
```

必须更严格。

M7 不应因为“统一 Engine”而放松 M2/M3 的安全要求。

---

# 213. Recycle

如果后续 Pipeline 中涉及：

```text
Delete / Recycle
```

必须复用：

```text
M3 Recycle abstraction
```

不要重新实现。

---

# 214. Batch Engine 不改变工具语义

M7 的职责：

```text
orchestration
```

不是：

```text
reinterpret tool behavior
```

例如：

```text
Image Convert
```

仍然遵守：

```text
alpha policy
metadata policy
format capability
```

M7 不应修改它。

---

# 215. M7 不应拥有图片逻辑

不要出现：

```text
if extension == jpg
    ...
```

这种：

```text
domain-specific logic
```

写入：

```text
weave-batch
```

应该由：

```text
weave-media
```

提供 capability。

---

# 216. M7 不应拥有 CSV 逻辑

不要：

```text
if csv
```

写入：

```text
batch
```

CSV 语义继续属于：

```text
weave-data
```

---

# 217. M7 不应拥有 Text 逻辑

同理：

```text
weave-text
```

保持 Text domain ownership。

---

# 218. Architecture Dependency Audit

最终确认：

```text
weave-batch
```

没有产生：

```text
weave-media-specific
weave-data-specific
weave-text-specific
```

硬耦合。

应通过：

```text
trait
capability
adapter
operation abstraction
```

完成。

---

# 219. AI Coding Workflow

整个 M7 严格按照：

```text
Audit
 ↓
Gap Analysis
 ↓
Architecture Decision
 ↓
Small Implementation
 ↓
Unit Tests
 ↓
Integration
 ↓
Fault Tests
 ↓
Performance
 ↓
UI
 ↓
Recovery
 ↓
Security
 ↓
Regression
 ↓
Final Audit
 ↓
Documentation
 ↓
Commit
```

---

# 220. 第一阶段：Baseline Audit

开始时：

> 不修改代码。

先检查：

```text
git status
git branch
HEAD
M0–M6 docs
Cargo workspace
weave-batch
weave-files
weave-history
UI
IPC
tests
```

并输出：

```text
FACT
HYPOTHESIS
INFERENCE
```

---

# 221. 第二阶段：Architecture Gap Report

审计：

```text
已有 Batch capability
已有 Operation model
已有 Transaction
已有 History
已有 Progress
已有 Cancellation
已有 M6 multi-image batch
```

然后输出：

```text
REUSE
EXTEND
REPLACE
MIGRATE
NOT NEEDED
```

严禁：

> 先写代码再发现已有实现。

---

# 222. P0 / P1 规则

只允许：

```text
P0
P1
```

分类。

### P0

影响：

```text
data loss
corruption
unsafe overwrite
path escape
incorrect recovery
wrong result
security
```

### P1

影响：

```text
core milestone functionality
major test failure
major architecture boundary
serious usability blocker
```

不要制造大量：

```text
P2
P3
```

噪声。

---

# 223. 最小修改原则

禁止：

```text
rewrite whole architecture
rename every crate
upgrade every dependency
redesign UI
replace state management
rewrite M2–M6
```

只修改：

> Batch Engine 真正需要的部分。

---

# 224. Dependency Policy

如果新依赖：

必须说明：

```text
Why
Alternative
License
Maintenance
Security
Platform
Size
```

没有必要：

> 不加。

---

# 225. Build Policy

不要：

```text
change Rust version
change frontend architecture
```

除非：

```text
verified necessary blocker
```

---

# 226. CI Gate

最终至少实际执行：

```text
cargo fmt --check
cargo clippy
cargo test
frontend typecheck
frontend lint
frontend test
Tauri build
license check
```

license check 为无条件必过项，不得跳过。如果项目已有：

```text
integration
security audit
```

继续通过。

---

# 227. No Fake Tests

禁止：

```text
skip
ignore
todo
assert!(true)
empty test
mock core filesystem
```

Mock 仅用于：

```text
fault injection
```

而不能代替：

```text
real filesystem integration
real tool execution
real codec
real parser
```

---

# 228. UI Completion Criteria

至少真实验证：

```text
Create pipeline
Preview
Start
Progress
Cancel
Failure
Retry
Result
History
```

至少一个：

```text
multi-stage real pipeline
```

必须成功完成。

---

# 229. End-to-End Acceptance

以下必须全部真实跑通：

```text
10 images
Resize
→ Convert
→ Strip Metadata
→ Rename
```

验证：

```text
Preview correct
Execution correct
Progress correct
Result correct
History correct
Retry semantics correct
```

---

# 230. Final Audit A — Baseline

检查：

```text
Git
Branch
HEAD
Working tree
Existing changes
```

不得覆盖用户未提交修改。

---

# 231. Final Audit B — Architecture

检查：

```text
UI
IPC
Application
Batch
Tool
Domain
Filesystem
History
```

依赖方向正确。

---

# 232. Final Audit C — Job Lifecycle

检查：

```text
Created
Planned
Ready
Running
Cancelling
Cancelled
Completed
PartialFailure
Failed
Interrupted
Recoverable
Recovered
```

每个状态：

```text
valid
reachable
tested
```

---

# 233. Final Audit D — Pipeline

检查：

```text
Source
Filter
Transform
Export
Ordering
Compatibility
Validation
```

---

# 234. Final Audit E — Preview

检查：

```text
No mutation
Same Plan
Real calculation
Warnings
Collision
Changed Input
```

---

# 235. Final Audit F — Execution

检查：

```text
Real tool execution
Revalidation
Safe Write
Transaction
Output verification
```

---

# 236. Final Audit G — Failure

检查：

```text
Item failure
Job failure
Partial completion
Error propagation
Failure summary
```

---

# 237. Final Audit H — Retry

检查：

```text
Retryable
NonRetryable
Retry count
Stage retry
Side effect safety
```

---

# 238. Final Audit I — Resume

检查：

```text
Interrupted Job
Journal
Revalidation
Output conflict
Resume correctness
```

---

# 239. Final Audit J — Cancellation

检查：

```text
Propagation
Latency
Partial results
Cleanup
History
```

---

# 240. Final Audit K — Concurrency

检查：

```text
Bounded workers
Queue bound
Backpressure
Race
Deadlock
Shutdown
```

---

# 241. Final Audit L — Resource

检查：

```text
Memory
Threads
File handles
Temp files
Queue size
Generated output
```

---

# 242. Final Audit M — Security

检查：

```text
Traversal
Symlink
Root escape
Overwrite
Shell execution
Arbitrary code
Resource exhaustion
```

---

# 243. Final Audit N — Privacy

检查：

```text
No Telemetry
No Upload
No Cloud
No Remote Execution
No content leakage
```

---

# 244. Final Audit O — Regression

必须确认：

```text
M0 PASS
M1 PASS
M2 PASS
M3 PASS
M4 PASS
M5 PASS
M6 PASS
```

---

# 245. Final Audit P — Build / Git Hygiene

确认：

```text
format
clippy
test
typecheck
lint
frontend test
Tauri build
```

以及：

```text
no secrets
no temp files
no binaries
no database artifacts
no local absolute paths
no private test data
```

---

# 246. M7 Acceptance Criteria

只有以下全部满足：

```text
[ ] Unified Job model
[ ] Pipeline model
[ ] Tool adapter model
[ ] Capability validation
[ ] Plan
[ ] Preview
[ ] Execution
[ ] Progress
[ ] Cancellation
[ ] Bounded concurrency
[ ] Backpressure
[ ] Failure isolation
[ ] Structured result
[ ] Retry
[ ] Resume
[ ] Job journal
[ ] History integration
[ ] Undo integration
[ ] Safe Write reuse
[ ] TOCTOU revalidation
[ ] Output verification
[ ] Resource limits
[ ] Real filesystem tests
[ ] Fault injection
[ ] Concurrency tests
[ ] Crash/recovery tests
[ ] UI smoke
[ ] Real multi-stage pipeline
[ ] M0–M6 regression
[ ] Documentation
[ ] Security audit
[ ] Privacy audit
[ ] Build PASS
[ ] Git hygiene PASS
```

才能：

```text
M7 COMPLETE
```

---

# 247. M7 不算完成的情况

以下任何一项存在：

```text
Preview 是 mock
Execute 与 Preview 使用不同语义
一个文件失败导致全部 Job 停止
Retry 会产生重复副作用
Resume 会盲目重放
Cancellation 只是 UI 状态
Progress 是虚假百分比
并发无上限
Queue 无上限
UI 每个文件发送一次 IPC
History 与 Job 状态不一致
Undo 会覆盖用户后续修改
M6 有第二套 Batch Engine
M7 偷做 DAG Editor
M7 允许 Shell Script
路径能逃出 root
真实 filesystem integration 缺失
```

则：

```text
M7 NOT COMPLETE
```

必须明确：

```text
BLOCKED
FAIL
NOT VERIFIED
NOT SUPPORTED
KNOWN LIMITATION
```

---

# 248. M7 → M8 Handoff

M7 完成后必须输出正式 Handoff。

至少包括：

```text
BatchJob
JobPlan
Pipeline
Stage
Capability
ItemContext
ExecutionContext
Scheduler
Retry
Resume
Journal
Progress
Cancellation
Result
Transaction
History
Undo
Resource Limits
```

并明确：

> 后续 M8 Documents 必须直接复用 Batch Engine，而不是自己重新实现批量处理系统。

---

# 249. M7 → M9 Handoff

M9 Utilities 可以：

```text
调用 Batch Engine
```

但不要：

```text
copy execution logic
```

---

# 250. M7 → M10 Handoff

这是最重要的未来边界。

M7 提供：

```text
Execution Engine
Linear Pipeline
Structured JobPlan
Job State
Retry
Resume
History
```

M10 才负责：

```text
Workflow
Visual composition
Branch
Condition
Loop
Reusable workflows
Workflow editor
Workflow templates
```

因此：

```text
M7
=
“How to execute a pipeline reliably”

M10
=
“How users create and compose workflows”
```

---

# 251. M7 最终架构图

最终目标：

```text
                           ifuyo Weave
                                │
                         ┌──────┴──────┐
                         │ Batch Engine│
                         └──────┬──────┘
                                │
             ┌──────────────────┼──────────────────┐
             ▼                  ▼                  ▼
          Planner            Scheduler           Journal
             │                  │                  │
          Preview          Worker Pool          Recovery
             │                  │                  │
             └──────────────────┼──────────────────┘
                                ▼
                         Tool Adapters
                                │
        ┌───────────────┬───────┼───────┬───────────────┐
        ▼               ▼       ▼       ▼               ▼
      Files            Text    Data    Media        Documents
        │               │       │       │               │
        └───────────────┴───────┼───────┴───────────────┘
                                ▼
                         Safe Filesystem
                                │
                         History / Undo
```

---

# 252. M7 最终产品闭环

用户体验必须达到：

```text
Drop
 ↓
Weave
 ↓
Choose Tool
 ↓
Add Pipeline Step
 ↓
Preview
 ↓
See What Will Happen
 ↓
Confirm
 ↓
Run
 ↓
Observe
 ↓
Failure Isolation
 ↓
Retry / Resume
 ↓
Result
 ↓
History
 ↓
Undo / Repeat
```

最终用户不应该感受到：

> “我在使用一个复杂的任务调度框架。”

而应该感受到：

> **“我交给 Weave 一批文件，它知道该怎么处理，也知道出了什么问题，而且不会因为一个坏文件把整个任务搞砸。”**

---

# 253. M7 的工程核心原则

整个 M7 必须始终遵守：

```text
One Engine > Many Batch Implementations

One Plan > Separate Preview / Execute Logic

Truthful State > Optimistic State

Isolation > Cascading Failure

Bounded Concurrency > Unlimited Parallelism

Recoverable > Replay Blindly

Verified Output > Assumed Output

Explicit Capability > Implicit Guessing

Tool Semantics > Engine Reinterpretation

Linear Pipeline > Premature DAG

Execution Engine > Workflow Editor
```

---

# 254. 最终完成标准

只有：

```text
Architecture
+
Correctness
+
Safety
+
Recovery
+
Concurrency
+
Cancellation
+
Preview
+
Failure Isolation
+
Retry
+
Resume
+
History
+
Undo
+
Security
+
Privacy
+
Performance Evidence
+
UI Smoke
+
M0–M6 Regression
+
Documentation
+
Git Hygiene
+
Real Tauri Build
```

全部 PASS 后：

```text
M7 COMPLETE
```

否则：

```text
M7 NOT COMPLETE
```

绝不允许：

```text
隐藏失败
隐藏未支持能力
隐藏恢复限制
隐藏 concurrency 问题
隐藏 memory 问题
mock 冒充真实执行
假进度
假恢复
假 undo
假 preview
```

---

# 255. 最终执行方式

请严格执行：

```text
1. Audit
2. Baseline
3. Architecture Gap Analysis
4. P0/P1 Classification
5. Design
6. Minimal Implementation
7. Unit Tests
8. Integration Tests
9. Real Filesystem Tests
10. Fault Injection
11. Concurrency Tests
12. Cancellation Tests
13. Retry Tests
14. Resume / Recovery Tests
15. Performance Tests
16. UI Smoke
17. Security Review
18. Privacy Review
19. M0–M6 Regression
20. Final Audit
21. Documentation
22. Git Hygiene
23. Commit
```

任何一步失败：

> 先修复，再继续。

不要跳过。

---

# 256. 修改前必须报告

在真正改代码前，先输出：

```text
# M7 Baseline Audit

## A. Git
Branch:
HEAD:
Working Tree:

## B. Existing Architecture

## C. Existing Batch Capability

## D. Existing Operation Model

## E. Existing Progress / Cancellation

## F. Existing History / Transaction / Undo

## G. Existing M6 Multi-image Processing

## H. Current Gaps

## I. Reuse

## J. Extend

## K. Replace

## L. P0

## M. P1

## N. Implementation Plan
```

然后才开始修改。

---

# 257. 最终 Implementation Report

完成后输出：

# M7 Implementation Report

## 1. Baseline

```text
Branch:
HEAD before:
HEAD after:
Working Tree:
M0:
M1:
M2:
M3:
M4:
M5:
M6:
```

## 2. Architecture

```text
BatchJob:
JobPlan:
Pipeline:
Stage:
Adapter:
Scheduler:
Journal:
Recovery:
```

## 3. Implemented

列出真实实现：

```text
Planner
Preview
Execution
Progress
Cancellation
Concurrency
Failure Isolation
Retry
Resume
History
Undo
UI
```

## 4. Integration

列出真实接入：

```text
Files
Text
Data
Media
```

## 5. Verification

给出：

```text
cargo fmt --check
cargo clippy
cargo test
frontend typecheck
frontend lint
frontend test
Tauri build
```

真实结果。

不要只写：

```text
PASS
```

如果失败：

```text
FAIL
```

必须附原因。

## 6. Performance

真实记录：

```text
Dataset:
Workers:
Elapsed:
Peak Memory:
Success:
Failed:
Cancelled:
```

## 7. Recovery

说明：

```text
Retry:
Resume:
Crash Recovery:
Conflict Handling:
```

## 8. Security

说明：

```text
Path Safety:
Symlink:
Overwrite:
Shell:
Resource Limits:
```

## 9. Privacy

说明：

```text
Telemetry:
Upload:
Cloud:
Logs:
History:
```

## 10. Known Limitations

只写真实存在的：

```text
NOT SUPPORTED
KNOWN LIMITATION
```

## 11. M8 Handoff

明确：

```text
What M8 should reuse
What M8 must not reimplement
What remains deferred to M10
```

## 12. Final Status

只能是：

```text
M7 COMPLETE
```

或：

```text
M7 NOT COMPLETE
```

不得输出模糊状态。

---

# 258. 最终产品目标

M7 完成以后：

```text
ifuyo Weave
```

不再只是：

```text
一堆可靠的小工具
```

而应该开始成为：

> **一个真正拥有统一执行能力的本地工作工具箱。**

工具负责：

```text
What
```

Batch Engine 负责：

```text
How
```

History 负责：

```text
What happened
```

Transaction / Undo 负责：

```text
How to recover
```

Planner 负责：

```text
What will happen
```

Preview 负责：

```text
Let the user see it first
```

而最终完整语义是：

```text
What
 ↓
Plan
 ↓
Preview
 ↓
Execute
 ↓
Observe
 ↓
Recover
```

这才是：

> **M7 真正应该为 ifuyo Weave 建立的核心能力。**