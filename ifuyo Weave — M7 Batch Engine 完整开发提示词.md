# ifuyo Weave
# M7 — Batch Engine
## 统一批处理引擎、Pipeline、Preview、Execution、Failure Isolation、Retry、Resume、Transaction、History 与可组合执行

你现在进入：

> **ifuyo Weave 的 M7：Batch Engine**

项目：

`ifuyo Weave`

当前里程碑基线：

```text
M0 Foundation
      ↓
M1 File Core
      ↓
M2 Rename / Organizer
      ↓
M3 Duplicate Finder
      ↓
M4 Text
      ↓
M5 Data
      ↓
M6 Image
      ↓
M7 Batch Engine
```

本阶段的核心任务不是再增加一个“批量工具”。

而是：

> **把前 M0–M6 已经存在的可靠工具能力，统一提升为一个真正可复用、可组合、可预览、可执行、可取消、可重试、可恢复、可追踪的 Batch Engine。**

M7 完成后，Weave 应该第一次形成：

```text
Input
 ↓
Identify
 ↓
Plan
 ↓
Preview
 ↓
Confirm
 ↓
Execute
 ↓
Progress
 ↓
Partial Failure Handling
 ↓
Retry / Resume
 ↓
Result
 ↓
History
 ↓
Undo / Reuse
```

并能够支持：

```text
单工具批处理
+
多工具线性 Pipeline
+
过滤
+
转换
+
输出
```

例如：

```text
Folder
 ↓
Images only
 ↓
Resize 1920px
 ↓
Convert WebP
 ↓
Strip Metadata
 ↓
Rename
 ↓
Output
```

但必须明确：

> **M7 是 Execution Engine，不是完整 Workflow Builder。**

---

# 0. M7 的硬边界

## 0.1 本阶段必须完成

M7 必须至少建立：

```text
Batch Job
Job Input
Job Plan
Pipeline
Pipeline Stage
Item Context
Execution Context
Preview
Execution
Progress
Cancellation
Failure Isolation
Retry
Resume
Result
Transaction
History Integration
Undo Integration
Resource Limits
Deterministic Scheduling
```

并形成真实闭环：

```text
Tool
 ↓
Adapter
 ↓
Batch Engine
 ↓
Plan
 ↓
Preview
 ↓
Execute
 ↓
Result
```

---

# 0.2 本阶段明确禁止

M7 不得实现：

```text
自然语言 Workflow
AI Workflow Generation
LLM Agent
Cloud Workflow
Remote Execution
Server-side Batch
Distributed Workers
Plugin Marketplace
复杂 DAG 编辑器
可视化节点编辑器
拖线式 Workflow Designer
RPA
Cron Scheduler
任务调度平台
企业级队列系统
多机执行
网络文件编排平台
```

尤其禁止提前实现：

```text
M10 Workflow UI
```

M7 可以提供：

```text
Linear Pipeline
```

但不要在 M7 偷偷扩展成：

```text
Graph Workflow
Branch
Loop
Subworkflow
Conditional Node Editor
Visual Workflow Canvas
```

---

# 1. M7 的核心目标

M7 真正需要解决的问题是：

前面的工具虽然各自可以：

```text
Rename
Organizer
Duplicate Finder
Text Transform
CSV Clean
Image Resize
Image Convert
Image Metadata
```

但是如果每个工具自己实现：

```text
批量输入
并发
进度
取消
失败重试
结果统计
历史
恢复
```

最终一定会产生：

```text
N 套 Batch Logic
N 套 Progress
N 套 Error Handling
N 套 Cancellation
N 套 Retry
N 套 Result
```

这会使 Weave 后续越来越难维护。

因此 M7 必须把这些能力统一。

最终目标：

```text
Tool
    ↓
Tool Contract
    ↓
Batch Engine
    ↓
统一生命周期
```

而不是：

```text
RenameBatchEngine
ImageBatchEngine
CsvBatchEngine
TextBatchEngine
...
```

---

# 2. 最重要的设计原则

严格遵守：

```text
Correctness > Throughput
Safety > Convenience
Deterministic > Magical
Preview > Mutation
Isolation > Cascading Failure
Bounded > Unbounded
Reuse > Rebuild
Explicit > Implicit
Local > Cloud
Evidence > Assumption
```

以及 Weave 原有产品原则：

```text
Local-first
Utility-first
Batch-first
Preview-first
Reversible-first
Composable
```

---

# 3. M7 的总体执行模型

统一模型：

```text
BatchJob
├── Input
├── Pipeline
├── Options
├── Preview
├── Execution
├── Progress
├── Results
├── Failures
├── Retry
├── Resume
├── Transaction
└── History
```

推荐生命周期：

```text
Created
   ↓
Validated
   ↓
Planned
   ↓
Previewed
   ↓
Ready
   ↓
Running
   ↓
Paused / Cancelling
   ↓
Completed / PartialFailure / Cancelled / Failed
```

禁止使用模糊状态：

```text
done
finished
success
```

必须能够区分真实执行结果。

---

# 4. Job State Model

至少定义：

```text
Created
Planning
Planned
Previewing
Ready
Running
Paused
Cancelling
Cancelled
Completed
CompletedWithFailures
Failed
Interrupted
Recoverable
Recovered
```

其中 Running ↔ Paused 可双向转换（Pause / Resume）。

