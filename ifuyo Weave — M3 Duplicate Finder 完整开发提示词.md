# ifuyo Weave
# M3 — Duplicate Finder
## 精确重复文件检测、渐进式 Hash、重复组、预览、回收站、可逆操作与大规模扫描

你现在进入：

> **ifuyo Weave 的 M3：Duplicate Finder**

项目：

`ifuyo Weave`

当前基线：

```text
M0 = Foundation
M1 = File Core
M2 = Rename / Organizer
M3 = Duplicate Finder
```

M0 建立：

```text
Tauri 2
Rust
React
TypeScript
Vite
Workspace
IPC
Error Model
Logging
Testkit
```

M1 建立：

```text
Filesystem Abstraction
Path Safety
File Metadata
Directory Metadata
Hashing
File Facts
Directory Facts
Streaming
Progress
Cancellation
Structured Errors
Deterministic Results
Resource Limits
File Inspector
Directory Analyzer
```

M2 建立：

```text
OperationId
OperationPlan
OperationItem
Preview
Validation
Collision Detection
Execute
Progress
Cancellation
OperationResult
OperationTransaction
History
Undo
Organizer
Batch Rename
```

OperationResult 与 Error 沿用 charter #25：七字段 success / processed / skipped / failed / warnings / outputs / duration；错误五元组 Code / Message / Location / Recoverability / Suggestion。

M3 的目标不是继续堆积基础设施。

M3 的目标是让 Weave 第一次具备一个真正可靠的：

> **“扫描大量文件 → 逐级筛选 → 找出精确重复文件 → 按组审查 → 明确选择 → 预览处理 → 移入回收站 → History → Undo”**

能力。

核心流程：

```text
Select Root(s)
      ↓
Scan
      ↓
Collect File Facts
      ↓
Size Filter
      ↓
Partial Hash
      ↓
Full Hash
      ↓
Duplicate Groups
      ↓
Review
      ↓
Select
      ↓
Preview
      ↓
Validate
      ↓
Confirm
      ↓
Recycle
      ↓
Progress
      ↓
Result
      ↓
History
      ↓
Undo
```

本阶段最重要的产品原则：

> **Duplicate Finder 可以非常强大，但绝不能替用户决定哪些文件应该被删除。**

---

# 0. M3 硬边界

## 0.1 本阶段必须完成

### Duplicate Detection

```text
Recursive File Scan
Root Selection
File Candidate Collection
Size Filter
Partial Hash
Full Hash
Duplicate Grouping
Deterministic Group Ordering
Duplicate Statistics
```

### User Interaction

```text
Scan
Progress
Cancellation
Results
Duplicate Groups
Group Review
File Selection
Selection Summary
Preview
```

### Mutation

```text
Recycle Selected Duplicates
Safe Delete Semantics
Transaction Tracking
History
Undo
```

### Infrastructure Reuse

必须优先复用：

```text
M1 Filesystem Abstraction
M1 Path Safety
M1 Metadata
M1 Streaming Hash
M1 Cancellation
M1 Progress
M1 Structured Errors
M2 OperationId
M2 Plan
M2 Preview
M2 Transaction
M2 History
M2 Undo
```

---

# 0.2 本阶段明确禁止

M3 不得实现：

```text
Near Duplicate
Perceptual Hash
Image Similarity
Video Similarity
Audio Similarity
Semantic Similarity
AI Duplicate Detection
Content Embedding
Vector Search
Fuzzy Matching
Machine Learning
OCR
Visual Similarity
```

也不得提前实现：

```text
M4 Text
M5 Data
M6 Image
M7 Batch Engine
M8 Documents
M9 Utilities
M10 Workflow
```

尤其禁止把 M3 偷偷扩展成：

```text
Generic File Cleaner
Storage Optimizer
Disk Cleaner
System Cleaner
Smart Deletion Engine
```

M3 只是：

> **Exact Duplicate Finder**

---

# 1. M3 核心产品定义

第一版只认：

```text
Exact Duplicate
```

不认：

```text
Similar
Looks Similar
Probably Same
Same Name
Same Extension
Same Folder
```

以下情况不能单独构成重复：

```text
same filename
same size
same timestamp
same extension
same directory
```

核心判定流程：

```text
size
 ↓
partial hash
 ↓
full hash
 ↓
duplicate group
```

必须明确：

```text
size
```

是最便宜的第一层筛选。

```text
partial hash
```

是第二层候选缩减。

```text
full hash
```

才进入最终重复分组。

---

# 2. Exact Duplicate 的定义

M3 默认将两个文件视为 Exact Duplicate，当：

```text
size 相同
AND
full SHA-256 相同
```

其中：

```text
SHA-256
```

必须复用 M1 已有的 streaming hash implementation。

不得：

```text
重新实现另一套 SHA-256
```

也不得：

```text
使用 filename
```

作为重复判断依据。

也不得：

```text
使用 modified time
```

作为重复判断依据。

必须在文档中明确：

> Exact duplicate 的工程定义基于文件大小与 SHA-256 完整哈希匹配；这是一种基于密码学哈希的工程判定，仍应避免把它描述成数学意义上的绝对证明。

不要在 UI 上制造不必要的理论恐吓。

正常用户体验应保持简单：

```text
Exact Duplicate
```

---

# 3. Partial Hash 的定位

Partial Hash 是：

> **性能优化层，而不是最终事实层。**

它用于：

```text
size 相同
        ↓
只比较文件有限区域
        ↓
快速淘汰明显不同的文件
        ↓
少量候选进入 Full Hash
```

不得把：

```text
partial hash 相同
```

直接视为：

```text
duplicate
```

必须继续：

```text
full hash
```

---

# 4. Partial Hash 算法要求

不要在没有理由的情况下设计复杂算法。

优先：

```text
固定、稳定、文档化、可测试
```

的 partial hashing 策略。

例如根据实际 M1 streaming abstraction 设计：

```text
first chunk
+
last chunk
```

或项目现有 hash abstraction 支持的其他有限区域策略。

实际选择必须满足：

```text
Deterministic
Bounded I/O
Bounded Memory
Cross-platform
Easy to Test
```

不要依赖：

```text
random sampling
```

否则相同文件在不同运行中可能产生不一致行为。

如果采用：

```text
first N bytes
last N bytes
```

等策略：

必须把 N：

```text
集中定义
```

而不是：

```text
散落在算法代码
```

同时记录：

```text
DECISIONS.md
```

说明：

```text
选择该策略的原因
I/O 成本
限制
为什么不能把 partial hash 作为最终判断
```

验收硬化：chunk 读取策略（读哪些字节、N 的取值）必须是集中定义的单一常量，取值理由记入 DECISIONS.md；golden fixture 必须覆盖 first-chunk-differs 与 last-chunk-differs 两类用例。

---

# 5. 不允许“永远 Full Hash 所有文件”

以下实现不接受：

```text
扫描 100,000 个文件
→ 全部直接读取完整文件
→ 全部 SHA-256
```

除非经过真实 benchmark 证明当前数据规模下这样做没有实际问题，而且仍然符合产品设计。

默认必须：

```text
metadata scan
 ↓
group by size
 ↓
only duplicate-size candidates
 ↓
partial hash
 ↓
only partial-hash candidates
 ↓
full hash
```

---

# 6. Size Filter

第一层：

```text
size
```

如果一个目录中：

```text
A = 10 MB
B = 20 MB
C = 30 MB
D = 10 MB
```

只有：

```text
A
D
```

进入下一阶段。

一个文件大小在扫描根中只有一个实例时：

> 没有必要计算它的 duplicate hash。

这是 M3 最重要的性能基础之一。

---

# 7. Zero-byte Files

必须专门处理：

```text
0-byte files
```

例如：

```text
a.txt = 0 bytes
b.txt = 0 bytes
c.tmp = 0 bytes
```

它们：

```text
size 相同
```

且：

