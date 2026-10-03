# ifuyo Weave
# M1 — File Core
## 文件核心、文件事实、目录分析与安全文件系统边界

你现在进入 **ifuyo Weave 的 M1：File Core**。

项目：

`ifuyo Weave`

M0 已经完成后，M1 的目标不是增加大量工具，而是建立整个 Weave 文件处理体系真正可靠的第一层：

```text
Path
 ↓
Filesystem
 ↓
Metadata
 ↓
Hash
 ↓
File Facts
 ↓
Directory Facts
 ↓
File Inspector / Directory Analyzer
```

M1 完成后，Weave 应该第一次具备：

> **“我把一个文件或目录交给 Weave，它能够安全、确定性、可测试地告诉我：它是什么、在哪里、有多大、有什么元数据、目录里有什么，以及这些事实是否可信。”**

这层能力必须成为 M2 Rename / Organizer、M3 Duplicate Finder、后续 Batch Engine 的稳定基础。

---

# 0. M1 的硬边界

## 0.1 本阶段必须完成

### Core

```text
Filesystem Abstraction
Path Safety
File Metadata
Directory Metadata
Hashing
File Facts
Directory Facts
```

### Tools

```text
File Inspector
Directory Analyzer
```

### Infrastructure

```text
Streaming File Access
Cancellation
Progress
Deterministic Results
Structured Errors
Resource Limits
Test Fixtures
Fault Tests
```

### Integration

```text
Rust domain/application
        ↓
Tauri IPC
        ↓
React UI
```

形成真实可运行闭环。

---

# 0.2 本阶段明确禁止

M1 不得实现：

```text
Batch Rename
File Organizer
Duplicate Finder UI
Recycle Bin integration
History
Undo
Preview/Apply for mutation
Workflow Pipeline
Image Resize
Image Compress
Image Convert
CSV tools
JSON tools
Text tools
PDF tools
Archive tools
AI
Natural Language Operations
Cloud
Telemetry
Accounts
Remote File Processing
```

尤其禁止提前实现：

```text
M2 Rename / Organizer
M3 Duplicate Finder
M7 Batch Engine
```

M1 的 Hash 是：

> **通用底层文件事实能力**

不是：

> Duplicate Finder。

可以建立 hash abstraction，但不要在 M1 建立 duplicate groups、delete duplicates、recycle duplicate files 等业务。

---

# 1. M1 核心设计原则

严格遵守：

```text
Local-first
Utility-first
Batch-ready
Preview-first
Reversible-first
Composable
```

M1 重点体现：

```text
Safe-first
Fact-first
Streaming-first
Deterministic-first
```

尤其注意：

> **File Core 只能报告事实，不应该擅自推测用户意图。**

例如：

```text
FACT:
file size = 12.4 MB

FACT:
extension = ".jpg"

FACT:
MIME detection = image/jpeg

FACT:
SHA-256 = ...

INFERENCE:
这个文件可能是照片
```

后者如果没有充分证据，不要当作事实。

---

# 2. 开始前：完整 Baseline Audit

在任何修改前：

先审计真实仓库。

必须执行：

```powershell
git status
git branch
git log --oneline -15
git diff
git ls-files
```

检查：

```text
Cargo.toml
Cargo.lock
workspace
src-tauri/
ui/
weave-core
weave-files
weave-testkit
Tauri configuration
capabilities
IPC contracts
logging
error model
path safety
cancellation
progress
CI
deny/license configuration
```

同时确认 M0 实际完成状态：

```text
Tauri shell
Rust workspace
React
TypeScript
Vite
IPC
error model
logging
testkit
CI
docs
```

然后输出：

# M1 Baseline Audit

## A. Git

```text
branch
HEAD
working tree
last commit
```

## B. M0 Status

```text
implemented
missing
partially implemented
```

## C. Existing File Abstractions

```text
filesystem
path
metadata
logging
error
operation
cancellation
progress
```

## D. Existing UI / IPC

```text
commands
events
payloads
generated types
capabilities
```

## E. Tests

```text
Rust tests
integration tests
frontend tests
fixture tests
```

## F. Risks

只列：

```text
P0
P1
```

不得为了凑数量制造风险。

---

# 3. M0 修复规则

如果 M0 有问题：

不得重做 M0。

只有当问题：

```text
直接阻塞 M1
```

或：

```text
会导致 M1 建立错误架构
```

才修复。

例如：

```text
IPC 无法安全传递路径
Filesystem abstraction 完全缺失
Error model 无法表达 permission failure
Cancellation contract 无法使用
```

非阻塞问题：

记录到：

```text
PROGRESS.md
DECISIONS.md
```

不要顺便大重构。

---

# 4. M1 总体架构

最终保持：

```text
React UI
   ↓
Application Layer
   ↓
weave-core
   ↓
weave-files
   ↓
Filesystem Adapter
   ↓
OS
```

更具体：

```text
File Inspector
       ↓
Inspect Application Service
       ↓
File Fact Resolver
       ↓
Filesystem abstraction
       ↓
OS filesystem
```

