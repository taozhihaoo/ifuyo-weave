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

# 105. UI Information Architecture

建议：

```text
Home
Utilities
 ├── Hash
 ├── Checksum
 ├── Base64
 ├── UUID
 ├── Timestamp
 ├── URL
 ├── Regex
 └── Color
```

---

# 106. Utility Shared UX

尽量统一：

```text
Input
Options
Result
Copy
Clear
Swap
Reset
```

但：

> 不要为了视觉统一而强行让不同工具使用完全相同的交互。

---

# 107. Hash UI

建议：

```text
Input
Algorithm
Encoding
Result
Copy
```

文件模式：

```text
Drop files
 ↓
Hash
 ↓
Result table
```

---

# 108. Base64 UI

建议：

```text
Encode | Decode
Text | File
Encoding
Input
Output
```

支持：

```text
Swap
Copy
Clear
```

---

# 109. UUID UI

建议：

```text
Version
Count
Format
Generate
```

结果：

```text
Scrollable list
Copy All
```

---

# 110. Timestamp UI

建议：

```text
Timestamp → Date
Date → Timestamp
Unit
Timezone
```

并显示：

```text
UTC
Local
Normalized
```

---

# 111. URL UI

建议：

```text
Component Encode
Component Decode
Query Encode
Query Decode
```

避免：

```text
一个按钮
“URL Encode Everything”
```

---

# 112. Regex UI

建议：

```text
Pattern
Flags
Input
Match count
Results
Capture Groups
```

可以提供：

```text
Highlight matches
```

但不要：

```text
无限实时执行
```

每次输入变化必须受到：

```text
resource limit
debounce
```

控制。

---

# 113. Color UI

建议：

```text
Input Color
HEX
RGB
HSL
HSV
Alpha
Preview
```

最好支持：

```text
click copy
```

但不要做成：

```text
完整 Photoshop Color Picker
```

---

# 114. Preview 原则

对于纯计算 Utility：

```text
计算结果本身就是 Preview
```

因此：

```text
Preview == Execute
```

不得：

```text
Preview 使用简化算法
Execute 使用另一套算法
```

---

# 115. Input Normalization

所有工具都要明确：

```text
Trim?
Normalize Unicode?
Decode BOM?
Case normalization?
Whitespace preservation?
```

绝不能：

```text
默认做了一堆用户看不见的修改。
```

---

# 116. Base64 Unicode

必须测试：

```text
中文
日文
韩文
emoji
combining marks
```

例如：

```text
你好 🌱
```

确保：

```text
UTF-8
→ Base64
→ UTF-8 decode
```

完全 round-trip。

---

# 117. Hash Unicode

必须测试：

```text
ASCII
Chinese
emoji
combining characters
NFC/NFD
```

非常重要：

```text
不要主动 Unicode normalize
```

除非用户显式要求。

因为：

```text
NFC text
```

与：

```text
NFD text
```

的 byte sequence 可以不同，因此 hash 也可能不同。

---

# 118. Regex Unicode

必须测试：

```text
中文
emoji
Unicode classes
multibyte offsets
```

尤其确认：

```text
byte offset
character offset
line/column
```

不会混淆。

---

# 119. Offset Semantics

Regex Match 必须明确：

```text
start/end
```

是：

```text
byte offset
Unicode scalar offset
UTF-16 code unit offset
```

如果 Rust 与 JS 存在不同索引语义：

> 必须在 IPC DTO 中定义清楚。

不要让前端自行猜测。

---

# 120. Regex Line / Column

建议：

```text
1-based line
1-based column
```

或者沿用项目已有 convention。

但必须：

```text
global consistency
```

---

# 121. Invalid Regex

例如：

```text
(
```

必须显示：

```text
Invalid Pattern
```

以及：

```text
engine diagnostic
position if available
```

不要：

```text
panic
```

---

# 122. Invalid Timestamp

例如：

```text
2026-99-99
```

必须：

```text
structured error
```

而不是：

```text
best-effort guessing
```

---

# 123. Invalid Color

例如：

```text
rgb(999, -10, abc)
```

必须：

```text
InvalidColor
```

不能：

```text
clamp silently
```

---

# 124. Invalid Base64

例如：

```text
%%%INVALID%%%
```

默认：

```text
decode failed
```

---

# 125. Unsupported Algorithm

例如用户选择：

```text
BLAKE2b
```

但项目没有实现：

必须：

```text
UnsupportedAlgorithm
```

而不是：

```text
fallback to SHA-256
```

---

# 126. Security — Path

M9 如果操作文件：

必须继续复用：

```text
M1 Path Safety
```

不得：

```text
直接拼接用户路径
```

---

# 127. Security — No Shell

禁止任何 Utility：

```text
spawn shell
Command::new("cmd")
Command::new("powershell")
Command::new("bash")
```

来完成核心功能。

例如：

```text
Hash
```

不得通过：

```text
certutil
openssl
sha256sum
```

完成默认功能。

---

# 128. Security — No Network

M9 必须：

```text
offline-first
```

禁止：

```text
HTTP requests
API calls
remote validation
online regex
online color service
```

---

# 129. Security — Regex

必须专项审计：

```text
CPU
Memory
Input Length
Pattern Length
Match Count
```

---

# 130. Security — Base64

必须防：

```text
decoded size explosion
output allocation explosion
```

---

# 131. Security — File Hash

对于：

```text
symlink
junction
special file
device file
```

必须遵守：

```text
M1 filesystem policy
```

不要在 M9 中重新定义。