```text
完整内容为空
```

因此属于同一 Exact Duplicate 组。

不得因为：

```text
0 bytes
```

而出现错误。

也不要为了 0-byte 文件制造不必要的磁盘读取。

可以通过合理的：

```text
empty-content semantics
```

直接进入稳定分组。

---

# 8. File Type 不参与 Exact Duplicate 判定

例如：

```text
a.bin
b.jpg
```

只要：

```text
size 相同
full SHA-256 相同
```

就可以属于同一 Exact Duplicate Group。

因为：

> Duplicate Finder 判断的是文件内容，而不是扩展名。

不能：

```text
extension mismatch
→ automatically not duplicate
```

但 UI 可以展示：

```text
Different Extensions
```

作为事实信息。

---

# 9. Path 与 Scope

Duplicate Finder 必须明确：

```text
Scan Root
```

和：

```text
Scope
```

例如：

```text
D:/Downloads
```

扫描范围只能由用户明确选择。

不得偷偷：

```text
scan entire C:
```

不得扫描：

```text
系统目录
```

除非用户明确选择且 M1 Path Safety 与平台权限允许。

默认：

```text
Selected Root
+
configured recursion
```

---

# 10. Recursive Scan

默认可以递归扫描：

```text
selected directory
 ↓
subdirectories
 ↓
files
```

但必须：

```text
reuse M1 symlink policy
```

不能在 M3 重新定义：

```text
symlink behavior
```

如果 M1 明确规定：

```text
do not follow directory symlinks
```

M3 就必须一致遵守。

如果 M1 存在平台差异：

必须：

```text
reuse abstraction
```

而不是：

```text
DuplicateFinder 自己判断 Windows / Unix
```

---

# 11. Multiple Roots

如果当前 M1/M2 架构已经支持多输入：

M3 应支持：

```text
Root A
Root B
Root C
```

并允许跨根发现重复。

例如：

```text
Pictures/a.jpg
Backup/a.jpg
Downloads/a.jpg
```

三个文件可以进入：

```text
Duplicate Group
```

但：

> 如果当前项目的输入模型尚未支持多 root，则不要为了 M3 强行扩展成复杂 workspace 输入系统；优先采用现有 abstraction，记录限制。

---

# 12. Duplicate Group

Duplicate Group 至少包含：

```text
GroupId
FileCount
TotalSize
WastedSize
Files
```

例如：

```text
Group #17

3 files
12 MB each
36 MB total
24 MB reclaimable
```

其中：

```text
WastedSize
```

建议定义为：

```text
(FileCount - 1) × FileSize
```

但必须在文档中明确：

> 这是理论可回收空间，不代表用户实际选择全部回收。

UI 不要把：

```text
Potential reclaimable size
```

写成：

```text
Freed immediately
```

---

# 13. Canonical Copy

Duplicate Group 中可以建立：

```text
Recommended Keep
```

但这里必须非常谨慎。

第一版不要让系统擅自删除“最差”的文件。

推荐做法：

```text
Group
├── File A
├── File B
└── File C

用户自己选择：
[Keep]
[Recycle]
```

如果 UI 提供：

```text
Keep this one
```

推荐可以根据明确、可解释的事实辅助：

```text
Path depth
Modification time
Filename
Location
```

但：

> **不要自动把某个文件标记为必须删除。**

尤其禁止：

```text
Oldest = Delete
Newest = Keep
```

这种未经用户确认的自动决策。

如需提供：

```text
Select Others
```

必须只是一种：

> 明确可见、可撤销的选择辅助。

不是后台删除策略。

---

# 14. Selection Model

Selection 必须是：

```text
User-owned state
```

而不是：

```text
Server / backend hidden decision
```

至少支持：

```text
Select file
Unselect file
Select all in group
Select all duplicates except one
Clear group selection
Clear all
```

默认策略：

> **不自动选择任何文件进行删除。**

可以提供：

```text
Keep one / Select others
```

但用户必须清楚看到具体选中的文件。

---

# 15. Group Selection Safety

一个 Group：

```text
A
B
C
```

如果用户：

```text
Select B
Select C
```

UI 应明确：

```text
2 files selected for recycle
A will remain
```

不要显示：

```text
Clean Group
```

而隐藏具体文件。

---

# 16. Scan Snapshot

扫描过程中必须明确：

```text
Scan Started
```

之后：

```text
filesystem may change
```

真实世界中用户可能：

```text
delete file
rename file
modify file
move file
replace file
```

因此 Duplicate Finder 必须把：

> 扫描结果

和：

> 当前文件系统实际状态

区分开。

---

# 17. File Mutation During Scan

如果扫描时：

```text
File disappears
```

结果应：

```text
Skipped / FileMissing
```

而不是：

```text
fake duplicate
```

如果文件：

```text
modified during hash
```

必须尽可能检测。

至少应该在：

```text
metadata before hash
metadata after hash
```

之间发现明显变化，例如：

```text
size changed
mtime changed
```

如当前 abstraction 无法完全保证内容稳定：

必须明确：

```text
ChangedDuringScan
```

或者等价结构化状态。

不得把一个可能已经变化的文件：

```text
当成可靠 duplicate
```

继续交给删除阶段。

---

# 18. TOCTOU

M3 必须重点考虑：

```text
Time Of Check
vs
Time Of Use
```

例如：

```text
Scan
 ↓
A 与 B 被判断为 duplicate
 ↓
用户修改 B
 ↓
点击 Recycle B
```

Execute 阶段必须重新验证：

```text
source exists
current metadata
expected identity
```

至少应尽量验证：

```text
expected size
expected hash where required
```

如果发现：

```text
expected hash != current hash
```

必须：

```text
拒绝删除该文件
```

并返回清晰状态：

```text
ChangedSinceScan
```

而不是：

```text
继续删除
```

---

# 19. Preview First

用户点击：

```text
Recycle Selected
```

不能直接：

```text
delete
```

必须：

```text
Selection
 ↓
Build Operation Plan
 ↓
Validate
 ↓
Preview
 ↓
Confirm
 ↓
Execute
```

Preview 至少显示：

```text
File
Current Path
Action
Estimated Reclaim
Validation
Warnings
```

例如：

```text
/Downloads/a.jpg
→
Recycle Bin

12.4 MB
Valid
```

---

# 20. Preview 必须是真实计算

禁止：

```text
Preview = hardcoded text
```

必须来自：

```text
actual selected files
actual current filesystem state
actual operation plan
```

不得让 UI 自己重新猜测：

```text
which files will be affected
```

---

# 21. Execute 必须使用 Preview 对应的 Plan

必须遵守：

```text
Build Plan
 ↓
Preview
 ↓
User Confirm
 ↓
Revalidate Plan
 ↓
Execute
```

不得：

```text
Preview 一个东西
Execute 时重新扫描全部目录并自行猜一套选择
```

Execute 必须处理：

```text
Plan identity
OperationId
Selected items
Expected source identity
```

---

# 22. Recycle，而不是 Permanent Delete

根据 Weave Charter：

默认删除策略：

```text
Move to Recycle Bin
```

不是：

```text
permanent delete
```

Duplicate Finder 的默认流程：

```text
Find
 ↓
Select
 ↓
Review
 ↓
Recycle
```

禁止：

```text
Find
 ↓
Auto Delete
```

---

# 23. Recycle Bin Abstraction

如果 M2 已存在：

```text
filesystem mutation abstraction
```

M3 可以增加：

```text
Recycle / Trash
```

相关 adapter。

但必须：

```text
platform-specific implementation
```

隔离。

UI / Domain 不得直接调用：

```text
Windows Shell API
macOS API
Linux command
```

除非通过明确的 Adapter Boundary。

---

# 24. 不允许使用 Shell 删除

禁止：

```text
cmd
powershell
bash
rm
del
trash CLI
```

作为业务删除路径。

尤其禁止：

