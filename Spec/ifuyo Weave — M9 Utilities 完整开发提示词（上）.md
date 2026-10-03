# ifuyo Weave — M9 Utilities 完整开发提示词（上）

> 本文件是原《ifuyo Weave — M9 Utilities 完整开发提示词》的 **第 1/2 部分**，
> 覆盖原第 0–104 节：M9 核心定义、硬边界、最重要的原则、产品定位与明确禁止、与 M1/M4/M6/M7/M8/M10 的关系，第一阶段 Baseline Audit、审计输出格式与状态标签、P0/P1，Utility Domain Architecture、Utility Layering 与"不允许 React 直接实现核心语义"、Utility Capability Model、输入模型、输出模型、Error Model、Diagnostics，Hash（Algorithm/语义/Text/File Hash/Output/Copy/Security）、Checksum（Algorithm/Capability Matrix/CRC/Test Vectors）、Base64（编码语义/Text/Decode/File/MIME 换行/Decode Security）、UUID（版本/v4/v7/Batch/输出格式/Validation）、Timestamp（单位/解析/Human Time/时区/DST/精度/相对显示）、URL Encoder / Decoder（Component/Query/Full URL/Decode/Unicode）、Regex Tester（引擎统一/Capability Matrix/Match Result/Flags/Replace Preview/资源限制/灾难性回溯/Fuzz）、Color Converter（格式/色彩空间基线/HEX/RGB/Alpha/HSL/HSV/HWB/Parsing/Normalization/精度/Round-trip/Contrast），Clipboard/History/Export，Batch-readiness 与 Batch 通过 M7、Utility Result Table、Large Result Handling，以及 Determinism 全套（Randomness Audit、Time Provider、URL/Regex/Color Determinism）。
> **必须与（下）一起阅读执行**：UI/UX 与 a11y/i18n、测试架构与 Golden/Property/Fuzz 测试矩阵、安全与隐私、依赖审计、基准与质量门、文档、Final Audit A–S、验收标准、推荐实施顺序、Final Audit 与提交纪律、M9→M10 交接条款均在（下）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---
# ifuyo Weave — M9 Utilities 完整开发提示词

> **Milestone：M9 — Utilities**
>
> 本提示词用于指导 Claude Code / Codex / 其他 AI Coding Agent 在真实 `ifuyo Weave` 仓库上实施 M9。
>
> 本阶段必须基于当前真实代码进行审计、复用和增量实现。
>
> **禁止假设 M0–M8 已经实现。**
>
> **禁止为了“完成 M9”而重写既有基础设施。**
>
> **禁止为了“看起来功能很多”而扩大范围。**

---

# 0. M9 的核心定义

M9 的职责不是：

```text
再增加一批杂乱的小工具
```

而是建立：

> **一组统一、可靠、可测试、可组合的基础 Utilities。**

核心工具：

```text
Hash
Checksum
Base64
UUID
Timestamp
URL Encode / Decode
Regex Tester
Color Converter
```

它们共同构成：

```text
                    ifuyo Weave
                         │
                         ▼
                     Utilities
                         │
       ┌─────────────────┼─────────────────┐
       ▼                 ▼                 ▼
   Encoding          Identity           Parsing
       │                 │                 │
   Base64            UUID              Timestamp
       │                                   │
       ▼                                   ▼
    Hash /                         URL / Regex
   Checksum                            │
                                       ▼
                                    Color
```

M9 的目标不是：

> “拥有 8 个工具页面。”

而是：

> **建立一套真正可被其他工具、Batch Engine 和未来 Workflow 复用的通用 Utility capabilities。**

---

# 1. M9 硬边界

## 1.1 本阶段必须完成

必须至少完成：

```text
Utility Foundation
Utility Capability Model

Hash Generator
Checksum Tool
Base64 Tool
UUID Generator
Timestamp Converter
URL Encoder / Decoder
Regex Tester
Color Converter
```

并且全部具备：

```text
Real Implementation
Deterministic Behavior
Structured Input
Structured Output
Diagnostics
Error Handling
Tests
UI Integration
IPC Integration
Accessibility
i18n
```

在适用工具中提供：

```text
Preview
Copy
Copy All
Export
Batch Processing
```

但不得为了“统一体验”强行为所有 Utility 引入文件 mutation。

---

# 2. M9 最重要的原则