如果某状态最终没有实际语义，可以减少。

不要为了“看起来完整”制造无意义状态。

状态机必须：

```text
Explicit
Validated
Tested
```

禁止：

```text
stringly-typed random state
```

---

# 5. Job 与 Operation 必须分离

这是 M7 最重要的架构决策之一。

```text
Tool
```

代表：

> 一个具体能力。

例如：

```text
RenameTool
ResizeTool
ConvertTool
CsvCleanTool
TextTransformTool
```

而：

```text
BatchJob
```

代表：

> 一次实际批处理任务。

因此：

```text
Tool ≠ Job
```

例如：

```text
ResizeTool
```

可以被执行：

```text
100 images
```

形成：

```text
Job #123
```

也可以：

```text
Job #124
```

执行：

```text
1000 images
```

---

# 6. Operation Contract 审计

首先检查 M2–M6 当前真实工具接口。

不要假设 Charter 中的接口已经完全存在。

检查：

```text
preview()
execute()
canUndo()
undo()
```

以及：

```text
OperationPlan
OperationItem
OperationResult
OperationTransaction
HistoryEntry
```

实际状态。

对于每项标记：

```text
FACT
HYPOTHESIS
INFERENCE
```

如果现有模型足够：

> 直接复用。

如果不够：

> 最小扩展。

严禁：

> 为 M7 新建一套完全平行的 Operation 系统。

---

# 7. Tool Capability Model

M7 必须能够知道：

```text
Tool ID
Category
Input Types
Output Types
Mutation Type
Supports Preview
Supports Batch
Supports Undo
Supports Retry
Supports Streaming
Supports Pipeline
```

建议考虑：

```text
ToolCapability
```

例如：

```text
Rename
Inputs: File
Outputs: File
Mutation: PathMutation
Batch: Yes
Preview: Yes
Undo: Yes
```

而：

```text
Duplicate Finder
```

可能属于：

```text
Input: Directory / FileSet
Output: DuplicateReport
Mutation: None
```

因此：

> **不是所有旧工具都必须强行适配为相同的 Item Transform。**

必须保留能力差异。

---

# 8. Batch Tool Contract

建立统一的 Batch Adapter / Executor 抽象。

概念模型可以类似：

```text
BatchTool
├── describe()
├── validate()
├── plan()
├── preview()
├── execute()
└── capabilities()
```

具体命名必须服从仓库现有架构。

重点：

> 不要为了 API “漂亮”而大规模重构 M2–M6。

---

# 9. Batch Input Model

输入必须明确区分：

```text
File
Directory
FileSet
DataDocument
TextDocument
ImageDocument
Structured Result
```

不要假设所有任务：

```text
Vec<PathBuf>
```

例如：

```text
CSV Cleaner
```

真正处理：

```text
DataDocument
```

而：

```text
Image Resize
```

处理：

```text
ImageDocument
```

但是：

```text
Batch Engine
```

负责协调，不负责重新实现这些领域逻辑。

---

# 10. Input Snapshot

Job 开始前必须建立：

```text
Input Snapshot
```

至少记录：

```text
Path
Identity
Size
Modified Time
Relevant Hash / Identity Token
Input Order
```

具体字段根据现有 File Core 能力决定。

目的：

> Execute 时重新验证，避免 Preview 与 Execute 之间输入已经发生变化。

典型问题：

```text
Preview:
A.jpg

用户等待 30 秒

另一个程序替换 A.jpg

Execute:
不能直接处理“现在的 A.jpg”
```

必须能够识别：

```text
ChangedSincePreview
```

或等价结构化错误。

---

# 11. Pipeline Model

M7 首先实现：

> **Linear Pipeline**

模型：

```text
Source
 ↓
Filter
 ↓
Transform
 ↓
Transform
 ↓
Transform
 ↓
Export
```

例如：

```text
Folder
 ↓
Image Filter
 ↓
Resize
 ↓
Convert
 ↓
Strip Metadata
 ↓
Rename
 ↓
Output
```

允许：

```text
0..N Filters
0..N Transforms
1 Output
```

具体约束根据真实工具能力调整。

---

# 12. 为什么 M7 只做 Linear Pipeline

M7 不要做：

```text
DAG
```

因为：

```text
A
├── B
│   └── C
└── D
```

一旦支持复杂图：

```text
Branch
Merge
Loop
Condition
Retry branch
Shared state
```

系统复杂度会快速上升。

M7 应先建立：

> **稳定的执行语义。**

M10 再考虑：

```text
Workflow Graph
```

---

# 13. Pipeline Stage

至少区分：

```text
Source
Filter
Transform
Export
```

可选：

```text
Analyze
Validate
```

但不要把：

```text
Analyze
```

和：

```text
Transform
```

混为一谈。

---

# 14. Filter Semantics

Filter 的唯一责任：

> 决定一个输入项是否继续进入后续 Pipeline。

例如：

```text
Extension == jpg
```

或者：

```text
Size > 10MB
```

或者：

```text
MediaType == image
```

结果至少为：

```text
Accepted
Rejected
Error
```

`Rejected`：

> 不是失败。

必须区分：

```text
Filtered / Skipped
```

和：

```text
Failed
```

---

# 15. Transform Semantics

Transform：

```text
Input
 ↓
Transformation
 ↓
Output
```