---

# 132. Security — Clipboard

Clipboard 不得被：

```text
automatically uploaded
```

也不得：

```text
logged
```

---

# 133. Privacy Audit

最终明确：

```text
Telemetry = None
Cloud = None
Upload = None
Account = None
Remote Execution = None
```

所有 Utility 输入：

```text
stay local
```

---

# 134. Logging Policy

绝对不要日志记录：

```text
Full Base64 payload
Full Regex input
Full Regex pattern
Secret-like text
Full encoded file content
Clipboard content
```

日志最多记录：

```text
tool id
operation type
duration
status
error code
bytes processed
```

且必须符合项目现有 privacy policy。

---

# 135. Resource Limits

建立：

```text
UtilityResourceLimits
```

至少考虑：

```text
max text input
max base64 input
max decoded bytes
max regex pattern length
max regex input length
max regex matches
max UUID count
max file count
max result rows
```

实际数值必须：

```text
documented
configurable where appropriate
tested
```

---

# 136. Large Text

例如：

```text
500 MB
```

Regex：

```text
禁止默认直接载入并无限处理。
```

必须：

```text
bounded
```

或者：

```text
reject clearly
```

---

# 137. Large Base64

对于：

```text
large file → base64
```

优先：

```text
streaming
```

或者：

```text
bounded file-size threshold
```

必须测量：

```text
memory
CPU
output size
```

---

# 138. Large Hash

例如：

```text
10 GB file
```

应该：

```text
stream
```

而不是：

```text
Vec<u8>::read_to_end()
```

---

# 139. UUID Large Count

例如：

```text
1,000,000
```

必须考虑：

```text
allocation
UI rendering
clipboard
export
```

不要：

```text
1 million strings
```

全部直接塞 React state。

---

# 140. UI Cancellation

对于长时间：

```text
Hash
Checksum
Base64
Batch
```

必须通过：

```text
M7 cancellation
```

而不是：

```text
UI button only
```

---

# 141. Progress

Hash / Checksum / File Base64 等：

```text
bytes processed
total bytes
percentage
```

必须来自：

```text
actual processing
```

不是：

```text
timer animation
```

---

# 142. Batch Progress

批处理：

```text
10 files
```

必须能区分：

```text
job progress
item progress
```

继续使用：

```text
M7 Progress Model
```

---

# 143. Retry

M9 不实现 retry policy。

由：

```text
M7
```

决定。

M9 只需提供：

```text
structured error classification
```

以便 M7 判断：

```text
retryable
non-retryable
```

---

# 144. File Output

如果 Utility 输出：

```text
hash report
checksum report
base64 file
regex report
```

必须：

```text
OperationPlan
Safe Write
TOCTOU
History
```

按照已有体系执行。

---

# 145. Export Formats

可以支持：

```text
TXT
CSV
JSON
```

但不要为每个 Utility 重写：

```text
ExportEngine
```

应考虑统一：

```text
UtilityResultExporter
```

如果已有 Data/Text export infrastructure，则优先复用。

---

# 146. JSON Export

如果导出结构化 Utility Result：

```text
UTF-8
deterministic field ordering where project conventions permit
```

确保：

```text
stable
machine-readable
```

---

# 147. CSV Export

Hash 结果例如：

```text
path,algorithm,digest,status
```

必须正确处理：

```text
comma
quotes
newlines
Unicode paths
```

优先复用：

```text
M5 CSV infrastructure
```

---

# 148. No Duplicate CSV Writer

禁止：

```text
M5 CSV writer
+
M9 CSV writer
```

除非审计证明现有实现不能合理复用。

---

# 149. Test Architecture

必须新增：

```text
Unit Tests
Integration Tests
Real Filesystem Tests
Property Tests
Golden Tests
Fault Tests
Security Tests
UI Smoke Tests
Regression Tests
```

---

# 150. Hash Golden Tests

使用：

```text
known digest vectors
```

覆盖：

```text
empty
abc
Unicode
large input
```

---

# 151. Checksum Golden Tests

覆盖：

```text
standard test vectors
```

至少：

```text
empty
123456789
binary bytes
```

---

# 152. Base64 Golden Tests

覆盖：

```text
empty
f
fo
foo
hello world
Unicode
emoji
```

---

# 153. Base64 Round-trip Property

必须满足：

```text
decode(encode(bytes)) == bytes
```

对：

```text
random byte arrays
```

执行 property testing。

---

# 154. UUID Tests

验证：

```text
version bits
variant bits
format
uniqueness across reasonable sample
```

不能把：

```text
uniqueness
```

当成：

```text
formal proof
```

测试应验证：

```text
no unexpected collision in sample
```

---

# 155. UUID v7 Tests

验证：

```text
timestamp ordering properties
version
variant
randomness fields
```

使用：

```text
fixed clock
```

进行 deterministic testing。

---

# 156. Timestamp Golden Tests

至少测试：

```text
Unix epoch
positive timestamps
negative timestamps if supported
millisecond values
timezone conversion
DST boundaries if named zones supported
RFC 3339
```

---

# 157. Timestamp Edge Cases

测试：

```text
1970-01-01
before epoch
2038
far future
leap year
leap day
DST spring forward
DST fall back
```

根据：

```text
supported platform time range
```

定义实际边界。

---

# 158. URL Tests

测试：

```text
spaces
Unicode
reserved characters
slashes
question mark
ampersand
equals
percent
fragment
query
```

---

# 159. URL Round-trip

至少保证：

```text
decode(encode(component)) == component
```

