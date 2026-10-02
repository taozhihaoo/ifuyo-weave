# ifuyo Weave
# M2 — Rename / Organizer
## 批量重命名、文件整理、预览、冲突检测、可逆操作与历史记录

你现在进入：

> **ifuyo Weave 的 M2：Rename / Organizer**

项目：

`ifuyo Weave`

M0 已建立项目基础设施与架构边界。  
M1 已建立 File Core，包括：

```text
Path Safety
Filesystem Abstraction
Metadata
Hash
File Inspector
Directory Analyzer
Streaming
Progress
Cancellation
Structured Errors
Deterministic Results
Testkit
```

M2 的任务不是继续扩展 File Core，而是第一次让 Weave：

> **安全地对真实文件执行可预览、可验证、可追踪、可撤销的变更。**

本阶段必须建立：

```text
Input
 ↓
Identify
 ↓
Build Plan
 ↓
Preview
 ↓
Collision Detection
 ↓
Confirm
 ↓
Execute
 ↓
Progress
 ↓
Result
 ↓
History
 ↓
Undo
```

这是 Weave 第一个真正意义上的：

> **Read → Plan → Write → Recover**

阶段。

---

# 0. M2 的硬边界

## 0.1 本阶段必须完成

### Rename

```text
Batch Rename
Prefix
Suffix
Replace
Regex
Sequence
Date
Counter
Case Conversion
Extension
Template
```

必须支持：

```text
Preview
Dry Run
Collision Detection
Execute
Progress
Cancellation
Result
Undo
History
```

### Organizer

支持基于：

```text
Extension
Date
Size
Pattern
Name
Folder
```

制定整理规则。

例如：

```text
Downloads/
├── image/
├── document/
├── archive/
├── video/
└── other/
```

必须支持：

```text
Preview
Collision Detection
Dry Run
Execute
Progress
Cancellation
Result
Undo
History
```

### Common Infrastructure

建立足够稳定的：

```text
OperationPlan
OperationItem
OperationResult
OperationTransaction
HistoryEntry
Undo
```

OperationResult 沿用 charter #25 七字段：success / processed / skipped / failed / warnings / outputs / duration。所有错误携带 Code / Message / Location / Recoverability / Suggestion 五元组。

让未来 M3 / M6 / M7 能够复用。

---

# 0.2 本阶段明确禁止

M2 不得实现：

```text
Duplicate Finder
Duplicate Groups
Partial Hash for Duplicate Search
Full Duplicate Scan
Recycle Bin Integration for Duplicates
Image Resize
Image Compress
Image Convert
CSV Processing
JSON Processing
Text Formatter
Text Compare
PDF Tools
Archive Tools
Media Conversion
Workflow Pipeline
AI Rename
Natural Language File Operations
Cloud
Telemetry
Account
Remote Processing
```

特别禁止提前实现：

```text
M3 Duplicate Finder
M7 Batch Engine
```

注意：

M2 可以建立：

```text
Plan
Preview
Execution
Result
Transaction
History
Undo
```

但不得借此偷偷构建一个完整的通用 Workflow / Pipeline / Batch Engine。

M2 的批处理能力只服务于：

```text
Rename
Organizer
```

通用 Batch Engine 留给 M7。

---

# 1. M2 的核心目标

M2 完成之后，用户应该能够：

```text
拖入 83 个文件
        ↓
选择 Rename
        ↓
输入规则
        ↓
立即看到 Before → After
        ↓
系统自动发现冲突
        ↓
用户修正规则
        ↓
再次 Preview
        ↓
Confirm
        ↓
真实执行
        ↓
查看成功 / 失败 / 跳过 / 取消
        ↓
History
        ↓
Undo
```

Organizer：

```text
拖入 Downloads
        ↓
选择 Organizer
        ↓
建立整理规则
        ↓
Preview
        ↓
看到：

a.jpg
→ images/a.jpg

report.pdf
→ documents/report.pdf

game.zip
→ archives/game.zip
        ↓
Collision Detection
        ↓
Confirm
        ↓
Execute
        ↓
History
        ↓
Undo
```

核心要求：

> **任何真实文件变更，都必须先能被解释、预览、验证，再执行。**

---

# 2. M2 不允许出现的设计

禁止：

```text
click Rename
→
直接改文件名
```

禁止：

```text
click Organize
→
直接移动
```

禁止：

```text
发现重名
→
自动覆盖
```

禁止：

```text
发现重名
→
偷偷加 "(1)"
```

除非该行为是：

```text
明确的用户可见命名策略
```

并且：

```text
Preview 中提前显示
```

禁止：

```text
Preview 与 Execute 使用不同规则
```

禁止：

```text
Execute 时重新推导一个与 Preview 不一致的结果
```

禁止：

```text
Undo 静默覆盖用户之后产生的新文件
```

禁止：

```text
History 只保存 UI 文本，不保存真实操作事实
```

---

# 3. Baseline Audit

开始任何修改前：

> **先检查真实代码，再决定实现方式。**

执行完整 baseline audit。

至少检查：

```text
Git status
Current branch
Current HEAD
M0 artifacts
M1 crates
M1 IPC
M1 error model
M1 filesystem abstraction
M1 path safety
M1 progress
M1 cancellation
M1 testkit
M1 frontend architecture
Current dependencies
Current tests
Current build
```

重点确认：

```text
weave-core
weave-files
weave-history
weave-testkit
```

以及实际 UI 结构。

不要假设之前的架构已经实现。

---

# 4. Baseline 输出要求

在修改前先输出：

```text
A. Current Repository
B. Existing Architecture
C. M1 Reusable Components
D. Missing M2 Infrastructure
E. Dependency Status
F. Test Baseline
G. Build Baseline
H. Risk Assessment
I. M2 Implementation Order
```

必须区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

不得把猜测写成事实。

如果 M1 有缺陷：

```text
先修最小阻塞缺陷
再进入 M2
```

不得借 M2 做大规模重构。

---

# 5. M1 回归保护

M2 是建立在 M1 之上的。

