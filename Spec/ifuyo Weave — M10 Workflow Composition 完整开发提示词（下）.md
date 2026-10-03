# ifuyo Weave — M10 Workflow Composition 完整开发提示词（下）

> 本文件是原《ifuyo Weave — M10 Workflow Composition 完整开发提示词》的 **第 2/2 部分**，
> 覆盖原第 126–285 节：Schema Test、Property-Based/Fuzz Testing、Golden Files、Round-trip 与 Version Round-trip、Tool Registry/Workflow Compatibility Tests、M1–M9 全量回归与"不得产生重复工具"，Workflow Catalog/Categories、Workflow Documentation、Empty/Error State、Accessibility、Keyboard UX、Internationalization、Number/Unit Formatting，Performance 与 Workflow Builder Performance、Large Input/Preview Size/Backpressure/Lazy Preview/Preview Cache、文件系统变化、Preview Token/Preview Security，Workflow Clone/Execution History/Workflow Version vs Tool Version/Reproducibility、Workflow Description、No Natural Language Execution/No LLM/No Network、Workflow Security Model，API Contract、Repository 及其边界、Validator/Planner 不执行、Executor Adapter/Tool Adapter、Domain Events/UI Event、Progress/Nested Progress、Cancellation UX/Partial Completion/Output Summary/Open Results、No Hidden Side Effects，Workflow Inspection/Validation Panel/Capability Inspector/Tool Discovery/Search Tool Picker/Unsupported Tool，Workflow Portability/Platform Path/Line Endings/Workflow File Location/Config Migration、Dependency Audit、不引入通用自动化框架、v1 的正确复杂度与推荐第一版范围、可以延后的功能、不得偷渡 M11，开发第一阶段基线审计、寻找 Workflow 雏形与重复工具、审计报告格式、P0/P1 定义，Implementation Order 与 Phase B–M、最小 E2E 与推荐 E2E 1–5、Preview/Execute/Cancellation/Failure/Recovery/Undo/Import/Broken Import E2E、Serialization Tests、Validation Matrix/Security Test Matrix、Performance Benchmark 与禁止伪造，Build、Full Regression、Git Hygiene、Dependency Hygiene、Documentation、Workflow Schema 与 Tool Capability 文档，Architecture Guard、绝对禁止的最终架构、核心成功标准、禁止 Demo Completion、Completion Criteria，Final Audit A–S、最终报告格式、Security/Performance Report、Known Limitations、Deferred Features、M10→M11 Handoff、M11 不得重新实现、最终状态定义、Completion Gate 与最后执行纪律。
> **执行与收尾条款在本部分**：Final Audit A–S、最终报告格式、Completion Criteria、Completion Gate、Git Hygiene（提交纪律）、M10→M11 Handoff 与最后执行纪律。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

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