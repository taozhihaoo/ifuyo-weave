# ifuyo Weave — M10 Workflow Composition 完整开发提示词

你现在负责实现 **ifuyo Weave M10 — Workflow Composition（工作流编排）**。

这是一个建立在 M1–M9 之上的核心产品阶段。

M10 的职责不是继续增加大量新工具，而是把已经完成的能力统一组织为：

> **Input → Filter → Tool → Tool → Export**

形成一个：

> **可组合、可预览、可验证、可保存、可复用、可执行、可恢复、可撤销**

的本地工作流系统。

---

# 0. 总原则

必须严格遵守以下原则：

```text
M1 = Filesystem Truth
M2 = Safe Mutation
M3 = Duplicate Semantics
M4 = Text Semantics
M5 = Data Semantics
M6 = Image Semantics
M7 = Reliable Execution
M8 = Document Semantics
M9 = Utility Semantics
M10 = Workflow Composition
```

M10 不得吞掉前面任何一个模块的职责。

M10 是：

> **Orchestration / Composition Layer**

而不是：

> Tool Layer / Batch Engine / File Mutation Engine / Script Engine

---

# 1. M10 的最终目标

M10 完成以后，用户应该能够完成类似下面的工作：

### 示例 1：图片整理

```text
Input: 某个文件夹
    ↓
Filter: 仅图片
    ↓
Tool: Image Inspect
    ↓
Filter: 宽度 > 1920
    ↓
Tool: Image Resize
    ↓
Tool: Image Compress
    ↓
Export: 输出到新目录
```

### 示例 2：CSV 清洗

```text
Input: CSV 文件
    ↓
Filter: 行数 > 0
    ↓
Tool: Data Clean
    ↓
Tool: Data Transform
    ↓
Export: CSV
```

### 示例 3：批量文件重命名

```text
Input: 文件夹
    ↓
Filter: *.png
    ↓
Tool: Rename
    ↓
Export: Safe Write
```

### 示例 4：文件 Hash 工作流

```text
Input: 文件夹
    ↓
Filter: 文件大小 > 10MB
    ↓
Tool: SHA-256
    ↓
Export: CSV
```

### 示例 5：组合 M9 工具

```text
Input: 文本文件
    ↓
Tool: Base64 Decode
    ↓
Tool: URL Decode
    ↓
Tool: Regex Extract
    ↓
Export: Text
```

M10 的关键不是“能运行”。

关键是：

> 用户可以先看懂这个 Workflow 会做什么，然后再执行。

因此：

```text
Workflow = Definition + Validation + Preview + Execution
```

---

# 2. M10 的绝对边界

## 2.1 M10 不负责重新实现工具

禁止在 M10 中重新实现：

```text
Hash
Checksum
Base64
UUID
Timestamp
URL Encode
Regex
Color
CSV
JSON
Image Resize
Image Compress
PDF Merge
PDF Split
Rename
Duplicate Detection
```

必须调用现有能力。

例如：

```text
M10
  ↓
Tool Registry
  ↓
M9 Hash
```

而不是：

```text
M10 Hash Implementation
```

---

# 2.2 M10 不负责执行引擎

M7 已经负责：

```text
Job
Plan
Pipeline
Stage
ItemContext
ExecutionContext
Progress
Cancellation
Retry
Resume
Journal
Concurrency
Backpressure
Resource Budget
```

因此 M10 不得重新实现：

```text
BatchExecutor
RetryEngine
ThreadPool
ConcurrencyManager
ProgressEngine
ResumeEngine
JobScheduler
```

M10 只负责：

```text
Workflow Definition
→ Validation
→ Compilation / Planning
→ 调用 M7
```

---

# 2.3 M10 不负责 Safe Write

M2 已经拥有：

```text
OperationPlan
Transaction
Safe Write
TOCTOU Protection
History
Undo
Recovery
```

因此 M10 不得自己发明：

```text
WorkflowUndo
WorkflowWriteTransaction
WorkflowRollback
```

Workflow 只生成合法的底层操作计划。

真正的写入仍然必须进入 M2。

---

# 2.4 M10 不允许成为脚本执行器

严禁：

```text
Run arbitrary shell
Run PowerShell
Run cmd
Run bash
Run Python
Run JavaScript
Run user executable
```

不要出现：

```text
ExecuteCommandStep
ShellStep
ScriptStep
PowerShellStep
PythonStep
```

M10 是：

> Declarative Workflow

不是：

> Automation Script Runner

---

# 2.5 M10 不允许依赖云

禁止：

```text
Cloud Workflow
Remote Workflow
Webhook Trigger
Cloud Execution
Online Execution
Telemetry
User Account
Server Sync
```

本阶段默认：

```text
Local Workflow
Local Definition
Local Execution
Local History
Local Storage
```

---

# 2.6 M10 v1 不做任意 DAG

第一版必须保持：

> **Linear Pipeline**

即：

```text
Step1
 ↓
Step2
 ↓
Step3
 ↓
Step4
```

不允许在 M10 v1 引入：

```text
无限循环
递归 Workflow
任意 DAG
并行分支图
条件分支网络
子工作流递归
事件驱动图
```

可以拥有有限、明确的 Filter 语义，但不要把系统升级成复杂编程语言。

---

# 3. M10 产品定位

M10 要解决的问题：

以前用户只能：

```text
打开工具
→ 执行一次
```

M10 后用户可以：

```text
定义一次
→ Preview
→ Save
→ Run
→ Reuse
```

因此 M10 的价值是：

> **把“一次性操作”变成“可保存的操作方法”。**

---

# 4. M10 核心抽象

建议最终形成：

```text
Workflow
WorkflowStep
InputStep
FilterStep
ToolStep
ExportStep
Binding
Parameter
Capability
WorkflowSchema
WorkflowValidation
WorkflowPreview
WorkflowPlan
WorkflowRun
WorkflowTemplate
```

具体名称必须先审计现有代码，优先复用项目已有类型。

绝不能为了“命名统一”而无条件重构已有架构。

---

# 5. Workflow Domain Model

Workflow 至少应该表达：

```text
id
name
description
version
steps
metadata
```

例如概念模型：

```text
Workflow
├── id
├── name
├── description
├── schemaVersion
├── steps[]
└── metadata
```

Step：

```text
WorkflowStep
├── id
├── type
├── name
├── config
├── enabled
└── position
```

其中：

```text
type =
    input
    filter
    tool
    export
```

不要让 UI 决定领域模型。

---

# 6. Workflow Step 类型

M10 v1 只允许四类：

```text
INPUT
FILTER
TOOL
EXPORT
```

保持简单。

---

# 7. Input Step

Input 是 Workflow 数据来源。

支持的实际类型必须根据 M1–M9 已实现能力决定。

例如：

```text
File
Directory
Text
CSV
JSON
Image
Document
```

不要为了“未来扩展”一次性设计几十种输入类型。

Input 必须有明确的：

```text
Input Contract
```

例如：

```text
FileInput
DirectoryInput
TextInput
DataInput
```

---

# 8. Input 的核心原则

Input 不负责：

```text
Processing
Transformation
Mutation
Export
```

Input 只负责：

> “Workflow 从哪里获得数据？”

---

# 9. Filter Step

Filter 用于：

```text
筛选
```

而不是：

```text
修改
```

例如：

```text
Extension == ".png"
Size > 10MB
Name matches "*.jpg"
Width > 1920
Duplicate == true
RowCount > 100
Text contains "error"
```

实际 Filter 必须基于已有 M1–M9 capabilities。

不要凭空创造大量业务条件。

---

# 10. Filter 的语义必须稳定

Filter 必须定义：

```text
Input
Condition
Output
```

例如：

```text
Input: File[]
Condition: extension == png
Output: File[]
```

Filter 默认：

```text
不会改变实体内容
不会移动文件
不会删除文件
不会写磁盘
```

---

# 11. Filter 的失败语义

必须明确区分：

```text
Match
No Match
Evaluation Error
Unsupported
Invalid Configuration
```

不能把：

```text
Evaluation Error
```

默默当成：

```text
No Match
```

否则会导致 Workflow 静默漏数据。

---

# 12. Tool Step

Tool Step 是 M10 的核心。

但 Tool 本身不由 M10 实现。

M10 应通过统一：

```text
Tool Registry
```

发现已有能力。

例如概念：

```text
ToolRegistry
├── FileTools
├── TextTools
├── DataTools
├── ImageTools
├── DocumentTools
└── UtilityTools
```