```text
execute shell command with user path
```

这既是安全问题，也违反架构边界。

---

# 25. Recycle Failure

如果：

```text
Recycle succeeds
```

记录：

```text
Success
```

如果：

```text
Recycle fails
```

记录：

```text
Failed
```

不要：

```text
failed
→ pretend deleted
```

也不要：

```text
permission denied
→ silently skip
```

用户必须看到：

```text
what failed
why
```

---

# 26. Partial Success

删除 100 个文件：

```text
96 success
2 permission denied
1 missing
1 changed since scan
```

整体结果不能简单写：

```text
Failed
```

应表达：

```text
CompletedWithWarnings
```

或等价状态：

```text
Success = 96
Failed = 2
Skipped = 2
```

---

# 27. Cancellation

扫描过程中：

```text
Running
 ↓
Cancelling
 ↓
Cancelled
```

不得：

```text
Cancel clicked
→ UI instantly says Complete
```

扫描取消后必须保留：

```text
Scanned
Candidate files
Partial groups
Errors
Skipped
```

但必须清晰标记：

```text
Partial Result
```

而不是：

```text
Completed
```

---

# 28. Cancellation During Hash

Hash 阶段同样必须支持取消。

例如：

```text
large.iso
```

正在：

```text
Full SHA-256
```

用户点击：

```text
Cancel
```

必须：

```text
stop cooperatively
close stream
release resources
```

不能：

```text
继续读取几十 GB
```

---

# 29. Result Semantics

Duplicate Scan 至少支持：

```text
Completed
CompletedWithWarnings
Cancelled
Failed
```

结果至少包括：

```text
files_scanned
candidate_files
partial_hashed
full_hashed
duplicate_groups
duplicate_files
potential_reclaimable_size
skipped
failed
```

不要让：

```text
UI自己计算这些业务事实
```

Retry Failed：对 scan / recycle 的失败项提供重跑入口（最小实现即可），对齐 charter #26。

---

# 30. Deterministic Results

相同输入、相同规则、文件未变化时：

```text
Group membership
```

必须稳定。

目录枚举顺序不同：

不得导致：

```text
Group #1
Group #2
```

完全随机变化。

建议建立：

```text
stable file ordering
```

例如：

```text
normalized path
```

排序。

实际排序标准必须：

```text
documented
deterministic
cross-platform aware
```

---

# 31. Hash Pipeline

推荐逻辑：

```text
Directory Scan
        ↓
File Metadata
        ↓
Group By Size
        ↓
Groups with count >= 2
        ↓
Partial Hash
        ↓
Groups with partial-hash count >= 2
        ↓
Full SHA-256
        ↓
Groups with full-hash count >= 2
        ↓
Exact Duplicate Groups
```

这是 M3 的主算法骨架。

---

# 32. 不要建立巨大内存结构

禁止：

```text
read all file contents
```

更禁止：

```text
all files → byte[]
```

必须：

```text
metadata in memory
hash in streaming
```

对于 10,000、50,000 甚至更多文件：

尽量让内存增长主要取决于：

```text
number of candidates
```

而不是：

```text
total file bytes
```

---

# 33. Candidate Representation

候选文件内部至少需要：

```text
Path
Size
Metadata identity
PartialHash?
FullHash?
Status
```

可以使用紧凑结构。

不要为每个文件保存：

```text
full content
```

或：

```text
large metadata blob
```

---

# 34. Full Hash Cache

第一版默认：

> 不要为未来数据库缓存建立复杂系统。

如果 M1/M2 已经存在简单 hash cache infrastructure：

可以复用。

否则：

```text
M3 不要求 persistent hash cache
```

除非真实 benchmark 证明这是 M3 运行体验的必要条件。

不要为了：

```text
“以后可能很快”
```

提前加入：

```text
SQLite
Database ORM
Complex Cache Service
```

---

# 35. Hash Cache 正确性

如果未来使用 cache：

绝不能只依赖：

```text
Path
```

例如：

```text
D:/a.jpg
```

同一路径可能被替换。

缓存身份至少应考虑：

```text
path
size
modification metadata
```

以及任何现有 identity abstraction。

如果内容变化导致：

```text
identity mismatch
```

必须重新 hash。

M3 如果没有可靠 cache：

> 不要做半吊子的 cache。

---

# 36. Hard Links

必须审查：

```text
hard link
```

语义。

两个路径可能：

```text
Path A
Path B
```

实际指向：

```text
same underlying file
```

这与“两个独立文件拥有相同内容”不是完全相同的概念。

第一版必须明确：

```text
是否把 hard links 放入同一 duplicate group
```

如果当前跨平台 abstraction 无法统一表达：

> 不要虚构统一语义。

选择一个简单、可解释、可测试的行为，并记录在：

```text
DECISIONS.md
Known Limitations
```

尤其不要因为处理 hard link 而破坏：

```text
普通文件 duplicate detection
```

---

# 37. Symlink / Junction

必须遵守 M1 已有 policy。

特别注意：

```text
symlink to same file
```

可能制造看似重复的路径。

不要在 M3 自己重新发明：

```text
follow / don't follow
```

规则。

必须测试：

```text
directory symlink
file symlink
junction
symlink cycle
```

并验证：

```text
不会无限递归
不会突破 root
不会意外回收链接目标
```

---

# 38. Root Boundary

例如：

```text
Root = D:/Downloads
```

如果存在：

```text
D:/Downloads/link → C:/Users/...
```

M3 不得因为跟随链接：

```text
扫描到 root 外
```

更不能：

```text
Recycle root 外文件
```

这是 P0 级安全要求。

---

# 39. Permission Errors

扫描过程中：

```text
PermissionDenied
```

必须是正常结构化状态。

例如：

```text
Scanned: 9821
Skipped: 13
Permission denied: 4
```

不得：

```text
one inaccessible folder
→ entire scan crash
```

但如果关键 root 本身不可读取：

可以：

```text
Failed
```

并提供明确错误。

---

# 40. Locked Files

扫描阶段：

```text
File locked
```

必须根据实际平台语义处理。

如果：

```text
metadata readable
hash unreadable
```

可以：

```text
partial result
```

而不是：

```text
fake hash
```

Recycle 阶段如果文件被锁定：

```text
Failed
```

并明确：

```text
Unable to recycle file
```

---

# 41. File Disappearance

扫描时：

```text
stat succeeded
```

但 hash 前：

```text
file missing
```

必须：

```text
Skipped / FileMissing
```

不能：

```text
continue with stale metadata
```

---

# 42. File Replacement

最危险场景之一：

```text
A.jpg
```

扫描时是：

```text
Hash X
```

用户随后替换成：

```text
Hash Y
```

Recycle 之前：

必须重新验证。

如果：

```text
Y != X
```

则：

```text
do not recycle
```

结果：

```text
ChangedSinceScan
```

---

# 43. Duplicate Group Identity

GroupId 不应依赖：

```text
scan order
```

例如：

```text
Group 1
Group 2
Group 3
```

最好能由稳定事实派生，或者至少在单次结果中稳定。

如果选择：

```text
full hash
```

作为核心 identity：

要明确：

```text
hash identity
```

与：

```text
UI GroupId
```

是否是同一个概念。

不要把 UI 临时编号写进持久化事务作为真正的文件身份。

---

# 44. Group Sorting

默认可以按：

```text
potential reclaimable size DESC
```

展示。

但：

> 这是 UI 排序策略，不是 duplicate truth。

Domain 中不要把：

```text
largest group
```

定义成：

```text
best duplicate
```

可以让 UI 支持：

```text
Reclaimable Size
File Count
Path
Name
Size
```

等排序。

---

# 45. Duplicate Statistics

建议显示：

```text
Files Scanned
Candidate Files
Duplicate Groups
Duplicate Files
Potential Reclaimable Space
Scan Duration
```

但：

```text
Potential Reclaimable Space
```