因此必须确保：

```text
File Inspector
Directory Analyzer
Hash
Path Safety
Metadata
Filesystem Abstraction
```

全部继续工作。

M2 不得为了方便：

```text
绕过 Path Safety
直接调用 std::fs
绕过 Filesystem abstraction
绕过 Structured Error
绕过 Cancellation
绕过 Progress
```

如果必须扩展 M1：

> 使用最小、向后兼容的方式扩展，并记录 DECISIONS.md。

---

# 6. M2 总体架构

保持：

```text
React UI
   ↓
Application / Command Layer
   ↓
Domain Plan / Operation
   ↓
weave-files
   ↓
Filesystem Abstraction
   ↓
OS
```

History：

```text
Operation
   ↓
Transaction
   ↓
weave-history
   ↓
Local Persistent Store
```

禁止：

```text
React
 ↓
直接 fs
```

禁止：

```text
React
 ↓
Rust 文件系统内部对象
```

IPC 必须继续使用稳定 DTO。

---

# 7. Crate 职责

## weave-core

保存跨工具稳定的数据模型和抽象：

```text
Path references
OperationId
OperationKind
OperationStatus
Plan
PlanItem
ExecutionSummary
```

不要把 Rename/Organizer 的所有业务塞进 Core。

Core：

> **定义稳定语言，而不是实现所有业务。**

---

## weave-files

负责：

```text
Rename
Move
Destination Validation
File Existence
File Metadata
Path Safety
Filesystem Operations
```

本阶段重点增加：

```text
Safe Rename
Safe Move
Collision Checks
Filesystem Preconditions
```

---

## weave-history

负责：

```text
OperationTransaction
HistoryEntry
Persistence
History Query
Undo
```

不得让 History 依赖 React。

---

## weave-testkit

扩展：

```text
Rename fixtures
Move fixtures
Collision fixtures
Unicode fixtures
Case fixtures
Long-name fixtures
Read-only fixtures
Locked fixtures
Symlink fixtures
Undo fixtures
```

---

# 8. Operation Identity

为每次真实执行建立：

```text
OperationId
```

例如：

```text
op_01...
```

OperationId 必须：

```text
唯一
稳定
可序列化
可进入 History
```

不要使用 UI index 作为操作 ID。

---

# 9. Operation Model

建立统一但足够小的模型。

推荐概念：

```text
Operation
├── id
├── kind
├── createdAt
├── status
├── plan
├── executionSummary
└── transaction
```

Operation Kind：

```text
Rename
Move
```

之后可扩展：

```text
ImageConvert
ImageResize
CSVTransform
PDFMerge
...
```

但 M2 不实现它们。

---

# 10. Plan Model

Plan 是 M2 最重要的概念之一。

Rename Preview 不应该只是字符串数组。

必须建立结构化 Plan。

例如：

```text
Plan
 ├── operationId
 ├── operationKind
 ├── createdAt
 ├── inputs
 ├── items
 ├── policy
 └── validation
```

每个 item 至少有：

```text
itemId
sourcePath
targetPath
operation
status
warnings
errors
```

可以进一步包含：

```text
sourceMetadata
targetParent
collisionState
```

要求：

> Preview 和 Execute 必须使用同一个 Plan 模型。

---

# 11. Plan 是事实快照

Preview 时：

```text
Filesystem State
      ↓
Plan
```

Execute 时：

```text
Plan
 ↓
Precondition Validation
 ↓
Execute
```

不得：

```text
Preview
 ↓
只保存规则
 ↓
Execute 时重新扫描并重新计算
```

可以重新验证当前文件状态。

但：

> **不能悄悄改变用户已经确认过的目标。**

如果文件在 Preview 和 Execute 之间发生改变：

```text
Precondition Failed
```

而不是：

```text
“智能纠正”
```

---

# 12. Rename Rule Model

Rename 必须是：

> **纯函数式规划。**

给定：

```text
input filename
rule
metadata
sequence context
```

输出：

```text
target filename
```

必须：

```text
deterministic
side-effect free
testable
```

---

# 13. Rename Features

至少实现：

## Prefix

```text
IMG_001.jpg
→
vacation-IMG_001.jpg
```

## Suffix

```text
IMG_001.jpg
→
IMG_001-final.jpg
```

## Replace

```text
draft_report.docx
→
final_report.docx
```

## Regex Replace

例如：

```text
IMG_001.jpg
IMG_002.jpg
```

规则：

```text
^IMG_
→
photo_
```

结果：

```text
photo_001.jpg
photo_002.jpg
```

Regex 输入必须：

```text
校验
错误提示
禁止执行无效规则
```

---

# 14. Sequence / Counter

支持：

```text
001
002
003
```

至少可配置：

```text
Start
Step
Width
```

例如：

```text
Start = 1
Step = 1
Width = 3
```

结果：

```text
001
002
003
```

必须明确：

> 序号分配依赖确定性排序，而不能依赖 OS 原始目录枚举顺序。

---

# 15. Date

支持基于已有可靠文件时间：

```text
Created
Modified
Accessed
```

默认优先：

```text
Modified
```

日期格式至少允许：

```text
YYYY-MM-DD
YYYYMMDD
YYYY-MM
```

具体支持的格式应通过结构化 formatter 实现，而不是在 UI 中手工拼字符串。

---

# 16. Case Conversion

至少支持：

```text
lowercase
UPPERCASE
Title Case
```

必须明确 Unicode 语义。

不要假设所有语言都可以使用 ASCII 简单大小写转换。

---

# 17. Extension Policy

必须把：

```text
Base Name
Extension
Full Name
```

分开处理。

例如：

```text
photo.final.jpg
```

默认：

```text
base = photo.final
extension = jpg
```

不得因为 Replace 操作导致：

```text
.jpg.jpg
```

除非这是用户明确要求的结果。

---

# 18. Template System

支持模板，例如：

```text
{counter}
{original}
{date}
{name}
{ext}
```

例如：

```text
vacation-{counter}.{ext}
```

