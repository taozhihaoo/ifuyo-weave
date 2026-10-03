# ifuyo Weave — M4 Text 完整开发提示词（上）

> 本文件是原《ifuyo Weave — M4 Text 完整开发提示词》的 **第 1/2 部分**，
> 覆盖原文件导语与第 0–97 节：M4 目标与硬边界、Formatter/Compare/Extractor/Transformer 四大工具范围与明确禁止、总体原则与真实仓库审计、架构位置与 weave-text/weave-core 职责、TextDocument/Encoding/BOM/Line Ending/Offset/Range/Diagnostics 领域模型、Formatter 总架构与各语言格式规范、Compare 与 Diff 模型、各 Extractor 与 Regex 引擎、各 Transformer 与 Preview/Apply/Save/TOCTOU/原子写/Transaction/Undo/History/Copy/Export 等实现规范。
> **必须与（下）一起阅读执行**：UI/UX、i18n/a11y、测试与 fixture、文档、Final Audit、提交纪律、交接条款在（下）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---

# ifuyo Weave
# M4 — Text
## 文本格式化、文本比较、文本提取、文本变换与安全文本工作流

你现在进入 **ifuyo Weave 的 M4：Text**。

项目：

`ifuyo Weave`

前置阶段：

```text
M0 Foundation
M1 File Core
M2 Rename / Organizer
M3 Duplicate Finder
```

当前阶段：

```text
M4 Text
```

本阶段必须以：

```text
当前仓库真实代码
+
Project Charter
+
M0/M1/M2/M3 已完成并通过审计的真实能力
```

作为唯一基线。

不要假设前序 milestone 的实现与旧提示词完全一致。

必须：

```text
先审计
↓
理解当前实现
↓
Gap Analysis
↓
最小设计
↓
实现
↓
测试
↓
UI 验证
↓
安全验证
↓
回归
↓
Final Audit
↓
Documentation
↓
Commit
```

---

# 0. M4 的一句话目标

M4 完成后，Weave 应该能够让用户：

```text
把文本拖进来
      ↓
Weave 自动理解文本
      ↓
格式化 / 比较 / 提取 / 变换
      ↓
Preview
      ↓
复制结果 / 导出结果 / 安全写回
      ↓
完成
```

最终用户应该感觉：

> **“有一段乱七八糟的文本，我扔进 Weave，它能立即帮我整理、比较、提取和转换，而且我始终知道结果会变成什么。”**

而不是：

> “Weave 里面有四个文本程序。”

---

# 1. M4 硬边界

## 1.1 本阶段必须实现

必须完成：

```text
Text Foundation
Text Document Model
Text Encoding Handling
Line Ending Handling
Text Size Limits
Text Preview Model
Text Diagnostics
```

四个核心工具：

```text
1. Text Formatter
2. Text Compare
3. Text Extractor
4. Text Transformer
```

---

# 2. Text Formatter 范围

Formatter 至少支持：

```text
JSON
XML
YAML
SQL
JavaScript
CSS
Markdown
```

每种格式尽可能支持：

```text
Format
Minify
Validate
Sort
Normalize
```

但是必须根据格式的真实语义决定哪些功能可用。

不要为了满足按钮数量而实现假的：

```text
Sort
Normalize
Minify
Validate
```

例如：

```text
SQL Sort
```

不能只是粗暴按字符串字母排序 SQL。

必须明确每个操作到底是什么意思。

---

# 3. Text Compare 范围

两个文本：

```text
A
vs
B
```

至少支持：

```text
Added
Removed
Changed
Moved
```

显示：

```text
Side-by-side
Unified Diff
```

比较选项：

```text
Whitespace Ignore
Case Ignore
```

Diff 必须：

```text
deterministic
bounded
testable
可解释
```

---

# 4. Text Extractor 范围

支持：

```text
URLs
Emails
File Paths
Numbers
IPv4
IPv6
JSON
Markdown Links
Regex
```

提取结果至少包含：

```text
matched text
start offset
end offset
line
column
type
```

允许：

```text
Copy
Copy All
Export
Filter
```

不要把 Text Extractor 偷变成 M9 Search Engine。

---

# 5. Text Transformer 范围

至少实现：

```text
Trim
Deduplicate Lines
Sort Lines
Add Prefix
Add Suffix
Case Conversion
Line Numbering
Find / Replace
Regex Replace
```