---

# 13. Tool Capability

每一个可被 Workflow 调用的 Tool 都必须有能力描述。

至少需要：

```text
toolId
name
version
inputSchema
outputSchema
configSchema
capabilities
supportedModes
```

例如：

```text
m9.hash
m9.base64
m9.regex-test
m6.resize-image
m5.csv-clean
m2.rename
```

实际 ID 必须服从当前项目已有命名规则。

M5 的 DataTransformPlan（可序列化清洗规则）作为 data clean ToolStep 的 config 复用，禁止为清洗规则再造第二套描述格式。

---

# 14. Workflow 不应该知道 Tool 内部实现

Workflow 层只能知道：

```text
ToolId
Input Contract
Configuration
Output Contract
```

不能直接依赖：

```text
Concrete Rust Struct
Concrete File API
Concrete UI Component
Concrete Database
```

---

# 15. Tool Contract

每个 Tool 必须明确：

```text
Consumes:
Produces:
Mutates:
Requires:
Supports:
```

例如：

```text
Tool:
Image Resize

Consumes:
Image

Produces:
Image

Mutates:
No

Requires:
Image capability

Supports:
Batch
Preview
```

对于修改型工具：

```text
Mutates:
Yes
```

必须通过 M2/M7 执行。

---

# 16. Pure Tool 与 Mutating Tool

必须明确分类。

### Pure Tool

例如：

```text
Hash
Base64
UUID
Timestamp
URL Encode
Regex Test
Color Convert
Image Inspect
Document Inspect
```

特点：

```text
Input
→ Calculation
→ Output
```

不会直接修改原文件。

---

### Mutating Tool

例如：

```text
Rename
Resize
Compress
Convert
Clean
Merge
Split
Save
```

必须经过：

```text
M2 Safe Write
M7 Execution
```

不得直接：

```text
std::fs::write
rename
remove
copy
```

绕过已有安全层。

---

# 17. Export Step

Export 是 Workflow 的终点。

职责：

```text
将 Workflow Result 转换为用户可获得的结果
```

例如：

```text
Save File
Save Directory
Export CSV
Export JSON
Copy Clipboard
Save Text
```

实际能力必须复用已有模块。

---

# 18. Export 不等于 Write Engine

Export 只描述：

```text
What to export
Where to export
How to name
```

真正的：

```text
Safe Write
Collision Detection
Transaction
Undo
Recovery
```

仍由 M2 负责。

---

# 19. Workflow Step Contract

每一步必须定义：

```text
Input Type
Output Type
Side Effects
Configuration
Validation Rules
Preview Capability
Execution Capability
```

如果 Tool 不支持 Preview：

不要伪造 Preview。

必须标记：

```text
previewSupported = false
```

然后由 UX 决定如何提示。

---

# 20. Typed Data Flow

禁止 Workflow 使用：

```text
any
Map<String, Any>
JSON Blob
String Everything
```

作为核心数据交换方式。

应建立明确的：

```text
WorkflowValue
```

或者使用现有类型系统。

例如：

```text
File
Directory
Text
Bytes
DataTable
JsonValue
Image
Document
Scalar
List<T>
```

具体名称根据现有项目代码确定。

---

# 21. 类型兼容性

Workflow Builder 必须阻止明显不合法连接。

例如：

```text
Image → CSV Tool
```

应该在保存/执行前被发现。

错误类型：

```text
TYPE_MISMATCH
```

而不是：

```text
runtime panic
```

---

# 22. Parameter Binding

Workflow 必须支持：

```text
Static Value
Previous Step Output
Workflow Input
```

例如：

```text
Resize Width = 1920
```

或者：

```text
Resize Width = Workflow Parameter: targetWidth
```

不要在第一版支持任意表达式语言。

---

# 23. Workflow Parameters

保存 Workflow 时，可以定义有限参数：

```text
targetDirectory
maxSize
pattern
hashAlgorithm
quality
width
height
```

每个 Parameter 必须有：

```text
id
label
type
default
required
validation
```

参数类型尽量保持：

```text
string
number
boolean
enum
path
```

不要先实现完整 DSL。

---

# 24. Configuration Schema

Tool 配置必须拥有 schema。

例如：

```text
image.resize
{
    width: number
    height: number
    keepAspectRatio: boolean
}
```

UI 可以根据 schema 渲染配置界面。

不要让 UI 写死几十种：

```text
if tool == ...
```

---

# 25. Tool Registry

建立统一的：

```text
ToolRegistry
```

用于：

```text
List Tools
Get Tool
Get Capability
Validate Config
Build Tool Step
```

Registry 必须是：

> 单一事实来源。

禁止：

```text
UI Tool List
Backend Tool List
Workflow Tool List
```

各维护一份重复清单。

---

# 26. Workflow Validation

Workflow 保存前必须经过 Validation。

至少检查：

```text
Workflow ID
Name
Schema Version
Step IDs
Step Order
Missing Input
Missing Tool
Missing Export
Type Compatibility
Required Parameters
Invalid Configuration
Unsupported Capability
Duplicate Step ID
Invalid Reference
```

---

# 27. Structural Validation

例如：

```text
Workflow
A → B → C
```

必须：

```text
A valid
B valid
C valid
```

禁止：

```text
step reference to missing step
duplicate IDs
broken bindings
unknown tool
unknown filter
```

---

# 28. Semantic Validation

不能只检查 JSON schema。

还必须检查：

```text
业务语义
```

例如：

```text
Hash Tool
algorithm = UNKNOWN
```

必须失败。

---

# 29. Validation Severity

建议：

```text
ERROR
WARNING
INFO
```

只有：

```text
ERROR
```

阻止执行。

例如：

```text
WARNING:
MD5 is not collision-resistant.
```

应该允许执行。

---

# 30. Preview 是 M10 的核心

任何可执行 Workflow：

必须首先能够：

```text
Build
→ Validate
→ Preview
```

然后：

```text
Execute
```

---

# 31. Preview ≠ Execute

必须建立：

```text
WorkflowPreview
```

它描述：

```text
Input Count
Matched Count
Skipped Count
Changed Count
Output Count
Potential Collisions
Warnings
Errors
Estimated Work
```

具体字段根据现实能力实现。

---

# 32. Preview 必须尽量真实

不要：

```text
random fake preview
```

不要：

```text
mock result
```

不要：

```text
static demo result
```

Preview 应来自真正的：

```text
Workflow Planning
Tool Capability
M7 Plan
M2 Safe Write Analysis
```

---

# 33. Preview → Execute 一致性

建立原则：

> Preview 使用的计划和 Execute 使用的计划必须来源一致。

理想流程：

```text
Workflow
 ↓
Validate
 ↓
Compile
 ↓
Plan
 ├── Preview
 └── Execute
```

不要：

```text
Preview 一个系统
Execute 另一个系统
```

---

# 34. Compile / Planning

M10 应有明确的：

```text
Workflow Compiler
```

或者：

```text
Workflow Planner
```

职责：

```text
Workflow Definition
→ Validated Workflow
→ M7 Execution Plan
```

它不是编译器语言。

只是：

> Declarative Workflow → Executable Plan

---

# 35. M10 → M7

最终执行路径应尽量接近：

```text
Saved Workflow
 ↓
Load
 ↓
Validate
 ↓
Compile
 ↓
M7 Job
 ↓
M7 Pipeline
 ↓
M7 Execute
```

M10 不创建第二套执行循环。

---

# 36. M10 → M2

所有写入型 Workflow：

```text
Workflow
 ↓
M7
 ↓
M2 OperationPlan
 ↓
Safe Write
 ↓
History
```

必须保持。

---

# 37. Workflow Run

每次执行必须拥有独立：

```text
WorkflowRunId
```

并关联：

```text
WorkflowId
WorkflowVersion
M7 JobId
StartTime
EndTime
Status
```

状态示例：

```text
Pending
Validating
Planning
Running
Completed
Failed
Cancelled
PartiallyCompleted
```

实际状态应尽可能复用 M7。

---

# 38. Workflow Version

保存 Workflow 时必须具备：

```text
schemaVersion
```

例如：

```text
1
```

不要使用：

```text
latest
current
new
```

作为持久化版本。

---

# 39. Workflow Migration

未来修改结构时必须支持：

```text
old schema
→ migration
→ current schema
```

M10 v1 可以只实现：

```text
schemaVersion = 1
```

但必须留下清晰迁移边界。

---

# 40. Workflow Persistence