结果：

```text
vacation-001.jpg
vacation-002.jpg
vacation-003.jpg
```

模板 parser 必须：

```text
明确
可测试
错误可定位
```

未知变量：

```text
Plan Error
```

不能静默输出原始文本。

---

# 19. Rename Pipeline

不要为每个选项写一套互相独立的 rename implementation。

应建立稳定 pipeline：

```text
Original
 ↓
Base/Extension Split
 ↓
Transform
 ↓
Template
 ↓
Case Conversion
 ↓
Extension Policy
 ↓
Validation
 ↓
Final Name
```

实际顺序必须明确记录。

例如：

```text
Replace
→
Trim
→
Case
→
Template
```

不要让 UI 控件出现就临时决定执行顺序。

---

# 20. Deterministic Ordering

Rename 的序号必须确定。

默认排序应明确，例如：

```text
Path ascending
```

或：

```text
Natural filename ascending
```

一旦确定：

> 所有平台使用同样的排序语义。

必须测试：

```text
a.jpg
A.jpg
a10.jpg
a2.jpg
中文.jpg
é.jpg
```

不得依赖：

```text
Windows enumeration order
Linux enumeration order
macOS enumeration order
```

---

# 21. Collision Detection

这是 M2 的核心。

Collision 必须在 Execute 前完成。

至少检测：

### Existing Target

目标已经存在：

```text
a.jpg
→
b.jpg

b.jpg 已存在
```

状态：

```text
Conflict
```

---

### Internal Collision

当前批处理中两个 source 产生同一个 target：

```text
a.jpg
→
photo.jpg

b.jpg
→
photo.jpg
```

必须发现。

---

### Identity Collision

Source 与 Target 实际是同一个对象：

```text
photo.jpg
→
photo.jpg
```

不能把这当普通错误。

应表示：

```text
NoOp
```

---

### Case Collision

重点测试：

```text
Photo.jpg
→
photo.jpg
```

在大小写不敏感文件系统上尤其重要。

必须根据实际平台语义正确处理。

---

# 22. Rename Cycle

必须处理：

```text
A → B
B → A
```

以及：

```text
A → B
B → C
C → A
```

不能简单按列表：

```text
rename A B
```

否则可能导致：

```text
collision
data loss
```

必须采用安全的 two-phase strategy。

例如：

```text
Phase 1
temporary unique names

A → temp_1
B → temp_2
C → temp_3

Phase 2

temp_1 → B
temp_2 → C
temp_3 → A
```

临时名称必须：

```text
合法
唯一
不会与用户文件冲突
不会泄漏为最终结果
```

完整过程必须进入：

```text
transaction
```

---

# 23. Case-only Rename

必须单独测试：

```text
foo.txt
→
Foo.txt
```

在 Windows 等大小写不敏感文件系统上：

> 不得假设这是一次普通 rename 就能安全完成。

必要时使用：

```text
temporary path
```

中间步骤。

Preview 必须仍然显示：

```text
foo.txt
→
Foo.txt
```

而不是暴露内部实现细节。

---

# 24. Name Validation

目标文件名必须经过 M1 Path Safety。

检查：

```text
empty
.
..
invalid Windows characters
reserved device names
trailing dot
trailing space
path separator
null
path too long
name too long
```

注意：

> Path Safety 必须继续由统一层负责。

RenameTool 不得复制另一份 Windows path validation。

---

# 25. Absolute Path Escape

Template / rule 绝不能制造：

```text
C:\...
..\...
../../...
/etc/...
```

任何用户输入都必须进入：

```text
Normalize
→
Validate
→
Resolve
```

流程。

---

# 26. Organizer

Organizer 的目标：

> **根据明确规则把选中的文件移动到确定目标。**

第一版只处理：

```text
Files
```

不要把 Organizer 偷偷扩展成目录同步器。

默认：

```text
No recursive re-processing
```

---

# 27. Organizer Conditions

支持：

```text
Extension
Date
Size
Pattern
Name
Folder
```

例如：

```text
Extension = jpg/png/webp
→ images
```

```text
Extension = pdf/docx/xlsx
→ documents
```

```text
Extension = zip/7z/rar
→ archives
```

---

# 28. Organizer Rule Model

采用：

```text
ordered rules
```

例如：

```text
Rule 1
Images
→ image/

Rule 2
Documents
→ documents/

Rule 3
Archives
→ archives/

Rule 4
Fallback
→ other/
```

默认：

> **First matching rule wins**

规则匹配顺序必须稳定。

不要让多个规则产生不可预测行为。

---

# 29. Organizer Action

第一版 action：

```text
Move
```

目标：

```text
relative destination folder
```

例如：

```text
source:
Downloads/a.jpg

destination:
Downloads/image/a.jpg
```

目标必须经过：

```text
Path Safety
```

且不能逃出用户选择的 organizer root。

---

# 30. Organizer Root

用户选择：

```text
Downloads/
```

之后规则只允许产生：

```text
Downloads/...
```

禁止默认产生：

```text
C:\
D:\
..\...
```

如果未来允许跨 root move：

> 单独设计，不在 M2 偷渡。

---

# 31. Organizer Self-processing

不要在 Execute 中：

```text
move a.jpg → image/a.jpg
↓
scanner 又发现 image/a.jpg
↓
再次处理
```

一个 Organizer operation 必须明确：

> Input Snapshot

然后只对 snapshot 中的输入执行。

---

# 32. Organizer Destination Collision

至少处理：

```text
目标不存在
目标已存在
多个输入映射到同一个目标
目标实际上就是 source
```

默认禁止静默覆盖。

状态：

```text
Ready
Conflict
NoOp
Invalid
```

---

# 33. Duplicate Names in Organizer

例如：

```text
A/a.jpg
B/a.jpg
```

同时移动到：

```text
images/
```

结果：

```text
images/a.jpg
images/a.jpg
```

必须在 Preview 阶段直接发现。

绝对不能：

```text
Execute first file
Execute second file
然后失败
```

用户必须在执行前看到：