Directory Analyzer：

```text
Directory Analyzer
       ↓
Directory Scan Application Service
       ↓
Walker
       ↓
Ignore / Safety Rules
       ↓
Filesystem abstraction
       ↓
OS
```

Hash：

```text
Hash Service
       ↓
streaming read
       ↓
filesystem abstraction
```

---

# 5. crate/module 职责

## 5.1 `weave-core`

只放稳定、通用、与 OS 无关的领域契约。

允许：

```text
FileId
OperationId
Path reference contract
FileType
FileKind
HashAlgorithm
HashResult
FileMetadata
DirectoryMetadata
ScanStatus
Progress
Cancellation
Warnings
Structured Error
```

禁止：

```text
std::fs::read(...)
Windows API
Tauri
React
UI state
具体文件系统实现
```

---

# 5.2 `weave-files`

负责：

```text
filesystem domain
file inspection
directory scanning
metadata collection
hashing
path safety integration
```

这是 M1 的主体。

建议内部拆分：

```text
weave-files/
├── path/
├── filesystem/
├── metadata/
├── hashing/
├── inspector/
├── analyzer/
├── scanning/
└── classification/
```

不要建立一个：

```text
FileManager.rs
```

塞入几千行代码。

---

# 5.3 `weave-testkit`

继续扩展 M0 已有 testkit。

至少支持：

```text
temporary workspace
fixture files
fixture directories
unicode names
large files
zero-byte files
nested trees
read-only cases
permission cases where feasible
collision fixtures
symlink/junction fixtures where feasible
```

---

# 6. Path Safety

这是 M1 最重要的安全基础之一。

所有文件系统访问必须经过：

```text
Input
 ↓
Normalize
 ↓
Validate
 ↓
Resolve where needed
 ↓
Operate
```

禁止：

```text
UI string
 ↓
std::fs
```

---

# 6.1 Path Model

不要简单把路径定义为：

```text
String
```

建议明确区分：

```text
UserPath
NormalizedPath
AbsolutePath
RelativePath
CanonicalizedPath
```

实际命名以现有架构为准。

目标：

> 防止不同层把“字符串路径”随意当作已经验证过的路径。

---

# 6.2 Path Safety 必须覆盖

测试至少覆盖：

```text
.
..
../x
../../x
absolute path
relative path
root path
empty path
whitespace path
unicode path
emoji path
very long path
reserved Windows device names
UNC path
invalid path
non-existent path
existing file
existing directory
symlink
junction where feasible
```

Windows 重点考虑：

```text
C:\
\\server\share
\\?\...
CON
PRN
AUX
NUL
COM1
LPT1
```

不要为了“看起来安全”写过于武断的规则。

例如：

> 合法 Windows 路径不能因为某个字符串形式特殊就直接拒绝。

所有规则必须有测试和理由。

路径比较与规范化必须定义 Windows 大小写不敏感语义：保留用户原始大小写、比较时 case fold、提供大小写不敏感的相等判断与冲突检测契约。

该契约为 M2 碰撞检测的基础，必须有对应测试（含 case-only rename 场景）。

---

# 6.3 Symlink / Junction Policy

M1 必须明确策略。

至少回答：

```text
是否默认跟随 symlink？
是否扫描 junction？
是否避免循环？
如何记录 symlink？
是否允许跨 root？
```

对于 Directory Analyzer，默认应优先：

> **防止递归进入链接目标导致目录循环或扫描范围失控。**

推荐结果模型明确区分：

```text
RegularFile
Directory
Symlink
Other
```

如果某平台无法可靠提供某类信息：

明确：

```text
Unsupported / Unavailable
```

不要伪造。

---

# 7. Filesystem Abstraction

建立一个真正可测试的 filesystem abstraction。

目标不是包装 `std::fs` 的所有 API。

只抽象业务真正需要的边界。

至少表达：

```text
exists
stat
read metadata
open read stream
list directory
```

未来可以扩展：

```text
move
copy
create
delete
write
```

但 M1 不需要为了未来过度设计。

---

# 7.1 Streaming Read

Hash 与未来大型文件处理必须支持：

```text
open
 ↓
read chunk
 ↓
process
 ↓
read chunk
 ↓
...
```

禁止：

```text
read entire file into memory
```

尤其禁止为了计算 hash：

```rust
std::fs::read(path)
```

读取 GB 级文件到内存。

---

# 7.2 Chunk Size

不要硬编码一个“神奇最佳值”。

选择一个合理默认值。

并允许未来统一调整。

记录：

```text
Decision
```

原因。

---

# 7.3 Partial Read / Failed Read

必须正确处理：

```text
0 bytes
short read
interrupted read
permission denied
file locked
file disappeared
file changed during read
I/O failure
```

不能：

```text
read error
→ return empty
```

也不能：

```text
hash failed
→ fake hash
```

---

# 8. File Metadata

File Inspector 是 M1 的主要用户工具。

根据 Charter，至少展示：

```text
Path
Name
Size
Type
Extension
Created
Modified
Accessed
Hash
Permissions
Encoding (where applicable)
Metadata
```

