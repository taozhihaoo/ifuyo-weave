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

# 98. Text Formatter UI

推荐统一布局：

```text
┌─────────────────────────────────────┐
│ Text Formatter                      │
├─────────────────────────────────────┤
│ Input                               │
│                                     │
│ [JSON ▼] [Format] [Minify]          │
│ [Validate] [Sort] [Normalize]       │
├─────────────────────────────────────┤
│ Options                             │
├─────────────────────────────────────┤
│ Preview                             │
│ Before       │ After                │
├─────────────────────────────────────┤
│ Copy      Save As      Apply        │
└─────────────────────────────────────┘
```

UI 仅为结构参考。

必须遵守现有 Weave design system。

---

# 99. Compare UI

结构：

```text
┌─────────────────────────────────────┐
│ Text Compare                        │
├────────────────┬────────────────────┤
│ Text A         │ Text B             │
│                │                    │
├────────────────┴────────────────────┤
│ [Side-by-side] [Unified]            │
│ [Ignore WS] [Ignore Case]           │
├─────────────────────────────────────┤
│ Diff Summary                        │
│ Added:     12                       │
│ Removed:   8                        │
│ Changed:   4                        │
│ Moved:     2                        │
└─────────────────────────────────────┘
```

---

# 100. Extractor UI

结构：

```text
Input
 ↓
Extractor type
 ↓
Options
 ↓
Extract
 ↓
Results
```

结果表至少：

```text
Type
Value
Line
Column
```

支持：

```text
Copy
Copy All
Export
Unique
```

---

# 101. Transformer UI

结构：

```text
Input
 ↓
Operation
 ↓
Options
 ↓
Preview
 ↓
Copy / Save / Apply
```

不要让：

```text
每一种 transformer
```

拥有完全不同 UI 架构。

---

# 102. Drag & Drop

必须支持：

```text
Drop text file
```

然后：

```text
Identify
 ↓
Open Text
```

如果文件不是文本：

不能：

```text
强制当 UTF-8 读
```

必须给出：

```text
Unsupported / Binary / Unknown Encoding
```

---

# 103. Binary Detection

不能简单：

```text
contains zero byte
→ binary
```

作为唯一判断。

应复用 M1 已有文件事实/encoding 能力。

如果当前已有：

```text
text detection
```

必须复用。

---

# 104. 大文本策略

必须建立：

```text
Max Text Size
Max Preview Size
Max Compare Size
Max Extract Input Size
Max Transformation Size
```

这些可以：

```text
不同
```

不能所有工具统一一个魔法数。

原因：

```text
Formatter
Diff
Regex
Transformation
```

计算模型不同。

---

# 105. 大文本处理

能做流式就：

```text
Streaming
```

不能安全流式的：

```text
Diff
Parser-backed Formatter
Complex Regex
```

必须使用：

```text
bounded memory
```

超出限制时：

```text
明确拒绝
```

不能：

```text
swap machine into death
```

---

# 106. UI 非阻塞

Rust 计算不得：

```text
block UI thread
```

需要：

```text
async task
worker
spawn_blocking
```

或者当前架构对应的等价方案。

必须支持：

```text
Progress
Cancellation
```

适用于：

```text
large text
large diff
large extraction
large transformation
```

---

# 107. Cancellation

取消必须是真取消。

禁止：

```text
按钮灰掉
但后台仍继续完整处理
```

必须在：

```text
parse
format
diff
regex extraction
transform
write
```

等可能长时间运行的阶段检查 cancellation。

---

# 108. Cancellation 后状态

取消后必须：

```text
Cancelled
```

而不是：

```text
Failed
```

如果：

```text
write has already started
```

必须根据实际 transaction 状态准确报告。

不能：

```text
Cancel
→ 显示 Success
```

---

# 109. Progress

不要伪造：

```text
99%
```

如果无法精确计算进度：

可以：

```text
Indeterminate
```

或者分阶段：

```text
Reading
Parsing
Processing
Generating Result
Writing
```

如果能提供：

```text
processed bytes
total bytes
```

优先。

---

# 110. Error Model

M4 至少覆盖：

```text
InvalidInput
UnsupportedFormat
InvalidEncoding
DecodeFailed
ParseFailed
InvalidRegex
TextTooLarge
DiffTooLarge
OperationCancelled
FileMissing
FileChanged
PermissionDenied
ReadFailed
WriteFailed
AtomicReplaceFailed
PathUnsafe
DestinationExists
```

不得：

```text
catch all → "Something went wrong"
```

---

# 111. Parser Error

UI 至少告诉用户：

```text
What failed
Where failed
Why failed
```

例如：

```text
JSON syntax error
Line 18, Column 7
Unexpected token
```

而不是：

```text
Parse error
```

---

# 112. Format Detection Error

如果：

```text
Auto
```

无法判断：

UI：

```text
Format could not be detected.
Choose a format manually.
```

不能：

```text
guess JSON
```

然后把用户输入破坏掉。

---

# 113. Formatter Dependency Policy

首先检查当前：

```text
Cargo.toml
Cargo.lock
package.json
```

已经存在什么。

优先：

```text
Reuse
```

只有确有必要时增加依赖。

每个新依赖必须检查：

```text
maintenance
license
security
build impact
binary size
platform compatibility
```

不要为了：

```text
一个简单 parser
```

引入：

```text
巨大 runtime
```

推荐候选与评估顺序（最终选型仍须走依赖评估并记录 DECISIONS.md）：
- YAML：serde_yaml 已停止维护（archived），必须评估维护中的替代（如 serde_yml、yaml-rust2），不得默认引入 archived crate。
- Diff：similar 或等价成熟 crate。
- 编码检测：encoding_rs + chardetng（Mozilla 系，Apache-2.0，覆盖 GB18030）。
- SQL / JS / CSS formatter：优先轻量 tokenizer-based 方案；禁止引入完整编译器工具链级重型依赖（如 JS 全家桶）；任何超出常规体量的依赖必须先在 DECISIONS.md 论证体积与 license 成本。

---

# 114. 禁止 Node Runtime Sneak-in

如果项目目标是：

```text
Tauri desktop app
```

不要为了 JS/CSS/Markdown formatter：

```text
偷偷依赖用户本机 Node.js
```

也不要要求用户：