Workflow 必须本地持久化。

建议采用：

```text
JSON
```

或项目现有的本地结构化存储。

格式必须：

```text
Human-readable
Deterministic
Versioned
Stable
```

---

# 41. 不要保存 UI 状态作为 Workflow 核心数据

例如：

```text
panel width
collapsed state
selected tab
scroll position
window position
```

不得污染：

```text
Workflow Definition
```

UI State 与 Domain State 必须分离。

---

# 42. Workflow ID

不能用：

```text
name
```

作为唯一 ID。

应使用稳定 ID。

重命名 Workflow：

```text
ID 不变
```

---

# 43. Workflow Name

名称允许用户修改。

名称不是执行语义。

---

# 44. Workflow Duplicate

用户可以：

```text
Duplicate Workflow
```

复制后：

```text
Workflow ID 改变
名称可带 Copy
定义内容保持
```

不要共享同一 mutable object。

---

# 45. Workflow Delete

删除 Workflow Definition：

不应删除：

```text
已产生的文件
```

也不应自动删除：

```text
History
```

除非已有统一 History 生命周期。

---

# 46. Workflow Import / Export

M10 v1 可以支持：

```text
Export Workflow JSON
Import Workflow JSON
```

但必须：

```text
Validate
Schema Check
Tool Capability Check
```

不能导入后直接运行。

---

# 47. 外部 Workflow 导入安全

导入 JSON 时不得允许：

```text
arbitrary path traversal
arbitrary command
arbitrary script
arbitrary native execution
```

Workflow JSON 只能描述：

```text
Declarative operations
```

---

# 48. Missing Tool

如果 Workflow 使用了当前版本不存在的 Tool：

必须显示：

```text
Tool unavailable
```

而不是：

```text
silently skip
```

---

# 49. Compatibility

Workflow 可以包含：

```text
requiredCapabilities
```

例如：

```text
m6.image.resize
m9.hash
m7.batch
```

加载时检查 capability availability。

---

# 50. Graceful Degradation

原则：

```text
不能安全执行
→ 不执行
```

不要：

```text
猜测配置
自动替换 Tool
静默跳过
```

---

# 51. Workflow Builder UI

M10 必须提供真实 UI。

核心布局可以是：

```text
┌───────────────────────────────────────┐
│ Workflow Name                         │
├────────────┬──────────────────────────┤
│ Tool List  │      Workflow Canvas     │
│            │                          │
│ Input      │  Input                   │
│ Filter     │    ↓                     │
│ Tool       │  Filter                  │
│ Export     │    ↓                     │
│            │  Tool                    │
│            │    ↓                     │
│            │  Export                  │
├────────────┴──────────────────────────┤
│ Validation / Preview / Run / Save     │
└───────────────────────────────────────┘
```

具体视觉设计服从现有 Weave UI。

---

# 52. Builder v1 不需要复杂节点编辑器

不要为了“像专业自动化软件”而引入：

```text
无限画布
节点连线系统
Bezier connectors
DAG graph editor
复杂 zoom
mini-map
```

M10 v1 是：

> Linear Workflow Builder

可以使用：

```text
Vertical Step List
```

实现更可靠。

---

# 53. Step Reordering

必须支持：

```text
Move Up
Move Down
Insert
Delete
Duplicate
Enable / Disable
```

并保持：

```text
step order deterministic
```

---

# 54. Step Configuration

选中一个 Step 时显示：

```text
Name
Description
Configuration
Input binding
Output
Warnings
```

配置修改必须实时触发：

```text
Validation
```

---

# 55. Step Enable / Disable

允许暂时禁用 Step。

语义必须明确：

```text
disabled step = execution bypass
```

不能变成：

```text
execution failure
```

但 Preview 必须显示：

```text
Step disabled
```

---

# 56. 不允许静默跳过 Step

只有用户显式 Disable 才能跳过。

遇到：

```text
Error
Unsupported
Missing capability
Invalid config
```

不能静默跳过。

---

# 57. Workflow Preview UI

Preview 页面应该至少能回答：

```text
会处理多少项？
哪些项会被过滤？
哪些会被修改？
哪里可能发生冲突？
会生成什么输出？
有什么警告？
```

---

# 58. Preview Diff

对于修改型操作：

应尽可能支持：

```text
Before
After
```

例如 Rename：

```text
old.png
→
001.png
```

Resize：

```text
1920×1080
→
1280×720
```

Document：

```text
pages 12
→
pages 8
```

实际能力不足则明确标记。

---

# 59. Collision Preview

任何可能产生文件写入的 Workflow：

必须在 Preview 中检测：

```text
Existing destination
Duplicate output
Name collision
Path collision
Case collision
```

复用：

```text
M2 Collision Detection
```

---

# 60. Path Safety

所有 Workflow path：

必须经过 M1/M2 的：

```text
Path normalization
Containment checks
Traversal prevention
```

拒绝：

```text
../../
```

以及等价绕过。

---

# 61. Absolute Path

Workflow 如果保存绝对路径：

需要明确语义。

建议支持：

```text
Explicit absolute path
```

但必须：

```text
显示
验证
```

不要在导入其它机器时假装路径仍然有效。

---

# 62. Portable Workflow

可以支持：

```text
relative paths
workflow parameters
```

但 v1 不需要复杂变量系统。

---

# 63. File Selection

Input UI 应支持：

```text
File Picker
Directory Picker
Drag & Drop
Recent
Workflow Parameter
```

实际支持能力以现有 Tauri/UI 为准。

---

# 64. Clipboard

M10 可以利用已有 Clipboard 能力。

但：

```text
Clipboard
```

不是 Workflow Engine。

大数据复制必须：

```text
bounded
```

不得在 UI 中无限制复制 GB 级内容。

---

# 65. Batch Input

Workflow 可以接受：

```text
single item
multiple files
directory
```

最终仍进入 M7 Batch Engine。

---

# 66. Item Context

M10 不得重新定义 M7 的：

```text
ItemContext
ExecutionContext
```

Workflow 的 Step Binding 应映射到 M7 已有上下文。

---

# 67. Context Binding

必须明确：

```text
Current Item
Previous Step Output
Workflow Parameter
Static Value
```

不能把整个 Workflow State 暴露给 Tool。

---

# 68. Output Mapping

一个 Tool 产生的 Output：

必须定义：

```text
Primary Output
Metadata
Diagnostics
```

后续 Step 可以明确消费。

---

# 69. Metadata

允许逐步传递有限 metadata，例如：

```text
filename
extension
size
dimensions
mime
encoding
hash
```

但必须使用已有模型。

不要把所有 Rust 对象塞进一个动态 dictionary。

---

# 70. Filter Composition

M10 v1 可以支持：

```text
AND
OR
NOT
```

但必须保持有限。

例如：

```text
Extension == png
AND
Size > 10MB
```

不要实现完整 expression language。

---

# 71. Filter Expression 安全

禁止：

```text
eval()
script execution
dynamic code execution
```

过滤表达式必须是：

> typed declarative predicate

---

# 72. Regex Filter

如果使用 Regex：

必须调用 M4/M9 既有 Regex 能力。

不得在 M10：

```text
new regex engine
```

尤其不能把 UI regex 与 backend regex 做成两个不同引擎。

---

# 73. Filter Resource Limits

Regex Filter 必须继承 M9 的：

```text
input length limit
execution limit
pattern validation
```

不能因为 Workflow 批处理而取消资源限制。

---

# 74. Workflow Runtime Limits

M10 必须尊重 M7 resource budget。

不得：

```text
无限文件
无限内存
无限 output
无限 regex input
无限 Base64 decode
```

---

# 75. Workflow Size Limits

至少限制：

```text
maximum step count
maximum serialized workflow size
maximum parameter count
maximum input count
```

具体值根据真实测试结果设置。

不要随意写一个“看起来专业”的数字而没有依据。

---

# 76. Cycle Detection

虽然 M10 v1 是线性 Pipeline：

仍必须防止：

```text
self-reference
cyclic reference
recursive workflow
```

---

# 77. Saved Workflow List

需要一个 Workflow Library。

展示：

```text
Name
Description
Updated At
Version
Last Run
Status
```

可提供：

```text
Open
Run
Duplicate
Export
Delete
```

---

# 78. Workflow Search

如果已有统一 Search：

可以复用。

否则不要在 M10 实现完整全文搜索系统。

基础：

```text
name
description
```

足够。

---

# 79. Workflow Tags