对于合法输入。

---

# 160. Regex Golden Tests

必须测试：

```text
basic literals
classes
anchors
groups
named groups
flags
Unicode
multiline
zero-width
```

以及所有：

```text
Unsupported features
```

必须明确失败。

---

# 161. Regex Fuzzing

必须使用：

```text
fuzz / property tests
```

检测：

```text
panic
unbounded behavior
unexpected allocations
invalid UTF-8 handling
```

---

# 162. Color Golden Tests

至少测试：

```text
black
white
red
green
blue
gray
transparent
alpha colors
```

---

# 163. Color Property Tests

对于支持的颜色空间：

```text
convert
→ convert back
```

必须满足：

```text
within tolerance
```

---

# 164. Cross-platform Tests

尤其测试：

```text
Windows
Linux
macOS
```

可能受影响：

```text
Timezone
Filesystem
Unicode Path
Clipboard
Line ending
```

---

# 165. Real Filesystem Tests

必须真实创建：

```text
small file
large file
Unicode filename
empty file
binary file
```

验证：

```text
Hash
Checksum
Base64
Export
```

---

# 166. Fault Injection

至少测试：

```text
file disappears
file changes during processing
permission denied
output collision
disk full simulation where practical
cancel during large file processing
malformed Base64
invalid Regex
invalid Color
invalid Timestamp
```

---

# 167. TOCTOU

任何：

```text
file → output
```

流程继续使用：

```text
snapshot
validate
process
write
```

并处理：

```text
input changed
input deleted
output changed
```

---

# 168. Preview / Execute Consistency

对于 Utility：

```text
Preview
```

必须与：

```text
actual result
```

保持一致。

尤其：

```text
Regex matches
Timestamp result
Color result
Base64 result
Hash result
```

不得：

```text
preview uses browser implementation
execute uses Rust implementation
```

导致 mismatch。

---

# 169. IPC

IPC 必须保持：

```text
bounded
typed
structured
```

避免：

```text
巨大 String
无限 JSON
per-character IPC
per-match IPC
```

---

# 170. Regex IPC

绝对不要：

```text
每个 match
→ 一次 IPC
```

应该：

```text
single request
→ structured result
```

并且：

```text
result count bounded
```

---

# 171. Base64 IPC

大型 Base64：

不要：

```text
Rust → IPC → enormous JSON string
```

如果当前架构支持：

```text
file-based transfer
streaming
temporary artifact
```

则优先复用。

---

# 172. UI Rendering

对于：

```text
UUID 10,000
Hash 10,000 files
Regex 10,000 matches
```

必须：

```text
virtualize
paginate
limit
```

不能：

```text
unbounded DOM
```

---

# 173. Accessibility

至少检查：

```text
Keyboard navigation
Focus order
Labels
ARIA
Contrast
Error announcements
Copy feedback
Status updates
```

Regex match highlight 不能成为：

```text
screen-reader inaccessible
```

---

# 174. Keyboard Shortcuts

可以统一：

```text
Ctrl/Cmd + Enter
Ctrl/Cmd + C
Ctrl/Cmd + A
Escape
```

但必须避免：

```text
global shortcut collisions
```

---

# 175. Command Palette

如果已有：

```text
Command Palette
```

M9 工具必须注册。

例如：

```text
Hash
Checksum
Base64
UUID
Timestamp
URL Encode
Regex Tester
Color Converter
```

不要另建：

```text
Utility Launcher
```

---

# 176. Drag & Drop

适合文件输入的工具支持：

```text
Drop file
```

例如：

```text
Hash
Checksum
Base64
```

但：

```text
UUID
Timestamp
Color
Regex
```

无需强行支持。

---

# 177. Quick Drop Intelligence

如果用户拖入：

```text
file.zip
```

M9 不应自作主张：

```text
Hash
Base64
Checksum
```

而应该由已有：

```text
file detection
tool recommendation
```

决定展示候选工具。

---

# 178. Tool Recommendation

M9 可以提供：

```text
File
→ Inspect
→ Hash
→ Checksum
→ Base64
```

但推荐逻辑属于：

```text
Application Layer
```

不是：

```text
Hash Tool
```

---

# 179. Error UX

错误必须：

```text
human readable
machine readable
actionable
```

例如：

```text
Invalid regex pattern
```

同时显示：

```text
unsupported lookbehind syntax
```

如果底层 engine 能给出。

---

# 180. Loading UX

不得使用：

```text
“Processing...” permanent spinner
```

长任务必须：

```text
actual progress
```

短计算则：

```text
immediate result
```

---

# 181. No Fake Progress

禁止：

```text
setInterval(() => percent += 5)
```

模拟：

```text
Hash
Checksum
Base64
```

进度。

---

# 182. No Fake Capability Matrix

例如底层：

```text
doesn't support CRC32C
```

不能 UI 显示：

```text
CRC32C ✅
```

---

# 183. Dependency Audit

任何新 dependency 必须输出：

```text
Package
Version
Reason
Alternative
License
Security Consideration
Maintenance Status
```

---

# 184. Minimal Dependency Rule

优先：

```text
existing dependency
```

其次：

```text
small focused dependency
```

最后才：

```text
large framework
```

禁止：

```text
utility framework
```

为了 3 个函数引入几十 MB 依赖。

---

# 185. Crypto Library

如果 Hash / UUID 使用密码学库：

必须确认：

```text
actual random source
algorithm implementation
platform support
license
maintenance
```

不得自行实现：

```text
cryptographic primitive
```