所有 Transformer：

```text
Preview-first
deterministic
side-effect free until Apply
```

---

# 6. 本阶段明确禁止

M4 不得提前实现：

```text
CSV Studio
CSV Cleaning
CSV/JSON Converter
JSON/CSV Converter
Data Inspector
Schema Inference
Data Profiling
Image Tools
PDF Tools
DOCX Tools
XLSX Tools
PPTX Tools
Archive Tools
Media Tools
Batch Engine
Workflow Pipeline
Natural Language Batch
AI Text Processing
LLM Rewrite
Cloud AI
Online Formatter API
Telemetry
Account
Cloud Storage
Remote Text Processing
Plugin SDK
```

特别注意：

### JSON

M4 可以：

```text
JSON format
JSON minify
JSON validate
JSON sort
JSON normalize
```

但不能提前实现：

```text
JSON → CSV
JSONL data studio
schema analysis
column extraction
data profiling
```

这些属于 M5 Data。

---

# 7. M4 总体原则

整个 milestone 严格遵守：

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
```

同时遵守：

```text
Correctness > Feature Count
Semantic Correctness > Visual Cleverness
Preview > Mutation
Explicitness > Magic
Reuse > Rebuild
Bounded > Unlimited
Local > Cloud
```

---

# 8. 第一阶段：真实仓库审计

在任何代码修改之前，先完整审计当前仓库。

必须检查：

```text
Git branch
HEAD
Working tree
M0 status
M1 status
M2 status
M3 status
```

检查：

```text
Cargo workspace
crate boundaries
Tauri commands
IPC DTOs
frontend command layer
filesystem abstraction
path safety
metadata
hash
OperationId
OperationPlan
OperationTransaction
HistoryEntry
Preview infrastructure
Undo infrastructure
progress
cancellation
error model
drag & drop
i18n
design tokens
```

重点寻找：

```text
weave-text
text-related commands
shared DTOs
existing text utilities
existing encoding handling
existing file-write helpers
existing preview components
existing history UI
```

---

# 9. 审计输出必须区分事实

如果发现前序实现与设计文档不同：

必须区分：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

```text
FACT:
weave-text exists but currently contains only text DTOs.

FACT:
Tauri already exposes preview command.

HYPOTHESIS:
the preview DTO may be reusable for M4.

INFERENCE:
M4 can probably extend the DTO instead of creating a second preview model.
```

禁止：

```text
猜测
假设
为了省时间直接重写
```

---

# 10. M4 的架构位置

目标架构：

```text
React UI
   ↓
Application / Command Layer
   ↓
weave-text
   ↓
weave-core
   ↓
shared infrastructure
   ↓
filesystem adapter
```

文件写回：

```text
UI
 ↓
Application
 ↓
weave-text
 ↓
shared OperationPlan / Transaction
 ↓
weave-files
 ↓
Filesystem
```

禁止：

```text
React
 ↓
直接 fs API
```

禁止：

```text
React
 ↓
自行判断路径安全
```

禁止：

```text
Text tool
 ↓
自己实现另一套 write/delete/move
```

---

# 11. weave-text 职责

`weave-text` 负责：

```text
TextDocument
Encoding
LineEnding
Language Detection
Formatter
Comparator
Extractor
Transformer
Text Diagnostics
Text Operation Results
```

不负责：

```text
Tauri window
React component
filesystem implementation
path validation implementation
history persistence
OS recycle bin
```

---

# 12. weave-core 职责

如果需要共享 DTO，应放入：

```text
weave-core
```

但不要因为“以后可能复用”而把所有 Text 类型都塞进 core。

只有真正跨模块稳定的类型进入：

```text
weave-core
```

例如：

```text
OperationId
TextDocumentId
OperationResult
Severity
Diagnostic
Range
Offset
```

应保持：

```text
core = stable language
text = text semantics
files = filesystem semantics
```

---

# 13. TextDocument 模型

建立统一文本输入模型。

建议包含：

```text
TextDocument
```

至少能够表达：

```text
source kind
content
encoding
BOM
line ending
language / format
original path
size
```

例如逻辑上：

```text
TextDocument
├── source
├── content
├── encoding
├── bom
├── line_ending
├── format
├── path
└── size
```

不要让业务工具自己维护：

```text
string + encoding + newline + path
```

的四五套平行结构。

---

# 14. 文本来源

M4 至少支持：

```text
Dropped File
Opened File
Pasted Text
Typed Text
```

未来可扩展：

```text
Clipboard
Selection
Generated Text
```

但是：

> 正常文本工具不依赖网络。

---

# 15. Encoding 原则

M1 已经存在的 encoding detection 能力必须优先复用。

M4 必须明确处理：

```text
UTF-8
UTF-8 BOM
UTF-16 LE
UTF-16 BE
```

以及当前架构已经明确支持的其他编码。

必须显式支持 GB18030 / GBK 与 Latin-1 fallback（charter #27）。若 M1 尚未覆盖 GB18030 检测，则在本里程碑补齐——不要假设代码已存在。

不要：

```text
看到文本
→ 强制 utf-8 decode
→ decode failure
→ 静默替换字符
```

必须做到：

```text
Detect
 ↓