只能基于当前检测结果。

如果用户只选择某些文件：

必须重新显示：

```text
Selected Reclaimable Size
```

不要继续显示整个 scan 的潜在空间。

---

# 46. Progress Model

扫描阶段进度至少表达：

```text
Scanned
Current Stage
Candidates
Partial Hash
Full Hash
Duplicate Groups
```

如果 total 可知：

```text
821 / 5000
```

如果 total 不可靠：

```text
Scanned 821 files
```

不要伪造：

```text
73%
```

---

# 47. Stage Progress

推荐状态：

```text
Scanning
Filtering
Partial Hashing
Full Hashing
Grouping
Completed
```

UI 应让用户知道：

> 为什么扫描还没有结束。

但不要为了炫技做：

```text
fake animation
```

---

# 48. UI 不应显示“正在扫描 100%”然后继续很久

如果 pipeline 是：

```text
Scan
→ Hash
```

不要：

```text
scan 100%
```

然后突然：

```text
hash 继续 10 分钟
```

应该按 stage 表达：

```text
Scan complete
Candidates found: 482
Now hashing candidates...
```

---

# 49. Empty Result

如果没有重复：

```text
No exact duplicates found
```

同时显示：

```text
Files scanned
```

不要显示：

```text
0 duplicates
```

然后让用户猜是不是扫描失败。

必须区分：

```text
Completed with 0 groups
```

和：

```text
Failed before scan
```

---

# 50. Partial Result

例如：

```text
10000 files
```

用户在：

```text
full hash
```

阶段取消。

可以展示：

```text
Scan cancelled

Scanned:
10000

Partial hashed:
1832

Exact duplicate groups found so far:
41
```

同时明确：

```text
This result is partial.
```

不要假装：

```text
Scan complete
```

---

# 51. Search / Filter Within Results

M3 第一版可以提供轻量结果筛选：

```text
Search filename
Filter by extension
Filter by group size
Filter by file size
```

但不要把 M3 扩展成：

```text
general-purpose file search engine
```

M9 Search 以后处理。

结果内搜索只是：

> Duplicate Result UI convenience.

---

# 52. Duplicate Group View

推荐 UI：

```text
Duplicate Group
────────────────────────

3 files
24 MB reclaimable

[x] A.jpg   12 MB
[ ] B.jpg   12 MB
[ ] C.jpg   12 MB

[Keep A]
[Select Others]
```

但真正执行前仍需：

```text
Preview
```

---

# 53. File Detail

展开一个重复文件时，可以复用 M1 File Inspector 的事实：

```text
Path
Name
Size
Modified
Type
Hash
```

不要重新实现另一套 metadata provider。

---

# 54. Preview of Recycle

Preview 必须显示：

```text
Selected Files
Current Path
Action = Recycle
Size
Validation
Warnings
Potential reclaim
```

例如：

```text
3 files selected

Recycle:
--------------------------------
D:/Downloads/a.jpg   12 MB   OK
D:/Backup/a.jpg       12 MB   OK
D:/Temp/a.jpg         12 MB   CHANGED
--------------------------------

2 files will be recycled
1 file will be skipped
24 MB selected
```

---

# 55. Execute Button

不要使用：

```text
Delete
```

作为默认按钮文案。

优先：

```text
Recycle 24 MB
```

或者：

```text
Recycle 2 Files
```

让用户明确知道：

```text
发生什么
处理多少
```

---

# 56. Double Confirmation

对于：

```text
Recycle
```

第一层：

```text
Preview
```

第二层：

```text
Confirm
```

即可。

如果平台 recycle 实现本身无法保证恢复：

必须明确。

不要搞：

```text
非常恐怖的连续 4 层弹窗
```

安全与 UX 都需要平衡。

---

# 57. Permanent Delete

M3 默认：

> **不提供永久删除。**

除非 Charter / 后续明确增加范围。

本阶段不要因为：

```text
“顺便把删除做全”
```

加入：

```text
Shift+Delete
Permanent Delete
Secure Erase
Wipe
```

---

# 58. Transaction

回收操作必须使用 M2 已存在的：

```text
OperationTransaction
```

记录实际发生的 mutation。

至少表达：

```text
OperationId
OperationType
Timestamp
Original Path
Recycle Destination / Platform Token if safely available
Original Metadata where applicable
Status
```

具体字段必须适配平台 recycle abstraction。

不要为了跨平台而伪造：

```text
recycle destination path
```

如果平台只返回：

```text
success/failure
```

就诚实表达能力限制。

---

# 59. Undo

Undo 必须明确：

> Undo = 尝试恢复本次实际成功回收的文件。

如果平台能够可靠获得：

```text
recycle token
```

则应使用。

如果只能：

```text
recover from known recycle location
```

则必须谨慎。

不允许：

```text
随便猜回收站路径
```

更不允许：

```text
直接复制一个同名文件回来
```

冒充恢复。

---

# 60. Undo Safety

Undo 前必须重新验证：

```text
Original path state
Destination / recycle state
```

尤其：

```text
original path already occupied
```

时：

> 不得覆盖用户现有文件。

如果无法安全恢复：

```text
Undo unavailable
```

或：

```text
Undo partially available
```

而不是：

```text
强行覆盖
```

---

# 61. History

History 必须记录：

```text
OperationId
OperationType = DuplicateRecycle
Timestamp
Files Selected
Files Recycled
Files Failed
Files Skipped
Reclaimable Size
Undoability
```

不要写入：

```text
file contents
```

不要写入：

```text
binary data
```

只保留：

```text
operation metadata
```

---

# 62. History Restart Test

执行：

```text
Recycle
 ↓
History
 ↓
Close app
 ↓
Reopen
 ↓
History
```

必须验证：

```text
record still exists
```

如果 M2 已建立持久化 History：

M3 必须复用。

---

# 63. History Write Failure

如果：

```text
Recycle succeeded
```

但：

```text
History persistence failed
```

不能回报：

```text
Recycle failed
```

真实结果应该是：

```text
Operation completed
History persistence failed
Undo may be unavailable
```

这与 M2 的事实状态原则一致。

---

# 64. Operation Result

至少支持：

```text
Success
Failed
Skipped
Cancelled
CompletedWithWarnings
```

每个文件应可追踪：

```text
Source
Action
Status
Error
```

---

# 65. Error Model

推荐明确错误：

```text
PermissionDenied
FileMissing
ChangedSinceScan
FileLocked
RecycleUnavailable
RecycleFailed
PathUnsafe
RootEscape
SymlinkPolicyBlocked
AlreadyExists
Unsupported
Cancelled
HashFailed
```

不要：

```text
Error: something went wrong
```

这种无上下文错误。

---

# 66. Security

M3 是潜在删除工具，因此安全等级高。

至少测试：

```text
Root Escape
Path Traversal
Symlink Escape
Junction Escape
Unexpected Root
File Replacement
File Missing
Overwrite Prevention
Shell Injection
Malformed Paths
Permission Errors
```

---

# 67. No Silent Overwrite

无论：

```text
Recycle
Undo
Temporary operation
```

都不允许：

```text
silent overwrite
```

尤其是 Undo：

```text
restore a.txt
```

但用户后来已经创建：

```text
a.txt
```

必须：

```text
do not overwrite
```

---

# 68. Temporary Resources

如果 recycle adapter / transaction 需要：

```text
temporary metadata
```

必须：

```text
unique
bounded
cleaned
```

不得把内部：

```text
.tmp
```

永久留在用户目录。

---

# 69. Resource Limits

M3 面对恶意目录也要有边界。

例如：

```text
maximum directory entries
maximum recursion depth
maximum candidate count
maximum path length
```

具体数值根据：

```text
M1 existing limits
```

复用。

不要重新定义互相矛盾的数字。

---

# 70. Huge File

必须测试：

```text
large binary
large ISO
large archive
```

至少验证：

```text
does not load whole file into memory
```