除非这是项目明确的研究目的。

---

# 186. Regex Dependency

若当前 crate：

```text
regex
```

已经存在：

优先：

```text
reuse
```

若没有：

```text
evaluate crate
```

不要直接把：

```text
PCRE
Oniguruma
JavaScript engine
```

全部引入。

决策规则：默认使用 regex crate（不支持反向引用 / lookbehind）；若确需这些能力，走 DECISIONS.md 评估 fancy-regex，并在能力矩阵如实标注支持范围。

---

# 187. Color Dependency

Color Converter 只要数学转换即可实现：

```text
pure domain code
```

若已有 color crate：

```text
audit and reuse
```

不要为了：

```text
HEX ↔ RGB
```

增加大型 UI color framework。

---

# 188. File Naming

遵守当前仓库 naming convention。

不要强制：

```text
utils.rs
helpers.rs
common.rs
misc.rs
```

这种垃圾抽象。

优先语义命名：

```text
hash.rs
checksum.rs
base64.rs
uuid.rs
timestamp.rs
url.rs
regex.rs
color.rs
```

---

# 189. Avoid God Module

禁止：

```text
weave-utilities/src/lib.rs
```

塞入：

```text
3000 lines
```

所有 Utility 必须保持：

```text
bounded responsibility
```

---

# 190. Shared Utilities

只有真正跨模块共享：

```text
error
limits
result
diagnostics
```

才建立：

```text
shared abstraction
```

禁止为了减少文件数量：

```text
everything → utility_common.rs
```

---

# 191. Data Ownership

必须明确：

```text
M1 owns file/hash foundation
M4 owns text semantics
M5 owns structured data
M6 owns image semantics
M8 owns document semantics
M9 owns generic utility semantics
M7 owns execution
M10 owns workflow composition
```

---

# 192. Dependency Direction

目标：

```text
weave-utilities
      ↓
weave-core
      ↓
existing capability
```

不要形成：

```text
weave-utilities
↔
weave-data
↔
weave-documents
↔
weave-media
```

循环依赖。

---

# 193. Utility Registration

建议建立：

```text
UtilityRegistry
```

用于：

```text
id
name
category
capabilities
supported inputs
```

但如果已有统一：

```text
ToolRegistry
```

则：

```text
reuse
```

不要再建立：

```text
UtilityRegistry
ToolRegistry
DocumentRegistry
ImageRegistry
```

大量重复注册系统。

---

# 194. Tool Metadata

每个 Utility 至少定义：

```text
id
name
description
category
input kinds
output kinds
capabilities
```

例如：

```text
hash
category=integrity
input=file|text
output=digest
```

---

# 195. Composability

工具的 Domain API 应尽量：

```text
stateless
typed
deterministic
serializable
```

例如：

```text
HashOptions
HashResult
```

而不是：

```text
HashPageState
HashReactContext
```

---

# 196. Workflow Readiness

虽然 M10 尚未实现：

M9 的 capability model 应允许未来：

```text
M10
```

调用：

```text
Hash
Base64
Regex
Timestamp
Color
```

但：

```text
M9 不知道 Workflow 存在。
```

即：

```text
M9
→ capability
```

而不是：

```text
M9
→ workflow node
```

---

# 197. Serialization

如果工具配置需要保存：

必须使用：

```text
stable schema
```

例如：

```text
HashOptions
Base64Options
RegexOptions
ColorOptions
```

版本化：

```text
schema_version
```

如确实需要。

---

# 198. Presets

可以支持少量：

```text
built-in presets
```

例如：

```text
SHA-256
SHA-512
UTF-8
UTC
```

但不要实现：

```text
Preset Marketplace
Cloud Preset Sync
```

---

# 199. Recent Inputs

不要默认持久化：

```text
sensitive regex
Base64
clipboard data
```

若产品已有：

```text
recent tools
```

仅记录：

```text
tool opened
```

而不是：

```text
full user payload
```

---

# 200. Secure Memory Consideration

M9 并非密码管理器。

因此不得声称：

```text
cryptographic secure memory
```

除非真实实现。

但对于：

```text
temporary sensitive data
```

必须避免：

```text
unnecessary logging
```

和：

```text
unbounded duplication
```

---

# 201. Benchmark Matrix

至少测量：

```text
Hash
Checksum
Base64
Regex
Color
```

针对：

```text
1 KB
1 MB
100 MB
1 GB
```

或项目实际可接受范围。

---

# 202. Memory Benchmark

重点测：

```text
Hash
Base64
Regex
```

记录：

```text
peak RSS
allocation behavior
throughput
```

---

# 203. Regex Benchmark

至少比较：

```text
simple pattern
complex pattern
unicode
large input
zero matches
many matches
```

---

# 204. Color Benchmark

Color Converter 应保持：

```text
O(1)
```

级别单色转换。

不要引入：

```text
image-sized structures
```

---

# 205. UUID Benchmark

记录：

```text
UUID/s
allocation
batch size
```

确保：

```text
large count
```

不会导致异常性能退化。

---

# 206. M0–M8 Regression

必须完整运行：

```text
M0
M1
M2
M3
M4
M5
M6
M7
M8
```

所有：

```text
unit
integration
filesystem
UI smoke
build
```

必须通过。

---

# 207. Rust Gates

实际运行项目已有：

```text
cargo fmt --check
cargo check
cargo test
cargo clippy
```

或：

```text
repository's actual equivalent
```

不得假设命令固定。

---

# 208. Frontend Gates