但实现时必须区分：

```text
OS filesystem facts
detected facts
derived facts
```

---

# 8.1 FileMetadata

建议至少支持：

```text
name
relative / absolute path representation
size
extension
file kind
mime / media type when detectable
created_at
modified_at
accessed_at
permissions
hidden flag where available
readonly flag where available
symlink flag
```

具体字段根据平台能力和 M0 已有 contract 决定。

不要建立几百字段的大型万能 FileMetadata。

---

# 8.2 时间字段

明确：

```text
created
modified
accessed
```

的来源。

注意不同操作系统可能：

```text
不可用
精度不同
语义不同
```

不要把平台差异隐藏成“绝对一致”。

对于不可用值：

```text
None / unavailable
```

而不是：

```text
1970-01-01
```

---

# 8.3 Permissions

M1 不要求构建复杂 ACL 编辑器。

只需要可靠展示核心事实。

至少考虑：

```text
readonly
owner information where safely available
basic permissions
```

Windows / Unix 差异必须通过 abstraction 隔离。

不要把 Unix permission bits 直接当成 Windows permission semantics。

---

# 8.4 File Type

不要只依赖：

```text
extension
```

例如：

```text
foo.jpg
```

不能因为后缀是 `.jpg` 就断言内容一定是 JPEG。

应尽可能区分：

```text
Extension
Declared Type
Detected Type
```

例如：

```text
extension = .jpg
detected_type = image/jpeg
```

如果 M1 采用 magic-byte sniffing：

必须：

```text
有边界
有资源限制
有测试
```

不要实现一个巨型 MIME database。

---

# 9. Text Encoding Detection

Charter 中 File Inspector 允许：

```text
Encoding（文本）
```

M1 可以实现：

> **轻量级、有限范围的文本编码识别。**

建议支持：

```text
UTF-8
UTF-8 BOM
UTF-16 LE/BE where detectable
ASCII-compatible cases
```

不要在 M1 建立复杂 encoding framework。

GB18030 / GBK / Latin-1 fallback 属 charter #27 必须识别范围，推迟到 M4 Encoding Detection 补齐。

本里程碑必须在 Known Limitations 与 DECISIONS.md 记录该限制。

File Inspector 对非 UTF 系文本的 Encoding 字段显示 Unknown，不得猜测误报。

如果准确性不足：

显示：

```text
Unknown
```

而不是猜测。

---

# 10. Hash Service

M1 建立通用 Hash service。

至少支持：

```text
SHA-256
```

如果现有依赖已经提供可靠实现，可以复用。

不要因为“以后可能需要”就一次加入：

```text
MD5
SHA-1
SHA-256
SHA-384
SHA-512
BLAKE3
xxHash
CRC32
```

只实现当前确认需要的核心能力。

---

# 10.1 HashResult

结果至少包含：

```text
algorithm
digest
bytes_processed
duration
```

必要时可增加：

```text
cancelled
source_size
```

但不要把结果结构搞得过度复杂。

---

# 10.2 Hash Determinism

同一文件：

```text
same bytes
→
same hash
```

测试必须验证。

---

# 10.3 File Changed During Hash

这是 M1 必须认真处理的问题。

场景：

```text
Open file
 ↓
Hashing
 ↓
file changed
```

必须定义行为。

至少不要：

> 默默返回一个用户以为代表最终文件状态的 hash。

可采用：

```text
best-effort hash + changed-during-read warning
```

或：

```text
detect mismatch and return unstable-file error
```

具体方案依据当前实现选择，但必须：

```text
document
test
```

---

# 10.4 Cancellation

Hash 必须支持：

```text
cooperative cancellation
```

不能：

```text
强杀线程
```

取消后：

```text
Cancelled
```

而不是：

```text
Failed
```

---

# 11. File Inspector

建立真正的工具：

```text
files.inspect
```

输入：

```text
path
options
```

输出：

```text
FileInspectionResult
```

---

# 11.1 File Inspector 输出

至少：

```text
Identity
├── Name
├── Path
└── Kind

Size
├── Bytes
└── Human readable

Type
├── Extension
├── Detected type
└── MIME if available

Time
├── Created
├── Modified
└── Accessed

Attributes
├── Readonly
├── Hidden where available
└── Symlink

Permissions
├── Basic representation
└── Platform notes

Hash
├── Algorithm
├── Digest
└── Status

Encoding
└── when text-like

Warnings
└── all non-fatal uncertainty
```

---

# 11.2 Inspector 必须区分错误与警告

例如：

```text
Hash unavailable
```

不等于：

```text
整个 File Inspector 失败
```

可以：

```text
Metadata = success
Hash = failed
Overall = partial
```

结果模型需要允许：

```text
Complete
Partial
Failed
Cancelled
```

---

# 11.3 非常重要：Inspector 不应该默认读取整个文件

除非用户显式请求 Hash 或特定 metadata。

默认：

```text
stat → metadata
```

应该非常快。

Hash：

```text
explicit expensive operation
```

或者在 UI 中作为单独阶段执行。