v1 不需要复杂 tag system。

除非项目已有统一 metadata infrastructure。

优先保持：

```text
Name
Description
Category
```

---

# 80. Built-in Workflow Templates

可以提供少量官方模板。

例如：

```text
Find Large Images
Hash Files
Resize Images
Convert CSV to JSON
Clean CSV
Extract Text
```

但模板必须使用：

```text
真实已实现 capabilities
```

不能创建无法执行的 demo workflow。

---

# 81. Template 与 User Workflow

模板是：

```text
copy-on-create
```

而不是：

```text
shared mutable object
```

---

# 82. Workflow Naming

推荐默认命名：

```text
Untitled Workflow
```

保存时要求有效名称。

名称不参与安全判断。

---

# 83. Workflow Autosave

M10 v1 可以做草稿保护。

但不得让：

```text
Draft
```

污染：

```text
Saved Workflow
```

可采用：

```text
temporary draft
```

机制。

---

# 84. Crash Recovery

如果已有统一 application state：

可以复用。

不要额外建立一套复杂数据库。

---

# 85. Workflow Execution Dialog

点击 Run 后建议：

```text
1. Validation
2. Preview summary
3. Confirmation
4. Execute
```

对于纯读取 Workflow：

可以减少确认。

但涉及写入时必须清晰提示。

---

# 86. Destructive Operations

任何可能：

```text
delete
overwrite
replace
move
rename
```

的 Workflow：

必须显示明确影响范围。

默认：

```text
No silent destructive operation
```

---

# 87. Delete Workflow ≠ Delete Data

必须严格区分：

```text
Delete Workflow
```

只删除：

```text
Workflow Definition
```

而不是：

```text
Files
Database Rows
History Data
```

---

# 88. Undo

Workflow 的 Undo：

必须落到底层 M2 History。

禁止：

```text
M10 custom undo
```

---

# 89. History

History 至少可以显示：

```text
Workflow Name
Run Id
Timestamp
Status
Affected Items
```

具体历史实现复用 M2/M7。

---

# 90. Re-run

允许从 History：

```text
Re-run Workflow
```

但必须重新：

```text
Validate
Plan
Check paths
Check conflicts
```

不能直接执行旧 plan。

---

# 91. Old Plan 不可直接复用

因为：

```text
filesystem may change
workflow may change
tool version may change
capability may change
```

因此：

```text
History stores intent/result
```

而不是：

```text
trusted executable plan
```

---

# 92. Cancellation

必须复用 M7 cancellation。

M10 UI 只负责：

```text
Cancel
```

不自己杀线程。

---

# 93. Retry

必须复用 M7 retry。

M10 可以提供：

```text
Retry
```

但不重新实现：

```text
RetryPolicy
```

---

# 94. Resume

必须复用 M7 resume。

Workflow Definition 不需要自己保存：

```text
item progress state
```

---

# 95. Failure Policy

Workflow 必须定义清晰失败策略。

最低支持：

```text
Fail Fast
Continue Safe Items
```

实际实现应基于 M7。

不允许默认：

```text
ignore all errors
```

---

# 96. Error Presentation

每一个失败必须尽量关联：

```text
Workflow Step
Item
Tool
Error Code
Message
Recovery Suggestion
```

例如：

```text
Step:
Resize Images

Item:
photo-021.jpg

Error:
OUTPUT_COLLISION
```

---

# 97. No Raw Internal Error Leakage

UI 不应直接显示：

```text
Rust panic
stack trace
internal filesystem structure
```

开发环境可以记录详细 diagnostics。

用户界面应使用 structured error。

---

# 98. Diagnostics

开发诊断至少应该包括：

```text
WorkflowId
WorkflowVersion
StepId
ToolId
RunId
ErrorCode
```

避免记录：

```text
full file content
password
secret
huge payload
clipboard data
```

---

# 99. Privacy

M10 是本地工具。

默认：

```text
No telemetry
No workflow upload
No cloud validation
No analytics
```

Workflow 内容可能包含：

```text
local paths
personal filenames
patterns
```

因此必须按私人数据处理。

---

# 100. Logging

禁止默认日志包含：

```text
entire file contents
large text blobs
clipboard content
API secrets
full binary payload
```

尤其：

```text
Base64
Regex input
Text content
```

必须避免写入日志。

---

# 101. Security：Path Injection

所有 Path Parameter：

必须通过现有：

```text
Path Safety
```

不能：

```text
String concat
```

拼接路径。

---

# 102. Security：Configuration Injection

Workflow JSON：

必须作为：

```text
Data
```

处理。

绝不能：

```text
eval(JSON)
```

---

# 103. Security：Prototype / Deserialization

TS 层处理 Workflow JSON：

注意：

```text
prototype pollution
unsafe object merge
unchecked dynamic keys
```

Rust 端同样要求严格 schema validation。

---

# 104. Security：Malformed Workflow

必须测试：

```text
empty
broken
unknown version
unknown type
huge payload
duplicate ids
invalid references
deeply nested data
invalid UTF-8 where relevant
```

---

# 105. Resource Exhaustion

必须考虑：

```text
10,000 files
100,000 files
huge text
huge Base64
many regex matches
large images
large PDFs
large workflow JSON
```

不能因为 Workflow 把之前的资源边界绕过去。

---

# 106. Workflow Determinism

对于：

```text
same input
same workflow
same configuration
same relevant environment
```

核心计算应尽可能得到一致结果。

随机型工具：

例如 UUID：

其随机性必须明确。

不要伪造：

```text
deterministic UUID
```

除非明确使用 deterministic namespace/version。

---

# 107. Time-dependent Workflow

Timestamp 之类工具必须使用：

```text
M9 Clock semantics
```

而不是 UI 当前时间。

---

# 108. Execution Context

M10 可以提供：

```text
WorkflowExecutionContext
```

但它只做：

```text
workflow-level identity / parameter binding
```

真正执行 context 仍属于 M7。

---

# 109. Workflow Compiler Boundary

推荐：

```text
crates/weave-core
    workflow domain

crates/weave-application
    workflow validation/planning

crates/weave-batch
    execution

crates/weave-history
    history

frontend
    Workflow Builder
```

具体 crate 必须先检查当前项目实际结构。

---

# 110. Domain / Application / Adapter

严格遵守：

```text
UI
↓
Application
↓
Domain
↓
Adapters
↓
OS
```

React 不可以：

```text
直接访问文件系统
```

Workflow Domain 不可以：

```text
直接调用 window API
```

---

# 111. IPC

Tauri Commands 应保持：

```text
small
typed
bounded
```

例如：

```text
workflow_list
workflow_get
workflow_validate
workflow_preview
workflow_save
workflow_delete
workflow_run
workflow_cancel
workflow_duplicate
workflow_import
workflow_export
```

实际命令名必须审计已有 conventions。

---

# 112. IPC 禁止

不要暴露：

```text
execute_arbitrary_step
run_shell
evaluate_expression
execute_script
```

---

# 113. Frontend State

不要把整个 Workflow execution engine 放入 React。

React 只管理：

```text
UI State
Selection
Editing State
Preview State
Execution Presentation
```

---

# 114. Backend State

Rust/application/domain 管理：

```text
Workflow Definition
Validation
Planning
Execution coordination
Persistence
```

---

# 115. UI State 与 Domain State

严格分离：

```text
WorkflowModel
≠
WorkflowEditorState
```

例如：

```text
selectedStepId
dragging
expandedPanel
previewTab
```

不能进入 Workflow JSON。

---

# 116. Drag & Drop

M10 可以提供：

```text
Tool → Workflow
```

但底层仍然必须经过：

```text
typed insertion
validation
```

不要仅通过 UI 字符串拼 JSON。

---

# 117. Step Insertion

加入 Tool 时：

```text
Create Step
Resolve Capability
Apply Defaults
Validate
Insert
```

不要：

```text
Tool Drop
→ blindly add JSON
```

---

# 118. Default Configuration

每个 Tool 可以提供：

```text
defaultConfig
```

但必须来自 Tool Schema/Registry。

UI 不得重复定义默认配置。

---

# 119. Configuration Validation

配置修改后：

```text
UI
→ typed config
→ application validation
```

UI validation 只能：

```text
辅助反馈
```

不能成为唯一防线。

---

# 120. Workflow Save

保存流程：

```text
Draft
 ↓
Serialize
 ↓
Schema Validation
 ↓
Semantic Validation
 ↓
Atomic Save
```