Decode
 ↓
Validate
 ↓
Present
```

如果编码无法可靠识别：

```text
明确错误
```

而不是：

```text
静默乱码
```

---

# 16. BOM

必须正确区分：

```text
UTF-8 BOM
UTF-16 LE BOM
UTF-16 BE BOM
No BOM
```

Formatter / Transformer / Save 必须有明确策略：

```text
Preserve
Remove
Add
```

默认行为必须记录到：

```text
DECISIONS.md
```

不要让不同工具出现：

```text
Formatter 保留 BOM
Transformer 自动删除 BOM
Save As 又重新生成 BOM
```

这种不一致。

---

# 17. Line Ending

至少检测：

```text
LF
CRLF
CR
```

显示时不应该无意义破坏原始换行风格。

Formatter / Transformer / Save 必须有明确策略。

建议：

```text
Preserve by default
```

用户显式选择时才：

```text
LF
CRLF
```

转换。

必须测试：

```text
LF-only
CRLF-only
mixed line endings
empty lines
final newline
no final newline
```

---

# 18. Offset 模型

整个 Text 模块必须明确：

```text
byte offset
character offset
UTF-16 code-unit offset
line/column
```

UI 不得误把 Rust byte offset 当成 JavaScript string index。

这是高风险边界。

必须建立明确 contract：

```text
Domain internal offset
IPC serialized offset
UI display position
```

并测试：

```text
ASCII
é
汉字
emoji
surrogate pairs
combining characters
```

---

# 19. Text Range

建立统一：

```text
TextRange
```

至少：

```text
start
end
```

并能够转换：

```text
offset
→ line
→ column
```

必须 deterministic。

---

# 20. Text Diagnostics

格式化/验证失败不能只返回：

```text
Invalid JSON
```

至少提供：

```text
code
message
severity
range
line
column
```

例如：

```text
JSON_INVALID
Unexpected token
line 12
column 8
```

如果 parser 能提供：

```text
expected token
actual token
```

可保留。

但不要伪造解析器本身没有提供的信息。

---

# 21. Language / Format Detection

输入可能没有扩展名。

M4 可以使用：

```text
extension
user selection
content sniffing
```

但检测必须是：

```text
bounded
deterministic
conservative
```

例如：

```text
.foo
```

不能强行猜成某种语言。

UI 必须允许：

```text
Auto
JSON
XML
YAML
SQL
JavaScript
CSS
Markdown
```

手动覆盖。

---

# 22. Formatter 总架构

不要为七种格式写七套完全不相关 UI。

统一流程：

```text
Input
 ↓
Detect / Select Format
 ↓
Parse / Analyze
 ↓
Operation
 ↓
Result
 ↓
Preview
 ↓