---

# 12. Directory Analyzer

建立：

```text
files.analyze_directory
```

对应 Charter 的：

```text
Directory Analyzer
```

至少统计：

```text
Total Files
Total Size
File Type Distribution
Largest Files
Oldest Files
Newest Files
Empty Folders
Directory Depth
```

并为未来报告保留：

```text
Tree
Table
Chart
Report
```

所需数据结构。

M1 不需要做复杂数据可视化。

---

# 12.1 Directory Scan Pipeline

推荐：

```text
Input Path
 ↓
Validate
 ↓
Open Directory
 ↓
Walk
 ↓
Classify Entry
 ↓
Collect Metadata
 ↓
Accumulate Statistics
 ↓
Finalize
```

不要：

```text
一次性把全部 FileInfo 加进巨大 Vec
```

再统计。

原则：

> **统计优先 streaming / incremental accumulation。**

---

# 12.2 Directory Scan Result

建议包含：

```text
scan_id
root
started_at
finished_at
duration
status

directories_scanned
files_scanned
other_entries

total_size

max_depth

empty_directories

file_type_distribution

largest_files
oldest_files
newest_files

warnings
errors
```

具体结构以实际架构为准。

---

# 12.3 Largest / Oldest / Newest

不要保存整个目录所有文件再排序。

例如：

```text
10,000 files
```

如果只需要 Top 20：

维护：

```text
bounded Top-K structure
```

而不是：

```text
Vec<10000>
→ sort
```

尤其为未来：

```text
100k
1m
```

规模保留扩展空间。

---

# 12.4 Empty Folder

定义：

```text
empty = no direct regular files and no direct subdirectories
```

还是别的语义？

必须明确并写入：

```text
ARCHITECTURE.md / DECISIONS.md
```

不要让 UI 自己解释。

---

# 12.5 Directory Depth

定义 root depth：

```text
0
```

还是：

```text
1
```

必须统一。

建议：

```text
root = 0
```

子目录：

```text
root/a = 1
root/a/b = 2
```

测试必须覆盖。

---

# 13. Default Scan Safety

M1 不应该毫无边界地扫描：

```text
整个 C:\
整个用户目录
UNC 巨型共享目录
系统目录
```

但也不能无理由拒绝。

建立：

```text
scan limits
```

至少考虑：

```text
max depth option
max entries guard
resource budget
cancellation
permission error handling
```

如果触发资源上限：

不要：

```text
pretend scan completed
```

必须：

```text
partial / limited
```

并明确：

```text
reason
```

---

# 14. Error Handling

继续使用 M0 的结构化 error model。

M1 至少正确表达：

```text
InvalidPath
NotFound
NotDirectory
PermissionDenied
AlreadyExists
IoFailure
SymlinkCycle
PathTooLong
Unsupported
Cancelled
ResourceLimit
MetadataUnavailable
HashFailure
```

具体 enum/type 以 M0 contract 为准。

禁止：

```rust
Err(anyhow!("something went wrong"))
```

然后把所有错误压成一个通用字符串。

允许：

```text
low-level error
 ↓
domain error
 ↓
application error
 ↓
localized UI message
```

---

# 15. Error Aggregation

Directory Analyzer 不能因为一个文件：

```text
PermissionDenied
```

就直接：

```text
整个扫描失败
```

默认应尽量：

```text
continue
↓
record warning/error
↓
scan remaining entries
```

最终：

```text
998 files scanned
2 files failed
```

这才符合 Weave 的 Batch-first 原则。

但如果发生：

```text
root cannot be opened
```

则：

```text
scan failed
```

必须正确区分。

---

# 16. Determinism

M1 的结果必须尽量 deterministic。

同一个 fixture：

```text
same input
+
same options
+
same filesystem state
```

应该得到：

```text
same facts
same ordering
same statistics
```

因此不要依赖：

```text
filesystem enumeration order
```

作为用户看到的最终排序。

必须明确排序规则。

例如：

```text
relative path ascending
```

或其他 deterministic rule。

---

# 17. Ordering Rules

统一规定：

### File list

```text
relative path ASC
```

### Type distribution

```text
count DESC
then type ASC
```

### Largest files

```text
size DESC
then path ASC
```

### Oldest

```text
timestamp ASC
then path ASC
```

### Newest

```text
timestamp DESC
then path ASC
```

具体排序可根据当前 UI 设计微调，但：

> 必须固定。

---

# 18. IPC Contract

不要让 React 直接获得 Rust 内部对象。

建立明确 DTO：

```text
InspectFileRequest
InspectFileResponse

AnalyzeDirectoryRequest
AnalyzeDirectoryResponse
```

request 必须限制：

```text
path
options
hash settings if enabled
scan limits
```

response 必须是稳定、可序列化的公开 contract。

---

# 18.1 IPC Validation

所有来自 UI 的数据都视为不可信。

检查：

```text
path length
option ranges
enum values
limit bounds
```

不要允许：

```text
frontend → arbitrary shell
frontend → arbitrary filesystem API
```

---

# 19. Tauri Commands