例如：

```text
Image
 ↓
Resize
 ↓
Image
```

或者：

```text
Text
 ↓
Normalize
 ↓
Text
```

或者：

```text
Data
 ↓
Clean
 ↓
Data
```

Transformation 不应该直接绕过 Batch Engine 自己管理 Job 状态。

---

# 16. Export Semantics

Export 负责：

```text
Output destination
Serialization
Safe Write
Naming
Collision handling
```

不要让每个 Tool 自己再发明一套：

```text
overwrite policy
atomic write
destination validation
```

必须复用：

```text
M2 Safe Write
Filesystem
Path Safety
Transaction
History
```

---

# 17. Item Context

每一个 pipeline item 都应有稳定上下文。

概念：

```text
ItemContext
├── ItemId
├── OriginalInput
├── CurrentValue
├── CurrentPath
├── StageIndex
├── StageResults
├── Diagnostics
├── Metadata
└── Cancellation
```

禁止把大型数据对象无限复制。

特别是：

```text
Image
DataDocument
Large Text
```

必须考虑：

```text
ownership
borrowing
streaming
temp representation
```

实际实现根据 Rust 架构决定。

---

# 18. Item Identity

ItemId 必须：

```text
Stable
Unique within Job
Deterministic where possible
```

不要使用：

```text
array index only
```

因为：

```text
filter
retry
resume
parallel execution
```

都会改变数组结构。

---

# 19. JobPlan

M7 的核心数据结构之一：

```text
JobPlan
```

至少包含：

```text
JobId
Input Snapshot
Pipeline
Options
Execution Policy
Output Policy
Resource Policy
```

Plan 必须：

> **可序列化、可审计、可复现。**

不要只把：

```text
React state
```

直接传给 Rust。

---

# 20. Preview 必须基于 Plan

Preview 流程：

```text
Input Snapshot
 ↓
Validate
 ↓
Build JobPlan
 ↓
Simulate Pipeline
 ↓
Produce Preview Result
```

Execute：

```text
Same JobPlan
 ↓
Revalidate
 ↓
Execute
```

禁止：

```text
Preview 用一套逻辑
Execute 再重新猜一次
```

这是 M2–M6 已确立原则的延续。

---

# 21. Preview 不是 Execute

Preview：

```text
No destructive mutation
```

允许：

```text
read
parse
calculate
simulate
temporary preview output
```

但不得：

```text
rename source
move source
delete source
overwrite destination
```

除非实际架构中的预览引擎已经严格隔离临时空间。

---

# 22. Preview Result

至少要能够显示：

```text
Total Inputs
Accepted
Filtered
Would Change
Would Generate
Would Skip
Potential Failures
Estimated Output
```

按工具类型补充：

```text
Rename:
old → new

Resize:
3840×2160 → 1920×1080

Convert:
PNG → WebP

CSV:
10240 rows → 9842 rows
```

不要所有工具都强行输出相同“After”结构。

---

# 23. Preview Accuracy

Preview 应尽可能：

> 使用与 Execute 相同的 domain transformation。

允许：

```text
Preview adapter
```

但不能：

```text
mock result
hardcoded estimate
fake file size
假成功
```

特别是 Image：

```text
Preview
```

应尽量使用 M6 的实际 transform pipeline。

---

# 24. Execution Policy

至少支持：

```text
Continue On Error
Stop On First Fatal Error
Skip Invalid Item
```

推荐统一语义：

```text
Item Failure
```

默认：

```text
Continue
```

因为：

```text
1000 files
997 success
3 failed
```

不应该因为 1 个文件失败而放弃其余 999 个。

但是：

```text
Job-level fatal error
```

必须能够停止整个 Job。

---

# 25. Item Failure vs Job Failure

这是 M7 最核心的可靠性边界之一。

### Item-level failure

例如：

```text
A.jpg locked
```

结果：

```text
A.jpg = Failed
B.jpg = Success
C.jpg = Success
```

继续运行。

### Job-level failure

例如：

```text
Output root became inaccessible
Transaction journal corrupted
Critical engine invariant violated
Resource safety failure
```

可以：

```text
Stop Job
```

必须明确分类。

---

# 26. Result Model

最终 Job Result：

```text
JobResult
├── JobId
├── Status
├── StartedAt
├── FinishedAt
├── Total
├── Succeeded
├── Failed
├── Skipped
├── Cancelled
├── Retried
├── Outputs
└── Errors
```

不要只返回：

```text
bool
```

与 charter 契约的映射：

- JobResult ↔ charter #25 OperationResult：Succeeded ↔ processed；warnings 聚合自 ItemResult / Errors；duration 由 StartedAt / FinishedAt 派生。
- Progress 的 Rate / ETA 为派生显示字段，由真实吞吐估计计算；估计不可靠时不得显示（charter #26）。

---

# 27. Item Result

每项至少：

```text
ItemResult
├── ItemId
├── Initial State
├── Final State
├── Current Input
├── Current Output
├── Stage Results
├── Error
├── Retry Count
└── Transaction Info
```

状态至少：

```text
Success
Failed
Skipped
Cancelled
```

---

# 28. Stage Result

为了支持真正的诊断：

```text
Item
 ↓
Stage 0
 ↓
Stage 1
 ↓
Stage 2
```