实际运行：

```text
npm test
npm run lint
npm run typecheck
npm run build
```

或当前项目真实 script。

---

# 209. Tauri Build

最终必须：

```text
real Tauri build
```

确认：

```text
application builds
```

不允许：

> 源码看起来应该可以。

---

# 210. Real Integration

必须至少真实执行：

```text
Hash file
Checksum file
Base64 text
Base64 file
UUID generation
Timestamp conversion
URL encode/decode
Regex match
Color conversion
```

不是：

```text
mock backend result
```

---

# 211. Real UI Smoke

至少验证：

```text
Open Utility
Input
Execute
Result
Copy
Clear
Error
Back
Command Palette
```

---

# 212. Clipboard Smoke

真实测试：

```text
Copy
Paste
```

并确认：

```text
Unicode preserved
```

---

# 213. Export Smoke

如果支持导出：

```text
export
→ file exists
→ content correct
```

必须真实验证。

---

# 214. Unicode Smoke

整个 M9 至少测试：

```text
中文
日本語
한국어
emoji
accented Latin
combining characters
```

---

# 215. Empty Input

每个 Utility 都必须明确：

```text
empty input semantics
```

例如：

Hash：

```text
empty text
```

应正常 hash：

```text
empty byte sequence
```

Regex：

```text
empty pattern
```

必须根据 engine 语义明确处理。

---

# 216. Whitespace

必须测试：

```text
empty
space
tabs
newlines
CRLF
LF
```

不要：

```text
trim by default
```

导致数据语义改变。

---

# 217. Locale

Timestamp 与 Color 必须避免：

```text
locale-dependent core parsing
```

例如：

```text
03/04/2026
```

不能在核心逻辑中默认为：

```text
MM/DD
```

或：

```text
DD/MM
```

而不明确。

---

# 218. Locale-aware UI

UI 可以展示：

```text
localized date
```

但核心结果必须保持：

```text
machine-stable
```

---

# 219. Accessibility Error States

Error message 必须：

```text
visible
keyboard reachable
screen-reader announced
```

并且不能只：

```text
show red border
```

---

# 220. i18n

新增：

```text
tool names
tool descriptions
errors
warnings
labels
algorithm names where localized
```

都必须进入：

```text
existing i18n system
```

不得在 React 中大量硬编码。

---

# 221. Localization Semantics

不要翻译算法标准名称：

```text
SHA-256
CRC-32C
UUID v7
RFC 3339
```

这些应保持标准可识别性。

可以翻译：

```text
Description
Tooltip
Warning
```

---

# 222. Documentation

M9 必须更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
THIRD_PARTY_LICENSES.md
```

并新增必要：

```text
docs/utilities.md
docs/security-utilities.md
docs/performance-utilities.md
```

具体文件名以当前仓库结构为准。

---

# 223. Utility Capability Matrix Documentation

必须文档化：

```text
Hash algorithms
Checksum variants
Base64 modes
UUID versions
Timestamp units
Timezone support
URL encoding modes
Regex engine capabilities
Color models
```

---

# 224. Unsupported Capability Documentation

尤其必须写清楚：

```text
NOT SUPPORTED
```

而不是只列：

```text
Supported
```

---

# 225. Example Documentation

至少提供：

```text
Hash example
Base64 example
UUID example
Timestamp example
URL example
Regex example
Color example
```

使用：

```text
safe non-sensitive sample data
```

---

# 226. License Audit

所有新 dependency：

```text
license compatible
```

并更新：

```text
THIRD_PARTY_LICENSES.md
```

不得留下：

```text
unknown license
```

---

# 227. Security Audit

最终必须专项检查：

```text
Regex DoS
Memory exhaustion
Base64 expansion
File path safety
Symlink behavior
Unbounded UUID generation
Clipboard leakage
Log leakage
Malformed inputs
IPC oversized payloads
Dependency security
```

---

# 228. Privacy Audit

最终确认：

```text
No telemetry
No cloud
No uploads
No account
No remote processing
No persistent sensitive input logs
```

---

# 229. Performance Audit

最终至少记录：

```text
Hash throughput
Checksum throughput
Base64 throughput
Regex latency
UUID throughput
Color conversion latency
```

以及：

```text
peak memory
```

---

# 230. Regression Audit

必须确认：

```text
M1 Hash still correct
M2 File mutation still safe
M3 Duplicate Finder still correct
M4 Regex/Text semantics still correct
M5 CSV export still correct
M6 Image functionality still correct
M7 Batch Engine still correct
M8 Document utilities still correct
```

---

# 231. Architecture Audit

重点检查：

```text
No second hash engine
No second filesystem abstraction
No second export engine
No second batch engine
No second history
No second clipboard layer
No second i18n system
No second error model
No circular dependency
```

---

# 232. UI Audit

检查：

```text
No business logic in React
No raw filesystem operations in React
No shell calls
No network
No fake progress
No giant DOM lists
No duplicated utility semantics
```

---

# 233. Utility Capability Audit

每一个 Utility 输出：

```text
Supported
Partial
Unsupported
```

并附：

```text
Evidence
Test
Implementation Location
```

---

# 234. M9 Hard Failure Conditions

以下任何一项存在：

```text
Hash result incorrect
Checksum variant incorrect
Base64 silently corrupts data
UUID randomness is predictable
Timestamp timezone handling incorrect
URL encoding semantics incorrect
Regex engine mismatch
Regex resource exhaustion
Color round-trip materially incorrect
File processing bypasses M1
Batch engine duplicated
Safe Write bypassed
Sensitive data logged
Network dependency introduced
Fake UI implementation
Mock pretending to be real functionality
M1–M8 regression
```

必须：

```text
M9 NOT COMPLETE
```

---

# 235. M9 Acceptance Criteria

必须全部满足：

```text
[ ] Utility domain exists
[ ] Utility capability model exists