并验证：

```text
partial hash
full hash
cancellation
```

---

# 71. Many Small Files

也必须测试：

```text
10,000
50,000
```

级别的小文件场景。

关注：

```text
directory traversal overhead
metadata collection
candidate grouping
UI result rendering
memory
```

---

# 72. Memory

不能因为：

```text
50,000 files
```

而创建：

```text
50,000 giant UI components
```

如果结果很多：

考虑：

```text
virtualized list
```

但不要无意义增加依赖。

先用真实 benchmark 判断。

---

# 73. UI Responsiveness

当扫描：

```text
10k+
```

文件时：

用户必须仍然能够：

```text
Cancel
Scroll
Review partial state where supported
```

不得：

```text
UI hard freeze
```

---

# 74. Parallel Hashing

可以考虑：

```text
bounded concurrency
```

但禁止：

```text
每个文件一个线程
```

或：

```text
unbounded async task
```

必须考虑：

```text
HDD contention
SSD throughput
CPU
memory
OS open-file limits
```

默认优先：

```text
bounded worker pool
```

具体并发度通过 benchmark 决定。

不要拍脑袋写：

```text
128 workers
```

---

# 75. Concurrency Policy

建议将：

```text
scan
metadata
partial hash
full hash
```

视为 pipeline stage。

可以：

```text
bounded parallelism
```

但必须保证：

```text
deterministic result
```

不要让并行完成顺序影响：

```text
duplicate group membership
```

---

# 76. Progress Under Parallelism

不要出现：

```text
Hashing...
████████████
```

但后台其实：

```text
queue stalled
```

至少保证 progress 数据来自：

```text
actual processed work
```

不是：

```text
timer animation
```

---

# 77. Logging

可以记录：

```text
scan started
root
counts
stage
duration
errors
operation id
```

禁止记录：

```text
full file contents
```

也尽量避免日志中完整输出：

```text
大量敏感文件名
```

除非实际诊断需要且已被设计允许。

---

# 78. Privacy

M3 必须保持：

```text
No Telemetry
No Cloud
No Upload
No Remote Hashing
```

所有：

```text
metadata
partial hash
full hash
duplicate groups
```

默认：

> 只在本机处理。

特别禁止：

```text
上传 hash 到云端寻找 duplicates
```

---

# 79. AI

M3 不需要 AI。

不得引入：

```text
LLM
Embedding API
Cloud Vision
AI similarity
```

普通 Exact Duplicate 应该：

> **100% 本地、确定性代码即可完成。**

---

# 80. IPC

根据 M2 已有 IPC 风格，最少需要表达类似：

```text
scan_duplicates
cancel_duplicate_scan
get_duplicate_scan_result
build_recycle_plan
validate_recycle_plan
execute_recycle_plan
get_history
get_operation
undo_operation
```

具体名称：

> 必须根据项目真实现有命名调整。

不要为了“看起来统一”而重写已有 IPC。

---

# 81. IPC 不暴露内部 Rust 类型

DTO 必须：

```text
Serializable
Explicit
Stable
Version-conscious
```

不要把：

```text
Rust internal structs
```

直接作为 UI contract。

---

# 82. Frontend Command Layer

继续使用：

```text
ui/commands/
```

统一管理：

```text
invoke(...)
```

组件不能到处：

```text
invoke("scan_duplicates")
```

---

# 83. Application Layer

Application Service 负责：

```text
validate request
load context
run duplicate scan
build result
build recycle plan
execute operation
```

Tauri command 只负责：

```text
DTO
Application Call
DTO
```

不要在 Tauri command 里写：

```text
几百行 hashing algorithm
```

---

# 84. Crate Responsibility

推荐：

```text
weave-core
```

负责：

```text
stable domain contracts
common ids
operation models
result semantics
```

```text
weave-files
```

负责：

```text
filesystem scan
file metadata access
hash integration
path safety
recycle/mutation adapters
```

如果 Duplicate Finder 自身逻辑明显独立：

可以在：

```text
weave-files
```

内部建立：

```text
duplicate/
```

或者创建新的：

```text
weave-duplicate
```

但：

> 不要为了一个工具机械增加 crate。

必须根据实际代码规模判断。

如果新增 crate：

记录：

```text
DECISIONS.md
```

说明原因。

---

# 85. weave-testkit

必须继续扩展 test fixtures。

至少包含：

```text
duplicate files
different size
same size different content
same partial hash hypothetical fixtures
same full hash
zero-byte files
unicode names
nested folders
missing files
changed files
locked files
permission denied
symlink
junction where supported
hard links where supported
large files
many files
```

---

# 86. Test Fixtures 不要提交超大真实文件

避免提交：

```text
100 MB binary
1 GB ISO
真实用户数据
```

可以：

```text
generate deterministic fixtures
```

运行时创建。

例如：

```text
repeating bytes
pseudo-random deterministic bytes
known patterns
```

---

# 87. Deterministic Test Data

测试数据必须稳定。

禁止：

```text
random() without seed
```

导致：

```text
测试偶发失败
```

如果生成 pseudo-random fixture：

使用固定 seed。

---

# 88. Unit Tests

至少测试：

### Size Grouping

```text
same size
different size
zero size
```

### Partial Hash

```text
same content
different beginning
different ending
same size
```

### Full Hash

```text
same file contents
different contents
large files
```

### Grouping

```text
2 files
3 files
100 files
```

### Ordering

```text
stable results
```

---

# 89. Important Hash Tests

测试：

```text
same content, different path
```

必须：

```text
duplicate
```

测试：

```text
different content, same size
```

必须：

```text
not duplicate
```

测试：

```text
same first chunk
different last chunk
```

验证：

```text
partial hash can filter correctly
```

并最终：

```text
full hash separates
```

---

# 90. Collision / Cryptographic Semantics

不要编造：

```text
fake SHA collision
```

来宣称“hash 不可靠”。

测试重点应是：

```text
implementation correctness
```

即：

```text
known file
→ expected SHA-256
```

如果项目使用：

```text
hex
lowercase
```

必须统一。

例如：

```text
ABC...
```

与：

```text
abc...
```

不要在不同层产生不同 equality semantics。

---

# 91. Fault Injection

必须设计 fault tests：

```text
file disappears before hash
file disappears after hash
file changes during hash
permission denied
read interrupted
partial read
hash cancellation
recycle unavailable
recycle failure
history write failure
undo conflict
```

不要只测 happy path。

---

# 92. Integration Test

至少有完整真实文件系统闭环：

```text
create temp root
 ↓
create A
create B
create C
create unique D
 ↓
scan
 ↓
size grouping
 ↓
partial hash
 ↓
full hash
 ↓
duplicate groups
 ↓
select
 ↓
preview
 ↓
execute recycle
 ↓
verify filesystem
 ↓
history
 ↓
undo
 ↓
verify restore
```

这是 M3 最关键的一组测试。

---

# 93. Cross-platform Tests

至少检查：

```text
Windows
Linux
```

如果项目 CI 环境支持：

也应检查：

```text
macOS
```

特别关注：

```text
path semantics
recycle behavior
symlink
permissions
timestamps
case sensitivity
```

---

# 94. Platform-specific Behavior

不要为了统一而伪造：

```text
same recycle internals
```

允许：

```text
platform-specific adapter
```

但 Domain contract 必须稳定。

例如：

```text
RecycleResult
```

可以表达：

```text
success
unsupported
failed
```

而不是把 Windows API 类型泄漏到 Domain。

---

# 95. Case Sensitivity

Duplicate Finder 是内容比较工具，因此：

```text
A.TXT
a.txt
```

即使平台大小写规则不同：

只要：

```text
content same
```

就应该属于：

```text
duplicate
```

Path comparison 不得污染 content identity。

---

# 96. Unicode

必须测试：

```text
中文
日文
韩文
emoji
accented characters
combining characters
long Unicode names
```