整个 M9 必须继续遵守：

```text
Local-first
Utility-first
Preview-first
Deterministic-first
Safety-first
Fact-first
Bounded-first
Reuse-first
Minimal-dependency
Composable
```

同时遵守：

```text
Correctness > Feature Count
Semantic Correctness > Convenience
Explicit Input > Magic
Actual Capability > Marketing Claim
Reuse > Rebuild
Bounded > Unlimited
Local > Cloud
Real Result > Mock Result
Stable Semantics > UI Tricks
```

---

# 3. M9 产品定位

用户在日常开发、调试、文件处理过程中，经常会遇到：

```text
这个字符串的 SHA-256 是什么？
这个 Base64 解码后是什么？
这个 UUID 怎么生成？
这个 Unix 时间戳对应什么时间？
这个 URL 参数应该怎么编码？
这个正则为什么匹配不到？
这个颜色 HEX 和 HSL 怎么转换？
这个文件的 checksum 是否变化？
```

这些任务目前经常需要：

```text
打开搜索引擎
打开在线工具网站
复制敏感数据
切换多个网页
```

Weave 应该把这些高频动作收拢到：

```text
Local Utility
```

因此：

> **M9 的价值是减少日常开发 / 文件处理中的小摩擦。**

不是建立一个：

```text
“万能开发者工具平台”
```

---

# 4. M9 明确禁止

M9 不得实现：

```text
Online API Client
HTTP Client
API Testing Platform
Postman Clone
Database Client
SQL IDE
SSH Client
Terminal Emulator
Code Runner
Script Runner
JavaScript Evaluator
Python Evaluator
Shell Executor
AI Assistant
LLM Regex Generator
LLM Data Converter
Cloud Encoding Service
Remote Hashing
Telemetry
Account
Cloud Sync
Online Clipboard Sync
Password Manager
Secret Manager
Certificate Manager
JWT Platform
OAuth Debugger
Full Developer IDE
Full Color Grading Suite
Professional Color Management
Image Editor
Workflow Editor
Visual Workflow Canvas
DAG Engine
Plugin SDK
Automation Platform
Cron Scheduler
Search Engine
```

尤其禁止偷做：

```text
M10 Workflow
M11 Polish
```

中的内容。

---

# 5. M9 与 M1 的关系

M1 已经建立：

```text
Filesystem
Path Safety
Metadata
Streaming
Hash Foundation
File Inspector
Directory Analyzer
```

因此 M9 不得重新建立：

```text
Second File Hash Engine
Second Streaming Layer
Second Filesystem Abstraction
```

必须首先检查 M1：

```text
Hash capability
Hash algorithms
Streaming interface
Cancellation
Progress
File access
```

然后判断：

```text
REUSE
EXTEND
REFACTOR
REPLACE
```

原则：

> **M9 Hash 是用户工具层，M1 Hash 是基础 capability。**

即：

```text
M1
    ↓
Hash Capability
    ↓
M9 Hash UI / Application Tool
```

而不是：

```text
M1 Hash
+
M9 Hash
=
Two Hash Engines
```

---

# 6. M9 与 M4 的关系

M4 已经建立：

```text
TextDocument
Encoding
Diagnostics
Regex-related text handling
Text I/O
Preview
```

M9 必须复用：

```text
TextDocument
Encoding
Diagnostics
```

尤其是：

```text
UTF-8
UTF-16
BOM
Line Ending
```

不要重新定义：

```text
StringInput
EncodingInput
TextError
```

---

# 7. M9 与 M6 的关系

M6 已经有：

```text
Metadata
Binary File Handling
Image Data
```

M9 不应进入：

```text
Image Editing
```

Color Converter 处理的是：

```text
Color Value
```

而不是：

```text
Image Pixel Processing
```

例如：

```text
#FF8800
```

可以进入：

```text
Color Converter
```

但是：

```text
10000 images
→ replace orange pixels
```

不是 M9。

---

# 8. M9 与 M7 的关系

M7 是：

```text
Execution
Batch
Retry
Resume
Journal
Progress
Cancellation
```

M9 是：

```text
Tool Semantics
```

因此：

```text
M9 不实现 Batch Engine
```

但 M9 的工具必须尽量设计成：

```text
Batch-ready
```

例如：

```text
10 files
→ SHA-256
→ result table
```

可以通过：

```text
M7 Batch Engine
```

