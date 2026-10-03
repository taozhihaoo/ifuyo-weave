# ifuyo Weave — M3 Duplicate Finder 完整开发提示词（下）

> 本文件是原《ifuyo Weave — M3 Duplicate Finder 完整开发提示词》的 **第 2/2 部分**，
> 覆盖原第 85–156 节：weave-testkit 与测试 fixture、单元/集成/跨平台测试与性能基准（85–104）；UX 原则与 Review/Revalidation 等执行保障（105–115）；架构规则、前端状态与渲染、无障碍、No Mock 类纪律与依赖策略（116–128）；文档与安全/隐私/性能审计（129–135）；测试矩阵、不变量、回归与 Build Gates、真实 Smoke（136–146）。
> **执行流程、质量门、Final Audit、Known Limitations、提交纪律、Git Hygiene、M3 Handoff、执行方式与最终输出格式均在本部分（147–156）**。
> **必须与（上）一起阅读执行**：硬边界、领域模型与全部后端实现规范在（上）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

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