文件写入必须使用安全写入机制。

---

# 121. Workflow Save Failure

必须避免：

```text
partial workflow file
corrupt saved workflow
```

应使用现有 Safe Write 基础设施，或者项目已有可靠的原子配置存储机制。

---

# 122. Workflow Import

流程：

```text
Read
 ↓
Parse
 ↓
Schema Validation
 ↓
Semantic Validation
 ↓
Capability Check
 ↓
User Preview
 ↓
Save
```

不能：

```text
Import → Execute
```

---

# 123. Workflow Export

导出的 JSON 应：

```text
deterministic
stable
human readable
```

排序规则应固定。

---

# 124. Workflow JSON

概念示例：

```json
{
  "schemaVersion": 1,
  "id": "workflow-id",
  "name": "Resize Large Images",
  "description": "Resize large images to 1920px",
  "parameters": [],
  "steps": [
    {
      "id": "input-1",
      "type": "input",
      "config": {}
    },
    {
      "id": "filter-1",
      "type": "filter",
      "config": {}
    },
    {
      "id": "resize-1",
      "type": "tool",
      "toolId": "image.resize",
      "config": {}
    },
    {
      "id": "export-1",
      "type": "export",
      "config": {}
    }
  ]
}
```

这是概念模型。

示例为省略写法；完整步骤字段以 #5 WorkflowStep 模型为准（id / type / name / config / enabled / position）。

必须根据实际实现建立正式 schema。

---

# 125. 不允许将这个 JSON 示例直接当最终 Schema

必须：

```text
Audit
→ Design
→ Type
→ Validate
→ Test
```

---

# 126. Schema Test

至少测试：

```text
valid workflow
minimal workflow
empty workflow
unknown property
missing property
wrong type
unknown step
unknown tool
wrong schema version
```

fixtures / golden 目录规划（对齐 charter #42）：workflow JSON golden、各步骤输入 fixture、类型不兼容矩阵用例。

Schema 与步骤类型兼容矩阵是 Spec 产物，必须先于 Builder UI 实现并冻结。

---

# 127. Property-Based / Fuzz Testing

Workflow parser/validator 非常适合：

```text
fuzz
property tests
```

至少覆盖：

```text
random step ids
invalid references
deep configurations
large strings
malformed JSON
```

---

# 128. Golden Files

Workflow serialization 应有：

```text
golden JSON
```

确保：

```text
same model
→ same serialized representation
```

---

# 129. Round-trip

必须测试：

```text
Workflow
→ Serialize
→ Parse
→ Serialize
```

结果必须稳定。

---

# 130. Version Round-trip

确保：

```text
saved workflow v1
→ load
→ save
```

不会无意义改变结构。

---

# 131. Tool Registry Tests

必须验证：

```text
Every registered tool
→ has valid schema
→ has unique ID
→ resolves correctly
→ capability metadata exists
```

---

# 132. Workflow Compatibility Tests

建立最小矩阵：

```text
Input → Tool
Input → Filter
Filter → Tool
Tool → Tool
Tool → Export
```

验证非法组合被拒绝。

---

# 133. M1 Regression

测试 Workflow 不得绕过：

```text
Path Safety
File Metadata
Filesystem Abstraction
Streaming
Cancellation
```

---

# 134. M2 Regression

测试：

```text
Safe Write
Collision
History
Undo
Recovery
TOCTOU
```

---

# 135. M3 Regression

Duplicate Finder 作为 Workflow Tool 时：

必须仍然保持：

```text
exact duplicate semantics
```

M10 不能把：

```text
similar files
```

伪装成：

```text
duplicates
```

---

# 136. M4 Regression

Text Workflow 必须保持：

```text
encoding semantics
line endings
diagnostics
regex semantics
```

---

# 137. M5 Regression

Data Workflow 必须保持：

```text
CSV parsing
JSON
TSV
JSONL
row/column semantics
```

---

# 138. M6 Regression

Image Workflow 必须保持：

```text
decode
orientation
alpha
animation safety
resource limits
```

---

# 139. M7 Regression

这是 M10 最重要的回归对象之一。

确保：

```text
Workflow
→ M7
```

实际执行，而不是：

```text
Workflow
→ second execution engine
```

---

# 140. M8 Regression

Document Workflow：

必须继续使用：

```text
DocumentCapabilities
Inspect
Merge
Split
Page model
Safe Write
```

---

# 141. M9 Regression

M10 使用：

```text
Hash
Base64
UUID
Timestamp
URL
Regex
Color
Checksum
```

时必须直接调用 M9 capability。

不能 copy implementation。

---

# 142. M10 不得产生重复工具

完成后全局搜索：

```text
hash
base64
uuid
timestamp
url_encode
regex
color
checksum
resize
csv
pdf
```

检查是否出现第二份业务实现。

出现重复核心实现：

```text
P0
```

---

# 143. Workflow Catalog

可以建立：

```text
Built-in
User
Imported
Recent
```

但只在确有 UX 价值时实现。

保持简单。

---

# 144. Workflow Categories

建议有限：

```text
Files
Text
Data
Images
Documents
Utilities
```

分类只是 UX metadata。

不能成为第二套 capability registry。

---

# 145. Workflow Documentation

每一个官方模板必须说明：

```text
Purpose
Input
Steps
Output
Side Effects
Limitations
```

---

# 146. Empty State

首次进入 Workflow 页面：

不要展示空白屏。

应该解释：

```text
Create workflow
Choose input
Add tools
Preview
Run
```

---

# 147. Error State

必须提供：

```text
What happened
Where
Why
What can be done
```

而不是：

```text
Failed
```

---

# 148. Accessibility

至少保证：

```text
Keyboard navigation
Focus visibility
Semantic buttons
Screen-reader labels
No color-only state
```

Workflow Step list 应支持键盘操作。

---

# 149. Keyboard UX

建议支持：

```text
Ctrl/Cmd + S
Ctrl/Cmd + Z
Ctrl/Cmd + Shift + Z
Delete
Arrow Up
Arrow Down
Enter
Escape
```

但必须避免与系统/已有快捷键冲突。

---

# 150. Internationalization

所有用户可见文本：

不得硬编码。

需要：

```text
zh
en
```

至少覆盖：

```text
Workflow
Input
Filter
Tool
Export
Preview
Run
Save
Validation
Error
Warning
```

---

# 151. Number / Unit Formatting

不要在 domain 层：

```text
format "10 MB"
format localized timestamp
```

保持：

```text
typed value
```

UI 再本地化显示。

---

# 152. Performance

必须实测：

```text
Workflow validation
Workflow serialization
Large workflow loading
Tool lookup
Preview planning
Execution startup overhead
```

---

# 153. Workflow Builder Performance

大量 Step 时：

```text
100 steps
```

不应导致明显卡顿。

但由于 v1 是线性工作流：

建议控制最大 step 数。

实际限制必须来自：

```text
UX
benchmark
memory
```

而不是拍脑袋。

---

# 154. Large Input

例如：

```text
100k files
```

Workflow UI 不允许：

```text
render all rows
```

必须使用：

```text
virtualization
pagination
bounded preview
```

复用已有大数据 UI 能力。

---

# 155. Preview Size

Preview 不必显示所有项目。

例如：

```text
Showing first N items
```

但是：

```text
Total count
```

必须准确。

---

# 156. Backpressure

M10 不得因为 Preview：

```text
一次性读取所有文件到内存
```

必须调用 M1/M7 的：

```text
streaming
bounded concurrency
```

---

# 157. Lazy Preview

对于昂贵操作：

```text
Image
PDF
Large Text
Hash
```

Preview 应尽量：

```text
metadata first
deeper computation only when needed
```

具体以已有 capability 支持为准。

---

# 158. Preview Cache

可以缓存：

```text
preview result
```

但必须清晰定义 invalidation。

不能因为缓存导致：

```text
stale preview executed as truth
```

---

# 159. 文件系统变化

Preview 后文件可能被：

```text
move
delete
modify
```

因此 Execute 前必须重新：

```text
validate / plan / TOCTOU check
```

不能完全信任旧 Preview。

---

# 160. Preview Token

可以引入：

```text
PreviewId
```

用于关联 UI。

但：

```text
PreviewId ≠ trusted execution authority
```

---

# 161. Preview Security

Execute 时必须重新检查：

```text
path
permissions
existence
collision
tool capability
configuration
```

---

# 162. Workflow Clone

Workflow Clone 不应该复用：