Apply / Copy / Export
```

Formatter API 逻辑上：

```text
format(document, options)
minify(document, options)
validate(document, options)
sort(document, options)
normalize(document, options)
```

但实际可以根据语言能力返回：

```text
Supported
Unsupported
InvalidInput
```

不能用：

```text
try random heuristic
```

冒充支持。

---

# 23. Formatter 的输入输出原则

每个 formatter 必须满足：

```text
same input
+
same options
=
same output
```

除非操作显式依赖：

```text
timestamp
randomness
environment
```

M4 不应该依赖这些。

---

# 24. JSON Formatter

JSON 必须实现：

```text
Validate
Format
Minify
Sort
Normalize
```

## Validate

必须使用真实 JSON parser。

禁止：

```text
brace counting
regex pseudo parser
```

作为最终 validation。

---

# 25. JSON Format

至少支持：

```text
indent size
indent style
final newline
```

默认值必须统一。

例如：

```text
2 spaces
```

但以项目实际 UX 决策为准，并写入：

```text
DECISIONS.md
```

---

# 26. JSON Minify

必须：

```text
remove insignificant whitespace
preserve semantic content
```

不能修改：

```text
string content
number value
boolean
null
key names
array order
```

---

# 27. JSON Sort

必须明确 sort 语义。

推荐：

```text
Object keys sort
```

不要默认排序：

```text
array elements
```

因为：

```text
[3,1,2]
```

与：

```text
[1,2,3]
```

语义通常可能不同。

JSON Sort 应至少支持：

```text
recursive object key sort
root only
recursive
```

如实现成本允许。

---

# 28. JSON Normalize

Normalize 必须明确是：

```text
parse
→ canonical serialization
```

还是：

```text
whitespace normalization
```

不要含糊使用“Normalize”。

建议建立明确 options：

```text
Whitespace
Key Ordering
Final Newline
Unicode escaping policy
```

所有决策记录到：

```text
DECISIONS.md
```

---

# 29. XML Formatter

必须使用真实 XML parser / tokenizer。

支持：

```text
Validate
Format
Minify
Normalize
```

注意：

```text
attributes
text nodes
comments
CDATA
processing instructions
namespaces
```

不能因为格式化而改变 XML 语义。

必须测试：

```text
nested nodes
attributes
empty elements
CDATA
comments
namespace
```

---

# 30. XML Sort

XML 的 Sort 极其危险。

禁止：

```text
默认排序 child elements
```

因为：

```text
<item>A</item>
<item>B</item>
```

与交换顺序可能改变语义。

因此：

```text
Sort
```

默认仅支持明确安全的范围，例如：

```text
attributes
```

或者：

```text
明确指定的 node set
```

如果当前实现无法提供语义安全 Sort：

```text
标记 Unsupported
```

不要 fake it。

---

# 31. YAML Formatter

YAML 必须真正解析 YAML。

至少处理：

```text
mapping
sequence
scalar
null
quoted strings
comments
multiline strings
```

必须特别注意：

```text
YAML != JSON with comments
```

不要把 YAML parser 写成：

```text
line indentation parser
```

作为最终实现。

---

# 32. YAML Validate

必须：

```text
parse
→ success / failure
```

错误包含：

```text
line
column
message
```

对于 parser 不支持的 YAML feature：

```text
明确 unsupported
```

不要偷偷改变 YAML 语义。

---

# 33. YAML Sort

默认：

```text
sort mapping keys
```

不要：

```text
sort sequence
```

因为 sequence 通常有顺序语义。

如果递归：

```text
nested mappings sort
```

必须测试：

```text
comments
anchors
aliases
quoted values
multiline
```

至少记录 parser 支持边界。

---

# 34. SQL Formatter

SQL 是 M4 中非常容易做成“看起来能用，实际不可靠”的部分。

必须先确认当前项目依赖与可用 parser。

优先：

```text
parser-backed
```

而不是：

```text
regex-only formatting
```

如果项目采用 SQL parser：

必须记录支持的：

```text
dialect
statements
```

例如：

```text
SELECT
INSERT
UPDATE
DELETE
CREATE
ALTER
```

支持哪些方言必须写清楚。

---

# 35. SQL Minify

SQL Minify 必须避免破坏：

```text
string literals
quoted identifiers
comments
operators
```

不能直接：

```text
split_whitespace
```

然后拼回字符串。

---

# 36. SQL Format

至少应做到：

```text
SELECT
FROM
WHERE
GROUP BY
ORDER BY
JOIN
ON
VALUES
```

具有稳定、人类可读的布局。

但是：

> 不要求 M4 成为完整 SQL formatter IDE。

本 milestone 重点：

```text
常见 SQL
稳定格式化
稳定 minify
基本 validation
```

---

# 37. JavaScript Formatter

JavaScript formatter 必须明确支持边界。

至少覆盖常见：

```text
variables
functions
arrow functions
objects
arrays
imports
exports
conditions
loops
classes
async/await
```

不能通过：

```text
regex
```

判断：

```text
semicolon
brace indentation
comma
```

作为最终 formatter。

优先 parser / formatter library。

---

# 38. JavaScript Minify

M4 的 JS Minify 必须把：

```text
whitespace
comments
```

与真正：

```text
minification
```

区别开。

默认可以先实现安全的：

```text
syntax-aware whitespace minimization
comment removal
```

如果没有成熟、可靠的 bundler/minifier 依赖：

不要擅自实现：

```text
variable mangling
dead code elimination
advanced compression
```

M4 不需要做完整 JS build optimizer。

---

# 39. JavaScript Validate

使用真实 parser。

不能：

```text
括号数一致
→ 判定 JS valid
```

---

# 40. CSS Formatter

支持：

```text
rules
selectors
declarations
at-rules
comments
media queries
```

必须避免错误处理：

```text
strings
url(...)
calc(...)
custom properties
```

---

# 41. CSS Minify

至少安全处理：

```text
whitespace
comments
```

不能破坏：

```text
quoted strings
URLs
custom property values
calc expressions
```

---

# 42. CSS Validate

真实 parse / syntax validation。

禁止：

```text
brace counter
```

作为唯一 validation。

---

# 43. Markdown Formatter

Markdown 不应该被当成普通文本乱格式化。

至少考虑：

```text
headings
lists
ordered lists
code fences
quotes
links
tables
emphasis
inline code
```

尤其：

```text
code fence 内部
```

不应该被普通 whitespace formatter 随意修改。

---

# 44. Markdown Normalize

至少明确：

```text
heading spacing
list marker policy
blank line policy
final newline
```

不能擅自改变：

```text
code block content
link target
raw HTML
inline code
```

---

# 45. Markdown Sort

Markdown Sort 必须谨慎。

禁止默认：

```text
把所有段落排序
```

因为这会直接破坏文档语义。

推荐只提供：

```text
heading sections sort
```

作为明确 opt-in capability。

例如：

```text
Sort top-level sections by heading text
```

如果无法安全实现：

```text
Unsupported
```

---

# 46. Formatter 的 Unsupported 原则

任何 formatter capability 如果做不到可靠语义保持：

必须返回：

```text
Unsupported
```

而不是：

```text
HeuristicSuccess
```

这条规则优先级高于：

```text
“所有按钮都得亮”
```

---

# 47. Text Compare 总体架构

Compare：

```text
Input A
Input B
   ↓