```text
2 conflicts
```

---

# 34. Collision Policy

第一版至少支持：

```text
Fail
```

即：

> 发现冲突后，不执行该冲突项。

默认：

```text
No Overwrite
```

可以设计预留：

```text
Skip
Auto Rename
Overwrite
```

但：

> 未明确实现的策略不要显示成已支持。

尤其不要偷偷实现 Overwrite。

---

# 35. Preview

Preview 必须是真正的：

> **Execution Plan**

不是：

> “给 UI 看看的假列表”。

Rename：

```text
Before
After
Status
Reason
```

Organizer：

```text
From
To
Status
Reason
```

至少允许：

```text
All
Ready
Conflict
Invalid
NoOp
```

筛选。

---

# 36. Preview 必须无副作用

Preview 阶段不得：

```text
rename
move
delete
copy
create destination files
```

允许：

```text
create temporary application cache
```

但不得写入用户目标目录。

---

# 37. Dry Run

Dry Run：

```text
Plan
+
Validation
```

但：

```text
Execute = false
```

必须证明：

> Dry Run 不会修改用户文件。

最好通过测试：

```text
snapshot before
dry-run
snapshot after
```

验证完全一致。

---

# 38. Revalidation

用户点击 Execute 后：

```text
Plan
 ↓
Revalidate
 ↓
Execute
```

重新检查：

```text
source exists
source metadata
target collision
permissions
path safety
```

如果条件已经变化：

```text
PreconditionFailed
```

而不是自动修正规则。

---

# 39. TOCTOU

M2 需要考虑：

```text
Preview
→
Validate
→
Execute
```

之间文件状态可能变化。

因此：

> Preview/validation 不是安全性的终点。

执行每一步时仍然必须处理：

```text
NotFound
PermissionDenied
AlreadyExists
Locked
Changed
```

不要把 race condition 当成“不可能发生”。

---

# 40. Execute Semantics

一批文件：

```text
100 items
```

不要求：

```text
100 个必须全部成功
```

允许：

```text
97 Success
2 Failed
1 Skipped
```

结果必须完整记录。

---

# 41. 不允许全局假原子性

M2 默认不承诺：

```text
all or nothing
```

因为真实文件系统操作可能发生：

```text
permission failure
disk full
external modification
locked file
process interruption
```

因此模型必须允许：

```text
PartialSuccess
```

但是：

> 在 rename cycle / multi-phase rename 中，内部阶段必须维护事务一致性，尽可能避免留下临时名称。

---

# 42. Failure Result

每项至少有：

```text
Success
Failed
Skipped
Cancelled
NoOp
```

每项失败必须有：

```text
structured error code
human-readable safe message
```

禁止只返回：

```text
"Something went wrong"
```

---

# 43. Error Model

至少考虑：

```text
SourceNotFound
TargetAlreadyExists
PermissionDenied
PathInvalid
PathTooLong
NameInvalid
ReadOnly
FileLocked
FileChanged
UnsupportedOperation
Cancelled
FilesystemError
TransactionRecoveryRequired
```

最终命名可以根据已有 M1 error model 调整。

要求：

> 不重复建立第二套完全不同的错误体系。

---

# 44. Cancellation

用户可以：

```text
Cancel
```

取消必须是 cooperative。

不能：

```text
强制 kill worker
```

导致：

```text
temporary rename
```

遗留不可恢复。

---

# 45. Cancellation Semantics

取消状态必须明确：

```text
NotStarted
Running
Cancelling
Cancelled
Completed
CompletedWithWarnings
```

Pause / Resume 属 M7 Batch Engine 范围，本里程碑不实现；状态机设计不得阻止 M7 追加 Paused 状态。

如果：

```text
40/100
```

时取消：

结果要明确：

```text
38 Success
2 In-flight / resolved state
60 NotStarted
```

最终系统必须知道：

> 哪些真实发生过改变。

---

# 46. Transaction

建立：

```text
OperationTransaction
```

至少保存：

```text
OperationId
OperationType
Timestamp
Items
OriginalPath
NewPath
OriginalMetadata
ExecutionState
Reversibility
```

对于 Move：

```text
Source
Destination
```

对于 Rename：

```text
OriginalPath
NewPath
```

---

# 47. Transaction Item

每项至少记录：

```text
sourcePath
targetPath
status
timestamp
```

必要时：

```text
originalSize
originalModifiedTime
originalCreatedTime
```

用于 Undo 前检查。

---

# 48. Undo Principle

Undo 不是：

```text
再次执行“rename from target to source”
```

这么简单。

Undo 必须首先：

```text
load transaction
↓
validate current state
↓
check target
↓
ensure original destination is safe
↓
perform reverse operation
```

---

# 49. Undo Safety

例如：

第一次：

```text
a.jpg
→
b.jpg
```

之后用户手工把：

```text
b.jpg
```

替换成了另一文件。

此时点击 Undo：

> **绝对不能直接覆盖。**

必须：

```text
UndoConflict
```

告诉用户：

```text
Target has changed since this operation.
```

然后停止该 item 的自动恢复。

---

# 50. Partial Undo

如果一次操作：

```text
100 files
```

成功：

```text
97
```

失败：

```text
3
```

Undo 应该允许：

```text
reverse successful items
```

但不能对：

```text
never-executed items
```

制造假恢复记录。

多 item undo 必须按事务内应用顺序的逆序（LIFO）执行；cycle rename 的 undo 依赖两阶段重命名（temp 路径）的逆过程。

---

# 51. Undo of Organizer

例如：

```text
Downloads/a.jpg
→
Downloads/images/a.jpg
```

Undo：

```text
Downloads/images/a.jpg
→
Downloads/a.jpg
```

前提：

```text
destination still matches original operation state
source destination is safe
```

如果：

```text
Downloads/a.jpg
```

已经出现另一个文件：

> 不得覆盖。

---

# 52. History

History 必须是真实持久化数据。

至少保存：

```text
OperationId
Type
Timestamp
Summary
ItemCount
SuccessCount
FailedCount
Undoable
Status
```