必须知道：

```text
哪个 stage 成功
哪个 stage 失败
哪个 stage 被跳过
哪个 stage 产生了输出
```

例如：

```text
A.jpg

Filter       SUCCESS
Resize       SUCCESS
Convert      SUCCESS
Strip EXIF   FAILED
Export       SKIPPED
```

这是比：

```text
A.jpg FAILED
```

更有价值的结果。

---

# 29. Progress Model

M7 必须统一 Progress。

至少：

```text
Job Progress
Pipeline Progress
Item Progress
Stage Progress
```

UI 至少能知道：

```text
processed
total
current item
current stage
succeeded
failed
skipped
cancelled
```

---

# 30. Progress 的诚实原则

禁止：

```text
总进度 = 当前已开始任务数量
```

然后实际后台还在大量工作。

Progress 必须定义清楚：

```text
item-based
byte-based
stage-weighted
indeterminate
```

如果无法准确预测：

```text
Indeterminate
```

比：

```text
虚假的 73%
```

更正确。

---

# 31. Weighted Progress

不要为了“好看”强行制造：

```text
33%
66%
100%
```

如果不同 Stage 成本差异很大，可以使用：

```text
Stage Weight
```

但必须：

```text
documented
deterministic
tested
```

不要产生无法解释的跳动。

---

# 32. Cancellation

M7 必须支持：

```text
Cooperative Cancellation
```

取消来源：

```text
UI
Job Controller
Application Shutdown
Resource Safety
```

取消必须传播：

```text
Job
 ↓
Pipeline
 ↓
Stage
 ↓
Tool
 ↓
Filesystem / Codec / Parser
```

---

# 33. Cancellation Semantics

取消不等于：

```text
rollback everything automatically
```

必须明确：

```text
Already Completed
In Progress
Not Started
```

例如：

```text
100 files

43 already succeeded
44 running
45–100 not started

Cancel
```

最终：

```text
43 Success
1 Cancelled
56 Cancelled/NotStarted
```

实际语义必须根据当前执行模型确定。

---

# 34. Cancellation Safety

取消操作不能导致：

```text
half-written file
corrupt destination
lost source
broken transaction
inconsistent history
```

因此：

> cancellation 必须与 Safe Write / Transaction 紧密结合。

---

# 35. Concurrency

M7 支持并发，但必须：

```text
Bounded
Configurable
Safe
```

禁止：

```text
spawn one task per 100000 files
```

推荐：

```text
bounded worker pool
```

具体并发数：

> 由实际 workload / platform / operation capability 决定。

不要拍脑袋固定：

```text
64 workers
128 workers
```

---

# 36. Resource Budget

M7 必须支持资源边界。

至少考虑：

```text
Max Concurrent Items
Max Memory Budget
Max Queue Size
Max Generated Bytes
Max Preview Items
Max Retry Count
Max Pipeline Depth
```

根据实际情况增加。

特别注意：

```text
Image
Large Text
Large CSV
```

不能因为 Batch Engine 开了并发而同时把几十个大对象全部读入内存。

---

# 37. Retry Model

Retry 必须是：

> **明确策略，不是简单重新执行。**

至少区分：

```text
Retryable
NonRetryable
Unknown
```

例如：

### 可能 Retry

```text
Transient IO failure
Temporary lock
Temporary resource contention
```

### 不应 Retry

```text
Invalid path
Malformed image
Unsupported format
Collision without changed options
Permission denied
```

最终由 Tool / Error Capability 决定。

---

# 38. Retry Policy

支持：

```text
No Retry
Retry Failed
Retry Retryable Only
Max Attempts
Backoff
```

M7 第一版不要做复杂：

```text
distributed exponential scheduler
```

只要：

```text
bounded retry
deterministic
observable
```

即可。

---

# 39. Retry 必须避免副作用重复

最危险的问题：

```text
Stage A 已成功写出文件

Retry 整个 Item

Stage A 再写一次
```

因此必须明确：

```text
Retry from failed stage
```

与：

```text
Retry from item start
```

之间的区别。

优先：

> **从可安全恢复的失败 stage 开始。**

若无法安全 resume：

```text
明确标记为 non-resumable
```

而不是猜。

---

# 40. Resume

M7 应建立基础 Resume 能力。

当 Job 因：

```text
App crash
system restart
unexpected interruption
```

停止后，能够知道：

```text
哪些 item 已完成
哪些 item 未完成
哪些 item 状态未知
```

注意：

> **“Resume”不是“从数组第 N 个元素继续”。**

必须基于：

```text
Job Journal
Item State
Stage State
Output Verification
Input Identity
```

重新验证。

---

# 41. Resume Safety

重新启动后必须：

```text
Revalidate Input
Revalidate Existing Output
Revalidate Transaction State
```

禁止：

```text
blind replay
```

例如：

```text
Job 原本生成 output/a.webp

程序崩溃

用户后来手工修改了 output/a.webp

Resume:
不能无条件覆盖
```

应出现：

```text
OutputChangedSinceInterruption
```

或等价错误。

---

# 42. Job Journal

建议建立：

```text
JobJournal
```

记录：

```text
Job Created
Plan Created
Preview Completed
Execution Started
Item Started
Stage Completed
Item Completed
Item Failed
Retry
Cancellation
Execution Interrupted
Resume
Execution Completed
```