```text
npm install global package
```

M4 应该成为：

> **用户安装 Weave 后立即可用的桌面工具。**

如果项目确实选择嵌入：

```text
JS/WASM formatter
```

必须记录构建与许可证成本。

---

# 115. Test Architecture

测试至少四层：

```text
Unit
Integration
Fault
UI Smoke
```

如果适合：

```text
Property
Fuzz
Performance
```

---

# 116. Unit Test：JSON

至少覆盖：

```text
valid JSON
invalid JSON
nested object
nested array
escaped string
unicode
number
null
empty object
empty array
duplicate keys policy
```

重复 key 行为必须与 parser 的真实语义一致，并记录。

---

# 117. Unit Test：XML

覆盖：

```text
valid
invalid
nested
attributes
comments
CDATA
namespace
empty node
```

---

# 118. Unit Test：YAML

覆盖：

```text
mapping
sequence
nested
comments
quoted scalar
multiline
null
unicode
invalid indentation
invalid syntax
```

---

# 119. Unit Test：SQL

至少：

```text
SELECT
INSERT
UPDATE
DELETE
JOIN
WHERE
GROUP BY
ORDER BY
nested query
quoted strings
comments
invalid SQL
```

根据实际 dialect 支持范围建立测试。

---

# 120. Unit Test：JavaScript

至少：

```text
function
arrow function
object
array
class
import
export
async
await
template literal
comments
string containing braces
```

---

# 121. Unit Test：CSS

覆盖：

```text
selector
declaration
nested at-rule
media query
custom property
url
string
comments
```

---

# 122. Unit Test：Markdown

覆盖：

```text
heading
paragraph
unordered list
ordered list
quote
link
code span
code fence
table
raw HTML
```

---

# 123. Compare Tests

至少：

```text
identical
one added line
one removed line
one changed line
multiple hunks
moved block
whitespace only
case only
empty A
empty B
empty both
long lines
unicode
emoji
CRLF vs LF
```

---

# 124. Extractor Tests

### URL

```text
http
https
punctuation
multiple urls
unicode text
```

### Email

```text
valid
invalid
multiple
punctuation
```

### Path

```text
Windows
UNC
Unix
relative
quoted
```

### Number

```text
integer
decimal
negative
scientific
```

### IP

```text
valid IPv4
invalid IPv4
valid IPv6
invalid IPv6
```

### JSON

```text
nested
escaped braces
multiple candidates
invalid candidate
```

### Markdown

```text
inline link
autolink
code block
```

### Regex

```text
valid
invalid
zero-length
unicode
multiple captures
```

---

# 125. Transformer Tests

### Trim

```text
leading
trailing
blank lines
mixed
```

### Deduplicate

```text
duplicates
keep first
keep last
empty
case-sensitive
```

### Sort

```text
ascending
descending
numeric
natural
unicode
case
```

### Prefix/Suffix

```text
all lines
blank lines
empty document
unicode
```

### Case

```text
upper
lower
title
sentence
unicode
```

### Numbering

```text
start
increment
padding
separator
blank lines
```

### Replace

```text
first
all
no match
special chars
```

### Regex Replace

```text
captures
invalid
zero-length
unicode
large input
```

---

# 126. Property / Invariant Tests

适合的 operation 应验证：

```text
Format then Validate → valid
Minify then Validate → valid
Whitespace Ignore → changing whitespace only should not create semantic diff
Case Ignore → case-only changes disappear
Deduplicate → result contains no duplicate according to selected equality
Sort → deterministic
Prefix + reverse → original recoverable where mathematically applicable
```

注意：

不要为了测试而强行定义错误的数学性质。

---

# 127. Formatter Idempotence

对安全的 canonical formatter：

```text
Format(Format(x)) == Format(x)
```

应该成为重要测试。

适用于：

```text
JSON
XML
YAML
SQL
CSS
Markdown
```

在实际 formatter 语义允许时执行。

如果不满足：

必须调查：

```text
bug
non-canonical formatting
unsupported syntax
```

不能简单忽略。

---

# 128. Normalize Idempotence

同样：

```text
Normalize(Normalize(x))
==
Normalize(x)
```

如果设计为 canonical normalize。

必须明确：

```text
哪些 formatter 有此 invariant
```

---

# 129. Minify / Format Round-trip

例如：

```text
valid JSON
→ Format
→ Minify
→ Parse
```

应该：

```text
Parse succeeds
```

并保持语义等价。

对于 object key order，如果 Sort 开启则按实际选项判断。

---

# 130. Text Encoding Tests

必须至少测试：

```text
UTF-8
UTF-8 BOM
UTF-16 LE
UTF-16 BE
LF
CRLF
Unicode
emoji
中文
combining characters
```

并验证：

```text
read
transform
write
read again
```

不会莫名损坏。

---

# 131. Round-trip File Integration

建立真实临时目录：

```text
create input
 ↓
open
 ↓
detect
 ↓
transform
 ↓
preview
 ↓
apply
 ↓
read back
 ↓
assert
```

必须使用：

```text
real filesystem
```

而不是只 mock。

---

# 132. TOCTOU Integration Tests

至少：

```text
open file
→ preview
→ external modification
→ apply
→ expect rejection
```

以及：

```text
open file
→ preview
→ external delete
→ apply
→ expect FileMissing
```

以及：

```text
save as
→ destination externally created
→ apply
→ expect collision
```

---

# 133. History Integration Test

必须：

```text
transform file
→ history entry exists
→ restart
→ history still exists
→ undo
→ file restored
```

具体步骤按当前项目 History 实现。

---

# 134. Undo Conflict Test

场景：

```text
original
→ M4 transform
→ user manually edits
→ Undo
```

结果必须：

```text
Undo refused safely
```

不能：

```text
overwrite user edit
```

---

# 135. Permission Fault Test

至少：

```text
read permission denied
write permission denied
destination permission denied
```

根据 Windows 当前测试环境和 CI 可模拟的程度：

可以使用：

```text
fault injection
```

但至少还必须保留：

```text
real filesystem integration
```

---

# 136. Large File Tests

至少验证：

```text
1 MB
10 MB
50 MB
100 MB
```

具体值根据当前机器/CI 能力。

重点不是：

> “必须处理无限大文件。”

重点是：