实现。

结构应该是：

```text
M9 Utility
      ↓
Capability
      ↓
M7 Batch Engine
```

不是：

```text
M9
 ↓
another BatchEngine
```

---

# 9. M9 与 M8 的关系

M8 建立：

```text
Document Detection
Document Inspection
PDF
DOCX
XLSX
PPTX
```

M9 可以复用：

```text
DocumentDetector
FileFacts
Metadata
Diagnostics
```

但是：

```text
M9 = Generic Utility
M8 = Document Semantics
```

不要把：

```text
PDF
DOCX
XLSX
PPTX
```

逻辑塞进：

```text
weave-utilities
```

---

# 10. M9 与 M10 的硬边界

M9 可以提供：

```text
Utility Capability
```

M10 才负责：

```text
Workflow Composition
```

例如：

```text
File
 ↓
Hash
 ↓
Filter
 ↓
Rename
 ↓
Export
```

如果是固定工具内部使用，可以。

如果变成：

```text
用户自由拖拽
连接
保存
复用
条件分支
循环
```

则属于：

```text
M10
```

M9 不得提前实现：

```text
Workflow Graph
Workflow Node
Workflow Editor
Workflow Persistence
Conditional Workflow
Loop
Branch
```

---

# 11. 第一阶段：Baseline Audit

在任何代码修改之前：

```text
DO NOT MODIFY CODE
```

先完整审计：

```text
git status
git branch
HEAD
working tree
Cargo workspace
crate structure
frontend structure
Tauri commands
IPC DTOs
shared types
error system
logging
config
i18n
theme
clipboard handling
filesystem layer
M1 hash
M4 encoding
M4 text
M7 batch
M8 documents
tests
CI
docs
```

检查是否已经存在：

```text
hash utilities
checksum utilities
base64 helpers
UUID helpers
timestamp helpers
URL helpers
regex helpers
color parsers
clipboard helpers
copy components
export components
```

严禁：

> 看到工具不存在就立即新写。

先确认：

```text
是否已有相同 capability
是否已经存在于其他 crate
是否已有 shared abstraction
是否存在 UI-only implementation
是否存在不完整实现
```

---

# 12. 审计输出格式

必须区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

```text
FACT:
M1 already provides streaming SHA-256.

FACT:
Frontend has clipboard abstraction.

HYPOTHESIS:
The current UUID helper may be unsuitable for batch generation.

INFERENCE:
M9 can reuse M1 hash core and expose a separate application-layer tool.
```

不得把：

```text
“我认为”
```

写成：

```text
FACT
```

---

# 13. 状态标签

所有审计项统一使用：

```text
CONFIRMED
RESOLVED
OPEN
WORSENED
NOT APPLICABLE
DEFERRED
```

问题严重等级仅允许：

```text
P0
P1
```

---

# 14. P0

P0 包括：

```text
数据错误
结果错误
安全漏洞
路径越界
任意代码执行
Regex 导致无界资源消耗
严重内存风险
错误的 Hash 结果
错误的 Timestamp 解析
错误的 URL encoding semantics
错误的 Color conversion semantics
应用崩溃
数据泄漏
M1–M8 regression
Architecture boundary violation
```

---

# 15. P1

P1 包括：

```text
缺失核心 capability
重大 UX 问题
不可解释错误
无法取消
无法复制结果
批处理无法正确接入
i18n 缺失
Accessibility 缺失
性能明显不合理
Documentation 缺失
License metadata 不完整
```

不要创造：

```text
P2
P3
P4
```

---

# 16. Utility Domain Architecture

建议结构：

```text
weave-core
weave-files
weave-text
weave-data
weave-media
weave-documents
weave-batch
weave-history
weave-search
weave-testkit
weave-utilities
```

M9 新增：

```text
weave-utilities
```

但是否新增 crate：

> 必须依据真实仓库状态决定。

如果已有：

```text
weave-utils
```

或：

```text
weave-utilities
```

则：

```text
REUSE
```

不得建立：

```text
weave-utils
+
weave-utilities
```

两个职责高度重叠的 crate。

---

# 17. Utility Layering

推荐：

```text
UI
 ↓
Application Command
 ↓
Utility Service
 ↓
Utility Domain
 ↓
Existing Core Capability
```

例如 Hash：