注意：

> Journal 不是 verbose log。

Journal 必须：

```text
structured
compact
durable where needed
```

---

# 43. History Integration

M7 不重新实现 History。

复用：

```text
weave-history
```

最终 History 至少知道：

```text
JobId
Operation Type
Pipeline Summary
Input Count
Success Count
Failure Count
Output Count
Timestamp
Reversible
Undo Status
```

不要存：

```text
raw file contents
image pixels
CSV entire data
```

---

# 44. History 与 Job Journal 的区别

必须明确：

```text
Job Journal
=
execution recovery / diagnostics

History
=
user-facing operation history
```

不要混成一个巨大 JSON。

---

# 45. Undo Semantics

M7 不应该重新发明 Undo。

Batch Engine 应负责：

```text
collect transaction records
```

具体 Undo：

```text
delegated to operation/tool transaction
```

Tool 层保留 charter #24 的 canUndo / undo；BatchTool 是其上的执行适配层，undo 责任仍在各工具事务。

例如：

```text
Rename
```

由：

```text
Rename Undo
```

负责。

而：

```text
Image Export
```

可能只能：

```text
remove generated output
```

那么必须诚实标记：

```text
Reversible:
Partial
```

或者：

```text
Not Reversible
```

---

# 46. Job-level Undo

不要假设：

```text
100 items
+
10 stages
=
100% rollback possible
```

真实情况可能：

```text
90 reversible
10 non-reversible
```

因此 Job Undo 必须支持：

```text
Full Undo
Partial Undo
Unsafe Undo
Not Reversible
```

---

# 47. Undo Conflict Safety

如果：

```text
M7:
A.txt → B.txt
```

之后：

```text
用户手工创建新的 B.txt
```

Undo：

> 不允许无条件覆盖新的 B.txt。

必须重新验证：

```text
expected output state
```

与当前状态。

这是 M2 已建立的安全原则，M7 不得削弱。

---

# 48. Atomicity 的边界

必须非常诚实地定义：

```text
Atomic Job
```

是否存在。

通常：

```text
1000-file Job
```

无法做到像单文件 transaction 那样真正原子。

因此不要宣传：

> 整个 Job 是 atomic。

更现实的是：

```text
Per-item atomicity
+
Per-operation transaction
+
Durable journal
+
Safe cancellation
+
Partial result recovery
```

---

# 49. Failure Isolation

这是 M7 的核心能力。

一个 item 出错：

```text
Item A FAIL
```

不能导致：

```text
B–Z 全部失败
```

除非：

```text
Job-level fatal error
```

因此 Engine 应实现：

```text
item isolation
```

---

# 50. Failure Aggregation

最终用户应该看到：

```text
Completed with failures

Success      997
Failed         3
Skipped        0
Cancelled      0
```

点击 Failed：

```text
A.jpg
PermissionDenied

B.jpg
UnsupportedFormat

C.jpg
OutputCollision
```

而不是：

```text
Something went wrong
```

---

# 51. Failure Categories

至少标准化：

```text
Input
Validation
Permission
PathSafety
Collision
Read
Decode
Transform
Encode
Write
Transaction
Cancellation
ResourceLimit
Unsupported
ChangedSincePlan
ChangedSincePreview
Internal
```

必须尽量复用已有 Structured Error。

---

# 52. Error Chaining

例如：

```text
Image Convert
 ↓
Encode WebP
 ↓
Write output
```

最终错误不要只显示：

```text
WriteFailed
```

应该尽可能形成：

```text
Tool: Image Convert
Stage: Export
Operation: WebP Encode + Write
Cause: PermissionDenied
Path: ...
```

注意：

> 不得在错误和日志里泄漏不必要的敏感文件内容。

---

# 53. Structured Diagnostics

每个：

```text
Job
Item
Stage
```

都应可产生：

```text
Diagnostic
```

至少：

```text
Code
Severity
Message
Source
ItemId
Stage
Recoverability
```

---

# 54. Deterministic Ordering

M7 必须保证：

```text
同样输入
+
同样 Pipeline
+
同样 Options
```

得到：

> **稳定的处理顺序与稳定的结果排序。**

并发执行：

> 可以改变执行时序，但不得让用户看到非确定性的最终结果顺序。

---

# 55. Deterministic Input Enumeration

如果输入来自目录：

```text
Directory
```

不得依赖：

```text
OS enumeration order
```

必须：

```text
collect
normalize
sort
```

再进入 Job。

排序规则必须明确并测试。

---

# 56. Pipeline Validation

Preview 前必须检查：

```text
Stage compatibility
Input type compatibility
Output type compatibility
Required options
Path safety
Destination validity
Capability restrictions
```

例如：

```text
Text Transform
 ↓
Image Convert
```

如果类型不兼容：

> Plan 必须直接失败。

不能等 Execute 才发现。

---

# 57. Type Compatibility

建立：

```text
InputType
OutputType
```

至少能够表达：

```text
File
Directory
Text
Data
Image
Audio
Video
Binary
Report
```

不必一次建立复杂的泛型类型系统。

重点是：

> **防止明显错误的 Pipeline 在运行时才炸。**

---

# 58. Tool Adapter 不得隐式转换

例如：

```text
CSV
 ↓
Image
```

不要偷偷：