> **明确知道 M4 在什么边界内可靠工作。**

---

# 137. Memory Tests

禁止：

```text
std::fs::read()
```

在所有文本路径上无脑读取巨型文件。

对于需要 whole-document memory 的操作：

必须：

```text
bounded
```

并说明：

```text
memory model
maximum supported input
```

---

# 138. Performance Scenarios

至少测：

```text
10k lines
100k lines
1M lines
```

用于：

```text
Trim
Deduplicate
Sort
Prefix
Replace
Extract
```

Compare 至少测试：

```text
10k
100k
```

或者根据实际算法限制。

Formatter 根据各 parser 的真实能力测试。

---

# 139. Performance Report

完成后必须记录真实数据：

```text
Operation
Input Size
Line Count
Elapsed
Peak Memory
Result Size
```

不能写：

```text
Fast enough
```

这种没有证据的结论。

---

# 140. Regex Performance

必须测试：

```text
large text
pathological input
many matches
zero-length patterns
```

如果 engine 本身提供线性保证：

记录为：

```text
design property
```

而不是自己宣称：

```text
all regexes are safe
```

---

# 141. Compare Performance

至少记录：

```text
A size
B size
A lines
B lines
elapsed
memory
hunks
```

对超限：

```text
explicit rejection
```

---

# 142. UI Smoke Test

必须真实运行：

```text
Tauri dev
```

并验证：

### Formatter

```text
drop JSON
format
preview
copy
save
```

### Compare

```text
paste A
paste B
side-by-side
unified
toggle whitespace
toggle case
```

### Extractor

```text
paste text
select extractor
run
show results
copy all
```

### Transformer

```text
paste text
transform
preview
copy
save/apply
```

---

# 143. Real UX Smoke

必须验证：

```text
new user
```

在没有阅读 README 的情况下：

```text
Drag
Understand
Preview
Execute
```

能够完成至少一次真实操作。

---

# 144. UX 原则

遵守：

```text
常用操作 ≤ 3 步
错误可理解
Preview 明确
成功结果明确
没有静默破坏
```

Charter 的核心产品交互仍然是：

```text
Drop
→ Understand
→ Preview
→ Weave
→ Done
```



---

# 145. UI 不得堆满高级选项

默认界面优先：

```text
核心操作
```

高级设置放：

```text
Options
Advanced
```

不要第一屏显示：

```text
20 个 parser flag
15 个 formatting option
```

---

# 146. Empty State

Formatter：

```text
Drop a text file or paste text.
```

Compare：

```text
Add two texts to compare.
```

Extractor：

```text
Paste text or drop a file.
```

Transformer：

```text
Enter text to transform.
```

文案走：

```text
i18n
```

禁止散落 hardcoded strings。

---

# 147. Error UX

不要：

```text
Error
```

应该：

```text
JSON is invalid.
Line 18, Column 7.
```

必要时：

```text
[Go to location]
```

---

# 148. Result UX

成功后明确：

```text
Formatted successfully
24 changes
```

或者：

```text
Found 47 URLs
```

或者：

```text
Removed 18 duplicate lines
```

不要：

```text
Done
```

却不告诉用户做了什么。

---

# 149. i18n

所有用户可见文案必须进入：

```text
zh-CN
en
```

包括：

```text
formatter names
errors
warnings
progress
result
empty state
confirmation
unsupported
```

禁止业务代码散落：

```text
"Format"
"格式化"
"Failed"
```

---

# 150. Accessibility

至少保证：

```text
keyboard focus
buttons have labels
contrast
error not color-only
diff status not color-only
```

Side-by-side diff 不得仅靠：

```text
red / green
```

判断变化。

必须有：

```text
Added
Removed
Changed
Moved
```

文本语义。

---

# 151. Theme / Design Tokens

必须复用现有：

```text
Weave design tokens
```

禁止：

```text
业务组件裸写大量 hex/rgb
```

并继续满足项目已有的设计系统 lint 约束。

---

# 152. Text Editor

M4 如果需要编辑区域：

优先使用当前项目已经存在、或最小必要的编辑方案。

不要为了：

```text
代码高亮
```

突然引入：

```text
完整 IDE editor
```

除非真实需求与当前架构都支持。

M4 的 editor 目标是：

```text
文本输入
阅读
选择
预览
```

不是：

```text
VS Code clone
```

---

# 153. Syntax Highlighting

可以加入：

```text
basic syntax highlighting
```

但它属于：

```text
UX enhancement
```

不是核心正确性。

如果实现：

必须保证：

```text
highlighting failure
```

绝不能影响：

```text
actual formatter / parser
```

---

# 154. No Syntax Highlighting Dependency Trap

不要让：

```text
syntax highlight
```

成为：

```text
formatter
```

的依赖。

正确：

```text
parser/formatter
≠
presentation highlighter
```

---

# 155. Text Tool Commands

Tauri command 层建议逻辑上区分：

```text
text.detect
text.validate
text.format
text.minify
text.sort
text.normalize
text.compare
text.extract
text.transform
```

具体命名根据仓库现有 convention 调整。

不要创建：

```text
do_everything(text, mode, options_json)
```

这种不可维护的超级 command。

---

# 156. IPC DTO

IPC 必须使用：

```text
serializable DTO
```

不要把：

```text
Rust parser internal type
```

直接暴露给前端。

Frontend 不应该知道：

```text
serde_json internal AST
XML parser node
SQL parser internal token
```

---

# 157. IPC Error Serialization

错误应稳定：

```text
code
message
range?
details?
```

前端不得：

```text
parse Rust error string
```

来判断错误类型。

---

# 158. Structured Output

Formatter：

```text
success
output
diagnostics
stats
```

Compare：

```text
summary
hunks
moves
diagnostics
```

Extractor：

```text
matches
count
unique_count
diagnostics
```

Transformer：

```text
output
change_count
diagnostics
```

---

# 159. Stats

可以提供：

```text
input bytes
input lines
output bytes
output lines
changes
matches
duration
```

但不要为了显示漂亮而计算：

```text
假数据
```

---

# 160. Structured Errors 与 Partial Failure

Extractor 可以：

```text
某一项无法解析
```

但不要把：

```text
entire operation
```

错误地标成成功。

应区分：

```text
Success
SuccessWithWarnings
Failed
Cancelled
Unsupported
```