```text
UI
 ↓
hash_file command
 ↓
HashService
 ↓
M1 Hash Capability
 ↓
Filesystem stream
```

Base64：

```text
UI
 ↓
Base64Service
 ↓
Base64Domain
```

UUID：

```text
UI
 ↓
UuidService
 ↓
UuidDomain
```

---

# 18. 不允许 React 直接实现核心语义

禁止：

```text
React
  ↓
hash string
```

禁止：

```text
React
  ↓
timestamp conversion
```

禁止：

```text
React
  ↓
regex execution
```

禁止：

```text
React
  ↓
color conversion
```

核心语义必须位于：

```text
Rust
```

或项目已经验证存在的：

```text
shared domain capability
```

React 只负责：

```text
Input
Presentation
Interaction
Clipboard
Navigation
```

---

# 19. Utility Capability Model

建议统一：

```text
UtilityId
UtilityCategory
UtilityCapability
UtilityInput
UtilityOutput
UtilityDiagnostic
UtilityError
```

至少支持：

```text
CanInspect
CanTransform
CanGenerate
CanDecode
CanEncode
CanBatch
CanExport
```

不要为了“统一”强迫所有工具实现无意义接口。

---

# 20. 输入模型

每个 Utility 都应明确：

```text
Input Type
Input Constraints
Input Encoding
Input Size
Input Normalization
```

例如：

```text
Hash:
String
File
Bytes
```

Base64：

```text
Text
Bytes
File
```

UUID：

```text
Count
Version
Format
```

Timestamp：

```text
Timestamp
DateTime
Timezone
```

Regex：

```text
Pattern
Input Text
Flags
```

Color：

```text
Color String
```

---

# 21. 输出模型

统一考虑：

```text
value
display_value
normalized_value
diagnostics
warnings
metadata
```

例如 Color：

```text
input
normalized
hex
rgb
hsl
alpha
```

Regex：

```text
match
start
end
groups
named_groups
```

Hash：

```text
algorithm
digest
bytes_processed
```

---

# 22. Error Model

不得直接返回：

```text
"error"
```

必须结构化：

```text
UtilityError
```

至少包含：

```text
code
message
kind
recoverable
details
```

UtilityError 最终必须映射到统一 Weave Error（charter #25：Code / Message / Location / Recoverability / Suggestion）。

错误类型建议：

```text
InvalidInput
UnsupportedEncoding
UnsupportedAlgorithm
InvalidPattern
InvalidTimestamp
InvalidTimezone
InvalidUrl
InvalidColor
ResourceLimitExceeded
IoError
InternalError
```

---

# 23. Diagnostics

区分：

```text
Error
Warning
Info
```

例如：

```text
MD5 is cryptographically broken for collision resistance.
```

应是：

```text
Warning
```

而不是：

```text
Error
```

因为用户可能就是为了：

```text
legacy checksum compatibility
```

而需要 MD5。

---

# 24. Hash Tool

M9 Hash 工具必须首先审计 M1。

必须建立真实：

```text
Hash Capability Matrix
```

至少显示：

```text
Algorithm
Text
Bytes
File
Streaming
Batch
Availability
Security Note
```

---

# 25. Hash Algorithm

具体算法不得凭想象声明。

必须根据实际 Rust dependency 与实现确认。

建议能力至少评估：

```text
MD5
SHA-1
SHA-224
SHA-256
SHA-384
SHA-512
BLAKE3
```

但：

> **只把真实实现并正确测试过的算法标记为 Supported。**

不强制要求全部存在。

---

# 26. Hash 语义

必须区分：

```text
Text Hash
Byte Hash
File Hash
```

例如：

```text
"abc"
```

如果 UTF-8：

```text
61 62 63
```

Hash 必须基于：

```text
actual encoded bytes
```

而不是：

```text
字符数量
```

---

# 27. Text Hash

UI 必须明确：

```text
Input
Encoding
Algorithm
Output Case
```

例如：

```text
Input:
hello

Encoding:
UTF-8

Algorithm:
SHA-256
```

避免用户误以为：

```text
String
```

天然具有唯一字节表示。

---

# 28. File Hash

必须复用：

```text
M1 streaming hash
```

不得：

```text
read whole file into memory
```

必须支持：

```text
large file
progress
cancellation
```

如果当前 M1 已支持。

---

# 29. Hash Output

默认：

```text
lowercase hexadecimal
```