```text
RunId
HistoryId
TransactionId
```

---

# 163. Execution History

保存：

```text
workflowVersion
```

非常重要。

否则以后 Workflow 修改后：

```text
history becomes ambiguous
```

---

# 164. Workflow Version vs Tool Version

History 最好区分：

```text
WorkflowVersion
ToolCapabilityVersion
```

不必第一版做到全量锁定依赖版本，但至少记录真实可获得的信息。

---

# 165. Reproducibility

对于纯计算 Workflow：

应该尽量可重复。

对于环境依赖：

必须显示限制。

例如：

```text
Timestamp
Current filesystem
System timezone
```

本身不是固定输入。

---

# 166. Workflow Description

建议用户可以写：

```text
“Resize large PNGs to 1920px and export to optimized folder.”
```

但 Description 是：

```text
metadata
```

不能驱动执行。

---

# 167. No Natural Language Execution

M10 v1 不做：

```text
“帮我把这些图片处理一下”
```

这种自然语言 → Workflow 自动生成。

原因：

这会把 M10 变成 AI Agent。

AI 不属于 M10。

---

# 168. No LLM

严禁：

```text
OpenAI
Anthropic
Gemini
local LLM
AI planning
```

M10 必须：

```text
deterministic
local
explicit
```

---

# 169. No Network

除开发依赖安装外：

运行时默认不得需要：

```text
Internet
```

---

# 170. Workflow Security Model

最终原则：

```text
Workflow can describe
what Weave already knows how to do.

Workflow cannot describe
arbitrary code execution.
```

---

# 171. API Contract

建立稳定接口：

```text
WorkflowRepository
WorkflowValidator
WorkflowPlanner
WorkflowExecutorAdapter
ToolRegistry
```

具体命名以现有 architecture 为准。

---

# 172. Repository

WorkflowRepository 至少支持：

```text
list
get
save
delete
duplicate
```

可选：

```text
import
export
```

---

# 173. Repository 不负责执行

Repository：

```text
Persistence
```

不是：

```text
Runtime
```

---

# 174. Validator 不执行

Validator：

```text
Validate
```

不得：

```text
Mutate
Write
Run
```

---

# 175. Planner 不执行

Planner：

```text
Compile
Build Plan
```

不得：

```text
execute filesystem mutations
```

---

# 176. Executor Adapter

M10 可以有：

```text
M7WorkflowAdapter
```

负责：

```text
Workflow Plan
→ M7 Job
```

---

# 177. Tool Adapter

如果现有 Tools API 不适合作为 Workflow Capability：

可以增加 Adapter。

但 Adapter 必须很薄：

```text
Workflow contract
↔ existing capability contract
```

不能重新实现业务逻辑。

---

# 178. Domain Events

如果项目已有事件系统，可以复用：

```text
WorkflowSaved
WorkflowStarted
WorkflowCompleted
WorkflowFailed
WorkflowCancelled
```

但不要为了 M10 单独引入复杂 Event Bus。

---

# 179. UI Event

React UI 可以监听：

```text
run progress
step progress
completion
failure
```

但实际状态由 M7 提供。

---

# 180. Progress

显示：

```text
Overall
Current Step
Current Item
```

数据来源必须：

```text
M7
```

---

# 181. Nested Progress

不应该让 M10 自己计算：

```text
fake progress %
```

如果 M7 已提供真实阶段权重：

直接复用。

---

# 182. Cancellation UX

执行中至少：

```text
Cancel
```

取消后明确：

```text
Cancellation Requested
Cancelling
Cancelled
```

不要立即 UI 假装完成。

---

# 183. Partial Completion

如果底层允许：

```text
some items succeeded
some failed
```

必须真实显示。

不能：

```text
overall failed
→ hide successful work
```

---

# 184. Output Summary

完成后显示：

```text
Processed
Succeeded
Skipped
Failed
Created
Modified
```

实际字段根据 M7/M2 result 模型。

---

# 185. Open Results

执行完成后可以：

```text
Open Output Folder
Open Result
Copy Summary
```

但这些都必须调用已有 platform capability。

---

# 186. No Hidden Side Effects

执行前用户必须知道：

```text
input
output
write behavior
overwrite behavior
```

---

# 187. Workflow Inspection

提供一个：

```text
Workflow Details
```

查看：

```text
Steps
Parameters
Capabilities
Side Effects
Version
```

---

# 188. Workflow Validation Panel

建议显示：

```text
✓ Input configured
✓ Tools available
✓ Types compatible
⚠ MD5 warning
✕ Output directory missing
```

---

# 189. Capability Inspector

Tool Step 可以提供：

```text
Capability Info
```

帮助用户理解：

```text
what it does
what it consumes
what it produces
whether it mutates files
```

---

# 190. Tool Discovery

Tool Picker 必须从：

```text
ToolRegistry
```

生成。

不得写死重复数组。

---

# 191. Search Tool Picker

允许：

```text
search by name
search by category
```

即可。

不要引入复杂 AI search。

---

# 192. Unsupported Tool

如果 capability 存在但平台不支持：

显示：

```text
Unsupported on this platform
```

而不是：

```text
hide it
```

至少在 diagnostics / import context 中可见。

---

# 193. Workflow Portability

跨平台 Workflow：

```text
Windows
macOS
Linux
```

必须避免假设：

```text
path separator
drive letters
shell syntax
```

---

# 194. Platform Path

Path 处理必须通过：

```text
Rust Path
```

或者统一 filesystem abstraction。

禁止手工：

```text
"\"
"/"
```

进行跨平台业务判断。

---

# 195. Line Endings

Workflow JSON 本身可以保持稳定格式。

Workflow 内容处理由：

```text
M4
```

负责。

---

# 196. Workflow File Location

必须审计已有 App Data 路径。

不要自行决定：

```text
C:\Users\...\weave-workflows
```

之类硬编码路径。

---

# 197. Config Migration

如果 Workflow Storage 发生变化：

必须提供：

```text
migration
```

而不是要求用户手工清理文件。

---

# 198. Dependency Audit

M10 添加依赖前必须：

```text
Search existing dependencies
Check version
Check license
Check maintenance
Check binary size
Check security
Check necessity
```

原则：

> 不为了一个小 UI 功能引入大型 Workflow Framework。

---

# 199. 不要引入通用自动化框架

禁止直接加入：

```text
Node-RED
n8n
Temporal
Airflow
Dagster
```

或者同类重量级 Workflow Runtime。

Weave 自己拥有足够小的领域模型。

---

# 200. M10 v1 的正确复杂度

目标不是：

```text
企业级 Workflow Platform
```

而是：

> **桌面工具的可复用组合层。**

---

# 201. 推荐第一版范围

M10 v1 必须优先完成：

```text
Workflow Definition
Tool Registry
Typed Step
Validation
Parameter Binding
Linear Step Builder
Preview
Plan
Save
Load
Duplicate
Import
Export
Run
Cancel
History Integration
M7 Integration
M2 Integration
```

---

# 202. 可以延后的功能

若工作量过大，下列功能可以明确 DEFER：

```text
Workflow sharing
Cloud sync
Remote execution
Triggers
Scheduled execution
DAG
Branching
Loops
Sub-workflow
Version branches
Collaborative editing
AI workflow generation
```

---

# 203. M10 不得偷渡 M11

后续阶段如果存在：

```text
workflow marketplace
plugin system
sharing
automation trigger
```

不得为了“提前做好架构”而实现。

只留下扩展点。

---

# 204. 开发第一阶段：基线审计

开始任何修改前：

**严禁直接写代码。**

先完整检查：

```text
Git
Cargo workspace
Rust modules
Tauri commands
IPC
React routes
State management
Storage
i18n
Error model
Logging
M1
M2
M3
M4
M5
M6
M7
M8
M9
```

---

# 205. 必须寻找已有 Workflow 雏形

全仓搜索：

```text
workflow
pipeline
operation
job
tool
capability
plan
task
step
batch
history
```

确认是否已经存在：

```text
partial implementation
prototype
unused abstraction
duplicate type
```

---

# 206. 必须寻找重复工具

搜索：

```text
hash
checksum
base64
uuid
timestamp
url
regex
color
```

任何重复核心实现必须优先复用或删除重复层。

---

# 207. 审计报告格式

必须使用：

```text
FACT
HYPOTHESIS
INFERENCE
```

不要把推测写成事实。

状态使用：

```text
CONFIRMED
OPEN
RESOLVED
WORSENED
N/A
DEFERRED
```