验证：

```text
display
sorting
selection
recycle
history
```

不会乱码。

---

# 97. Long Paths

复用 M1：

```text
Path Safety
```

不要 Duplicate Finder 自己处理另一套：

```text
long path
```

如果平台存在：

```text
unsupported
```

必须诚实表达。

---

# 98. Directory Scanner 不要“先把所有 Paths 全读入再处理”作为唯一实现

如果实际规模允许：

可以：

```text
incremental scan
```

更优。

推荐：

```text
enumerate
→ metadata
→ size bucket
```

但如果 OS API 限制：

必须保持：

```text
bounded memory
```

---

# 99. Scan Cache 与 Repeat

M3 可以支持：

```text
Scan Again
```

但不要提前建设：

```text
persistent global file index
```

这属于未来优化。

第一版应优先：

```text
fresh scan
```

保证：

```text
correctness
```

---

# 100. Repeat Scan

如果用户：

```text
Scan
```

然后：

```text
Scan Again
```

必须：

```text
重新验证文件系统
```

不能直接展示旧结果然后假装重新扫描。

---

# 101. Performance Benchmark

至少建立真实 benchmark：

```text
1,000 files
10,000 files
50,000 files
```

并至少测：

```text
scan duration
metadata phase
partial hash phase
full hash phase
peak memory
candidate reduction ratio
UI responsiveness
```

不要追求绝对数字 KPI。

目标是：

> **建立真实基线，证明算法没有明显浪费。**

---

# 102. Benchmark 场景

至少有：

### Scenario A

```text
10,000 unique-size files
```

预期：

```text
几乎不进入 full hash
```

### Scenario B

```text
10,000 files
大量相同 size
```

预期：

```text
partial hash 明显减少 full hash 数量
```

### Scenario C

```text
大量 exact duplicates
```

预期：

```text
grouping 正确
```

### Scenario D

```text
大量 large files
```

预期：

```text
bounded memory
streaming hash
```

---

# 103. Partial Hash Efficiency

必须输出真实数据：

```text
Files scanned: 50,000
Size candidates: 8,400
Partial hashed: 8,400
Full hashed: 1,920
```

或者类似真实测量。

不要写：

```text
Partial hash improves performance significantly
```

却没有任何数据支持。

---

# 104. Avoid Premature Optimization

不要为了：

```text
“以后 1 million files”
```

提前加入：

```text
SQLite
Memory-mapped database
Distributed indexing
Background daemon
Persistent filesystem index
```

先完成：

```text
correct
safe
bounded
testable
```

的 M3。

---

# 105. UX Principle

M3 用户打开之后应该能迅速理解：

```text
Where am I scanning?
What did you find?
How much space is duplicated?
Which files will be affected?
What exactly happens when I click Recycle?
```

不要做：

```text
专业术语堆满整个界面
```

例如：

```text
Partial Hash Candidate Cohort
```

可以是内部概念。

用户界面更适合：

```text
Checking duplicate candidates...
```

i18n：所有 UI 与错误文案必须进入 i18n 资源文件（zh-CN / en），禁止硬编码；本节示例文案一律视为资源 key 示例（charter #34）。

---

# 106. Scan Entry UX

第一版至少：

```text
Drop Folder
Choose Folder
Recent Root
```

根据 M0 已有入口能力适配。

如果 Weave 已有统一 Drag & Drop：

必须复用。

不要给 Duplicate Finder 造第二套 drag/drop。

---

# 107. Scan Summary

完成后顶部显示：

```text
12,842 files scanned
37 duplicate groups
119 duplicate files
4.8 GB potential reclaim
```

但要明确：

```text
Potential reclaim
```

不是：

```text
You will free 4.8 GB
```

因为用户可能不选择所有 duplicate files。

---

# 108. Selected Summary

用户选择之后：

```text
7 files selected
1.2 GB selected for recycle
```

这个数字必须来自：

```text
actual selection
```

不要直接拿：

```text
group total
```

冒充。

---

# 109. Review Before Recycle

点击：

```text
Recycle Selected
```

必须进入：

```text
Review
```

Review 页面/区域至少显示：

```text
N files
Total size
Affected paths
Warnings
Changed files
Unavailable files
```

---

# 110. Revalidation

Confirm 后必须重新验证：

```text
source exists
expected size
expected hash
path safety
recycle availability
```

至少确保：

```text
file did not change since scan
```

如果 change：

```text
skip
```

---

# 111. Actual Mutation Tracking

Transaction 只记录：

> **真实发生的回收操作。**

不要记录：

```text
计划中有 20 个
所以 transaction 写 20 个 success
```

实际如果：

```text
15 success
3 failed
2 changed
```

Transaction 必须反映：

```text
15 actual mutations
```

以及对应结果。

---

# 112. Crash / Interrupted Execution

如果执行：

```text
Recycle 100 files
```

中途应用退出：

下次打开：

```text
History
```

必须尽可能准确反映：

```text
actual completed items
```

不要把：

```text
intended 100
```

写成：

```text
success 100
```

如果无法获得完整状态：

记录：

```text
Unknown / Interrupted
```

并在 Known Limitations 说明。

---

# 113. Recycle / Undo Audit

重点测试：

```text
recycle one file
recycle multiple files
partial recycle
cancel recycle
missing file
changed file
undo one
undo partial
undo conflict
```

---

# 114. M3 不重复实现 M2

明确禁止：

```text
Duplicate Finder
→ own Plan model

Duplicate Finder
→ own Transaction

Duplicate Finder
→ own History

Duplicate Finder
→ own Undo

Duplicate Finder
→ own Progress abstraction
```

必须：

```text
reuse
```

如果现有 M2 infrastructure 明显不足：

只做：

> **最小、通用、向后兼容的改进。**

不得为了 M3 推倒重来。

---

# 115. 如果 M2 基础设施有缺陷

先做：

```text
Baseline Audit
```

如果问题影响：

```text
correctness
security
data loss
architecture
```

则：

```text
P0
```

必须修。

如果只是：

```text
nice to have
future optimization
```

不要借 M3 偷渡大重构。

所有必要架构取舍：

```text
DECISIONS.md
```

---

# 116. Architecture Rules

最终必须保持：

```text
UI
 ↓
Application
 ↓
Domain
 ↓
Adapters
 ↓
Filesystem / OS
```

禁止：

```text
React
 ↓
std::fs
```

禁止：

```text
React
 ↓
Recycle Bin API
```

禁止：

```text
Tauri command
 ↓
hundreds of lines duplicate algorithm
```

禁止：

```text
Domain
 ↓
Windows API
```

禁止：

```text
Duplicate Finder
 ↓
shell command
```

---

# 117. Tool Abstraction

如果项目已经存在：

```text
Tool
```

模型：

```text
id
category
inputTypes
options
preview()
execute()
canUndo()
undo()
```

M3 应合理接入。

但不要为了追求“所有工具现在都完全一样”：

> 强行重构 M0/M1/M2 所有代码。

只建立：

```text
真正能支持未来工具复用
```

的最小连接。

---

# 118. Frontend State

建议区分：

```text
scan state
result state
selection state
preview state
execution state
history state
```

避免：

```text
一个巨大 store
```

同时也不要：

```text
几十个毫无必要的 store
```

遵循现有项目状态管理风格。

---

# 119. Result Rendering

如果重复结果非常多：

不要：

```text
一次性渲染所有文件的重型组件
```

根据真实 benchmark：

考虑：

```text
virtualized groups
```

但仍以：

```text
实际证明
```

为依据。

---

# 120. Accessibility

至少保证：

```text
keyboard navigation
clear selected state
focus visibility
buttons labels
confirmation semantics
```

尤其：

```text
Recycle
Undo
```

必须可明确辨识。

---

# 121. Error UX

不要：

```text
Error 0x80070005
```

直接扔给普通用户。

建议：