建立最小 commands。

例如：

```text
inspect_file
analyze_directory
hash_file
```

是否拆成三个 command，根据现有 architecture 选择。

原则：

> **Tauri command 只做 orchestration，不写业务逻辑。**

禁止：

```text
#[tauri::command]
fn inspect_file(...) {
    // 500 lines filesystem logic
}
```

---

# 20. UI

M1 必须有最小但真正可用的 UI。

不要继续制作复杂首页。

至少支持：

```text
Drop File
Drop Folder
Open File
Open Folder
```

然后自动判断：

```text
File
or
Directory
```

---

# 20.1 File Inspector UI

推荐结构：

```text
File Inspector
────────────────────────

Name
Path
Type
Size

Created
Modified
Accessed

Permissions

Hash
[Calculate]

Encoding

Metadata

Warnings
```

重点：

> 信息密度高，但不要做成后台管理系统。

---

# 20.2 Directory Analyzer UI

至少：

```text
Directory
Total Files
Total Size
Directories
Max Depth
```

下面：

```text
File Types
Largest Files
Oldest Files
Newest Files
Empty Folders
```

M1 不要求：

```text
复杂图表系统
动画
dashboard
```

可以先：

```text
table
simple bars
basic chart
```

但所有数据显示必须来自真实 Rust 结果。

禁止 mock data。

---

# 21. Drag & Drop

M1 将 Drag & Drop 真正接入文件核心。

流程：

```text
Drop
 ↓
Path Validation
 ↓
Determine File / Directory
 ↓
Dispatch
 ↓
Inspector / Analyzer
```

注意：

> 不允许 UI 自己通过 Node/browser filesystem API 绕过 Rust 边界。

---

# 22. Large File Behavior

至少准备 fixture：

```text
0 B
1 B
1 KB
1 MB
100 MB
```

如果环境允许：

```text
1 GB+
```

测试：

```text
metadata query
hash
cancellation
memory usage behavior
```

重点检查：

> Hash 是否保持 O(chunk size) 内存，而不是 O(file size)。

100MB 及以上的 large fixture 在测试运行时程序化生成（脚本/固定种子），不提交进 git。

仓库只保存生成脚本与小样本。

此为 #55 Git 卫生条款的执行方式。

---

# 23. Fault Injection

M1 必须大量使用 `weave-testkit`。

至少覆盖：

```text
Permission Denied
File Missing During Scan
File Locked
Partial Read
Interrupted Read
Malformed Metadata
Symlink Cycle
Too Many Entries
Path Too Long
Cancellation
```

不能只测 happy path。

---

# 24. Race / Mutation Tests

测试以下场景：

```text
scan starts
 ↓
file deleted
```

以及：

```text
scan starts
 ↓
file renamed
```

以及：

```text
hash starts
 ↓
file modified
```

目标不是让所有竞态都“成功”。

目标是：

> **系统不会静默产生错误事实。**

---

# 25. Test Fixtures

建立至少：

```text
fixture-basic/
├── documents/
├── images/
├── archives/
├── empty/
└── unicode/
```

另建：

```text
fixture-edge/
```

覆盖：

```text
empty file
dotfile
multiple extensions
uppercase extension
no extension
unicode filename
spaces
very long filename
nested directory
empty directory
```

再建：

```text
fixture-large/
```

用于 streaming / performance。

---

# 26. Property / Invariant Tests

至少验证：

### Hash invariant

```text
same content
→ same hash
```

### Size invariant

```text
sum(child file sizes)
=
directory total size
```

注意：

> 如果符号链接策略不跟随，需要按照该策略定义。

### Count invariant

```text
file count
=
counted regular files
```

### Depth invariant

```text
max_depth >= each directory depth
```

### Path invariant

```text
reported relative paths
must stay under selected root
```

---

# 27. Security Tests

必须增加：

```text
path traversal
UNC path
malformed path
very long input
symlink cycle
junction cycle
huge directory
huge file
malicious filename
Unicode normalization edge case
```

尤其：

> 用户控制的 path 永远不能让 scanner 无意中逃逸出预期扫描范围。

---

# 28. Privacy

M1 继续严格保持：

```text
No Telemetry
No Cloud
No Upload
No Account
```

File Inspector / Directory Analyzer 不得：

```text
上传文件
上传 hash
上传 metadata
调用远端 API
```

日志也不得记录：

```text
file contents
secret
password
tokens
```

路径日志默认应尽可能脱敏。

---

# 29. No Network Dependency

M1 必须：

> **完全离线可运行。**

特别检查：

```text
file type detection
hash
metadata
directory analyzer
```

不能隐式依赖互联网。

---

# 30. Performance Design

M1 性能原则：

```text
Metadata = cheap
Hash = explicit / controllable
Directory scan = streaming
UI = non-blocking
Cancellation = responsive
```

不要让：

```text
10,000 file scan
```

冻结 UI。

Rust scanning 通过：

```text
async / worker task / thread
```

等现有架构合理方式执行。

但不要过早建立大型 worker pool。

---

# 31. Progress