可以进一步保存：

```text
InputRoot
RuleSummary
```

但：

> 不要把完整文件内容写入 History。

---

# 53. History Privacy

History 中默认可以保存：

```text
paths
filenames
operation metadata
```

但不要记录：

```text
file contents
```

不要把：

```text
CSV contents
document text
image bytes
```

写入日志或 History。

---

# 54. History Persistence

优先复用已有 M0/M1 本地持久化基础设施。

如果没有足够能力：

> 选择最简单、最可靠、无需大型数据库的持久化形式。

要求：

```text
atomic write
versioned schema
corruption detection
recovery behavior
```

不要为了 History 引入大型 ORM。

---

# 55. History Corruption

测试：

```text
empty file
partial write
invalid JSON
old schema
unknown schema version
truncated record
```

程序不能因为 History 损坏：

```text
整个 Weave 无法启动
```

必须具备：

```text
safe degradation
```

并在 UI 中明确提示。

---

# 56. History Query

第一版支持：

```text
Recent Operations
```

至少：

```text
latest first
```

每条显示：

```text
Rename 83 files
Organizer 42 files
```

以及：

```text
time
status
undo available
```

---

# 57. Reuse

M2 可以为未来的：

```text
Reuse
Repeat
```

留下数据模型。

但不要实现一个完整 Workflow Builder。

至少保存：

```text
operation type
rule configuration
```

Organizer 规则配置必须可序列化并带 schema 版本号（为 M10 Workflow 复用预留）。

这样历史可以作为：

> 再次执行同一规则的来源。

但执行时仍然重新生成新的：

```text
OperationId
Plan
Transaction
```

---

# 58. History 不等于日志

严格区分：

```text
Application Log
```

与：

```text
Operation History
```

Log：

```text
debug
warning
error
```

History：

```text
user-visible operations
undo
reuse
```

二者不能混在一起。

---

# 59. UI Information Architecture

至少提供：

```text
Rename
Organizer
Preview
History
```

建议结构：

```text
Home
 ├── Quick Drop
 ├── Rename
 ├── Organizer
 └── Recent Operations
```

M2 不需要完成最终首页全部智能推荐能力。

---

# 60. Rename UI

至少：

```text
Input Files
Rule Builder
Preview
Execute
Result
```

Rule Builder 支持：

```text
Prefix
Suffix
Replace
Regex
Counter
Date
Case
Template
Extension
```

必须避免一次性堆满复杂选项。

可以采用：

```text
Rule blocks
```

但最终 Plan 必须是结构化数据。

---

# 61. Organizer UI

至少：

```text
Root
Rules
Preview
Execute
Result
```

规则：

```text
WHEN
condition

THEN
move to
```

例如：

```text
WHEN extension in [jpg,png,webp]
THEN image/
```

---

# 62. Preview UI

必须突出：

```text
Before → After
```

而不是：

```text
Raw JSON
```

例如：

```text
IMG_001.jpg
→
vacation-001.jpg

IMG_002.jpg
→
vacation-002.jpg
```

状态需要清晰：

```text
Ready
Conflict
Invalid
NoOp
Failed
```

---

# 63. Conflict UI

不能只显示：

```text
3 errors
```

必须告诉用户：

```text
old.jpg
→
images/old.jpg

Conflict:
target already exists
```

对于 internal collision：

```text
a.jpg
→
photo.jpg

b.jpg
→
photo.jpg

Conflict:
multiple items target the same path
```

---

# 64. Execute Confirmation

对于真实文件变更：

用户必须明确确认。

按钮不要写：

```text
OK
```

优先：

```text
Rename 83 Files
Move 42 Files
```

让用户知道：

> 即将发生什么。

---

# 65. Destructive UX

M2 的 Rename/Move 通常不是删除级破坏操作，但仍属于真实文件修改。

因此：

```text
Preview required
Explicit Execute
Result visible
Undo visible when possible
```

---

# 66. Progress

复用 M1 progress。

显示：

```text
Processed
Remaining
Success
Failed
Skipped
```

避免：

```text
只显示 Spinner
```

---

# 67. Progress Accuracy

如果能够确定：

```text
total items
```

则显示：

```text
42 / 100
```

不要使用：

```text
假的时间估计
```

如果无法准确估计：

> 显示确定的 item progress，不要伪造百分比。

---

# 68. Result Screen

完成后至少显示：

```text
Completed
Success
Failed
Skipped
Cancelled
```

支持：

```text
View Failed
View Conflicts
Undo
Back
```

不要只显示：

```text
Done
```

---

# 69. Batch Result Export

第一版不是重点。

可以内部保留结构化：

```text
ExecutionResult
```

但不要急着做：

```text
CSV result export
JSON export UI
```

除非现有架构已经具备且成本很低。

---

# 69a. UI 纪律

所有新增 UI 文案必须进入 i18n 资源文件（zh-CN / en），禁止散落硬编码字符串（charter #34）。

Preview / Conflict / Result 界面必须满足 keyboard navigation、focus visible、语义标签（charter #35 基线；完整 a11y 验收在 M11）。

---

# 70. File System Semantics

Rename 与 Move 必须尽量使用：

```text
filesystem-native operation
```

避免：

```text
copy
delete original
```

来模拟 rename。

除非跨文件系统 move 被明确支持。

---

# 71. Cross Filesystem Move

Organizer M2 默认：

> 只保证同文件系统内 Move 的安全语义。

如果实际实现跨文件系统 Move：

必须先研究：

```text
copy
flush
verify
remove
rollback
```

不要把：

```text
rename()
failed
→
copy+delete
```

随手补上。

若没有充分可靠的事务模型：

> 明确拒绝跨文件系统 Move，而不是假装支持。

---

# 72. Symlink / Junction

严格复用 M1 的明确策略。

对于：

```text
symlink
junction
reparse point
```

不要擅自跟随。

必须明确：

```text
rename link itself
```

还是：

```text
skip
```

如果 M1 尚未定义：