Normalize Comparison View
   ↓
Diff Engine
   ↓
Diff Model
   ↓
Renderer
```

注意：

> 比较归一化 ≠ 修改原始文本。

例如：

```text
Whitespace Ignore
```

只影响：

```text
comparison
```

不能自动修改：

```text
original text
```

---

# 48. Diff Model

建立统一：

```text
Diff
DiffHunk
DiffLine
DiffChange
MoveBlock
```

至少可以表示：

```text
Added
Removed
Equal
Changed
Moved
```

必须记录原始位置：

```text
A line/range
B line/range
```

---

# 49. Side-by-side

UI：

```text
┌───────────────┬───────────────┐
│ A             │ B             │
├───────────────┼───────────────┤
│ unchanged     │ unchanged     │
│ removed       │               │
│               │ added         │
│ changed       │ changed       │
└───────────────┴───────────────┘
```

不能只是：

```text
两个 textarea
```

然后颜色全部由前端随机判断。

Diff 必须由：

```text
domain diff model
```

驱动。

---

# 50. Unified Diff

必须输出稳定格式：

```text
--- A
+++ B
@@ ...
```

格式细节可以根据实际 diff library。

必须 deterministic。

测试：

```text
empty
single line
multiple lines
final newline
completely different
identical
```

---

# 51. Whitespace Ignore

至少定义：

```text
Ignore Leading / Trailing
Ignore Repeated Internal Spaces
Ignore Line Ending
Ignore All Whitespace
```

或者项目最终采用的明确等级。

不要做一个模糊：

```text
Ignore whitespace = true
```

却无法知道到底忽略什么。

---

# 52. Case Ignore

必须明确：

```text
ASCII case insensitive
```

还是：

```text
Unicode-aware case folding
```

优先采用稳定 Unicode 规则。

必须测试：

```text
A/a
ABC/abc
中文
accented characters
```

---

# 53. Moved Detection

Moved 比 Added/Removed 更难。

M4 不需要实现：

```text
semantic move analysis
```

可以采用：

```text
identical line/block content
+
different source position
```

推断：

```text
Moved
```

但必须明确：

> `Moved` 是 diff engine 的匹配结果，不是语义层“用户把这一段移动了”的绝对证明。

必须避免误把：

```text
same repeated line
```

全部标成 Move。

---

# 54. Diff Determinism

同样的：

```text
A
B
options
```

必须得到同样：

```text
diff hunks
change ordering
move detection
```

不能因为：

```text
HashMap iteration
thread scheduling
filesystem order
```

导致 diff 结果变化。

---

# 55. Large Text Compare

不要允许：

```text
100 MB
→ whole UI freeze
```

必须建立：

```text
max compare size
max lines
max diff work
```

这些限制必须：

```text
centralized
configurable where appropriate
documented
testable
```

当超出限制：

```text
Explicitly Unsupported / TooLarge
```

不能：

```text
UI hangs
process silently killed
fake "0 changes"
```

---

# 56. Text Extractor 总体架构

统一：

```text
Text
 ↓