```text
stringify
reinterpret
```

只有：

```text
明确存在 converter
```

才能连接。

---

# 59. Pipeline Execution Context

需要一个统一：

```text
ExecutionContext
```

包含：

```text
JobId
Cancellation
ResourceBudget
Filesystem
Logger
Transaction
ProgressReporter
Time
```

但注意：

> 不要把所有依赖塞成一个巨型上下文对象。

遵循最小依赖原则。

---

# 60. Clock / Time

涉及：

```text
Rename date
History
Retry backoff
Progress
Execution duration
```

尽可能提供：

```text
Clock abstraction
```

方便测试。

不要让核心逻辑到处直接调用：

```text
SystemTime::now()
```

---

# 61. Temporary Files

Pipeline 中允许使用：

```text
temporary output
```

但必须：

```text
bounded
tracked
cleaned
collision-safe
```

程序崩溃之后：

> 不应产生无限累积 temp 文件。

---

# 62. Intermediate Output

例如：

```text
Resize
 ↓
Convert
 ↓
Strip Metadata
```

不要默认每一步都：

```text
永久写一个文件
```

优先：

```text
in-memory
stream
temporary representation
```

只有真正需要时才落盘。

尤其：

```text
Image
Text
Data
```

必须控制中间副本。

---

# 63. Pipeline Memory Model

严禁：

```text
1000 Images
 ↓
read all into Vec<Image>
```

应：

```text
stream / bounded queue
```

即：

```text
Input
 ↓
Bounded Queue
 ↓
Worker
 ↓
Stage
 ↓
Output
```

---

# 64. Backpressure

如果：

```text
Resize 很快
Encode 很慢
```

不能无限积压：

```text
intermediate results
```

必须有：

```text
bounded queue
backpressure
```

这是 M7 与简单 `for loop` 的关键区别之一。

---

# 65. Bounded Queue

所有：

```text
Producer
Consumer
```

之间必须考虑：

```text
queue bound
```

尤其：

```text
Directory scanner
 ↓
Worker pool
```

如果输入：

```text
1,000,000 files
```

也不应：

```text
一次性把 1,000,000 个 item
塞进内存
```

---

# 66. Large Job

至少验证：

```text
100
1,000
10,000
50,000
```

如果机器能力允许：

```text
100,000
```

但不要为了 benchmark 无限制生成数据。

必须记录：

```text
elapsed
peak memory
throughput
success
failure
cancellation latency
```

---

# 67. Mixed Failure Job

必须测试：

```text
100 items
```

其中：

```text
90 success
5 filtered
3 failed
2 cancelled
```

最终统计必须精确：

```text
Success 90
Skipped 5
Failed 3
Cancelled 2
```

且：

```text
90 outputs
```

必须确实存在。

---

# 68. Retry Test

测试：

```text
Item A:
fail first
success second
```

验证：

```text
Retry count = 1
Final = Success
```

同时：

```text
NonRetryable failure
```

不能无限 retry。

---

# 69. Resume Test

构造：

```text
Job:
100 items

simulate interruption:
43 complete
44 interrupted
45–100 pending
```

重新启动：

> 必须恢复正确。

不能：

```text
重新无脑处理 1–100
```

也不能：

```text
漏掉 44
```

---

# 70. Resume Conflict Test

：

```text
Job output already exists
but output has changed after interruption
```

必须：

```text
detect conflict
```

不能：

```text
blind overwrite
```

---

# 71. Pipeline Partial Failure

测试：

```text
Filter
 ↓
Transform A
 ↓
Transform B
 ↓
Export
```

让：

```text
Transform B
```

对部分 input 失败。

必须出现：

```text
A success
B failed
Export skipped for B
```

而其它 items 正常完成。

---

# 72. Stage Retry

验证：

```text
Stage 0 Success
Stage 1 Success
Stage 2 Fail
```

Retry 时：

> 尽可能从 Stage 2 开始。

不要无条件：

```text
Stage 0
Stage 1
Stage 2
```

全部重新执行。

除非 capability 明确声明：

```text
RestartFromBeginning
```

---

# 73. Side-effect Classification

每个 Stage 应尽可能声明：

```text
Pure
ReadOnly
TemporaryWrite
Mutation
Irreversible
```

这对于：

```text
preview
retry
undo
resume
```

非常重要。

例如：

```text
Text Transform
=
Pure

Rename
=
Mutation + Reversible

Delete
=
Irreversible
```

实际分类必须以真实工具语义为准。

---

# 74. Preview Side Effect Audit

建立测试：

```text
Before Preview
filesystem snapshot

Preview

After Preview
filesystem snapshot
```

要求：

```text
No source mutation
No destination mutation
No unexpected persistent file
```

允许的临时文件必须：

```text
isolated
tracked
cleaned
```

---

# 75. Execute Revalidation

执行前重新检查：

```text
Input
Output
Path
Options
Capabilities
```

确保：

```text
Preview Plan
```

仍适用。

若不适用：

```text
Do not silently mutate
```

而是：

```text
NeedsReplan
Conflict
ChangedSincePreview
```

---

# 76. Job Replan

当 Execute 发现：

```text
input changed
destination changed
tool capability changed
```

不要自动偷偷重新生成另一套结果。

应：

```text
invalidate plan
return structured conflict
```

再由用户：