如果实际适用。

---

# 161. Text Compare 不改变原始文本

再次强调：

```text
Compare
```

永远是：

```text
read-only
```

除非用户明确执行：

```text
Copy selected
Export diff
```

M4 不做：

```text
Auto merge
```

因为这会自然滑向：

```text
3-way merge
conflict resolution
Git merge
```

属于 Prism / 专用工具方向。

---

# 162. M4 与 Prism 的边界

Weave M4 做：

```text
general local text utility
```

不做：

```text
Git-aware diff
commit diff
branch comparison
code review
project-aware source navigation
AST refactoring
LSP
```

因为：

```text
Weave = files / text / data / documents
Prism = software / code / Git / project
```

必须保持产品边界。

---

# 163. M4 与 M5 的边界

M4：

```text
JSON as text
```

M5：

```text
JSON as data
CSV as data
TSV as data
schema
rows
columns
data cleaning
conversion
```

不要因为：

```text
JSON
```

属于两个 milestone：

```text
M4
M5
```

就实现两套 JSON parser infrastructure。

原则：

> parser 可以复用，但业务语义必须分层。

---

# 164. Shared Parser Infrastructure

如果多个工具都需要：

```text
JSON parse
```

统一底层能力：

```text
JSON parser adapter
```

例如：

```text
parse JSON
→ AST/value
→ formatter
→ validator
→ extractor
```

不要：

```text
formatter 自己 parse
extractor 再写一套 parser
validator 再写 regex
```

---

# 165. Shared Encoding Infrastructure

同样：

```text
Encoding
LineEnding
TextRange
Diagnostics
```

必须共享。

禁止：

```text
Formatter encoding logic
Transformer encoding logic
Extractor encoding logic
```

各写一套。

---

# 166. Shared Text Limits

所有 limits 必须统一来源，例如：

```text
TextLimits
```

而不是散落：

```text
const MAX = 1000000
```

到处都是。

---

# 167. Security：Path Safety

M4 任何：

```text
Open
Save
Save As
Export
```

都必须经过：

```text
Path Validation
```

不能由 UI 直接拼接：

```text
../../
```

然后交给 fs。

---

# 168. Security：Path Traversal

测试：

```text
../
..\ 
absolute escape
UNC
symlink
junction
```

继续遵守 M1 的统一 path safety policy。

---

# 169. Security：Symlink

不要重新制定：

```text
Symlink semantics
```

复用 M1。

如果 current implementation 有限制：

文档中明确：

```text
Supported
Blocked
Platform-dependent
```

---

# 170. Security：No Shell

禁止：

```text
shell out to jq
shell out to python
shell out to prettier
shell out to xmllint
shell out to diff
```

来完成核心功能。

尤其不能：

```text
user text
→ shell command
```

这种高风险路径。

---

# 171. Security：Regex

用户输入：

```text
pattern
replacement
```

必须被视为：

```text
untrusted input
```

禁止：

```text
shell interpolation
```

---

# 172. Security：File Content

文件内容：

```text
never sent to network
```

不能因为：

```text
“以后可以 AI 格式化”
```

而偷偷上传。

---

# 173. Privacy

继续保持：

```text
No Telemetry
No Account
No Cloud Requirement
No File Upload
Local-first
```

与 M0/M1/M2/M3 保持一致。

---

# 174. Logging

日志只能记录必要元数据：

可以：

```text
operation id
tool
duration
status
input size
result size
```

不要默认记录：

```text
full text
full regex
full file content
password-like content
tokens
secrets
```

---

# 175. Sensitive Text

即使用户贴入：

```text
API key
password
JWT
private key
```

系统也不能在正常日志中：

```text
echo full content
```

---

# 176. Clipboard Privacy

Copy 操作可以使用：

```text
local clipboard
```

但：

```text
不要上传 clipboard
```

同时不要将：

```text
clipboard content
```

写入诊断日志。

---

# 177. Test Fixtures

不要提交：

```text
大量真实项目源码
真实 secrets
真实用户文件
超大 binary
```

应生成：

```text
deterministic textual fixtures
```

例如：

```text
JSON fixture
YAML fixture
SQL fixture
JS fixture
CSS fixture
Markdown fixture
Unicode fixture
large synthetic text
```

---

# 178. Golden Tests

Formatter 非常适合：

```text
input
expected output
```

建立 golden tests。

结构：

```text
fixtures/text/
├── json/
├── xml/
├── yaml/
├── sql/
├── javascript/
├── css/
└── markdown/
```

每个 fixture 至少有：

```text
input
expected
options
```

或者当前项目采用等价结构。

---

# 179. Golden Test 原则

如果更新 formatter：

必须看到：

```text
golden diff
```

不能：

```text
regen all snapshots
```

然后说：

```text
tests pass
```

却不知道为什么输出全部改变。

---

# 180. Fuzz / Property Testing

如果当前工具链适合：

优先为：

```text
JSON
XML
YAML
Diff
Regex handling
```

加入 fuzz/property tests。

重点：

```text
不能 panic
不能无限循环
不能 OOM in bounded inputs
```

---

# 181. Malformed Input

所有 parser 都必须测试：

```text
empty
truncated
unexpected token
huge nesting
weird unicode
invalid encoding
invalid escape
```

目标：

```text
error
```

不是：

```text
panic
```

---

# 182. Deep Nesting

特别测试：

```text
JSON 1000+ nesting
XML deep nesting
YAML deep nesting
```

实际阈值根据 parser / stack 风险决定。

必须避免：

```text
stack overflow
```

---

# 183. Parser Resource Limits

如果 parser 本身允许：

必须配置：

```text
max depth
max input
max output
```

或者通过外层限制。

必须明确。

---

# 184. Formatter Output Explosion

尤其对：

```text
malformed
deep
repeated
```

输入：

不能产生：

```text
unbounded expansion
```

例如：

```text
input 1 MB
→ output 20 GB
```

必须有合理保护。

---

# 185. Result Size Limits

Extractor：

```text
可能产生大量 matches
```

必须防止：

```text
100 MB input
→ millions of UI rows
```

可以采用：

```text
result cap
pagination
virtualization
```

至少不能一次把巨量结果全部渲染到 DOM。

---

# 186. Virtualized Results

Extractor 与 diff UI 在大结果量下：