Extractor Definition
 ↓
Matcher
 ↓
Range / Match
 ↓
Dedup / Ordering
 ↓
Result
```

不同 extractor：

```text
URL
Email
File Path
Number
IP
JSON
Markdown Link
Regex
```

不能为每一种结果设计完全不同 DTO。

---

# 57. Extract Match 模型

建议：

```text
ExtractMatch
├── type
├── value
├── raw_value
├── start
├── end
├── line
├── column
├── confidence? 
└── metadata?
```

只有确实存在 confidence 才提供。

不要伪造：

```text
confidence = 0.98
```

---

# 58. URL Extractor

至少识别：

```text
http://
https://
```

可以根据实际设计增加：

```text
ftp://
mailto:
```

但：

```text
URL
```

不能只是：

```text
starts with http
```

必须避免把：

```text
https://example.com).
```

中的：

```text
).
```

错误并入。

需要：

```text
trim surrounding punctuation
```

但必须测试边界。

---

# 59. Email Extractor

至少识别：

```text
name@example.com
```

处理常见：

```text
punctuation
quotes
parentheses
multiple emails
```

不要宣称：

> 完整 RFC email parser

除非真实实现确实如此。

建议文档写：

```text
practical email extraction
```

并列出限制。

---

# 60. File Path Extractor

必须考虑平台：

```text
Windows
Unix
relative path
UNC
quoted path
```

至少：

```text
C:\foo\bar.txt
\\server\share\file.txt
/home/user/file.txt
../foo.txt
```

是否把：

```text
foo/bar.txt
```

识别为 path 必须有明确 heuristic。

不能让 path extractor 把普通：

```text
word/word
```

全部误认为文件路径。

---

# 61. Number Extractor

必须明确支持：

```text
integer
decimal
negative
scientific notation
percentage
```

是否包含：

```text
currency
hex
binary
octal
```

需要写到 DECISIONS。

不能简单：

```text
\d+
```

就称为完整 Number Extractor。

---

# 62. IP Extractor

至少：

```text
IPv4
IPv6
```

IPv4 要验证：

```text
0-255
```

不要把：

```text
999.999.999.999
```

视为有效 IP。

IPv6 必须使用真正 parser / validator，而不是仅 regex。

---

# 63. JSON Extractor

这是 M4 的重要能力。

输入可能：

```text
prefix text
{...}
suffix text
```

需要找到：

```text
JSON object
JSON array
```

并且支持：

```text
nested object
nested array
strings containing braces
escaped quotes
```

不能：

```text
first { → last }
```

简单截取。

必须使用：

```text
balanced structural scan
```

或者 parser-backed candidate extraction。

---

# 64. JSON Extraction Safety

对于：

```text
{"x":"}","nested":{"a":1}}
```

必须正确处理：

```text
braces inside strings
escaped quote
nested depth
```

测试必须覆盖。

---

# 65. Markdown Link Extractor

至少支持：

```markdown
[text](url)
```

可支持：

```markdown
<https://example.com>
```

结果至少：

```text
label
url
range
```

必须避免：

```text
code block
```

中的 markdown 被错误解释，具体规则需在 DECISIONS 中明确。

---

# 66. Regex Extractor

用户输入：

```text
regex
```

系统执行：

```text
compile
→ validate
→ execute
```

错误必须可理解。

例如：

```text
Invalid regular expression
```

而不是：

```text
Unknown Error
```

---

# 67. Regex Engine

必须明确当前实际采用：

```text
Rust regex
```

还是：

```text
JavaScript RegExp
```

还是其他实现。

如果使用不支持：

```text
lookbehind
backreference
```

等特性的 engine：

必须在 UI / docs 中明确。

禁止：

> UI 宣称支持 PCRE，实际上使用不支持 PCRE 的 engine。

---

# 68. Regex ReDoS

Regex 是安全边界。

必须避免：

```text
unbounded catastrophic backtracking
```

如果采用 Rust `regex` 类线性时间 engine：

应优先保持这一安全特性。

如果当前实现使用其他 engine：

必须建立：

```text
timeout
input size limit
execution guard
```

不能因为一个恶意 regex 卡死 UI。

---

# 69. Extract Result Ordering

所有 Extractor 默认：

```text
source order
```

即：

```text
start offset ascending
```

同 offset：

```text
deterministic secondary ordering
```

不同运行结果不能随机。

---

# 70. Duplicate Extract Results

必须定义：

```text
Deduplicate results
```

是否默认开启。

推荐：

```text
保留 source occurrence
```

同时允许：

```text
unique values view
```

因为：

```text
a@example.com
a@example.com
```

在提取场景中可能有：

```text
2 occurrences
1 unique value
```

两个有价值的事实。

---

# 71. Text Transformer 总体架构

统一：

```text
Input
 ↓