问题等级只允许：

```text
P0
P1
```

---

# 208. P0 定义

M10 P0 包括：

```text
arbitrary code execution
unsafe path execution
bypass M2 Safe Write
bypass M7 execution safety
silent destructive operation
wrong data mutation
corrupted workflow persistence
critical type unsafety
security vulnerability
major regression
fake successful execution
```

---

# 209. P1 定义

例如：

```text
major workflow UX break
important validation gap
missing core capability
serious performance problem
significant accessibility regression
serialization incompatibility
important i18n omission
```

---

# 210. 不要报告大量 P2/P3

M10 审计只关注：

```text
P0
P1
```

其它事项可以：

```text
Known Limitation
Deferred
```

---

# 211. Implementation Order

严格按下面顺序：

```text
Phase A
Baseline Audit

Phase B
Domain Model

Phase C
Tool Registry

Phase D
Validation

Phase E
Planning / Compilation

Phase F
Persistence

Phase G
M7 Adapter

Phase H
M2 Integration

Phase I
Workflow Builder UI

Phase J
Preview

Phase K
Run / Cancel / History

Phase L
Import / Export / Templates

Phase M
Security / Performance / Accessibility / i18n

Phase N
Final Audit
```

---

# 212. Phase B：Domain

先建立：

```text
Workflow
WorkflowStep
InputStep
FilterStep
ToolStep
ExportStep
WorkflowParameter
Binding
```

优先采用：

```text
strong typing
```

---

# 213. Phase C：Registry

建立：

```text
ToolCapability
ToolRegistry
ToolSchema
```

确保：

```text
M1–M9 existing capabilities
```

能够注册为 Workflow Tools。

---

# 214. Phase D：Validation

实现：

```text
Structural Validation
Type Validation
Config Validation
Capability Validation
Reference Validation
```

---

# 215. Phase E：Planning

实现：

```text
Workflow → Validated → Execution Plan
```

不能执行真实写入。

---

# 216. Phase F：Persistence

实现：

```text
save
load
list
delete
duplicate
```

并建立：

```text
schemaVersion
```

---

# 217. Phase G：M7

建立：

```text
Workflow → M7 Job
```

优先实现最小垂直链路：

```text
Input
→ Tool
→ Export
→ M7 Execute
```

---

# 218. Phase H：M2

加入真实：

```text
Safe Write
Collision
History
Undo
```

---

# 219. Phase I：UI

先完成：

```text
Workflow List
Workflow Editor
Tool Picker
Step List
Config Panel
Validation Panel
```

---

# 220. Phase J：Preview

实现：

```text
Preview
Impact Summary
Warnings
Collisions
Before/After where supported
```

---

# 221. Phase K：Runtime

接入：

```text
Run
Progress
Cancel
Failure
Completion
History
```

---

# 222. Phase L：Import/Export

实现：

```text
Workflow JSON Export
Workflow JSON Import
Schema Validation
Capability Validation
```

---

# 223. Phase M：Quality

执行：

```text
Security Audit
Privacy Audit
Dependency Audit
Performance Benchmark
Accessibility Audit
i18n Audit
License Audit
```

---

# 224. 最小 E2E Workflow

至少建立真实 E2E：

```text
Input
→ Filter
→ Tool
→ Export
```

并确保不是 Mock。

---

# 225. 推荐 E2E 1

```text
Directory
→ Filter PNG
→ Hash SHA-256
→ Export CSV
```

验证：

```text
M1
M9
M7
M5
```

全部真实工作。

---

# 226. 推荐 E2E 2

```text
Directory
→ Filter Large Images
→ Resize
→ Export
```

验证：

```text
M1
M6
M7
M2
```

---

# 227. 推荐 E2E 3

```text
CSV
→ Data Clean
→ Export JSON
```

验证：

```text
M5
M7
M2
```

---

# 228. 推荐 E2E 4

```text
Text
→ URL Decode
→ Regex Extract
→ Export
```

验证：

```text
M4
M9
M7
```

---

# 229. 推荐 E2E 5

```text
Files
→ Rename
→ Export/Finalize
```

验证：

```text
M2
M7
M10
```

---

# 230. Preview E2E

必须验证：

```text
Preview
→ no mutation
```

执行前确认：

```text
filesystem snapshot unchanged
```

---

# 231. Execute E2E

必须验证：

```text
Preview
→ Execute
```

执行后：

```text
actual result matches preview assumptions
```

遇到 TOCTOU：

必须正确失败，而不是继续执行。

---

# 232. Cancellation E2E

验证：

```text
large input
→ start
→ cancel
```

必须：

```text
stop safely
no corrupted output
correct status
```

---

# 233. Failure E2E

模拟：

```text
permission denied
missing input
collision
invalid configuration
tool failure
```

检查：

```text
correct structured error
correct History
correct M7 state
```

---

# 234. Recovery E2E

如果 M7/M2 支持：

```text
interrupted workflow
→ resume
```

必须走现有恢复逻辑。

---

# 235. Undo E2E

针对真实写入型 Workflow：

```text
Run
→ Success
→ Undo
```

必须通过 M2。

---

# 236. Import E2E

测试：

```text
export
→ import
→ validate
→ run
```

必须成功。

---

# 237. Broken Import E2E

构造：

```text
unknown tool
bad schema
invalid path
invalid configuration
```

必须：

```text
reject
```

---

# 238. Serialization Tests

必须至少覆盖：

```text
empty
single step
multiple steps
parameters
bindings
disabled step
warnings
```

---

# 239. Validation Matrix

必须建立表格：

| Case | Expected |
|---|---|
| No Input | ERROR |
| Tool Missing | ERROR |
| Wrong Type | ERROR |
| Invalid Config | ERROR |
| Unsupported Capability | ERROR |
| Disabled Step | VALID |
| Warning Only | VALID |
| Empty Filter Result | VALID |
| Collision | ERROR/WARNING by policy |
| Missing Optional Output | policy-defined |

最终以真实语义为准。

---

# 240. Security Test Matrix

至少覆盖：

```text
../
..\
absolute path
UNC path
symlink
junction
malformed JSON
huge workflow
huge parameter
regex abuse
large Base64
invalid unicode
unknown tool
```

---

# 241. Performance Benchmark

至少测量：

```text
1 step
10 steps
50 steps
100 steps
```

以及：

```text
1
100
1,000
10,000
```

输入规模下的：

```text
validation
planning
preview
```

---

# 242. 不允许伪造 Benchmark

禁止：

```text
预计 <100ms
应该很快
理论上 O(n)
```

代替真实结果。

最终报告必须：

```text
Measured
```

---

# 243. Build

必须执行：

```text
cargo check
cargo fmt --check
cargo clippy
cargo test
frontend typecheck
frontend lint
frontend test
frontend build
Tauri build
license check
```

实际命令根据项目配置确定。

以上全部为无条件必过项（charter #45）。

---

# 244. Full Regression

M10 完成前至少验证：

```text
M1
M2
M3
M4
M5
M6
M7
M8
M9
```

核心测试仍通过。

---

# 245. Git Hygiene

完成前检查：

```text
git status
git diff
git diff --cached
```

禁止提交：

```text
.env
secrets
private data
temporary database
large generated files
local absolute paths
debug dump
```

---

# 246. Dependency Hygiene

检查：

```text
new dependency
license
binary size
lockfile
```

没有明确价值的 dependency：

不要加入。

---

# 247. Documentation

至少更新：

```text
README
Architecture
Workflow format
Tool capability rules
Import/export format
Security notes
Known limitations
```

---

# 248. M10 文档必须说明

用户需要能理解：

```text
Workflow
Step
Input
Filter
Tool
Export
Preview
Run
History
```

---

# 249. Workflow Schema 文档

必须有：

```text
schemaVersion
example
validation rules
compatibility
migration policy
```

---

# 250. Tool Capability 文档

必须说明：

```text
how existing tools become workflow tools
```

尤其：

```text
M1–M9 reuse
```

---

# 251. Architecture Guard

最终执行架构必须接近：

```text
             ┌──────────────┐
             │ React UI     │
             └──────┬───────┘
                    │
                    ▼
             ┌──────────────┐
             │ M10 Workflow │
             │ Composition  │
             └──────┬───────┘
                    │
          ┌─────────┴───────────┐
          ▼                     ▼
   ┌─────────────┐       ┌──────────────┐
   │ Capability  │       │ Validation   │
   │ Registry    │       │ / Planner    │
   └──────┬──────┘       └──────┬───────┘
          │                     │
          └──────────┬──────────┘
                     ▼
              ┌─────────────┐
              │ M7 Batch    │
              │ Execution   │
              └──────┬──────┘
                     │
          ┌──────────┴──────────┐
          ▼                     ▼
      M1–M9 Tools             M2
                              Safe Write
                              History
                              Undo
```