优先：

```text
virtual list
```

避免：

```text
100000 DOM nodes
```

---

# 187. Compare Rendering

Diff UI 也必须避免：

```text
每个字符一个 React component
```

导致性能崩溃。

推荐：

```text
hunk
line
token
```

合理分层。

---

# 188. React State

不要为了 M4 建立一个：

```text
global mega store
```

包含：

```text
formatter
compare
extractor
transformer
history
filesystem
settings
```

按真实边界拆分。

---

# 189. Avoid Over-Abstraction

M4 禁止：

```text
TextOperationFactoryFactory
FormatterStrategyProviderFactory
GenericTransformRegistryManager
```

这种为了“架构漂亮”而过度设计。

先建立：

```text
small stable abstractions
```

只有存在真实重复时再抽象。

---

# 190. Formatter Registry

如果需要统一工具发现：

可以：

```text
FormatterRegistry
```

但至少要求：

```text
id
display name
supported operations
format detection
```

不要：

```text
dynamic plugin framework
```

提前进入 M4。

---

# 191. Transformer Registry

同理：

```text
TransformerDefinition
```

可以统一：

```text
id
label
options schema
transform
```

但：

> 不要提前实现通用 M7 Batch Tool SDK。

---

# 192. Preview Reuse Boundary

M4 可以：

```text
调用共享 Preview model
```

但不要把：

```text
Text Formatter
```

做成：

```text
M7 generic batch pipeline
```

M7 以后再泛化。

---

# 193. M4 是否需要 Batch

单次文本：

```text
one document
```

必须完整支持。

如果现有架构很自然地支持：

```text
multiple files
```

可以复用基础能力。

但是：

> M4 不实现通用多文件 Batch Engine。

也不得提前实现：

```text
batch recipe
pipeline
bulk tool composition
```

---

# 194. Multi-file Text

如果允许：

```text
select multiple text files
```

必须明确：

```text
这是 M4 的 convenience
```

不是：

```text
M7 Batch Engine
```

建议第一版重点保证：

```text
single document
```

体验完整。

---

# 195. Text Compare Input Sources

支持：

```text
paste A
paste B
drop A
drop B
```

可以允许：

```text
Open File A
Open File B
```

读取必须复用：

```text
filesystem abstraction
encoding
text detection
```

---

# 196. Compare Different Encodings

例如：

```text
A = UTF-8
B = UTF-16
```

如果都能正确 decode：

Compare 应在：

```text
decoded text
```

层比较。

不能因为编码不同就直接：

```text
binary diff
```

除非用户显式选择二进制比较——但那不属于 M4。

---

# 197. Compare Line Endings

默认建议：

```text
normalize line endings before textual diff
```

或者提供选项：

```text
Ignore line endings
```

必须明确。

不能：

```text
Windows file vs Linux file
```

因为 CRLF 差异造成海量 Added/Removed，却 UI 没有解释。

---

# 198. Formatter / Transformer Final Newline

必须统一策略：

```text
Preserve
Always
Never
```

默认行为要稳定。

不要：

```text
JSON formatter = always
Markdown formatter = never
Transformer = random
```

除非有格式语义理由并记录。

---

# 199. Unicode Normalization

M4 可以支持：

```text
NFC
NFD
```

但不是强制。

如果不支持：

必须确保：

```text
do not silently normalize Unicode
```

尤其比较工具。

例如：

```text
é
```

和：

```text
e + combining acute
```

如果不是 explicit normalize：

不能自动视为相同。

---

# 200. Character Equality

Compare 必须定义：

```text
Unicode code point
```

还是：

```text
grapheme cluster
```

默认推荐：

```text
text sequence aware
```

具体由 diff engine 决定。

必须测试：

```text
emoji
combining marks
CJK
```

---

# 201. Text Selection Export

Extractor results 如支持：

```text
Click match
```

可以：

```text
select in source
```

但 offset 必须经过统一：

```text
Range → UI position
```

不要直接假定 JS string index 与 Rust byte offset 相同。

---

# 202. Tool Navigation

Weave M4 至少应拥有：

```text
Formatter
Compare
Extractor
Transformer
```

可以在：

```text
Command Palette
```

中发现。

但 M4 不实现：

```text
完整 command palette
```

如果 M0 已建立，则只补注册。

---

# 203. Deep Link / Open Tool

如果现有架构支持：

```text
Drop JSON
```

可以自动：

```text
Open Formatter
```

但这个“智能推荐”必须：

```text
deterministic
non-invasive
```

不应该：

```text
根据内容乱猜
```

---

# 204. Auto Tool Recommendation

可以：

```text
.json → Formatter
.txt → Transformer
```

但必须允许用户：

```text
Change tool
```

尤其：

```text
JSON
```

也可能只是：

```text
plain text
```

所以推荐：

```text
suggest
not force
```

---

# 205. File Association

M4 不要求修改：

```text
Windows file associations
```

也不实现：

```text
Open With integration
```

除非当前项目已经存在基础设施。

---

# 206. Clipboard Flow

常见场景：

```text
Copy text from browser
↓
Paste into Weave
↓
Format
↓
Copy result
```

这应该是非常低摩擦的体验。

---

# 207. Copy Success

用户点击：

```text
Copy
```

必须反馈：

```text
Copied
```

但不要弹出：

```text
烦人的永久 notification
```

遵循现有 Weave UI language。

---

# 208. No Notification Spam

M4 不要：

```text
每次 formatter 完成
→ OS notification
```

优先：

```text
inline status
toast
```

并与现有项目一致。

---

# 209. Tool-Level Documentation

每个工具至少在项目文档中说明：

```text
Input
Supported formats
Operations
Limits
Known limitations
Encoding behavior
```

尤其：

```text
SQL dialect
Regex engine
Markdown normalization
XML/YAML capability
```

---

# 210. DECISIONS.md

M4 所有重要技术决策写入：

```text
DECISIONS.md
```

至少记录：

```text
Formatter libraries
SQL dialect
Regex engine
encoding policy
line ending policy
BOM policy
offset model
diff algorithm
move detection
large text limits
save strategy
JSON sort semantics
YAML sort semantics
XML sort semantics
Markdown normalize semantics
```

---

# 211. ARCHITECTURE.md

更新：