但如果已有产品规范不同：

```text
follow existing convention
```

可支持：

```text
lowercase
uppercase
```

禁止把：

```text
hex
base64
```

混为同一输出编码。

---

# 30. Hash Copy

支持：

```text
Copy
Copy All
```

多文件结果可以显示：

```text
Path
Algorithm
Digest
```

不要把完整文件内容存进：

```text
History
```

---

# 31. Hash Security Notes

UI 对已知弱算法必须给出准确提示：

```text
MD5:
not suitable for collision-resistant security purposes

SHA-1:
not suitable for collision-resistant security purposes
```

但不得把：

```text
legacy compatibility hash
```

禁掉。

---

# 32. Checksum Tool

Checksum 与 Hash 必须明确区分。

Hash：

```text
cryptographic / content digest
```

Checksum：

```text
integrity/error-detection oriented
```

M9 必须避免：

> “Checksum 只是 Hash 的另一个名字。”

---

# 33. Checksum Algorithm

根据实际 dependency 支持能力评估：

```text
CRC32
CRC32C
Adler-32
```

以及仓库中实际已经存在且可靠的其他 checksum 算法。

不要为了功能数量引入大量重复算法实现。

---

# 34. Checksum Capability Matrix

至少记录：

```text
Algorithm
Input
Incremental
Streaming
Known Standard
Output Format
```

---

# 35. CRC Semantics

CRC 绝不能仅仅写：

```text
CRC32
```

然后内部随便选择参数。

必须明确：

```text
Polynomial
Initial Value
Reflect In
Reflect Out
Xor Out
Check Value
```

或引用已验证的标准定义。

例如：

```text
CRC-32/ISO-HDLC
CRC-32C
```

必须严格区分。

---

# 36. Checksum Test Vectors

必须使用：

```text
known standard test vectors
```

例如：

```text
"123456789"
```

对应标准 checksum。

所有算法必须有 golden tests。

---

# 37. Base64 Tool

必须支持：

```text
Encode
Decode
```

并明确输入：

```text
Text
Bytes
File
```

---

# 38. Base64 Encoding Semantics

必须区分：

```text
Standard Base64
URL-safe Base64
```

以及：

```text
Padded
Unpadded
```

不得把：

```text
base64url
```

误称为：

```text
普通 Base64
```

---

# 39. Base64 Text

默认：

```text
UTF-8
```

但允许用户明确选择：

```text
UTF-8
UTF-16LE
UTF-16BE
Latin-1
GB18030
```

具体支持范围以真实实现为准。

GB18030 复用统一编码层（charter #27）。

---

# 40. Base64 Decode

Decode 必须严格验证：

```text
alphabet
padding
length
invalid characters
```

不得：

```text
silent repair
silent truncation
```

除非工具明确提供：

```text
Lenient Decode
```

并清楚告诉用户。

---

# 41. Base64 File

支持：

```text
File → Base64
Base64 → File
```

但必须有：

```text
resource limits
streaming where practical
output size estimation
```

禁止把超大文件完整一次性装入：

```text
RAM
```

---

# 42. Base64 MIME / Line Wrapping

默认不要自动插入：

```text
line breaks
```

除非用户明确选择：

```text
wrapped output
```

如支持：

```text
76-character wrap
```

必须作为显式选项。

---

# 43. Base64 Decode Security

必须考虑：

```text
Base64 expansion
```

因为：

```text
small encoded input
```

并不意味着：

```text
safe decoded output
```

必须设置：

```text
Max decoded bytes
```

---

# 44. UUID Tool

M9 UUID 工具必须首先定义：

```text
Generate
Inspect
Format
```

---

# 45. UUID Versions

优先评估：

```text
UUID v4
UUID v7
```

其他版本仅在确有需要且实现稳定时加入：

```text
v1
v3
v5
```

不得为了数量强行实现。

---

# 46. UUID v4

v4 必须使用：

```text
cryptographically secure random source
```

而不是：

```text
Math.random
timestamp
incrementing counter
```

---

# 47. UUID v7

如果实现：

```text
UUID v7
```

必须真实遵守标准语义。

必须明确：

```text
timestamp component
random component
version bits
variant bits
```

并使用可靠随机源补足随机部分。

---

# 48. UUID Batch

支持：

```text
Count
```

例如：

```text
10
100
1000
```

但必须有：

```text
reasonable upper bound
```