Directory Analyzer 必须支持统一 Progress。

例如：

```text
Scanned 1023 entries
```

或者：

```text
1023 / 5000
```

但是：

> 如果 total 不可预知，不要伪造百分比。

可以：

```text
processed = 1023
total = unknown
```

而不是：

```text
20%
```

---

# 32. Cancellation UX

用户点击：

```text
Cancel
```

后：

```text
Running
 ↓
Cancelling
 ↓
Cancelled
```

不要立即假装完成。

结果明确：

```text
Cancelled
Scanned: 1823
Skipped: ...
Errors: ...
```

---

# 33. Partial Result Semantics

Directory Analyzer 必须支持：

```text
Completed
CompletedWithWarnings
Failed
Cancelled
```

不要只有：

```text
success / error
```

这样后续 Batch Engine 才不会重新设计。

---

# 34. File Inspector 的 Hash UX

建议不要默认让每次 Inspector：

```text
打开文件
→ 自动 hash 20 GB 文件
```

默认先显示：

```text
Hash
Not calculated
[Calculate SHA-256]
```

这样：

```text
File Inspector
```

可以立即打开。

点击之后才：

```text
streaming hash
progress
cancel
result
```

---

# 35. Directory Analyzer 的 Hash

M1：

> **不要默认对目录所有文件做 hash。**

Directory Analyzer 第一版统计：

```text
metadata
size
type
time
```

Hash 留给：

```text
File Inspector
```

以及未来：

```text
M3 Duplicate Finder
```

---

# 36. Metadata Limits

任何自动读取的 metadata 都必须有资源边界。

例如：

```text
filename length
xattr size
text sniff bytes
magic detection bytes
```

不要：

```text
read arbitrary metadata blob
```

造成：

```text
memory explosion
```

---

# 37. File Type Classification

建立有限、可扩展分类：

```text
Text
Image
Audio
Video
Document
Archive
Executable
Code
Data
Unknown
```

但：

> 分类属于 derived fact。

必须保留：

```text
classification source/evidence
```

至少在内部让未来可以追溯。

例如：

```text
extension
magic bytes
known filename
```

不要让：

```text
foo.bin
→ Executable
```

凭空出现。

---

# 38. Unknown Handling

真实世界大量文件：

```text
没有扩展名
扩展名错误
类型未知
格式损坏
```

这些必须成为正常状态。

例如：

```text
Type = Unknown
```

而不是：

```text
InternalError
```

---

# 39. Directory Analyzer 的输出层级

M1 保持：

```text
Domain facts
 ↓
Application report
 ↓
UI
```

不要让 UI 自己：

```text
遍历 10,000 items
重新统计
计算 largest
判断 empty folder
```

所有关键统计：

> Rust 完成。

UI 只是展示。

---

# 40. CLI / JSON

如果 M0 已经建立 CLI contract：

M1 应同步接入：

```text
inspect
analyze
```

例如：

```powershell
weave inspect "C:\path\file.txt"
weave analyze "C:\path\folder"
```

支持：

```text
human-readable
--json
```

如果当前 M0 尚未建立 CLI：

不要为了 M1 大量扩展 CLI。

至少建立：

> 可被未来 CLI 消费的 application service。

---

# 41. JSON Contract

JSON 输出必须：

```text
stable
serializable
deterministic
versionable
```

不要把 Rust debug output 当 JSON API。

禁止：

```text
println!("{:?}", result)
```

冒充正式输出。

---

# 42. Documentation

M1 完成后必须更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
SECURITY.md
CHANGELOG.md
```

新增建议：

```text
docs/file-core.md
```

写清：

```text
Filesystem abstraction
Path safety
Metadata model
Hash model
Directory analyzer
Error handling
Cancellation
Performance assumptions
```

---

# 43. Architecture Decision Records

M1 必须记录关键决策。

至少：

```text
Path representation
Symlink policy
Hash algorithm
Hash cancellation
Changed-during-hash policy
Directory scan ordering
Directory size semantics
Empty directory semantics
Metadata failure semantics
File type detection strategy
Encoding detection scope
Resource limits
```

不要只写：

```text
Implemented feature X
```

而要说明：

```text
Decision
Reason
Alternatives
Consequence
```

---

# 44. Dependency Policy

任何新依赖：

必须先检查：

```text
Name
Version
License
Repository
Maintenance
Platform support
Transitive dependencies
```

记录：

```text
Dependency Decision
```

优先：

```text
existing dependency
```

然后：

```text
small mature dependency
```

最后才：

```text
large specialized library
```

不要因为一个很小的 metadata / MIME 判断需求引入大型框架。

---

# 45. No Giant File Library

M1 不要试图建立：

```text
Windows Explorer clone
```

也不要：

```text
File Core = full filesystem platform
```

本阶段只建立：

```text
safe
testable
streaming
extensible
```

的基础。

---

# 46. Integration Test

至少存在完整流程：

## File

```text
Create fixture
 ↓
inspect
 ↓
receive metadata
 ↓
calculate hash
 ↓
verify digest
 ↓