```text
weave-text
TextDocument
Text Diagnostics
Formatter
Compare
Extractor
Transformer
IPC
Preview integration
File write integration
```

并明确：

```text
weave-text
does not own filesystem implementation
```

---

# 212. PROGRESS.md

M4 完成后：

```text
M0 = COMPLETE
M1 = COMPLETE
M2 = COMPLETE
M3 = COMPLETE
M4 = COMPLETE
M5 = NEXT
```

必须记录：

```text
implemented
tested
performance
known limitations
decisions
handoff
```

---

# 213. README

README 至少增加：

```text
Text Formatter
Text Compare
Text Extractor
Text Transformer
```

并展示简洁 usage。

不要把 README 写成：

```text
全量技术设计文档
```

保持产品级可读性。

---

# 214. PRIVACY.md

确认 M4：

```text
No file upload
No clipboard upload
No telemetry
No network formatting service
```

---

# 215. SECURITY.md

补充：

```text
regex handling
parser input limits
text size limits
path write safety
TOCTOU
atomic write
```

---

# 216. Changelog

如果项目 convention 要求：

加入：

```text
M4 Text
```

只描述用户实际获得的功能。

---

# 217. License Audit

所有新增 parser / formatter 依赖必须：

```text
license compatible
```

并更新：

```text
THIRD_PARTY_LICENSES.md
```

如项目已有自动生成机制：

优先复用。

---

# 218. Dependency Audit

禁止因为 M4：

```text
一口气升级几十个 dependency
```

如依赖冲突：

```text
minimal resolution
```

优先。

---

# 219. Build Matrix

至少检查：

```text
Rust
frontend
Tauri
```

并使用当前项目 CI matrix。

如果 Windows 是正式目标：

必须：

```text
Windows build
```

真实通过。

---

# 220. Static Quality

执行当前项目已有：

```text
cargo fmt
cargo clippy
cargo test
frontend lint
frontend typecheck
frontend test
build
```

具体命令根据仓库真实配置。

不得凭空假设不存在的 scripts。

---

# 221. No Suppressed Tests

绝对禁止：

```text
#[ignore]
.skip()
.skip
expected failure
```

来隐藏 M4 问题。

除非是：

```text
明确记录的 platform-specific unavailable test
```

且 Final Audit 中明确说明。

---

# 222. No Fake Test

禁止：

```text
assert!(true)
```

或者：

```text
assert!(result.is_ok())
```

却不检查：

```text
实际 output
```

测试必须验证：

```text
real behavior
```

---

# 223. No Hardcoded Success

禁止：

```rust
Ok(TextResult {
    success: true,
    output: input,
})
```

冒充：

```text
formatter
```

禁止：

```text
Found 12 URLs
```

却没有真实 extractor。

---

# 224. No Placeholder

禁止：

```text
TODO
FIXME
placeholder
not implemented
mock
fake
temporary
```

出现在声称已完成的核心能力中。

如果某能力确实不属于本阶段：

明确：

```text
NOT IN SCOPE
```

而不是半实现。

---

# 225. M4 Acceptance Criteria

只有全部关键硬门禁满足，才能：

```text
M4 COMPLETE
```

至少：

```text
[ ] TextDocument model exists
[ ] Encoding policy is explicit
[ ] BOM policy is explicit
[ ] Line ending policy is explicit
[ ] Offset model is explicit

[ ] JSON Validate works
[ ] JSON Format works
[ ] JSON Minify works
[ ] JSON Sort works
[ ] JSON Normalize works

[ ] XML Validate works
[ ] XML Format works
[ ] XML Minify works
[ ] XML capabilities are documented

[ ] YAML Validate works
[ ] YAML Format works
[ ] YAML Minify works
[ ] YAML Minify 语义已定义（注释 / 多文档 / anchor 的处理）；做不到语义保持则显式标 Unsupported
[ ] YAML capabilities are documented

[ ] SQL Validate works within declared dialect
[ ] SQL Format works
[ ] SQL Minify works
[ ] SQL Sort / Normalize 有显式 PASS 或 NOT SUPPORTED 结论，不得沉默缺席

[ ] JavaScript Validate works
[ ] JavaScript Format works
[ ] JavaScript Minify works within declared scope

[ ] CSS Validate works
[ ] CSS Format works
[ ] CSS Minify works

[ ] Markdown Format works
[ ] Markdown Normalize works
[ ] Markdown boundaries documented

[ ] Side-by-side diff works
[ ] Unified diff works
[ ] Added works
[ ] Removed works
[ ] Changed works
[ ] Moved works within declared semantics
[ ] Whitespace ignore works
[ ] Case ignore works
[ ] Diff deterministic

[ ] URL extractor works
[ ] Email extractor works
[ ] File path extractor works
[ ] Number extractor works
[ ] IPv4 extractor works
[ ] IPv6 extractor works
[ ] JSON extractor works
[ ] Markdown link extractor works
[ ] Regex extractor works

[ ] Trim works
[ ] Deduplicate Lines works
[ ] Sort Lines works
[ ] Prefix works
[ ] Suffix works
[ ] Case Conversion works
[ ] Line Numbering works
[ ] Find/Replace works
[ ] Regex Replace works

[ ] Preview works
[ ] Preview has no side effect
[ ] Apply requires explicit action
[ ] Save As works
[ ] Safe overwrite works where supported
[ ] TOCTOU protection works
[ ] Transaction reuse works
[ ] History reuse works
[ ] Undo reuse works

[ ] Large text limits exist
[ ] Regex resource limits exist
[ ] Diff limits exist
[ ] UI remains responsive
[ ] Cancellation works where needed
[ ] Progress is honest

[ ] Encoding tests pass
[ ] Unicode tests pass
[ ] Line ending tests pass
[ ] Parser malformed input tests pass
[ ] Fault tests pass
[ ] Integration tests pass
[ ] Real filesystem tests pass
[ ] UI smoke passes

[ ] No shell execution
[ ] No network dependency
[ ] No telemetry
[ ] No file upload
[ ] No secret logging
[ ] No path traversal
[ ] No architecture violation

[ ] M0 regression passes
[ ] M1 regression passes
[ ] M2 regression passes
[ ] M3 regression passes

[ ] Rust quality gates pass
[ ] Frontend quality gates pass
[ ] Build passes
[ ] Documentation updated
[ ] Decisions updated
[ ] Progress updated
[ ] Git hygiene passes
```