[ ] Hash works
[ ] Hash reuses M1 foundation
[ ] Hash streaming works
[ ] Hash security warnings exist

[ ] Checksum works
[ ] CRC semantics documented
[ ] Known vectors pass

[ ] Base64 encode works
[ ] Base64 decode works
[ ] URL-safe mode documented
[ ] Invalid input handled
[ ] Large input bounded

[ ] UUID generation works
[ ] UUID randomness audited
[ ] UUID version/variant validated
[ ] UUID batch bounded

[ ] Timestamp conversion works
[ ] Units documented
[ ] UTC/local/timezone semantics correct
[ ] DST tested where supported

[ ] URL encode works
[ ] URL decode works
[ ] Component/query semantics distinguished
[ ] Unicode tested

[ ] Regex tester works
[ ] Engine capability matrix documented
[ ] Match results structured
[ ] Resource limits exist
[ ] Fuzz tests exist
[ ] No catastrophic resource issue

[ ] Color converter works
[ ] HEX works
[ ] RGB works
[ ] HSL works
[ ] Alpha works
[ ] Additional spaces only if real
[ ] Round-trip tests pass

[ ] Clipboard integration works
[ ] Export integration works where applicable
[ ] M7 integration works where applicable
[ ] Safe Write reused
[ ] No duplicate infrastructure

[ ] Real filesystem integration exists
[ ] Real UI smoke exists
[ ] Unicode tests pass
[ ] Resource-limit tests pass
[ ] Fault injection exists
[ ] Performance measured

[ ] Accessibility audit passed
[ ] i18n updated
[ ] Security audit passed
[ ] Privacy audit passed
[ ] License audit passed

[ ] M0–M8 regression passed
[ ] Rust tests passed
[ ] Frontend tests passed
[ ] Build passed
[ ] Documentation updated
[ ] Git hygiene passed
```

---

# 236. M9 不算完成的情况

以下任意情况都不能宣布：

```text
M9 COMPLETE
```

例如：

```text
Hash 页面有了
但算法结果未经 golden test

Base64 看起来能用
但 Unicode round-trip 有问题

UUID 能生成
但使用 Math.random

Timestamp 能显示
但 timezone 是浏览器猜测

Regex 能匹配
但 preview 与 backend engine 不同

Regex 支持 lookbehind
但实际上 engine 不支持

Color 可以转换
但 RGB/HSL round-trip 漂移严重

Checksum 写的是 CRC32
但没有定义具体 variant

文件 Hash 可用
但读取整个文件进 RAM

大型结果出现
但 UI 直接渲染 10000 行

M9 自己实现 Batch Engine

M9 自己实现 History

M9 bypass Safe Write

日志记录完整 Base64 / Regex payload

M1–M8 regression failure
```

任何一项都必须：

```text
M9 NOT COMPLETE
```

---

# 237. 推荐实现顺序

严格按照：

```text
Audit
 ↓
Capability Inventory
 ↓
Architecture Decision
 ↓
Utility Domain Foundation
 ↓
Hash
 ↓
Checksum
 ↓
Base64
 ↓
UUID
 ↓
Timestamp
 ↓
URL
 ↓
Regex
 ↓
Color
 ↓
UI Integration
 ↓
Clipboard
 ↓
Export
 ↓
M7 Integration
 ↓
Security
 ↓
Performance
 ↓
Regression
 ↓
Documentation
 ↓
Final Audit
```

---

# 238. 第一批优先实现

首先完成：

```text
Hash
Checksum
Base64
UUID
Timestamp
```

这些应建立：

```text
稳定 Utility Foundation
```

---

# 239. 第二批

然后：

```text
URL
Regex
```

因为它们：

```text
semantic complexity
```

明显高于：

```text
UUID
Hash
```

必须在实现前定义：

```text
engine semantics
resource constraints
```

---

# 240. 第三批

最后：

```text
Color
```

原因不是 Color 不重要。

而是：

```text
Color conversion
```

对：

```text
precision
rounding
color space
```

有明确数学语义，应在基础 Utility Architecture 稳定后实现。

---

# 241. AI Coding Workflow

整个 M9 严格遵循：

```text
Audit
 ↓
Gap Analysis
 ↓
Reuse Analysis
 ↓
Architecture Decision
 ↓
Small Implementation
 ↓
Unit Tests
 ↓
Golden Tests
 ↓
Integration
 ↓
Security
 ↓
Performance
 ↓
UI
 ↓
Accessibility
 ↓
Regression
 ↓
Documentation
 ↓
Final Audit
 ↓