Transform Operation
 ↓
Output
```

每一个 operation 必须：

```text
纯逻辑
无文件 side effect
```

直到：

```text
Apply / Save / Export
```

---

# 72. Trim

明确 Trim 的语义：

默认建议：

```text
trim each line leading/trailing whitespace
```

同时可以提供：

```text
trim document
```

必须区分：

```text
line trim
document trim
```

不要一个 `trim()` 按钮做一切。

---

# 73. Deduplicate Lines

至少支持：

```text
Keep First
Keep Last
```

默认：

```text
Keep First
```

必须保留：

```text
original ordering
```

例如：

```text
A
B
A
C
```

结果：

```text
A
B
C
```

不能：

```text
A
B
C
```

之后再偷偷 sort。

---

# 74. Deduplicate 空行

需要明确：

```text
blank lines count as duplicate lines
```

还是：

```text
blank lines preserved
```

推荐提供选项：

```text
Deduplicate all lines
Ignore blank lines
```

记录到：

```text
DECISIONS.md
```

---

# 75. Sort Lines

至少支持：

```text
Ascending
Descending
Case-sensitive
Case-insensitive
```

可以增加：

```text
Natural sort
Numeric sort
```

但不是强制。

排序必须 deterministic。

测试：

```text
a
A
a10
a2
10
2
中文
accented
empty
```

---

# 76. Empty Line Sorting

必须明确：

```text
empty lines first
empty lines last
preserve position
```

不能让平台比较器自行决定。

---

# 77. Prefix / Suffix

例如：

```text
input:
apple
banana
```

Prefix：

```text
- apple
- banana
```

Suffix：

```text
apple,
banana,
```

必须支持：

```text
blank line policy
selected lines
all lines
```

如果只做全量：

必须 UI 明确显示：

```text
All Lines
```

---

# 78. Case Conversion

至少：

```text
UPPERCASE
lowercase
Title Case
Sentence case
```

Unicode case conversion 必须使用稳定 Unicode 规则。

注意：

```text
Title Case
```

不应该简单：

```text
first char upper
rest lower
```

如果采用简单模式：

必须明确其行为。

---

# 79. Line Numbering

至少支持：

```text
start number
increment
separator
padding
```

例如：

```text
1. alpha
2. beta
3. gamma
```

必须确保：

```text
原内容本身不丢失
```

---

# 80. Find / Replace

至少：

```text
literal find
literal replace
replace first
replace all
```

必须提供：

```text
match count
```

Preview 中明确：

```text
12 matches
```

不能：

```text
点击 Replace All
```

直接改文件。

---

# 81. Regex Replace

流程：

```text
Compile regex
 ↓
Validate
 ↓
Find matches
 ↓
Generate result
 ↓