---

# 226. M4 不算完成的情况

任何以下情况存在，都不能宣布：

```text
M4 COMPLETE
```

例如：

```text
JSON formatter 可以用
但 XML/YAML/SQL 实际是假实现

Compare 有颜色
但 diff model 不准确

Moved 只是把随机相同行标成 moved

Regex extractor 可以用
但恶意 regex 可以卡死应用

Preview 看起来工作
但 Apply 不做 revalidation

Transform 能工作
但写文件会覆盖用户最新修改

UTF-16 文件打开后乱码
但 UI 显示 Success

CRLF 文件保存后被无意改成 LF

100MB 文本导致 UI 冻死

Parser panic

Diff OOM

Extractor 一次渲染十万 DOM 节点

Undo 会覆盖用户后来修改的文件

History 重启后消失

使用 shell 调 prettier / jq / diff

文件内容发送到网络

业务代码自行访问 filesystem

M3 regression failure
```

任何一项都应：

```text
M4 NOT COMPLETE
```

---

# 227. Final Audit

实施完成后，不要直接宣布完成。

必须输出：

# M4 Final Audit

## A. Baseline

```text
Branch:
HEAD:
Working Tree:
M0:
M1:
M2:
M3:
```

## B. Architecture

检查：

```text
weave-core
weave-text
weave-files
weave-history
Tauri
IPC
React
```

## C. Text Foundation

检查：

```text
TextDocument
Encoding
BOM
Line endings
Offsets
Diagnostics
Limits
```

## D. Formatter

检查：

```text
JSON
XML
YAML
SQL
JavaScript
CSS
Markdown
```

逐项：

```text
Format
Minify
Validate
Sort
Normalize
```

只报告：

```text
PASS
FAIL
NOT SUPPORTED
```

## E. Compare

检查：

```text
Added
Removed
Changed
Moved
Side-by-side
Unified
Whitespace
Case
Determinism
Limits
```

## F. Extractor

检查：

```text
URL
Email
Path
Number
IPv4
IPv6
JSON
Markdown Link
Regex
```

## G. Transformer

检查：

```text
Trim
Deduplicate
Sort
Prefix
Suffix
Case
Line Number
Replace
Regex Replace
```

## H. Preview / Mutation

检查：

```text
Preview
Apply
TOCTOU
Atomic Write
Transaction
History
Undo
```

## I. Security

检查：

```text
Path Traversal
Symlink
Regex Abuse
Parser Abuse
Oversized Input
Secret Logging
Shell Execution
Network
```

## J. Privacy

确认：

```text
No Telemetry
No Upload
No Cloud Requirement
No Remote Processing
```

## K. Performance

输出真实：

```text
Input
Operation
Elapsed
Peak Memory
Output
```

## L. Tests

必须列出真实：

```text
Unit:
Integration:
Fault:
Property:
Fuzz:
Performance:
UI:
```

## M. Build

列出真实结果：

```text
Rust:
Frontend:
Tauri:
CI:
```

## N. Documentation

检查：

```text
README
ARCHITECTURE
DECISIONS
PROGRESS
PRIVACY
SECURITY
CHANGELOG
THIRD_PARTY_LICENSES
```

## O. Known Limitations

必须诚实写：

```text
Not Implemented
Not Supported
Platform-specific
Size-limited
Parser-specific
Dialect-specific
Not Verified
```

## P. M5 Handoff

明确交接：

```text
TextDocument
Encoding
LineEnding
TextRange
Diagnostics
Formatter infrastructure
Parser adapters
Compare engine
Extractor engine
Transformer engine
Preview integration
Safe write
Transaction
History
Undo
```

并明确：

> 后续 M5 Data 必须复用 M4 的文本基础设施，尤其是 Encoding、TextDocument、Diagnostics、Parser Adapter 与安全文件读写能力，不得重新实现第二套文本 I/O 层。

---

# 228. M4 Performance Evidence

必须提供真实数据，而不是：

```text
works fine
fast enough
```

至少记录：

```text
JSON:
input bytes:
output bytes:
elapsed:
peak memory:

Compare:
A size:
B size:
lines:
elapsed:
peak memory:

Extractor:
input size:
matches:
elapsed:
peak memory:

Transformer:
input size:
output size:
elapsed:
peak memory:
```

---

# 229. M4 Security Evidence

最终必须直接给：

```text
Path Traversal: PASS
Symlink Boundary: PASS / PLATFORM-LIMITED
Regex Safety: PASS
Parser Resource Limits: PASS
Large Input Handling: PASS
No Shell: PASS
No Network: PASS
Secret Logging: PASS
TOCTOU Protection: PASS
Atomic Write: PASS
```

禁止写：

```text
Security looks good
```

这种无证据结论。

---

# 230. Git Audit

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
temp files
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

# 231. Commit Strategy

只有：

```text
Final Audit PASS
```

之后才允许 commit。

遵循当前仓库 Git convention。

推荐类似：

```text
feat(text): implement text tools
```

或者：

```text
feat(text): add formatter compare extractor transformer
```

不要：

```text
update
misc
fix
work
```

---

# 232. PROGRESS 状态

M4 完成后：

```text
M0 = COMPLETE
M1 = COMPLETE
M2 = COMPLETE
M3 = COMPLETE
M4 = COMPLETE
M5 = NEXT
```

记录：

```text
What was implemented
What was tested
Performance evidence
Known limitations
Architecture decisions
M5 handoff
```

---

# 233. M4 最终产品状态

完成后用户应该能够直接：

## Formatter

```text
拖入 JSON
↓
自动识别
↓
Format
↓
Preview
↓
Copy / Save
```

## Compare

```text
粘贴 A
+
粘贴 B
↓
Compare
↓
Added / Removed / Changed / Moved
↓
Side-by-side / Unified
```

## Extractor

```text
粘贴一大段文本
↓
选择 URLs
↓
Extract
↓
47 matches
↓
Copy All
```

## Transformer

```text
粘贴文本
↓
Deduplicate Lines
↓
Preview
↓
Apply
↓
History
↓
Undo
```

---

# 234. M4 最终用户体验

用户不应该感觉：