防止：

```text
1,000,000,000 UUIDs
```

这种请求直接导致 UI / memory exhaustion。

---

# 49. UUID Output Formats

支持：

```text
xxxxxxxx-xxxx-xxxx-xxxx-xxxxxxxxxxxx
```

可选：

```text
lowercase
uppercase
braces
```

如支持：

```text
compact
```

则：

```text
32 hex digits
```

必须明确。

---

# 50. UUID Validation

UUID 页面可以提供：

```text
Validate
Parse
Version
Variant
```

但不要偷偷变成：

```text
Full UUID Inspector Platform
```

---

# 51. Timestamp Tool

Timestamp 必须同时考虑：

```text
Unix Timestamp
Human DateTime
Timezone
```

---

# 52. Timestamp Units

至少考虑：

```text
seconds
milliseconds
microseconds
nanoseconds
```

但只有在底层时间范围与精度真实可靠时才标记为 Supported。

---

# 53. Timestamp Parsing

输入例如：

```text
1720000000
1720000000000
```

不得仅通过：

```text
digits length
```

盲猜。

必须允许用户明确指定：

```text
Unit
```

并可提供：

```text
auto-detect
```

但 Auto Detect 必须显示：

```text
Detected as milliseconds
```

而不是静默决定。

---

# 54. Human Time Parsing

至少支持：

```text
RFC 3339
ISO 8601 compatible forms actually supported
```

例如：

```text
2026-10-03T00:00:00Z
```

必须明确：

```text
UTC
```

---

# 55. Timezone

必须区分：

```text
UTC
Fixed Offset
System Local Time
Named Timezone
```

不要把：

```text
local timezone
```

误当成：

```text
UTC
```

---

# 56. DST

如果支持 Named Timezone：

```text
America/New_York
Asia/Singapore
Europe/London
```

必须处理：

```text
DST transitions
ambiguous local time
nonexistent local time
```

不能简单：

```text
+8
+9
```

硬编码。

---

# 57. Timestamp Precision

必须明确：

```text
actual precision
```

如果系统只可靠到：

```text
milliseconds
```

不要宣称：

```text
nanosecond exactness
```

---

# 58. Timestamp Relative Display

可以显示：

```text
UTC
Local
Relative
```

例如：

```text
2026-10-03 00:00:00 UTC
2026-10-03 08:00:00 Asia/Singapore
```

但：

```text
“2 hours ago”
```

仅作为 UI presentation。

核心结果必须仍然是：

```text
exact datetime
```

---

# 59. URL Encoder / Decoder

URL Utility 必须首先区分：

```text
Percent Encoding
URL Parsing
Query Encoding
Component Encoding
Form Encoding
```

不要把所有输入一律：

```text
encodeURIComponent
```

然后声称：

```text
URL Encode
```

---

# 60. URL Component

至少支持：

```text
encode component
decode component
```

输入：

```text
hello world
```

结果：

```text
hello%20world
```

---

# 61. Query Parameters

如果提供：

```text
Query Parameter Encode
```

必须使用明确的 query semantics。

区分：

```text
space = %20
```

与：

```text
application/x-www-form-urlencoded
```

可能出现的：

```text
+
```

两者不得混淆。

---

# 62. Full URL Parsing

可以增加轻量：

```text
Parse URL
```

显示：

```text
scheme
userinfo
host
port
path
query
fragment
```

但不得变成：

```text
Full HTTP Client
```

---

# 63. URL Decode

Decode 遇到：

```text
malformed %
invalid escape
```

必须：

```text
error
```

或用户明确选择：

```text
lenient
```

不能：

```text
silently drop bytes
```

---

# 64. Unicode URL

必须测试：

```text
中文
emoji
accented characters
spaces
reserved characters
```

例如：

```text
你好 world
```

确保：

```text
UTF-8 bytes
percent encoded
```

---

# 65. Regex Tester

Regex Tester 是 M9 中资源安全要求最高的工具之一。

它必须是：

```text
Pattern
+
Input
+
Flags
→
Matches
```

而不是：

```text
在线代码执行器
```

---

# 66. Regex Engine 必须统一

最重要原则：

> **Preview 中显示的 regex 结果，必须来自与最终执行语义一致的 regex engine。**

禁止出现：

```text
Frontend JavaScript Regex
```

与：

```text
Rust Regex
```