Commit
```

---

# 242. 每个 Utility 的实施循环

每个 Utility 都必须单独经过：

```text
1. Contract
2. Domain model
3. Real implementation
4. Unit tests
5. Golden/property tests
6. Error cases
7. Security cases
8. Application layer
9. IPC
10. UI
11. Real smoke test
```

不要：

```text
8 个页面一起写
最后一次性测试
```

---

# 243. Contract-first

例如 Hash：

```text
HashRequest
HashOptions
HashResult
```

先定义：

```text
semantics
```

再实现：

```text
code
```

---

# 244. No Copy-Paste Architecture

禁止：

```text
HashPage
ChecksumPage
Base64Page
UUIDPage
```

每个都复制一份：

```text
loading
error
copy
result
```

应该合理复用：

```text
shared UI primitives
```

但：

```text
domain semantics
```

仍保持清晰分离。

---

# 245. Shared Utility UI

可以建立：

```text
UtilityShell
UtilityInput
UtilityOptions
UtilityResult
UtilityError
CopyButton
ClearButton
```

但只有确实共享的部分才抽象。

---

# 246. Do Not Over-Abstract

不要为了：

```text
8 个工具
```

建立：

```text
UniversalUtilityRuntime
UniversalUtilityController
UniversalUtilityAdapter
UniversalUtilityStrategyFactory
UniversalUtilityPipeline
```

十层抽象。

原则：

> **最简单的可维护抽象优先。**

---

# 247. Test Data Policy

测试数据不得包含：

```text
真实密码
真实 API key
真实 token
真实个人数据
真实私密文件
```

只使用：

```text
synthetic fixtures
```

---

# 248. Fixture Organization

建议：

```text
tests/fixtures/utilities/
```

包括：

```text
hash/
checksum/
base64/
uuid/
timestamp/
url/
regex/
color/
```

如果仓库已有 fixture 结构：

```text
reuse
```

---

# 249. Golden Files

适合：

```text
JSON export
CSV export
Regex structured result
Hash report
```

通过：

```text
golden fixtures
```

锁定输出格式。

---

# 250. Property Testing

重点：

```text
Base64 round-trip
URL round-trip
Color round-trip
Hash determinism
Regex result stability where applicable
Timestamp conversion round-trip where reversible
```

---

# 251. Fault Test Matrix

建立：

```text
Utility × Fault
```

例如：

```text
Hash × file disappears
Checksum × permission denied
Base64 × malformed input
Regex × invalid pattern
Regex × oversized input
Timestamp × invalid timezone
Color × malformed syntax
UUID × excessive count
```

---

# 252. Performance Thresholds

不要随意给出漂亮数字。

必须：

```text
measure actual baseline
```

然后记录：

```text
Current
Target
Observed
Status
```

如果无法稳定 benchmark：

```text
mark unavailable
```

不得编造。

---

# 253. Benchmark Reproducibility

记录：

```text
CPU
RAM
OS
Rust version
Build mode
Input size
Algorithm
Result
```

保证：

```text
benchmark reproducible
```

---

# 254. Security Reproducibility

记录：

```text
Attack Case
Input
Expected
Observed
Mitigation
```

例如：

```text
Regex catastrophic pattern
```

必须给出：

```text
input size
execution behavior
limit
```

---

# 255. Final Documentation

M9 完成后：

```text
PROGRESS.md
```

必须更新：

```text
M9 status
implemented utilities
known limitations
```

---

# 256. DECISIONS.md

至少记录：

```text
Why M1 Hash was reused
Why chosen Regex engine
Why URL semantics are separated
Why UUID versions were limited
Why Color is sRGB based
Why M9 does not own Batch Engine
Why utility calculations do not use History
```

---

# 257. ARCHITECTURE.md

必须更新：

```text
weave-utilities
dependency direction
capability interfaces
IPC
M7 integration
M2 file export integration
```

---

# 258. Privacy Documentation

明确：

```text
Utility input stays local
Clipboard stays local
No network
No telemetry
No cloud processing
No remote regex
No remote hashing
```

---

# 259. Security Documentation

明确：

```text
Regex limits
Base64 limits
File limits
UUID count limits
IPC limits
Logging policy
Dependency audit
```

---

# 260. M9 Final Audit A — Baseline

确认：

```text
Git
Branch
HEAD
Workspace
M0–M8 status
```

---

# 261. Final Audit B — Architecture

确认：

```text
crate boundaries
dependency direction
no duplicate core
no circular dependency
```

---

# 262. Final Audit C — Hash

确认：

```text
algorithms
streaming
golden vectors
Unicode
large file
cancellation
```

---

# 263. Final Audit D — Checksum

确认：

```text
algorithm variants
parameters
test vectors
binary input
```

---

# 264. Final Audit E — Base64

确认：

```text
encode
decode
Unicode
URL-safe
padding
large input
round-trip
```

---

# 265. Final Audit F — UUID

确认：

```text
v4
v7
randomness
variant
format
batch limits
```

---

# 266. Final Audit G — Timestamp

确认：

```text
seconds
milliseconds
timezone
UTC
local
DST
precision
```

---

# 267. Final Audit H — URL

确认：

```text
component
query
decode errors
Unicode
round-trip
```

---

# 268. Final Audit I — Regex

确认：

```text
engine
features
unsupported syntax
limits
fuzzing
result structure
```

---

# 269. Final Audit J — Color

确认：

```text
HEX
RGB
HSL
alpha
additional models
round-trip
tolerance
```

---

# 270. Final Audit K — M7

确认：

```text
Batch Job reuse
progress
cancellation
failure classification
```

---

# 271. Final Audit L — M2

确认：

```text
Safe Write
TOCTOU
History
Undo
```

仅在：

```text
file mutation/export
```

场景适用。

---

# 272. Final Audit M — Security

确认：

```text
No network
No shell
No arbitrary execution
Regex bounded
Base64 bounded
large files bounded
logs safe
```

---

# 273. Final Audit N — Privacy

确认：

```text
No telemetry
No upload
No sensitive payload logging
No cloud
```

---

# 274. Final Audit O — Performance

确认：

```text
Hash benchmark
Checksum benchmark
Base64 benchmark
Regex benchmark
UUID benchmark
Memory measurements
```

---

# 275. Final Audit P — UX

确认：

```text
Navigation
Copy
Clear
Error
Keyboard
Accessibility
i18n
Virtualization
```

---

# 276. Final Audit Q — Git Hygiene

必须确认：

```text
no temp files
no debug outputs
no local absolute paths
no secrets
no generated junk
no giant fixtures
no private data
```

并检查：

```text
git status
git diff
git diff --cached
```

---

# 277. Final Audit R — Build

实际执行：

```text
Rust gates
Frontend gates
Tauri build
```

必须真实通过。

---

# 278. Final Audit S — Regression

完整执行：

```text
M0–M8 tests
M0–M8 build
M9 tests
M9 build
```

---

# 279. Completion Report

最终必须输出：

```text
# M9 Utilities Completion Audit
```

至少包含：

```text
1. Baseline
2. Architecture
3. Reuse Analysis
4. Implemented Utilities
5. Capability Matrix
6. Hash Audit
7. Checksum Audit
8. Base64 Audit
9. UUID Audit
10. Timestamp Audit
11. URL Audit
12. Regex Audit
13. Color Audit
14. M7 Integration
15. M2 Integration
16. Clipboard
17. Export
18. Tests
19. Security
20. Privacy
21. Performance
22. Accessibility
23. i18n
24. Dependency / License
25. Documentation
26. Known Limitations
27. M9 → M10 Handoff
28. Final Status
```

---

# 280. Known Limitations

只写真实存在：

```text
NOT SUPPORTED
PARTIAL
KNOWN LIMITATION
```

例如：

```text
Regex lookbehind not supported by selected engine
```

不能写：

```text
Regex fully compatible
```

---

# 281. No False Completion

禁止以下表达：

```text
should work
probably works
looks correct
implemented conceptually
placeholder complete
UI ready
backend ready
```

完成必须有：

```text
Evidence
```

---

# 282. Completion Evidence

每一个核心 capability 至少给：

```text
Implementation location
Test location
Test command
Observed result
```

---

# 283. M9 Final Status

最终只能是：

```text
M9 COMPLETE
```

或：

```text
M9 NOT COMPLETE
```

不得使用：

```text
Mostly complete
Almost done
Functionally complete
Beta complete
```

---

# 284. M9 → M10 Handoff

M9 必须交给 M10：

```text
Hash Capability
Checksum Capability
Base64 Capability
UUID Capability
Timestamp Capability
URL Capability
Regex Capability
Color Capability
```

以及：

```text
structured input
structured output
capability metadata
error model
resource limits
batch compatibility
```

---

# 285. M10 不应重新实现

M10 必须复用：

```text
M9 Utility capabilities
```

而不是重新实现：

```text
Hash node
Regex node
Timestamp node
Color node
```

真正的关系：

```text
M10 Workflow
       ↓