---

# 252. 绝对禁止的最终架构

禁止出现：

```text
UI
 ↓
Workflow
 ↓
Direct FS
```

禁止：

```text
Workflow
 ↓
Custom Batch Engine
```

禁止：

```text
Workflow
 ↓
Custom Hash
```

禁止：

```text
Workflow
 ↓
Shell
```

禁止：

```text
Workflow
 ↓
AI
```

---

# 253. M10 的核心成功标准

完成后，用户应该能够：

```text
创建 Workflow
 ↓
选择 Input
 ↓
添加 Filter
 ↓
添加 Tool
 ↓
配置参数
 ↓
添加 Export
 ↓
Validate
 ↓
Preview
 ↓
Save
 ↓
Run
 ↓
查看 Result
 ↓
History
 ↓
再次运行
```

整个路径必须是真实可用的。

---

# 254. 禁止 Demo Completion

以下任何情况都不能标记完成：

```text
按钮能点击但没执行
Preview 是 mock
Run 是 mock
Tool 是 mock
Export 是 mock
History 是假数据
Workflow Save 只是 local React state
M7 没真正接入
M2 没真正接入
```

---

# 255. Completion Criteria

只有同时满足以下条件才允许：

```text
M10 COMPLETE
```

必须满足：

```text
[ ] Workflow domain 完成
[ ] Tool Registry 完成
[ ] Typed Step 完成
[ ] Validation 完成
[ ] Planning 完成
[ ] Persistence 完成
[ ] Import/Export 完成
[ ] Builder UI 完成
[ ] Preview 完成
[ ] M7 真实接入
[ ] M2 真实接入
[ ] Run 完成
[ ] Cancel 完成
[ ] History 完成
[ ] Undo 路径正确
[ ] Security audit 通过
[ ] Privacy audit 通过
[ ] Performance audit 完成
[ ] Accessibility audit 完成
[ ] i18n audit 完成
[ ] License audit 完成
[ ] M1–M9 regression 通过
[ ] Build 通过
[ ] Tests 通过
[ ] Git clean
```

---

# 256. Final Audit

完成实现后必须先：

> 停止开发。

进入：

> **M10 Final Audit**

不得继续加功能。

---

# 257. Final Audit A — Baseline

确认：

```text
Git
Workspace
Architecture
M1–M9 state
```

---

# 258. Final Audit B — Domain

检查：

```text
Workflow
Step
Binding
Parameter
Capability
```

是否：

```text
typed
stable
deterministic
```

---

# 259. Final Audit C — Registry

检查：

```text
M1–M9 capabilities
```

是否真正注册。

---

# 260. Final Audit D — Validation

确认：

```text
invalid workflow cannot execute
```

---

# 261. Final Audit E — Planning

确认：

```text
Workflow
→ Plan
```

不会产生副作用。

---

# 262. Final Audit F — Preview

确认：

```text
Preview
```

不修改真实数据。

---

# 263. Final Audit G — Preview/Execute

确认：

```text
same definition
same configuration
same planning semantics
```

而不是两个独立逻辑。

---

# 264. Final Audit H — M7

确认：

```text
M7 is the only execution engine
```

---

# 265. Final Audit I — M2

确认所有写入：

```text
M2 Safe Write
History
Undo
TOCTOU
```

路径正确。

---

# 266. Final Audit J — Tool Reuse

确认：

```text
No duplicate implementation
```

---

# 267. Final Audit K — Security

重点检查：

```text
path traversal
arbitrary execution
deserialization
resource exhaustion
regex abuse
large payload
logging leakage
```

---

# 268. Final Audit L — Persistence

检查：

```text
schema version
round-trip
migration boundary
corruption resistance
atomic write
```

---

# 269. Final Audit M — UX

检查完整流程：

```text
Create
Edit
Validate
Preview
Save
Run
Cancel
History
Re-run
Duplicate
Delete
Import
Export
```

---

# 270. Final Audit N — Accessibility

检查：

```text
keyboard
focus
screen reader
labels
errors
contrast
```

---

# 271. Final Audit O — i18n

检查：

```text
all visible strings
errors
validation
tool names
workflow states
```

---

# 272. Final Audit P — Performance

报告真实：

```text
validation
planning
preview
large input
large workflow
```

测量结果。

---

# 273. Final Audit Q — Dependency / License

检查所有新增依赖。

---

# 274. Final Audit R — Regression

运行：

```text
M1
M2
M3
M4
M5
M6
M7
M8
M9
```

核心测试。

---

# 275. Final Audit S — Git

必须确认：

```text
Working tree clean
No temp files
No secrets
No local paths
No generated junk
```

---

# 276. 最终报告格式

必须输出：

```markdown
# M10 Workflow Composition Completion Audit
```

包含：

## A. Baseline

```text
Commit
Branch
Dirty/clean
```

## B. Architecture

```text
M10 responsibilities
M1–M9 reuse
M7 integration
M2 integration
```

## C. Workflow Model

```text
Workflow
Step
Binding
Parameter
Tool Registry
```

## D. UI

```text
Builder
Preview
Validation
Execution
History
```

## E. Runtime

```text
M7
Cancellation
Retry
Resume
```

## F. Mutation Safety

```text
M2
Safe Write
Collision
TOCTOU
Undo
```

## G. Tests

必须报告：

```text
实际命令
实际数量
实际结果
失败项
```

不能只说：

```text
Tests passed
```

---

# 277. Security Report

必须列出：

```text
Path Safety
Arbitrary Execution
Deserialization
Resource Limits
Regex
Logging
Privacy
```

每项：

```text
PASS / FAIL
```

---

# 278. Performance Report

必须：

```text
Measured
```

而不是：

```text
Expected
```

---

# 279. Known Limitations

明确列出：

```text
Unsupported tools
Platform-specific limitations
Preview limitations
Large-input limitations
Schema limitations
```

---

# 280. Deferred Features

明确列出：

```text
DAG
Branching
Loop
Triggers
Scheduling
Sub-workflow
Cloud
AI
Plugin
Collaboration
```

---

# 281. M10 → M11 Handoff

最终必须输出：

```text
M10 provides:
- Workflow Definition
- Workflow Validation
- Workflow Planning
- Tool Registry
- Parameter Binding
- Preview
- Saved Workflow
- Import/Export
- M7 execution integration
- M2 mutation integration
- History integration
```

并明确：

```text
M11 must reuse these capabilities.
```

---

# 282. M11 不得重新实现

下一阶段不得重新实现：

```text
Tool Registry
Workflow Parser
Workflow Validator
M7 Executor
M2 Safe Write
History
Undo
```

---

# 283. M10 最终状态定义

最终只能输出：

```text
M10 COMPLETE
```

或：

```text
M10 NOT COMPLETE
```

禁止使用：

```text
Mostly Complete
Basically Complete
Nearly Complete
Production Ready-ish
```

---

# 284. Completion Gate

任何以下情况：

```text
P0 unresolved
核心 E2E 失败
M7 未真实接入
M2 未真实接入
Preview 是 mock
Run 是 mock
Workflow 无法持久化
存在重复核心工具实现
存在任意代码执行能力
```

必须：

```text
M10 NOT COMPLETE
```

---

# 285. 最后执行纪律

整个 M10 开发过程必须严格遵守：

```text
Audit before Modify
Reuse before Rebuild
Type before Runtime
Validate before Execute
Preview before Mutation
M7 executes
M2 mutates
M10 composes
```

不要为了“看起来完整”加入不属于 M10 的功能。

不要因为某项功能难做而用：

```text
mock
fake
placeholder
hardcoded
```

冒充完成。

不要因为前面 M1–M9 已经实现而跳过验证。

所有最终结论必须建立在：

```text
真实代码
真实测试
真实运行结果
真实 Git 状态
```

之上。

**最终目标：**

> M10 完成后，ifuyo Weave 不再只是一个“拥有很多工具的桌面工具箱”，而开始真正具备一种独特的产品能力——  
> **把已经存在的工具能力组合成用户自己的、本地的、可保存的、可预览的、可重复执行的操作方法。**