verify deterministic result
```

## Directory

```text
Create fixture
 ↓
analyze
 ↓
verify count
 ↓
verify total size
 ↓
verify type distribution
 ↓
verify largest
 ↓
verify depth
```

---

# 47. Real UI Smoke Test

必须验证：

```text
Launch Weave
 ↓
Drop file
 ↓
File Inspector opens
 ↓
Metadata appears
 ↓
Click Hash
 ↓
Hash completes
```

以及：

```text
Drop folder
 ↓
Directory Analyzer
 ↓
Scan starts
 ↓
Progress updates
 ↓
Results shown
 ↓
Cancel works
```

不得使用 mock data。

---

# 48. Performance Validation

至少做实际 benchmark：

```text
100 files
1,000 files
10,000 files
```

在能做到的环境上实际测量。

至少记录：

```text
elapsed time
peak memory if measurable
files/sec
cancel responsiveness
```

不要凭感觉写：

```text
fast
very fast
excellent
```

所有性能测量结果统一写入 docs/PERF.md（charter #68），不得只散落在 PROGRESS.md 或报告中。

---

# 49. Memory Validation

重点观察：

```text
10,000 files
```

是否：

```text
UI freeze
memory spikes
huge allocations
```

Hash：

```text
large file
```

是否：

```text
memory ~= O(chunk size)
```

而不是：

```text
O(file size)
```

---

# 50. Fault Testing

必须实际构造或尽可能模拟：

```text
PermissionDenied
NotFound
FileDeletedDuringScan
BrokenSymlink
SymlinkCycle
LockedFile
ReadFailure
CancelMidScan
CancelMidHash
InvalidPath
TooManyEntries
```

对于无法稳定模拟的 OS-specific 场景：

明确：

```text
NOT AVAILABLE
MANUAL VERIFICATION REQUIRED
```

不要伪造 PASS。

---

# 51. Regression

M1 不得破坏 M0。

必须重新执行：

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo deny check
```

前端：

根据真实 `package.json`：

```powershell
npm run typecheck
npm run lint
npm test
npm run build
```

如果命令不存在：

```text
NOT AVAILABLE
```

不要伪造。

---

# 52. Build

尽量执行真实：

```powershell
npm run build
```

以及：

```powershell
cargo build
```

如果当前 Tauri 项目支持：

```powershell
npm run tauri build
```

或者项目真实 script 对应命令。

必须记录：

```text
PASS
FAIL
BLOCKED
NOT AVAILABLE
```

---

# 53. Security Gate

M1 最终至少审计：

```text
Path traversal
Symlink escape
UNC handling
Input size limits
Resource limits
IPC validation
No shell execution
No arbitrary network
No secret logging
No file content logging
```

任何 P0：

> 必须修复。

P1：

> 如果影响 M2，则必须修复；否则记录并继续。

---

# 54. Architecture Violation Audit

最终检查：

禁止：

```text
React
 ↓
filesystem
```

禁止：

```text
Tauri command
 ↓
hundreds of lines business logic
```

禁止：

```text
core
 ↓
Tauri
```

禁止：

```text
domain
 ↓
Windows API
```

禁止：

```text
UI
 ↓
raw filesystem path assumptions
```

禁止：

```text
M1
 ↓
Rename implementation
```

禁止：

```text
M1
 ↓
Duplicate Finder implementation
```

---

# 55. Git Hygiene