> 先补齐 M1 policy，再继续 M2。

不要让 Rename / Organizer 各自定义一套规则。

---

# 73. Read-only / Permission

执行前可以检测。

但不能假设：

```text
metadata says writable
→
execute guaranteed
```

真正执行时仍需处理：

```text
PermissionDenied
ReadOnly
Locked
```

---

# 74. File Mutation Race Tests

必须测试：

```text
Preview
→
external rename
→
Execute
```

以及：

```text
Preview
→
external delete
→
Execute
```

以及：

```text
Preview
→
target created externally
→
Execute
```

结果必须是：

```text
safe failure
```

不能覆盖外部变化。

---

# 75. Test Matrix — Rename

至少覆盖：

```text
single file
multiple files
empty selection
Unicode names
Chinese names
spaces
multiple dots
hidden files
extensionless files
case-only rename
existing collision
internal collision
no-op
regex
invalid regex
counter
date
template
invalid template
reserved names
invalid characters
path too long
locked file
permission denied
missing file
changed file
cancelled operation
```

---

# 76. Test Matrix — Rename Cycles

必须覆盖：

```text
A → B
B → A
```

```text
A → B
B → C
C → A
```

以及：

```text
case-only cycle
```

验证：

```text
no data loss
no duplicate target
no temp file leakage
transaction consistent
undo works
```

---

# 77. Test Matrix — Organizer

至少：

```text
extension match
date match
size match
name match
pattern match
folder match
first-match-wins
fallback
multiple inputs same target
existing target
source=target
destination outside root
destination creation
permission denied
locked source
source missing
cancel
partial success
```

---

# 78. Test Matrix — Undo

至少：

```text
simple rename undo
multi rename undo
cycle rename undo
simple move undo
multi move undo
partial success undo
target externally modified
original destination occupied
missing target
history restart
corrupt history
old history schema
```

---

# 79. Property / Invariant Tests

建立关键不变量：

### Preview Purity

```text
Preview(before filesystem)
=
filesystem unchanged
```

### Determinism

```text
same input
+
same rule
=
same plan
```

### Collision Safety

```text
conflict plan
→
cannot execute silently
```

### Undo Safety

```text
external change
→
undo refuses unsafe overwrite
```

### No Overwrite

默认策略：

```text
existing target
→
never silently overwrite
```

### Transaction Accuracy

```text
History
must describe actual mutation,
not intended mutation.
```

---

# 80. Snapshot Tests

对 Plan 建立 golden tests。

例如：

```text
fixture:
IMG_001.jpg
IMG_002.jpg
IMG_003.jpg

rule:
vacation-{counter}.{ext}
```

golden：

```text
IMG_001.jpg → vacation-001.jpg
IMG_002.jpg → vacation-002.jpg
IMG_003.jpg → vacation-003.jpg
```

不要只测试最终字符串。

应测试：

```text
Plan structure
statuses
collision state
```

---

# 81. Fault Injection

至少覆盖：

```text
PermissionDenied
TargetAlreadyExists
SourceMissing
FileLocked
PartialRename
InterruptedExecution
Cancellation
DestinationCreationFailure
HistoryWriteFailure
HistoryCorruption
```

重点：

> 即使部分执行成功，History 也必须正确记录真实状态。

---

# 82. Transaction Recovery

如果多阶段 Rename：

```text
temporary phase
+
final phase
```

中途崩溃：

必须考虑：

```text
temporary names remain
```

至少建立：

```text
recovery strategy
```

例如：

```text
operation transaction marked incomplete
```

启动时能够：

```text
detect
report
recover safely
```

不要假装不存在 crash recovery 问题。

---

# 83. Crash Safety

不要求 M2 做完整 filesystem transaction。

但要求：

> 不因为程序崩溃而静默丢失 Operation State。

尤其是：

```text
cycle rename
```

必须写入足够 transaction state。

---

# 84. History Write Failure

如果：

```text
file mutation succeeded
```

但：

```text
history persistence failed
```

不能返回：

```text
全部失败
```

应该明确：

```text
Operation completed
History persistence failed
Undo may be unavailable
```

这样用户不会得到错误事实。

---

# 85. Logging

日志中可以记录：

```text
OperationId
OperationKind
counts
error codes
duration
```

不要记录：

```text
file contents
```

除非路径本身属于必要诊断信息。

路径日志应遵守现有隐私策略。

---

# 86. Performance

Rename 1000 个文件：

不要：

```text
每个 item 启动一个独立线程
```

也不要：

```text
一次性构建巨大 UI state
```

要求：

```text
streamed / incremental processing where practical
bounded memory
cancellable
```

---

# 87. Large Batch

至少测试：

```text
100
1,000
10,000
```

文件。

关注：

```text
Plan memory
Preview render
Execute throughput
History size
UI responsiveness
```

10k 文件时：

> 不允许 UI 卡死到无法取消。

---

# 88. History Size

不要把：

```text
每个 filesystem byte
```

写入 History。

只存：

```text
operation metadata
operation items
```

必要元数据保持轻量。

---

# 89. React State

避免：

```text
10,000 rows
→
10,000 independent heavy components
```

根据实际 UI 需求考虑：

```text
virtualized list
```

但不要为“可能需要”而无意义增加依赖。

如果 10k 数据量下现有简单列表已经可接受：

> 用最简单实现，并用实际测试证明。

---

# 90. IPC

前后端至少定义：

```text
build_rename_plan
build_organizer_plan
validate_plan
execute_plan
cancel_operation
get_history
get_operation
undo_operation
```

具体命名根据 M0/M1 现有 IPC 风格调整。

不要让一个：

```text
execute_everything
```

命令承担所有逻辑。

---

# 91. IPC DTO

DTO 必须：

```text
stable
serializable
explicit
version-conscious
```

不要直接暴露 Rust 内部类型。

Preview response 至少能够表示：

```text
plan
items
validation
summary
```

---

# 92. Frontend Command Layer

前端应：

```text
commands/
```

统一管理：

```text
invoke(...)
```