Tool / Capability
       ↓
M9 Utility
```

---

# 286. M9 不应知道 M10

M9 不能产生：

```text
workflow nodes
```

不能创建：

```text
graph model
```

不能存：

```text
workflow definitions
```

不能处理：

```text
branch
loop
conditional
```

---

# 287. M9 的终极产品价值

完成 M9 以后：

```text
ifuyo Weave
```

应拥有一套真正可复用的：

```text
Utility Layer
```

从：

```text
Files
Text
Data
Images
Documents
```

中抽离出常见的小型基础任务：

```text
Hash
Checksum
Encode
Decode
Generate
Convert
Test
Inspect
```

---

# 288. 最终架构图

最终应该接近：

```text
                           ifuyo Weave
                                │
                    ┌───────────┴───────────┐
                    │                       │
               Application              Workflow
                    │                       │
          ┌─────────┼─────────┐             │
          ▼         ▼         ▼             │
        Tools     Batch    Utilities ◄──────┘
          │         │         │
          │         │    ┌────┼────┬────┬────┬────┐
          │         │    ▼    ▼    ▼    ▼    ▼    ▼
          │         │  Hash Base64 UUID Time URL Regex Color
          │         │
          └─────────┴──────────────┐
                                   ▼
                              Core Capabilities
                                   │
             ┌─────────────────────┼──────────────────────┐
             ▼                     ▼                      ▼
        Filesystem               Text                  Data
             │                     │                      │
             ▼                     ▼                      ▼
           Media                Documents              History
```

但：

```text
Utilities
```

不能变成所有模块的垃圾依赖。

---

# 289. M9 的架构哲学

M9 应建立：

```text
Small Tools
    ↓
Strong Semantics
    ↓
Stable Capabilities
    ↓
Reusable Execution
```

而不是：

```text
Many Pages
    ↓
Many Helpers
    ↓
Many Dependencies
    ↓
Unmaintainable Utility Soup
```

---

# 290. 最终目标

M9 完成后，用户面对一个很小但非常常见的问题时：

```text
Hash?
Base64?
UUID?
Timestamp?
URL?
Regex?
Color?
Checksum?
```

无需：

```text
打开浏览器
寻找在线工具
复制敏感数据
承担网络依赖
```

而是：

```text
打开 Weave
 ↓
选择 Utility
 ↓
输入
 ↓
立即得到可信结果
 ↓
Copy / Export
```

整个体验继续保持：

```text
Local
Fast
Predictable
Private
Composable
```

这才是：

> **M9 真正应该为 ifuyo Weave 建立的能力。**