Preview
```

必须处理：

```text
invalid regex
zero-length match
large input
replacement syntax error
```

---

# 82. Replacement Syntax

必须明确使用哪种语法：

例如：

```text
$1
$2
```

或者：

```text
\1
```

只能选择一种主要 contract。

不要让 UI 看起来兼容两者，后端却只支持其中一种。

---

# 83. Transformer 的 Selection

M4 第一版可以只支持：

```text
whole document
```

如果支持选区：

必须完整处理：

```text
selection range
offset conversion
newline preservation
```

不要为了“看起来高级”做半成品 selection。

---

# 84. Preview 系统

M4 必须复用 M2 已建立的：

```text
Plan
Preview
Validation
Transaction
```

不得创建：

```text
TextPreview
FilePreview
BatchPreview
```

三套互相不兼容的系统。

---

# 85. 文本 Preview

对于 Formatter / Transformer：

显示：

```text
Before
After
```

至少支持：

```text
side-by-side
unified
```

如果是大文本：

必须支持：

```text
preview truncation
```

但必须明确：

```text
Preview truncated
```

不能让用户误以为看到的是全部结果。

截断只影响预览显示，不影响 Apply 的完整执行语义——Apply 始终基于完整输入。

---

# 86. Preview 不允许副作用

绝对禁止：

```text
Preview
→ write file
```

或：

```text
Preview
→ clipboard replace
```

Preview 必须：

```text
side-effect free
```

---

# 87. Apply

用户明确点击：

```text
Apply
```

后才允许：

```text
write file
```

如果写回原文件：

必须：

```text
confirm destructive action
```

---

# 88. Save As

建议优先提供：

```text
Save As
```

作为低风险输出方式。

例如：

```text
input.json
→ output.formatted.json
```

或者由用户选择路径。

---

# 89. Overwrite Original

如果支持：

```text
Overwrite original
```

必须：

```text
Preview
→ explicit confirmation
→ revalidation
→ write
→ transaction
→ history
```

不得：

```text
Ctrl+S
→ silently overwrite
```

除非这是经过明确 UX 决策且仍具备安全门禁。

---

# 90. TOCTOU

这是 M4 必须继承 M2 的重要安全原则。

场景：

```text
用户打开 A.txt
↓
用户生成 Preview
↓
外部程序修改 A.txt
↓
用户 Apply
```

系统不能：

```text
直接覆盖外部最新文件
```

必须重新验证：

```text
expected identity
size
mtime
或者更强内容 identity
```

具体采用哪种机制，必须基于当前 M2 实现。

如果发生变化：

```text
FileChangedSincePreview
```

必须拒绝自动写回，并要求：

```text
Reload
Recalculate Preview
```

---

# 91. 文本写入策略

优先采用：

```text
temp file
→ flush
→ replace
```

或者当前项目已经采用的可靠原子写策略。

目标：

```text
partial write
crash
disk full
permission failure
```

都不会轻易造成：

```text
原文件损坏
```

---

# 92. Atomic Write

必须测试：

```text
write failure
rename failure
permission denied
destination exists
disk error injection where possible
```

不要假设：

```text
write_all
```

成功即可宣布事务完成。

---

# 93. Transaction

M4 文件修改必须复用：

```text
OperationId
OperationPlan
OperationTransaction
HistoryEntry
```

不要创建：

```text
TextUndoRecord
```

与 M2 History 平行存在。

---

# 94. Undo

Formatter / Transformer 写回后：

```text
Undo
```

必须：

```text
安全
可验证
不覆盖用户后续修改
```

如果用户写回后又手动修改：

```text
Undo
```

必须拒绝覆盖。

---

# 95. History

History 至少记录：

```text
operation id
tool
operation type
timestamp
source count
changed count
status
undoable
```

不记录：

```text
entire file contents
```

除非当前项目已有明确、安全、必要的 content history design。

M4 不应该因为 Undo 而突然建立巨大版本数据库。

---

# 96. Copy Result

对于：

```text
Formatter
Transformer
Extractor
Compare
```

优先提供：

```text
Copy Result
```

不一定需要：

```text
保存文件
```

例如：

```text
Regex Extractor
→ Copy All
```

不需要伪造一个 filesystem operation。

---

# 97. Export Result

允许：

```text
Save
Save As
Export
```

根据工具语义决定。

Extractor 可以导出：

```text
TXT
CSV
JSON
```

但注意：

> CSV structured data semantics 属于 M5。

因此 M4 如果导出 CSV，只能是：

```text
simple extraction result export
```

不要提前实现：

```text
CSV Studio
CSV schema
data cleaning
```

---