两边结果不同。

---

# 67. Regex Capability Matrix

必须明确：

```text
Lookahead
Lookbehind
Backreferences
Named Groups
Unicode
Multiline
Dotall
Case Insensitive
Extended Mode
```

哪些是：

```text
Supported
Unsupported
Partial
```

必须真实说明。

---

# 68. 不得模拟 Regex Capability

禁止：

```text
Rust regex 不支持 lookbehind
→ 自己写一堆 hack
→ 声称支持
```

应：

```text
Capability = Unsupported
```

如果产品未来需要第二套 engine：

```text
必须经过架构评估
```

不能在 M9 中临时偷偷加入。

---

# 69. Regex Match Result

每个 match 至少包含：

```text
full match
start offset
end offset
line
column
```

以及：

```text
capture groups
named groups
```

若 engine 支持。

---

# 70. Regex Flags

至少考虑：

```text
i
m
s
```

具体支持以底层 engine 为准。

UI 必须：

```text
flag
meaning
```

清楚可见。

---

# 71. Regex Replace Preview

可以提供：

```text
Find
Replace
Preview
```

但如果做：

```text
Regex Replace
```

必须复用 M4 的：

```text
Text Transformer
```

而不是复制第二套替换逻辑。

---

# 72. Regex Resource Limits

必须设置：

```text
Max pattern length
Max input length
Max match count
Max execution time
```

或者使用：

```text
regex engine with guaranteed/bounded behavior
```

具体采用方式必须基于真实 engine 能力说明。

---

# 73. Regex Catastrophic Backtracking

这是：

```text
P0
```

级安全问题。

必须验证：

```text
恶意 Pattern
+
恶意 Input
```

不会导致：

```text
UI freeze
CPU exhaustion
memory explosion
```

---

# 74. Regex Fuzz Test

至少测试：

```text
random patterns
random strings
invalid patterns
unicode
very long input
empty input
zero-width matches
repeated groups
```

并确保：

```text
no crash
bounded execution
deterministic diagnostics
```

---

# 75. Color Converter

Color Converter 处理：

```text
Color Values
```

而不是：

```text
Image Pixels
```

---

# 76. Color Formats

建议基础支持：

```text
HEX
RGB
RGBA
HSL
HSLA
HSV / HSB
HWB
```

具体范围必须根据：

```text
real implementation
test coverage
UX value
```

决定。

---

# 77. Color Space 基线

M9 默认以：

```text
sRGB
```

作为基础工作空间。

必须明确：

```text
Color model
Color space
```

不要把：

```text
RGB
```

直接写成：

```text
color space
```

---

# 78. HEX

支持：

```text
#RGB
#RGBA
#RRGGBB
#RRGGBBAA
```

具体支持范围必须有测试。

---

# 79. RGB

至少支持：

```text
0–255
```

如果提供：

```text
0–100%
```

必须明确这是：

```text
UI input representation
```

而不是改变底层颜色语义。

---

# 80. Alpha

Alpha 必须统一：

```text
0 = transparent
1 = opaque
```

如果 UI 显示：

```text
0–100%
```

必须稳定转换。

---

# 81. HSL

必须正确处理：

```text
Hue
Saturation
Lightness
```

其中：

```text
Hue
```

一般表示：

```text
degrees
```

必须明确：

```text
0–360
```

的边界策略。

---

# 82. HSV / HSB

如果支持：

```text
HSV
```

必须明确：

```text
Hue
Saturation
Value
```

不得把：

```text
HSV
```

误写成：

```text
HSL
```

---

# 83. HWB

如果支持：

```text
HWB
```

必须正确处理：

```text
Hue
Whiteness
Blackness
```

并测试：

```text
W + B >= 100%
```

的规范化语义。

---

# 84. Color Parsing

必须支持：

```text
strict parsing
```

例如：

```text
#12G45F
```

必须明确失败。

不能：

```text
remove invalid chars
```

然后继续产生一个看似合理的颜色。

---

# 85. Color Normalization

例如用户输入：

```text
#ff8800
```

可以输出：

```text
#FF8800
```

但必须区分：

```text
presentation normalization
```

与：

```text
semantic conversion
```

---

# 86. Color Conversion Precision

颜色转换过程中涉及：

```text
floating-point
rounding
```

必须统一：

```text
rounding policy
```

例如：