完成前：

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
temporary reports
cache
build output
large binary fixtures
personal information
```

必须继续保持：

```text
clean .gitignore
```

---

# 56. Commit Strategy

M1 可以拆成多个逻辑 commit，但如果当前项目 workflow 要求一个 milestone commit：

最终使用：

```text
M1: implement file core
```

或者：

```text
feat(files): implement file core
```

具体遵循现有 Git convention。

禁止：

```text
misc changes
update stuff
fix
```

这类无法表达 milestone 意图的提交信息。

---

# 57. PROGRESS.md

M1 完成后必须更新。

至少：

```text
M0 = COMPLETE
M1 = COMPLETE
M2 = NEXT
```

记录：

```text
implemented
tested
known limitations
decisions
performance measurements
```

---

# 58. M1 Acceptance Criteria

只有全部满足才能宣布：

# M1 COMPLETE

```text
[ ] Filesystem abstraction exists
[ ] Path safety is centralized
[ ] File metadata works
[ ] Directory metadata works
[ ] Hash service works
[ ] SHA-256 works deterministically
[ ] Hash is streaming
[ ] Hash cancellation works
[ ] File Inspector works
[ ] Directory Analyzer works
[ ] File type detection is bounded and testable
[ ] Text encoding detection is bounded where implemented
[ ] Symlink policy is explicit
[ ] Symlink cycles cannot cause uncontrolled recursion
[ ] Directory scan is non-blocking
[ ] Progress works
[ ] Cancellation works
[ ] Partial failures are represented correctly
[ ] Errors are structured
[ ] Results are deterministic
[ ] JSON contract works where exposed
[ ] Drag & Drop works
[ ] Real UI smoke test passes
[ ] Fault tests exist
[ ] Large file tests exist
[ ] Fixture suite exists
[ ] No file contents are sent to network
[ ] No telemetry
[ ] No arbitrary shell execution
[ ] No obvious architectural violation
[ ] M0 regression passes
[ ] Rust quality gates pass
[ ] Frontend quality gates pass
[ ] Build passes
[ ] Documentation updated
[ ] DECISIONS updated
[ ] PROGRESS updated
[ ] Git hygiene passes
```

---

# 59. M1 禁止“假完成”

绝对禁止：

```text
fake file inspector
fake hash
fake progress
fake directory statistics
mock UI presented as production
hardcoded metadata
hardcoded counts
hardcoded hashes
dummy JSON
silent error swallowing
```

例如绝对禁止：

```rust
Ok(FileInspection {
    size: 1234,
    hash: "demo"
})
```

如果某项无法实现：

明确：

```text
BLOCKED
```

而不是：

```text
implemented
```

---

# 60. Final Audit

完成实现后，先不要说：

```text
M1 complete
```

必须执行最终审计。

输出：

# M1 Final Audit

## A. Baseline

```text
M0 HEAD
working tree
branch
```

## B. Architecture

```text
weave-core
weave-files
weave-testkit
application
Tauri
UI
```

实际职责和依赖方向。

## C. Path Safety

```text
normalization
validation
traversal
symlink
UNC
long path
platform behavior
```

## D. File Inspector

```text
metadata
type
permissions
times
encoding
hash
warnings
errors
```

## E. Directory Analyzer

```text
walk
counts
size
types
largest
oldest
newest
empty folders
depth
```

## F. Hash

```text
algorithm
streaming
determinism
cancellation
changed-file handling
```

## G. Error Handling

```text
partial failure
root failure
cancellation
structured error
```

## H. Performance

提供真实：

```text
100 files
1k files
10k files
large-file hash
```

测试数据和结果。

## I. Security

只列：

```text
PASS
FAIL
BLOCKED
```

## J. Privacy

确认：

```text
No telemetry
No upload
No network dependency
No sensitive file content logging
```

## K. Tests

列出真实命令和真实结果。

## L. Build

列出真实结果。

## M. Git

```text
branch
HEAD
commit
working tree
```

## N. Decisions

列出 M1 实际做出的关键架构决策。

## O. Known Limitations

只记录真实存在的限制。

## P. M2 Handoff

明确 M2 可以直接复用哪些能力：

```text
Path Safety
Filesystem abstraction
File metadata
Hash service
Directory walker
Operation identity
Progress
Cancellation
Error model
```

以及：

```text
M2 must implement:
Rename
Organizer
Preview
Collision Detection
Undo
History
```

M1 不得抢做这些功能。

---

# 61. 最终执行纪律

整个 M1 严格遵守：

```text
Inspect
 ↓
Understand
 ↓
Plan
 ↓
Implement
 ↓
Test
 ↓
Audit
 ↓
Quality Gate
 ↓
Commit
 ↓
Update PROGRESS
```

每次修改遵守：

```text
最小变更
```

每个工具必须先落 fixtures 与期望输出（golden），再写实现。

对齐 charter #46 的 Specification → Test Fixture → Implementation 顺序。

禁止：

```text
大规模重构
无意义依赖升级
删除测试
吞掉错误
假测试
假结果
TODO placeholder
未来功能偷渡
```

---

# 62. M1 最终产品状态

M1 完成后，用户应该能够：

```text
打开 Weave
 ↓
拖入一个文件
 ↓
看到它的完整基础事实
```

并能够：

```text
拖入一个目录
 ↓
等待扫描
 ↓
看到目录统计
 ↓
看到最大的文件
 ↓
看到最老/最新文件
 ↓
看到类型分布
 ↓
看到空目录
 ↓
看到目录深度
```

同时：

```text
扫描可以取消
扫描不会冻结 UI
错误不会被吞掉
路径不会任意越界
Hash 不会把大文件全部读进内存
结果可以测试
结果可以复现
```

这就是 M1 的真正完成状态。

---

# 63. 最终目标

不要把 M1 做成：

> “一个能查看文件属性的小页面”。

真正目标是：

```text
           Weave File Core
                  │
       ┌──────────┼──────────┐
       ↓          ↓          ↓
 File Inspector Directory   Hash
                  Analyzer
       │          │          │
       └──────────┼──────────┘
                  ↓
        Filesystem Boundary
                  ↓
             Safe Core
                  ↓
        M2 Rename / Organizer
                  ↓
        M3 Duplicate Finder
                  ↓
        M7 Batch Engine
```

M1 是后续所有文件工具的地基。

因此：

> **宁可 M1 少做功能，也不要留下一个无法测试、无法扩展、无法保证安全的 File Core。**

> **开始执行 M1。**
>
> **自主解决实现层面的歧义。**
>
> **以真实代码、真实测试、真实构建结果作为唯一完成依据。**
>
> **未通过 Final Audit 和 Quality Gate，不得宣布 M1 COMPLETE。**