组件不应到处直接调用 Tauri IPC。

---

# 93. Application Orchestration

Command handler 只负责：

```text
parse DTO
load inputs
call application service
map result
return DTO
```

禁止：

```text
几百行 rename algorithm
```

塞进 Tauri command。

---

# 94. Dependency Policy

如需新增依赖：

先检查：

```text
Name
Version
License
Repository
Maintenance
Transitive Dependencies
Bundle Size / Runtime Cost
```

然后记录：

```text
Dependency Decision
```

不要：

```text
npm install xxx
cargo add xxx
```

看到问题就直接加。

---

# 95. Architecture Rules

最终检查：

```text
UI
 ↓
Application
 ↓
Domain
 ↓
Adapters
 ↓
Filesystem
```

禁止：

```text
React
→
std::fs
```

禁止：

```text
UI
→
History storage implementation
```

禁止：

```text
Rename
→
Duplicate Finder
```

禁止：

```text
Organizer
→
global batch engine
```

---

# 96. No God Object

禁止出现：

```text
RenameManager
```

里面同时：

```text
parse rules
scan files
validate path
detect collision
execute
persist history
undo
update UI
```

应该拆成明确职责：

```text
Rule Engine
Plan Builder
Collision Validator
Executor
Transaction Recorder
History Store
Undo Service
```

但也不要为了“架构漂亮”制造几十个微小类。

原则：

> **职责明确 + 文件数量克制。**

---

# 97. No Duplicate Validation

尤其不要在：

```text
RenameTool
OrganizerTool
HistoryTool
UI
```

分别实现：

```text
Path Validation
```

统一复用 M1。

---

# 98. No Hidden Behavior

用户选择：

```text
No Overwrite
```

系统不能暗中：

```text
rename b.jpg to b (1).jpg
```

用户选择：

```text
Preview
```

系统不能：

```text
touch target file
```

用户选择：

```text
Cancel
```

系统不能继续偷偷执行剩余项目。

---

# 99. Real UI Smoke Test

必须有真实桌面环境验证。

至少验证：

```text
Drag files
→
Rename
→
Preview
→
Conflict
→
Fix rule
→
Preview
→
Execute
→
Result
→
History
→
Undo
```

Organizer 同样：

```text
Drag folder
→
Organizer
→
Rules
→
Preview
→
Execute
→
Undo
```

不得只依赖：

```text
unit tests
```

---

# 100. Real File Integration Test

必须在临时真实目录：

```text
temp workspace
```

中完成：

```text
create fixtures
→
build plan
→
execute
→
inspect filesystem
→
load history
→
undo
→
inspect filesystem again
```

最终验证：

```text
filesystem state returns to expected state
```

---

# 101. Cross-platform Verification

至少考虑：

```text
Windows
Linux
```

如果当前开发环境只能真实验证 Windows：

必须明确：

```text
FACT:
tested on Windows
```

以及：

```text
NOT YET VERIFIED:
Linux/macOS
```

不得写：

```text
cross-platform fully verified
```

除非真的测试。

---

# 102. Documentation

M2 完成后更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
```

必要时增加：

```text
docs/operations.md
docs/rename-rules.md
docs/history.md
```

至少明确说明：

```text
Preview semantics
Collision semantics
Overwrite policy
Undo semantics
History semantics
Failure semantics
Cancellation semantics
Symlink semantics
Cross-filesystem move limitations
```

---

# 103. ADR / Decisions

至少记录：

```text
Why Plan is first-class
Why Preview and Execute share one Plan
Why overwrite is forbidden by default
How rename cycles are solved
How case-only rename is solved
How Organizer rules are ordered
How Undo detects unsafe external changes
How History is persisted
How partial failure is represented
What cross-filesystem Move does
```

---

# 104. Security Review

重点检查：

```text
Path traversal
Absolute path escape
Reserved names
Invalid path separators
Symlink/junction escape
Destination outside root
Accidental overwrite
Unexpected deletion
Shell invocation
Arbitrary executable launch
```

本阶段：

> **禁止通过 shell 完成 Rename / Move。**

不要：

```text
cmd.exe
powershell
bash
mv
ren
```

拼接用户路径执行。

优先使用 Rust filesystem APIs。

---

# 105. Privacy Review

确认：

```text
No telemetry
No upload
No cloud dependency
No remote processing
```

确认：

```text
file contents stay local
```

History / logs：

```text
do not store file contents
```

---

# 106. Git Hygiene

修改前：

```text
check status
```

过程中：

```text
do not commit build artifacts
do not commit databases
do not commit temp files
do not commit secrets
```

检查：

```text
.gitignore
```

不要删除已有正确 ignore。

---

# 107. Commit Strategy

推荐：

```text
commit 1
M2 infrastructure / operation model

commit 2
Rename planning + preview

commit 3
Rename execution + collision handling

commit 4
Organizer

commit 5
History + Undo

commit 6
M2 tests / docs / release hygiene
```

实际可以根据修改规模合并。

每个 commit 之前，全部质量门必须通过；任何一门未绿不得提交。

原则：

> 每个 commit 都应该是可理解的逻辑单元。

---

# 108. Regression Gate

每个主要阶段后执行：

```text
cargo fmt --check
cargo clippy
cargo test
frontend typecheck
frontend lint
frontend test
```

以下各项为无条件必过项：

```text
integration tests
build
license audit
```

也必须继续通过。

---

# 109. Build Gate

最终必须实际执行：

```text
Tauri build
```

以项目当前真实平台配置为准。

不允许：

> “源码看起来可以编译。”

必须：

> **真实 build 成功。**

---

# 110. Test Gate

M2 完成不得使用：

```text
tests skipped
tests ignored
TODO
fake assertions
```

不得为了让测试通过：

```text
删除失败测试
降低断言
关闭功能
mock 掉核心 filesystem behavior
```

Mock 可以用于：

```text
fault injection
```

但真实 filesystem integration test 必须存在。

---

# 111. M2 Acceptance Criteria

M2 必须至少满足：

```text
Batch Rename
Prefix
Suffix
Replace
Regex
Counter
Date
Case Conversion
Template
Extension handling