```text
RGB
→
HSL
→
RGB
```

应通过：

```text
tolerance-based tests
```

而不是要求所有中间浮点完全 bit-identical。

---

# 87. Color Round-trip

必须测试：

```text
HEX
→ RGB
→ HEX

RGB
→ HSL
→ RGB

RGB
→ HSV
→ RGB
```

确保：

```text
误差在明确定义的 tolerance 内。
```

---

# 88. Color Contrast

只有在实际 scope 仍然合理时，可以考虑：

```text
Contrast Ratio
```

但它必须是：

```text
M9 Color Utility
```

的独立 capability。

不要因此开始实现：

```text
Accessibility Color Design Suite
```

---

# 89. Clipboard

M9 中大量工具需要：

```text
Copy
Paste
Copy All
```

必须复用已有：

```text
Clipboard abstraction
```

不得创建：

```text
second clipboard layer
```

---

# 90. Clipboard Privacy

对于：

```text
Hash
Base64
Regex
UUID
Timestamp
Color
```

用户复制的数据：

```text
不得上传
不得遥测
不得进入网络日志
```

---

# 91. History

M9 大部分 Utility 是：

```text
pure transformation
```

因此默认：

```text
不必把每一次转换都写入 History
```

例如：

```text
HEX → RGB
```

没有必要生成：

```text
Undo transaction
```

---

# 92. History 边界

需要持久化的通常是：

```text
file mutation
```

而不是：

```text
stateless utility calculation
```

如果某个 M9 Tool 产生：

```text
File Output
```

则必须复用：

```text
M2 Safe Write
History
Undo
M7 execution
```

---

# 93. Export

Utility 可以提供：

```text
Save
Export
```

但：

```text
Export to file
```

必须走：

```text
M2 Safe Write
```

不能在 UI 层：

```text
直接 fs::write
```

---

# 94. Batch-readiness

以下 Utility 应尽量具备：

```text
Batch-ready
```

包括：

```text
Hash
Checksum
Base64
Regex
```

但实际是否支持：

```text
Batch
```

必须根据：

```text
input/output semantics
```

决定。

---

# 95. Batch 通过 M7

例如：

```text
100 files
→ SHA-256
```

流程：

```text
M7 Batch Job
 ↓
Input
 ↓
M9 Hash Capability
 ↓
Result
```

而不是：

```text
M9 HashBatchEngine
```

---

# 96. Utility Result Table

对于适合表格展示的工具：

```text
Path
Input
Algorithm
Result
Status
Duration
```

建议使用：

```text
virtualized table
```

避免：

```text
10,000 results
```

直接渲染成：

```text
10,000 React nodes
```

---

# 97. Large Result Handling

必须考虑：

```text
large hash result sets
large regex match sets
large UUID batch
large Base64 result
```

任何工具都有：

```text
resource limits
```

---

# 98. Determinism

必须做到：

```text
same input
+
same options
```

产生：

```text
same result
```

例外：

```text
UUID generation
```

因为：

```text
randomness
```

属于明确的工具语义。

---

# 99. Randomness Audit

对于：

```text
UUID v4
UUID v7
```

必须审计：

```text
RNG source
entropy source
thread safety
platform behavior
```

不得使用：

```text
predictable PRNG
```

生成安全 UUID。

---

# 100. Timestamp Determinism

Timestamp 当前时间工具如果提供：

```text
Now
```

则：

```text
current time
```

本身是变化的。

测试不得直接断言：

```text
exact output
```

而应该：

```text
mock clock
```

或：

```text
inject clock provider
```

---

# 101. Time Provider

建议：

```text
Clock trait
```

例如：

```text
SystemClock
FixedClock
```

生产：

```text
SystemClock
```

测试：

```text
FixedClock
```

---

# 102. URL Determinism

URL encoding 必须：

```text
stable
```

例如相同：

```text
UTF-8 bytes
```

得到：

```text
same percent encoded output
```

---

# 103. Regex Determinism

必须测试：

```text
same pattern
same input
same flags
```

得到：

```text
same matches
```

结果排序必须：

```text
source order
```

而不是：

```text
hash-map order
```

---

# 104. Color Determinism

必须统一：

```text
rounding
normalization
uppercase/lowercase
alpha handling
```

避免：

```text
Windows:
#FF8800

macOS:
#ff8800
```

这种无意义的不确定性。

---