```text
Could not recycle this file.
The file may be locked or inaccessible.
```

同时可以提供：

```text
technical details
```

供高级用户查看。

---

# 122. Technical Details

高级错误信息可以包含：

```text
error code
operation id
path
stage
platform
```

但注意：

```text
privacy
```

不要无条件记录大量用户文件路径。

---

# 123. No Mock

绝对禁止：

```text
fake duplicate groups
fake hash
fake reclaimable size
fake recycle success
fake history
fake undo
```

例如：

```rust
return DuplicateResult {
    groups: sample_groups()
}
```

属于：

> FAIL

---

# 124. No Fake Progress

禁止：

```text
Timer
→ 0% 10% 20% ... 100%
```

而后台真实扫描完全不同步。

Progress 必须来自：

```text
actual work
```

---

# 125. No Swallowed Errors

禁止：

```rust
if hash_failed {
    continue;
}
```

却：

```text
不记录 failed
```

必须：

```text
structured result
```

---

# 126. No Silent Fallback

例如：

```text
Recycle unsupported
```

不能：

```text
fallback to permanent delete
```

例如：

```text
Recycle adapter failed
```

不能：

```text
fallback to fs::remove_file
```

这是：

> P0 安全问题。

---

# 127. Dependency Policy

如果需要新增依赖：

先检查：

```text
Name
Version
License
Repository
Maintenance
Transitive Dependencies
Bundle Size
Runtime Cost
Cross-platform support
```

并记录：

```text
DECISIONS.md
```

不要因为：

```text
“做 recycle 方便”
```

就无条件加入大型删除库。

回收站实现依赖（Windows Shell API 绑定或等价 crate）必须在实现前评估 license 与维护状态，记录进 THIRD_PARTY_LICENSES.md 与 DECISIONS.md。

---

# 128. Platform API Dependency

如果引入平台能力：

必须隔离到：

```text
adapter
```

不要泄漏到：

```text
weave-core
```

例如：

```text
Windows recycle implementation
```

不能让 Domain 出现：

```rust
WindowsShellRecycle(...)
```

---

# 129. Documentation

更新至少：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
```

文档必须说明：

```text
Exact Duplicate Definition
Hash Pipeline
Partial Hash Strategy
Recycle Policy
Undo Semantics
Scan Cancellation
Changed Since Scan
Symlink Policy
Hard Link Policy
Platform Limitations
Known Limitations
```

---

# 130. DECISIONS.md

至少记录：

```text
Partial Hash Strategy
Exact Duplicate Definition
Hard Link Semantics
Symlink Semantics
Recycle Abstraction
Undo Limitations
Hash Cache decision
Parallelism decision
Result ordering
```

不要只写：

```text
Implemented M3
```

---

# 131. PROGRESS.md

更新：

```text
M0 = COMPLETE
M1 = COMPLETE
M2 = COMPLETE
M3 = COMPLETE
M4 = NEXT
```

但只有真实通过 Quality Gate 后才能写：

```text
COMPLETE
```

记录：

```text
Implemented
Tested
Verified
Performance
Known Limitations
Not Verified
```

---

# 132. README

加入一个真实的 Duplicate Finder 使用流程：

```text
Choose Folder
 ↓
Scan
 ↓
Review Duplicate Groups
 ↓
Select Files
 ↓
Preview
 ↓
Recycle
 ↓
History
 ↓
Undo
```

不要写：

```text
“one-click clean your disk”
```

因为这会误导产品行为。

---

# 133. Security Audit

完成实现后，必须执行：

```text
Path traversal
Root escape
Symlink escape
Junction escape
Shell execution
Unexpected delete
Permanent delete fallback
Overwrite
Changed file
Missing file
Permission
Locked file
```

任何 P0：

> 必须修复。

---

# 134. Privacy Audit

检查：

```text
No telemetry
No network
No upload
No remote hash
No file contents in logs
No file contents in history
```

如果发现：

```text
网络请求
```

必须判断：

```text
是否属于 M3 范围
```

默认：

> 不允许。

---

# 135. Performance Audit

至少：

```text
1k
10k
50k
```

检查：

```text
scan
candidate reduction
partial hashing
full hashing
memory
UI
cancel
```

不要只执行：

```text
cargo test
```

然后宣称：

```text
performance good
```

---

# 136. Test Matrix

至少覆盖：

| 场景 | 期望 |
|---|---|
| same size, different content | 非重复 |
| size 不同的两个文件在 Exact Duplicate 语义下不可能互为重复（内容相同则 size 必然相同），无需进入 hash 阶段 | 非重复 |
| same content, different path | 重复 |
| zero-byte duplicates | 重复 |
| many duplicates | 正确分组 |
| unique files | 0 groups |
| file disappears | skipped |
| file changes | changed-since-scan |
| permission denied | structured failure |
| locked file | structured failure |
| cancellation | cancelled / partial |
| recycle success | success |
| recycle failure | failed |
| undo success | restored |
| undo conflict | no overwrite |
| symlink escape | blocked |
| root escape | blocked |
| shell injection path | blocked / irrelevant to shell |
| large file | streaming |
| large batch | bounded memory |

---

# 137. Property / Invariant Tests

如果现有 test stack 适合：

建立一些 property/invariant：

```text
Same bytes + same size
→ same full hash

Different bytes in controlled fixture
→ different expected hash

All files in a duplicate group
→ same size

All files in a duplicate group
→ same full hash

Unique-size file
→ never needs full hash
```

同时验证：

```text
group membership does not depend on enumeration order
```

---

# 138. Mutation Invariants

Recycle 执行前：

```text
No file changed
```

Execute 后：

```text
Only selected files affected
```

不得：

```text
unselected duplicate changed
```

更不得：

```text
unique file changed
```

---

# 139. Scope Invariant

如果：

```text
Root = D:/A
```

那么：

```text
selection
scan
recycle
undo
```

都必须保持在：

```text
allowed scope
```

除非用户明确操作已经允许的另一个 root。

---

# 140. Review Actual Diff

完成开发后，先不要 commit。

执行：

```powershell
git diff --stat
git diff
git status
```

逐项检查：

```text
是否偷渡 Near Duplicate
是否偷渡 AI
是否新增无必要依赖
是否重新实现 SHA-256
是否 duplicated Path Safety
是否 duplicated History
是否 duplicated Transaction
是否存在 shell delete
是否存在 silent overwrite
是否存在 fake progress
是否存在 fake UI
是否存在 swallowed error
是否存在 TODO placeholder
是否存在 ignored tests
是否存在 disabled assertion
是否存在 debug print
是否存在 secrets
是否提交大型 fixture
```

---

# 141. M1 Regression

M3 完成后必须重新验证：

```text
File Inspector
Directory Analyzer
Hash
Filesystem abstraction
Path Safety
Progress
Cancellation
Structured Errors
```

任何 M1 regression：

> M3 FAIL

---

# 142. M2 Regression

必须重新验证：

```text
Rename
Organizer
Preview
Collision Detection
Execute
History
Undo
```

尤其检查：

```text
Duplicate Finder recycle
```

是否污染：

```text
M2 transaction model
```

---

# 143. Build Gates

必须实际执行项目已有 quality gates。

至少尝试：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

License 门禁为无条件必过项：

```powershell
cargo deny check
```

前端至少：

```powershell
npm run typecheck
npm run lint
npm test
```

最后：

```powershell
npm run build
```

以及项目真实存在的：

```text
Tauri build command
```

---

# 144. Integration Build

必须验证：

```text
Rust
+
Frontend
+
Tauri IPC
```

真实连接。

不能：

```text
Rust tests pass
UI mock pass
```

就宣布完成。

---

# 145. Real UI Smoke Test

至少真实操作：

```text
Open app
 ↓
Choose temp folder
 ↓
Scan
 ↓
Show duplicate group
 ↓
Select one file
 ↓
Preview
 ↓
Recycle
 ↓