```text
我要先理解 parser
我要理解 encoding
我要理解 diff algorithm
我要先创建 workspace
```

而应该：

```text
Drop
↓
Tool
↓
Result
```

高级能力可以存在：

```text
Options
Advanced
Diagnostics
```

但不能成为：

```text
第一次使用的门槛
```

---

# 235. M4 核心判断标准

M4 真正要回答的不是：

> “我们是不是增加了 4 个 Text 工具？”

而是：

> **“Weave 是否已经成为一个用户遇到混乱文本时，可以立刻打开、立刻得到可靠结果的本地工具？”**

---

# 236. M4 最终工程原则

整个 M4 始终遵守：

```text
Correctness > Cleverness
Semantic Safety > Feature Count
Preview > Immediate Mutation
Real Parser > Regex Guessing
Reuse > Rebuild
Deterministic > Magical
Bounded > Unlimited
Explicit Unsupported > Fake Support
Local > Cloud
Structured Errors > Generic Errors
Real Tests > Green Theater
```

最重要的原则：

> **宁可明确告诉用户“当前格式或输入不支持”，也不能用看起来成功、实际上会改变语义的伪格式化器。**

以及：

> **文本处理的结果可以很快得到，但对文件的实际修改仍必须可预览、可验证、可追踪、可撤销。**

---

# 237. 执行纪律

整个 milestone 必须严格遵守：

```text
Audit
 ↓
Gap Analysis
 ↓
Architecture
 ↓
Specification / Test Fixture
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

每个能力先落 golden fixture 与期望输出，再写实现（Specification → Test Fixture → Implementation）。

不要：

```text
先写完全部代码
最后才测试
```

不要：

```text
先设计一套完美架构
```

不要：

```text
为了以后 M7
```

提前制造：

```text
Generic Batch Engine
Plugin SDK
Workflow DSL
Universal Parser Framework
```

---

# 238. 每次修改的纪律

每次修改前先确认：

```text
现有代码
现有接口
现有测试
现有依赖
现有文档
```

然后：

```text
最小必要修改
```

禁止：

```text
无关重构
大规模 rename
无意义依赖升级
删除已有测试
关闭 lint
降低断言
吞掉错误
绕过架构
```

---

# 239. 当发现前序问题时

如果 M4 实施过程中发现 M1/M2/M3 问题：

首先判断：

```text
P0
P1
P2
```

并判断：

```text
FACT
HYPOTHESIS
INFERENCE
```

只有真正阻塞 M4 正确性、安全性、架构完整性的：

```text
P0 / P1
```

才允许修复。

无关问题：

```text
记录
不要顺手重构
```

---

# 240. Regression Gate

最终必须重新执行：

```text
M0 tests
M1 tests
M2 tests
M3 tests
```

确认：

```text
M4 did not break previous milestones
```

尤其重点检查：

```text
Filesystem
Path Safety
History
Undo
Preview
Recycle
Duplicate Finder
```

---

# 241. M5 禁止偷渡

完成 M4 后，发现以下想法：

```text
CSV editor
data cleaner
schema detection
JSON to CSV
CSV to JSON
row filtering
column transformation
data profiling
```

全部记录：

```text
M5
```

不要顺手实现。

---

# 242. M5 Handoff

M5 应该直接建立在：

```text
M4 TextDocument
M4 Encoding
M4 LineEnding
M4 Diagnostics
M4 JSON Parser
M4 structured text foundation
```

之上。

特别是：

```text
M4 JSON formatting
```

与：

```text
M5 JSON data processing
```

应该共享底层 parser 能力，但业务层分离。

---

# 243. Final Output Format

整个 milestone 完成后，最终输出必须为：

# M4 Implementation Report

## 1. Baseline

```text
Branch:
HEAD:
Working Tree:
M0:
M1:
M2:
M3:
```

## 2. Implemented

```text
Text Foundation
Formatter
Compare
Extractor
Transformer
Preview
Safe Write
History
Undo
UI
```

逐项说明实际完成情况。

## 3. Verification

```text
Unit:
Integration:
Fault:
Property:
Fuzz:
Performance:
UI:
Build:
```

必须填写真实命令与真实结果。

## 4. Performance Evidence

真实：

```text
Dataset
Operation
Elapsed
Peak Memory
Output
```

## 5. Final Audit

```text
A PASS
B PASS
C PASS
...
```

如果存在：

```text
BLOCKED
NOT VERIFIED
NOT SUPPORTED
```

必须明确写出。

## 6. Known Limitations

只写：

```text
真实限制
平台限制
Parser 限制
Dialect 限制
Size 限制
```

## 7. Git

```text
Commit:
Working Tree:
```

## 8. M5 Handoff

说明：

```text
M4 now provides:
...
```

最后只有在所有硬门禁满足之后，才能写：

```text
M4 COMPLETE
```

否则必须写：

```text
M4 NOT COMPLETE
```

绝对不要为了让 milestone 看起来完成而：

```text
跳过测试
隐藏限制
降低断言
伪造功能
伪造性能
关闭错误
绕过安全
```

---

# 244. 最终 M4 产品闭环

M4 最终应该形成：

```text
                    ┌──────────────────────┐
                    │      ifuyo Weave     │
                    │         Text         │
                    └──────────┬───────────┘
                               │
          ┌────────────────────┼────────────────────┐
          │                    │                    │
          ▼                    ▼                    ▼
      Formatter             Compare            Extractor
          │                    │                    │
          └────────────────────┼────────────────────┘
                               │
                               ▼
                         Transformer
                               │
                               ▼
                         TextDocument
                               │
                    ┌──────────┴──────────┐
                    ▼                     ▼
                 Preview               Result
                    │                     │
                    ▼                     ▼
               Validation          Copy / Export
                    │
                    ▼
                Safe Write
                    │
          ┌─────────┴─────────┐
          ▼                   ▼
      Transaction           History
          │                   │
          └─────────┬─────────┘
                    ▼
                   Undo
```

最终：

> **M4 不是四个孤立的文本小工具，而是 Weave 第一套真正统一的“文本工作流”。**
>
> **同一个 TextDocument、同一套 Encoding、同一套 Diagnostics、同一套 Preview、同一套安全写入与 History/Undo，支撑四类高频文本任务。**
>
> 这才是 M4 的完成状态。