```text
Preview Again
Confirm
```

---

# 77. Collision Policy

统一支持：

```text
Fail
Skip
GenerateUniqueName
OverwriteExplicitly
```

但：

> `OverwriteExplicitly` 必须是工具和操作本身允许的安全策略。

默认：

```text
Never silent overwrite
```

---

# 78. Collision Resolution

批处理内部还可能出现：

```text
A.jpg → output/1.jpg
B.jpg → output/1.jpg
```

这种：

> Internal Collision

必须在 Preview 阶段发现。

不要等 Execute。

---

# 79. Naming Policy

Pipeline 如果包含：

```text
Convert
Resize
Rename
```

必须明确：

```text
name source
extension source
ordering
collision policy
```

例如：

```text
A.PNG
 ↓
WebP
 ↓
Rename
```

最终：

```text
new-name.webp
```

而不是：

```text
new-name.PNG
```

---

# 80. Pipeline Data Flow

必须定义：

```text
Input
 ↓
Filter
 ↓
Current Item
 ↓
Transform
 ↓
Current Item
 ↓
Transform
 ↓
Current Item
 ↓
Export
```

而不是每一步各自从原始文件读取：

```text
Original
 ↓
Transform A

Original
 ↓
Transform B

Original
 ↓
Export
```

前者才能真正形成 Pipeline。

---

# 81. Intermediate State

每个 Transform 后：

```text
Current Representation
```

可能是：

```text
Path
InMemoryDocument
TemporaryFile
```

必须显式。

不要：

```text
Option<T>
```

胡乱表达所有状态。

---

# 82. Large File Strategy

对于大型文件：

```text
Streaming
```

优先。

如果某个 Tool 本身必须：

```text
load whole file
```

Batch Engine 不应假装它是 streaming。

必须：

```text
declare memory characteristics
```

这样 Scheduler 才能合理控制并发。

---

# 83. Tool Cost Model

M7 可选支持：

```text
CostClass
```

例如：

```text
Tiny
Small
Medium
Large
MemoryHeavy
CPUHeavy
IOHeavy
```

用于：

```text
concurrency planning
```

但不要做复杂动态调度器。

---

# 84. Scheduler

M7 第一版 Scheduler：

```text
bounded worker pool
+
capability-aware execution
```

即可。

不要实现：

```text
AI scheduler
ML prediction
distributed queue
network scheduler
```

---

# 85. Fairness

当一个 Job：

```text
10000 images
```

运行时：

不能阻塞 UI 中的：

```text
File Inspector
Text Formatter
```

因此：

> Batch Engine 是后台执行系统，不应抢占整个应用主线程。

---

# 86. UI 不得承担 Engine Logic

React 不得实现：

```text
for each file
retry
rollback
filesystem mutation
pipeline execution
```

UI 只能：

```text
Create Job
Preview Job
Start Job
Cancel Job
Retry Job
Resume Job
Query Job
```

---

# 87. IPC Boundary

不要：

```text
每个文件一次 IPC
```

例如：

```text
10000 files
=
10000 IPC calls
```

应：

```text
Start Job
 ↓
Engine runs
 ↓
Progress events
 ↓
Result
```

IPC 必须：

```text
bounded
structured
cancelable
```

---

# 88. Progress Event Throttling

不能：

```text
每读取 1KB
发一个 IPC event
```

必须考虑：

```text
throttle
coalescing
bounded event frequency
```

但不能因此让 UI：

```text
几十秒没有任何更新
```

需要找到实际平衡点。

---

# 89. Job Query API

至少支持：

```text
get_job(job_id)
list_jobs()
get_job_result(job_id)
get_item_result(job_id, item_id)
```

是否支持：

```text
stream_job_events()
```

视现有 Tauri event architecture 决定。

---

# 90. Job Control API

至少需要：

```text
preview_job
start_job
pause_job
cancel_job
retry_job
resume_job
```

可选：

```text
retry_failed_items
retry_item
```

具体命名必须符合现有项目风格。

---

# 91. Persistence

M7 是否引入额外数据库：

> 必须先审计现有 History / persistence。

不要因为 Batch Engine 就：

```text
默认再加 SQLite
```

如果现有持久化已经足够：

> 直接复用。

只有确有必要：

> 最小引入。

---

# 92. Job Persistence

如果需要持久化：

至少记录：

```text
Job ID
Plan
Pipeline Definition
State
Item States
Journal
Retry Count
Outputs
```

不记录：

```text
file contents
image pixels
raw CSV
full text
```

除非用户明确选择把某种内容作为工具输出并持久化。

---

# 93. Crash Recovery

必须测试：

```text
process crash
```

场景：

```text
during preview
during stage
during write
after write before journal
after journal before UI
```

目标：

> 不把系统带入“看起来完成、实际上未知”的假状态。

---

# 94. Unknown State

任何时候如果 Engine 无法证明：

```text
did operation happen?
```

必须：

```text
Unknown
```

而不是：

```text
Success
```

这是 M7 最重要的诚信原则之一。

---

# 95. At-least-once vs Exactly-once

不要声称：

```text
Exactly Once
```

除非真的能证明。

尤其在：

```text
filesystem mutation
```

中：

```text
crash
```

可能导致：

```text
side effect happened
journal not committed
```

因此必须明确：

```text
execution guarantee
```