Verify file moved to recycle
 ↓
History
 ↓
Undo
 ↓
Verify restored
```

必须使用：

```text
real filesystem fixture
```

而不是：

```text
storybook mock
```

---

# 146. Real Fault Smoke Test

至少真实验证：

```text
delete source between scan and execute
modify source between scan and execute
create conflict at original path before undo
```

确认：

```text
no silent data loss
```

---

# 147. Final Audit

完成实现后不要直接说：

```text
M3 Complete
```

必须先输出：

# M3 Final Audit

## A. Baseline

```text
M2 HEAD
branch
working tree
```

## B. Architecture

```text
crate boundaries
application
IPC
UI
adapter
filesystem
```

## C. Detection

```text
size grouping
partial hash
full hash
duplicate grouping
determinism
```

## D. Scan

```text
recursive
symlink
root safety
multi-root if supported
cancellation
partial results
```

## E. Correctness

```text
same size
different content
same content
zero-byte
changed files
missing files
```

## F. Recycle

```text
plan
preview
validation
execute
partial failure
cancellation
```

## G. Transaction

```text
actual mutations
OperationId
transaction integrity
```

## H. History

```text
persistence
restart
failure semantics
```

## I. Undo

```text
restore
partial restore
conflict
external mutation
```

## J. Performance

```text
1k
10k
50k
memory
throughput
candidate reduction
```

## K. Security

```text
root escape
path traversal
symlink
junction
overwrite
shell
permanent delete
```

## L. Privacy

```text
No upload
No telemetry
No network
No content logging
```

## M. Tests

```text
Unit
Integration
Fault
Property
Regression
UI smoke
```

## N. Build

```text
Rust
Frontend
Tauri
```

## O. Documentation

```text
README
ARCHITECTURE
DECISIONS
PROGRESS
PRIVACY
```

## P. Scope Compliance

确认没有偷渡：

```text
Near Duplicate
AI Similarity
M4
M5
M6
M7
Workflow
Cloud
Telemetry
```

每项只允许：

```text
PASS
FAIL
BLOCKED
NOT VERIFIED
```

禁止：

```text
基本完成
应该没问题
预计通过
```

---

# 148. Known Limitations

必须显式列出：

```text
Not Implemented
Not Verified
Deferred
Platform-specific
```

例如可能包括：

```text
Hard-link semantics
Recycle limitations
Unsupported filesystem
macOS verification unavailable
Windows-only recycle behavior
```

只有真实存在的才列。

不要为了显得专业制造限制。

---

# 149. M3 不算完成的情况

以下任意一项存在，都不能宣布：

```text
M3 Complete
```

例如：

```text
扫描能工作
但 partial hash 根本没使用

partial hash 工作
但 partial hash 相同就直接当 duplicate

full hash 工作
但 duplicate groups 不稳定

duplicate groups 正确
但用户点击按钮就直接删除

Recycle 存在
但 failure 被吞掉

Preview 存在
但 Execute 没有 revalidation

扫描后文件被替换
但系统仍回收新文件

Undo 存在
但会覆盖用户后来创建的文件

History 存在
但重启后丢失

Symlink 可以逃出 root

使用 shell 命令删除文件

永久删除被当作 fallback

10k/50k 文件导致 UI 完全无法取消

Cancel 后后台仍继续大规模 hash

使用 std::fs::read 将巨大文件全部读入内存

提交大量无必要 binary fixtures

M1 regression

M2 regression

存在 fake implementation

存在 fake test

存在 suppressed assertion

存在 TODO placeholder 代替真实逻辑
```

---

# 150. Commit Strategy

只有：

```text
Final Audit PASS
```

之后才允许 commit。

遵循现有 Git convention。

推荐类似：

```text
feat(files): implement duplicate finder
```

或者：

```text
feat(duplicate): implement exact duplicate finder
```

不要：

```text
update
fix
misc changes
```

---

# 151. Git Hygiene

最终执行：

```powershell
git status
git diff --stat
git diff
git diff --cached
git ls-files
```

检查：

```text
.env
secrets
private keys
certificates
local paths
databases
temp reports
cache
build output
large fixtures
personal information
```

确保：

```text
.gitignore
```

正确。

---

# 152. M3 Handoff

M3 完成后，必须向 M4 / 后续阶段明确交接：

```text
Filesystem abstraction
Path safety
Metadata
Streaming hash
Progress
Cancellation
Deterministic result
OperationId
Plan
Preview
Validation
Transaction
History
Undo
Recycle abstraction
Duplicate scan pipeline
Duplicate group model
```

明确说明：

> **后续阶段继续复用这些基础能力，不得重新实现一套平行的文件操作系统。**

---

# 153. Final Product State

M3 完成后，用户应该已经能够：

```text
拖入一个目录
        ↓
扫描文件
        ↓
Weave 根据：
size
→ partial hash
→ full hash
找到：
Exact Duplicate
        ↓
按 Group 查看
        ↓
用户选择需要处理的文件
        ↓
Preview
        ↓
确认
        ↓
Recycle
        ↓
查看结果
        ↓
History
        ↓
Undo
```

最终用户感受到的不是：

> “Weave 又多了一个复杂算法工具。”

而是：

> **“Weave 能帮我找出真正重复的文件，而且我完全知道它准备处理什么。”**

---

# 154. M3 核心原则

整个 M3 必须始终遵守：

```text
Correctness > Cleverness
Safety > Convenience
Deterministic > Magical
Preview > Immediate Mutation
Reuse > Rebuild
Streaming > Whole-file Memory
Bounded > Unbounded
Local > Cloud
Facts > Guessing
User Selection > Automatic Deletion
```

最重要的工程原则：

> **宁可少找到，也不能把非重复文件错误地当作重复文件。**

更重要的产品原则：

> **宁可不执行，也不能在用户没有明确选择的情况下替用户删除文件。**

---

# 155. 执行方式

请严格采用：

```text
Audit
 ↓
Design / Gap Analysis
 ↓
Minimal Implementation
 ↓
Unit Tests
 ↓
Integration Tests
 ↓
Fault Tests
 ↓
Performance Tests
 ↓
UI Smoke
 ↓
Security Audit
 ↓
Regression
 ↓
Final Audit
 ↓
Documentation
 ↓
Commit
```

不要一开始就大规模修改。

每一步都基于：

> **当前仓库真实状态**

而不是假设仓库就是 Charter 描述的理想状态。

如果当前实现与本提示词存在差异：

先判断：

```text
FACT
HYPOTHESIS
INFERENCE
```

不要猜。

---

# 156. 最终输出格式

完成整个 milestone 后，输出：

# M3 Implementation Report

## 1. Baseline

```text
Branch:
HEAD:
Working Tree:
M0:
M1:
M2:
```

## 2. Implemented

列出：

```text
Duplicate Scan
Size Filter
Partial Hash
Full Hash
Grouping
Preview
Selection
Recycle
Transaction
History
Undo
UI
```

## 3. Verification

```text
Unit:
Integration:
Fault:
Property:
Performance:
UI:
Build:
```

## 4. Performance Evidence

必须给真实数据：

```text
Dataset
Elapsed
Peak Memory
Candidates
Partial Hash Count
Full Hash Count
Groups
```

## 5. Final Audit

```text
A PASS
B PASS
C PASS
...
```

如果有：

```text
BLOCKED
NOT VERIFIED
```

必须明确说明。

## 6. Known Limitations

真实列出：

```text
Not Implemented
Platform-specific
Deferred
Not Verified
```

## 7. Git

```text
Commit:
Working Tree:
```

## 8. M4 Handoff

说明：

```text
M3 now provides:
...
```

最后只有在所有硬门禁满足后，才能写：

```text
M3 COMPLETE
```

否则：

```text
M3 NOT COMPLETE
```

绝对不要为了形式完成而降低测试、跳过失败、伪造功能或者隐藏限制。