Preview
Dry Run
Collision Detection
Internal Collision Detection
Existing Target Detection
Case Collision Detection
NoOp Detection

Rename Cycle Safety
Case-only Rename Safety

Organizer
Extension
Date
Size
Pattern
Name
Folder
First-match-wins
Fallback

Safe Move
Root Boundary
Path Safety
No Silent Overwrite

Progress
Cancellation
Partial Failure
Structured Result

OperationId
OperationPlan
OperationTransaction

History
Persistent History
History Query
History Integrity

Undo
Rename Undo
Move Undo
Partial Undo
Unsafe Undo Detection

Fault Injection
Integration Tests
Real File Tests
Large Batch Tests
Unicode Tests

M1 Regression
Rust Gates
Frontend Gates
Build

Documentation
Decisions
Progress
Privacy

Git Hygiene
```

---

# 112. M2 不算完成的情况

以下任何一项存在，都不能宣布：

```text
M2 Complete
```

例如：

```text
Rename 可以工作
但 Preview 是 mock

Preview 可以工作
但 Execute 没有 revalidation

Collision 发现了
但会静默覆盖

Undo 存在
但会覆盖用户后来创建的文件

History 存在
但重启后丢失

Organizer 可以移动
但能逃出 root

Rename cycle 在普通顺序下失败

10k 文件时 UI 完全冻结

取消后仍然继续修改文件

部分失败后 History 与实际文件状态不一致

M1 regression
```

---

# 113. 最终审计

M2 实现完成后，不要直接宣布完成。

先执行完整最终审计：

## A. Baseline

```text
Git
Branch
HEAD
Workspace
```

## B. Architecture

```text
Layer boundaries
Crate boundaries
IPC boundaries
```

## C. Rename

```text
Rules
Plan
Preview
Execute
Cycles
Case-only
Collision
```

## D. Organizer

```text
Rules
Root safety
Destination
Collision
Move semantics
```

## E. Transaction

```text
Actual mutation tracking
Partial execution
Cancellation
Crash safety
```

## F. Undo

```text
Simple
Partial
Conflict
External mutation
```

## G. History

```text
Persistence
Versioning
Corruption
Restart
```

## H. Performance

```text
1k
10k
UI responsiveness
memory
```

## I. Security

```text
Traversal
Overwrite
Symlink
Shell
Root escape
```

## J. Privacy

```text
No upload
No telemetry
No contents in logs/history
```

## K. Tests

```text
Unit
Integration
Fault
Property
Regression
```

## L. Build

```text
Rust
Frontend
Tauri
```

## M. Git

```text
Clean
Tracked files
Artifacts
Secrets
```

## N. Documentation

```text
README
ARCHITECTURE
DECISIONS
PROGRESS
PRIVACY
```

## O. Known Limitations

明确列出：

```text
Not Implemented
Not Verified
Deferred
Platform-specific limitations
```

绝对不要隐藏限制。

## P. M3 Handoff

必须说明 M2 已向 M3 提供：

```text
Filesystem abstraction
Path safety
Metadata
Hash
OperationId
Plan
Preview
Collision infrastructure
Progress
Cancellation
Transaction
History
Undo
```

并明确：

> **M3 只新增 Duplicate Finder，不重复造这些基础设施。**

---

# 114. PROGRESS.md

更新：

```text
M0
M1
M2
M3
```

其中 M2 必须写：

```text
Implemented
Tested
Verified
Not Verified
Known Limitations
```

不要只写：

```text
M2 Done
```

---

# 115. Final Product State

M2 完成后，Weave 应达到：

```text
File Core
     ↓
Rename
     ↓
Preview
     ↓
Collision Detection
     ↓
Execute
     ↓
Result
     ↓
History
     ↓
Undo
```

以及：

```text
File Core
     ↓
Organizer
     ↓
Preview
     ↓
Collision Detection
     ↓
Execute
     ↓
Result
     ↓
History
     ↓
Undo
```

也就是说：

> Weave 第一次形成了真正的“安全文件操作闭环”。

---

# 116. M2 的最终原则

请始终遵守：

```text
Plan before mutation
Preview before execution
Validate before write
Never silently overwrite
Record actual mutations
Undo must be safer than redo
History must describe facts
Cancellation must be real
Partial failure must be explicit
Path Safety remains centralized
```

最重要的一条：

> **宁可拒绝执行，也不要在不确定状态下替用户修改真实文件。**

---

# 117. 执行纪律

你现在开始实施 M2。

严格按照：

```text
1. Baseline Audit
2. M1 Regression Verification
3. Minimal Blocking Fixes
4. Operation Model
5. Rename Planning
6. Rename Preview
7. Collision Detection
8. Rename Execution
9. Organizer Planning
10. Organizer Preview
11. Organizer Execution
12. Transaction
13. History
14. Undo
15. Fault Tests
16. Integration Tests
17. Performance Tests
18. Real UI Smoke
19. Documentation
20. Build
21. Final Audit
```

执行。

每一个阶段：

```text
先检查
→
实现
→
测试
→
审计
→
再进入下一阶段
```

不要一次性重写整个项目。

不要为了完成 M2：

```text
重构整个 architecture
更换技术栈
增加大型依赖
建立完整 Batch Engine
提前实现 M3
```

---

# 118. 完成标准

只有当真实代码、真实测试、真实 UI、真实文件系统操作、真实 build 均满足上述要求时：

```text
M2 = COMPLETE
```

否则必须明确：

```text
PARTIAL
BLOCKED
KNOWN LIMITATION
```

禁止假完成。

---

# 最终目标

把 ifuyo Weave 从：

```text
“能够查看文件”
```

推进到：

```text
“能够安全地理解文件，
制定文件变更计划，
让我先看到结果，
在确认后执行，
记录真实变化，
并在安全条件满足时撤销操作。”
```

这才是 M2 真正应该交付的能力。