优先：

> **可检测、可恢复、可验证。**

而不是虚假的 exactly-once。

---

# 96. Output Verification

重要输出应在写入后验证：

```text
exists
size
expected format
optional identity
```

对于某些工具：

```text
hash
```

可选。

不必所有 output 都重新计算 SHA-256。

必须根据成本与风险决定。

---

# 97. Source Preservation

生成新文件的 Pipeline：

```text
Input
 ↓
Transform
 ↓
Export New File
```

默认：

> 保留原始文件。

不要因为 Engine 为了方便而默认：

```text
replace source
```

---

# 98. Replace Source

只有当：

```text
Tool
+
User Option
+
Safe Write
+
Transaction
```

全部允许时，才可以：

```text
replace source
```

而且：

```text
Preview
```

必须明确显示：

```text
Source will be modified
```

---

# 99. Generated File Tracking

所有生成文件必须可追踪：

```text
GeneratedOutput
├── Path
├── CreatedBy
├── JobId
├── ItemId
├── Stage
```

这对：

```text
Undo
History
Cleanup
Recovery
```

都非常重要。

---

# 100. Failed Output Cleanup

如果：

```text
Stage generated temp/output
```

但后续 Stage 失败：

必须决定：

```text
Cleanup
Keep
Recover
```

不能默认：

> 留下一堆孤儿文件。

---

# 101. Partial Output Policy

例如：

```text
100 files
97 success
3 failed
```

输出目录应该保留：

```text
97 valid outputs
```

但需要让用户清楚：

```text
3 inputs failed
```

不要：

```text
rollback all 97
```

除非用户显式选择。

---

# 102. Job Result Summary

UI 最终应能展示：

```text
100 Items

97 Success
2 Failed
1 Skipped

Elapsed: 18.4s
Output: D:\output
```

注意：

> 所有统计必须来自实际 Engine Result。

不能：

```text
frontend count itself
```

---

# 103. Failed Item Retry UI

用户应该能够：

```text
View failed
↓
Review reason
↓
Retry failed
```

而不是：

```text
rerun entire job
```

---

# 104. Retry Preview

如果 Retry 修改了：

```text
options
destination
collision policy
```

则：

> 必须重新 Preview。

不能：

```text
把旧 Preview 当作新计划
```

---

# 105. Reuse / Repeat

History 应支持：

```text
Repeat Job
```

概念：

```text
Old Job Plan
 ↓
New Inputs / same options
 ↓
New Job
```

但必须注意：

> 不要把旧输入路径当成永远有效。

重新执行前：

```text
Resolve Inputs
Validate
Preview if needed
```

---

# 106. Job Template

M7 可以允许：

```text
save plan
```

但第一版：

> 只允许保存结构化 Pipeline Definition / Options。

不要做：

```text
template marketplace
sharing service
cloud sync
```

---

# 107. Pipeline Serialization

如果 Pipeline 可保存：

必须包含：

```text
Tool IDs
Stage Types
Options
Order
Version
```

例如：

```json
{
  "pipelineVersion": 1,
  "stages": [
    {
      "tool": "image.resize",
      "options": {}
    }
  ]
}
```

实际格式服从现有项目。

---

# 108. Versioning

Pipeline definition 必须有：

```text
version
```

因为未来：

```text
Tool options
```

可能改变。

旧 Job 不能因为版本更新而被静默解释成另一种操作。

---

# 109. Unknown Tool

如果读取旧 Job 时：

```text
tool id no longer exists
```

必须：

```text
UnsupportedPipelineVersion
UnknownTool
```

不能：

```text
silently drop stage
```

---

# 110. Security — Path Boundary

所有 Pipeline：

```text
source
destination
temp
```

必须继续通过：

```text
Path Safety
```

禁止任何 Stage：

```text
escape root
```

---

# 111. Security — Symlink

必须继承 M1：

```text
symlink/junction policy
```

M7 不得重新定义一套。

对于：

```text
source enumeration
destination creation
move/copy
```

都必须遵守。

---

# 112. Security — Shell

Batch Engine 不得依赖：

```text
cmd
powershell
bash
sh
mv
cp
ren
del
```

来完成核心文件操作。

优先：

```text
Rust APIs
```

---

# 113. Security — Resource Bomb

重点考虑：

```text
10 million files
Huge image dimensions
Huge CSV
Deep JSON
Huge text
Path explosion
Generated output explosion
Retry explosion
```

所有都必须有：

```text
limits
```

或：

```text
explicit unsupported semantics
```

---

# 114. Privacy

确认：

```text
No Telemetry
No Upload
No Cloud
No Remote Execution
```

并确认：

```text
Job Journal
History
Logs
```

不包含：

```text
raw file contents
```

除非某项诊断信息绝对必要且经过安全设计。

---

# 115. Logging

Logging 至少支持：

```text
job lifecycle
stage lifecycle
item failures
retry
recovery
cancellation
```

但不要：

```text
log every byte
log full file contents
log secret-bearing data
```

---

# 116. Logging Levels

至少：

```text
ERROR
WARN
INFO
DEBUG
```

生产默认不要：

```text
DEBUG everywhere
```

---

# 117. Deterministic Retry

若：

```text
MaxRetry=3
```

相同错误：

> 不应随机多跑或少跑。

测试必须可复现。

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