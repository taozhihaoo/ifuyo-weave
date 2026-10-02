# ifuyo Weave
# M5 — Data
## CSV Studio、Data Cleaner、JSON/CSV/TSV/JSONL 转换、Data Inspector

你现在进入：

> **ifuyo Weave 的 M5：Data**

项目：

`ifuyo Weave`

当前 milestone 基线：

```text
M0 = Foundation
M1 = File Core
M2 = Rename / Organizer
M3 = Duplicate Finder
M4 = Text
M5 = Data
```

M5 的定位不是：

> “再做一个 CSV 编辑器。”

而是：

> **让 Weave 第一次具备可靠、可解释、可预览、可批量处理的大众数据工具能力。**

M5 必须围绕：

```text
CSV
TSV
JSON
JSONL
```

建立一套统一的数据模型、解析、检查、清洗、转换与导出能力。

产品闭环：

```text
Drop / Open
    ↓
Detect
    ↓
Inspect
    ↓
View
    ↓
Filter / Sort / Transform
    ↓
Preview
    ↓
Validate
    ↓
Export
    ↓
Result
    ↓
History / Undo（仅对实际发生的文件变更）
```

同时严格保持：

```text
Local-first
Utility-first
Batch-ready
Preview-first
Reversible-first
Composable
```

以及：

```text
Correctness > Feature Count
Data Integrity > Convenience
Explicit Semantics > Magic
Preview > Mutation
Streaming > Whole-dataset Memory
Reuse > Rebuild
Bounded > Unbounded
Facts > Guessing
```

---

# 0. M5 硬边界

## 0.1 本阶段必须完成

M5 至少完成四个核心工具族：

```text
1. CSV Studio
2. Data Cleaner
3. JSON / CSV / TSV / JSONL Converter
4. Data Inspector
```

---

## 0.2 CSV Studio

必须支持：

```text
Open
View
Filter
Sort
Search
Column Rename
Column Delete
Column Reorder
Type Detection
Encoding Detection
```

并支持数据变换：

```text
Deduplicate
Trim
Normalize
Split Column
Merge Column
Fill Empty
Find / Replace
```

CSV Studio 必须至少能够处理：

```text
CSV
TSV
```

其中 TSV 本质上是：

```text
delimiter = '\t'
```

但 UI 与数据模型必须明确区分：

```text
format = CSV
format = TSV
```

不要简单地把：

```text
.tsv
```

当作：

```text
.csv
```

然后依赖文件扩展名决定全部行为。

---

## 0.3 Data Cleaner

建立：

```text
Detect
↓
Preview
↓
Rules
↓
Apply
↓
Export
```

至少支持：

```text
Whitespace Cleanup
Case Normalization
Empty / Null Handling
Duplicate Row Detection
Date Normalization
Numeric Normalization
Find / Replace
```

以及：

```text
Anomaly Discovery
```

注意：

> Anomaly Discovery 是“发现并报告”，不是默认“自动修改”。

例如：

```text
Potentially malformed date
Unexpected numeric format
Suspicious empty rate
Unexpected type
Outlier-like value
```

必须标明：

```text
Fact
Rule
Heuristic
Inference
```

禁止把启发式判断伪装成确定事实。

---

## 0.4 JSON / CSV / TSV / JSONL Converter

必须支持：

```text
JSON → CSV
CSV → JSON
CSV → TSV
TSV → CSV
JSONL → JSON
JSON → JSONL
```

同时必须认真处理：

```text
Nested Object
Nested Array
Null
Boolean
Number
String
Unicode
Encoding
Delimiter
Quote
Escaping
```

禁止出现：

```text
成功转换
但数据已经 silently changed
```

任何有信息损失的转换必须：

```text
可解释
可预览
可配置
可报告
```

---

## 0.5 Data Inspector

对于：

```text
.csv
.tsv
.json
.jsonl
```

必须能够报告：

```text
Format
Encoding
Rows
Columns
Schema
Null Rate
Unique Rate
Potential Type
```

显式支持 GB18030 / GBK / Latin-1 fallback（charter #27）；复用 M4 实现，若 M4 未覆盖则在本里程碑补齐并记录 DECISIONS。

必要时还可以报告：

```text
Delimiter
Quote Character
Header Presence
Line Ending
Estimated Size
Sampling Status
Parse Errors
```

但所有统计必须诚实说明：

```text
Exact
Sampled
Estimated
Unavailable
```

不能将：

```text
first 10,000 rows
```

得到的统计说成：

```text
whole dataset exact statistics
```

---

# 1. 本阶段明确禁止

M5 不得提前实现：

```text
Image Tools
PDF Tools
DOCX Tools
XLSX Full Editor
PPTX Tools
Archive Tools
Media Conversion
Workflow Pipeline
General Batch Engine
Plugin SDK
AI Data Cleaning
LLM Data Transformation
Cloud Data Processing
Remote Processing
Telemetry
Account
Cloud Storage
Online Data API
```

特别禁止：

```text
M6 Image
M7 Batch Engine
M8 Documents
M10 Workflow
```

本阶段可以：

```text
对一个数据集执行多个明确的数据规则
```

但不得把这些规则偷变成：

```text
通用 Workflow Engine
```

---

# 2. M5 最重要的架构原则

M5 与 M4 的关系必须是：

```text
Text
    ↓
TextDocument / Encoding / Diagnostics
    ↓
Structured Parser
    ↓
Data Model
    ↓
Data Operations
```

而不是：

```text
CSV
↓
大量字符串 replace
↓
“看起来像数据”
```

---

# 3. M5 与 M4 的边界

M4 已经提供：

```text
TextDocument
Encoding
LineEnding
TextRange
Diagnostics
JSON Parser / structured JSON foundation
Text I/O
Safe Write
Preview
Transaction
History
Undo
```

M5 必须优先复用：

```text
Encoding
BOM detection
Line ending handling
Diagnostics
Safe read
Safe write
Preview infrastructure
Transaction
History
Undo
Path Safety
Filesystem abstraction
```

这些基础能力不得重新实现第二套。

M4 已经存在：

```text
JSON Formatter
JSON Validate
JSON Normalize
```

M5 不再重做这些。

M5 只新增：

```text
JSON as Data
Schema / Profiling
JSON ↔ tabular data
JSONL handling
```

即：

```text
M4:
JSON = structured text

M5:
JSON = structured dataset
```

---

# 4. M5 数据模型

必须建立清晰的数据领域模型。

至少考虑：

```text
DataDocument
DataFormat
DataSource
DataSchema
ColumnDefinition
ColumnType
DataValue
DataRow
DataTable
DataDiagnostic
DataProfile
DataTransform
DataTransformRule
ConversionPlan
ExportOptions
```

不要让 React 组件自己定义一套：

```text
Row
Column
Cell
Schema
```

然后 Rust 又定义另一套完全不同的概念。

---

# 5. DataValue

CSV 本身没有原生：

```text
Boolean
Number
Null
Date
```

CSV 只是文本。

因此必须严格区分：

```text
Raw Cell Value
```

与：

```text
Detected / Potential Type
```

例如：

```text
"00123"
```

默认不能擅自变成：

```text
123
```

因为：

```text
00123
```

可能是：

```text
ID
Postal Code
Product Code
Account Code
```

而不是数学数字。

---

# 6. Type Detection 原则

至少支持：

```text
String
Integer
Decimal
Boolean
Date
DateTime
Null / Empty
Potential Mixed
```

类型推断必须是：

```text
Potential Type
```

而不是：

```text
Guaranteed Type
```

例如：

```text
00123
00124
00125
```

可能被判定：

```text
String / Identifier-like
```

而不是强制：

```text
Integer
```

---

# 7. Numeric Safety

禁止：

```text
所有数字 → f64
```

因为这可能导致：

```text
large integer precision loss
```

必须确保：

```text
整数精度
十进制表示
JSON number
```

不会因为内部转换无意义地变成：

```text
12345678901234567890
→
12345678901234568000
```

如果底层类型不支持某种精确表示：

必须：

```text
保留原始文本
或
明确报告 loss / unsupported
```

不能 silently round。

---

# 8. JSON 数值原则

JSON：

```text
integer
decimal
```

需要保持：

```text
lexical correctness
```

不能为了方便全部：

```text
parse as f64
```

再重新序列化。

---

# 9. CSV Parse 原则

优先使用：

> **成熟、经过充分验证的 Rust CSV parser。**

不要自己手写：

```text
split(',')
```

更不能：

```rust
line.split(',')
```

冒充 CSV parser。

推荐候选：csv crate（BurntSushi，MIT/Apache-2.0），选型记录进 DECISIONS.md。

必须正确处理：

```text
"a,b"
"hello ""world"""
"line
break"
```

以及：

```text
empty field
empty row
trailing delimiter
quoted field
UTF-8
BOM
CRLF
LF
```

---

# 10. TSV Parse

TSV 必须复用 CSV parser 能力：

```text
delimiter = '\t'
```

不要：

```text
split('\t')
```

冒充完整 TSV parser。

同样正确处理：

```text
quoted tab
embedded newline
escaped quote
empty field
```

---

# 11. Header 语义

必须明确：

```text
Has Header
No Header
Auto Detect
```

如果：

```text
Auto Detect
```

只是启发式判断：

必须明确：

```text
Detected
Confidence / Reason
```

而不是：

```text
Header: yes
```

然后隐藏它是猜的。

---

# 12. Duplicate Header

必须处理：

```text
name
name
email
```

不能产生不可区分的内部 column identity。

内部必须有：

```text
stable column id
```

例如：

```text
col_1
col_2
col_3
```

显示标题可以：

```text
name
name
email
```

但内部引用不能只依赖：

```text
column name
```

---

# 13. Empty Header

例如：

```csv
name,,email
```

必须明确支持：

```text
blank header
```

但内部必须给该列：

```text
stable identity
```

例如：

```text
Column 2
```

仅作为 UI fallback display name。

---

# 14. Ragged Rows

例如：

```text
A,B,C
1,2,3
4,5
6,7,8,9
```

必须定义：

```text
Missing Cell
Extra Cell
```

例如：

```text
row 2:
C = empty / missing

row 3:
extra cell
```

不能 silently truncate。

不能 silently shift values。

---

# 15. Data Table 语义

需要至少支持：

```text
Column order
Row order
Stable row identity
Cell values
Null / empty state
Diagnostics
```

排序后：

```text
row identity
```

仍然必须稳定。

过滤后：

```text
row identity
```

仍然必须稳定。

---

# 16. CSV View Architecture

CSV Studio 不允许：

```text
一次把 100 万行全部渲染进 DOM
```

必须：

```text
Virtualized Rows
```

或同等级的分页/虚拟化方案。

UI 只显示：

```text
当前 viewport
```

数据处理仍在：

```text
Rust / application backend
```

进行。

---

# 17. Data Session

默认建立最小 DataSession 实现（后续 #84-87 的分页 / session handle 机制依赖它）：

```text
DataSession
```

用于：

```text
Open Dataset
Query Preview
Page Rows
Filter
Sort
Transform Preview
Export
```

但要求：

```text
ephemeral
local-only
bounded
non-persistent
```

不要为了 M5 引入：

```text
SQLite dataset cache
embedded database engine
permanent indexing
```

除非真实仓库审计证明当前架构已经有类似基础设施且复用明显合理。

---

# 18. Data Session 不得成为数据库

明确禁止 M5 偷偷构建：

```text
mini database
full SQL engine
OLAP engine
analytics platform
```

M5 的目标仍然是：

> **桌面数据工具。**

而不是：

> 数据库客户端。

---

# 19. Filter

CSV Studio 至少支持：

```text
Contains
Equals
Not Equals
Starts With
Ends With
Empty
Not Empty
```

可按：

```text
Column
```

进行过滤。

必要时支持：

```text
numeric comparison
date comparison
```

但比较前必须根据：

```text
explicit user choice
```

或：

```text
declared detected type
```

决定。

不能对：

```text
0012
```

偷偷做数字比较而用户不知道。

---

# 20. Filter Semantics

必须保证：

```text
Filter
```

是：

```text
non-destructive view operation
```

而不是：

```text
delete rows
```

用户关闭 filter 后：

```text
original dataset
```

仍然存在。

---

# 21. Multiple Filters

支持多个条件时必须定义：

```text
AND
OR
```

不要默认让：

```text
UI 看起来 AND
```

但 backend 实际执行 OR。

规则模型应该结构化，例如：

```text
FilterGroup
FilterRule
Operator
Value
ColumnId
```

---

# 22. Sort

必须支持：

```text
Ascending
Descending
```

至少支持：

```text
String
Numeric
Date
```

排序必须：

```text
deterministic
stable
```

对于完全相等的行：

保持：

```text
original relative order
```

---

# 23. Null / Empty Sorting

必须明确：

```text
null / empty
```

到底排：

```text
first
```

还是：

```text
last
```

并保持一致。

不要：

```text
每次排序结果不同
```

---

# 24. Search

Search 应支持：

```text
Current dataset
Current visible data
Specific column
All columns
```

不要偷实现：

```text
M9 global search
```

搜索范围必须是当前 Data Session / 当前工具。

---

# 25. Column Rename

必须：

```text
Preview
```

而不是直接修改源文件。

rename 应只改变：

```text
column label / export header
```

除非用户明确 Apply / Export。

---

# 26. Column Delete

Column Delete 是：

```text
destructive data transformation
```

因此：

```text
Preview first
```

必须明确显示：

```text
Deleted columns
Remaining columns
Potentially affected output
```

原始文件默认不动。

---

# 27. Column Reorder

必须使用：

```text
stable ColumnId
```

而不是：

```text
column index alone
```

因为：

```text
rename
delete
insert
```

都可能改变 index。

---

# 28. Column Split

至少支持：

```text
Delimiter
```

例如：

```text
John|Doe
```

拆成：

```text
John
Doe
```

必须明确：

```text
Target columns
Delimiter
Max splits
Behavior for missing delimiter
```

缺少 delimiter 时：

```text
keep original
```

或：

```text
empty generated column
```

必须由明确规则定义。

---

# 29. Column Merge

至少支持：

```text
Selected Columns
Separator
Output Column Name
Source Column Handling
```

例如：

```text
first_name
last_name

→

full_name
```

不要默认删除源列，除非规则明确要求。

---

# 30. Fill Empty

必须区分：

```text
Empty String
Whitespace-only
Null-like token
```

例如：

```text
""
" "
"NULL"
"null"
"N/A"
```

绝不能把这些默认视为完全相同。

必须允许用户配置：

```text
Which representations count as empty
```

---

# 31. Null Policy

至少考虑：

```text
Empty String
Whitespace-only
Known Tokens
Actual JSON null
```

对于：

```text
CSV
```

JSON `null` 不存在天然对应关系。

因此：

```text
CSV → JSON
```

必须允许选择：

```text
empty string remains ""
```

或者：

```text
configured null tokens become null
```

并在 preview 中显示。

---

# 32. Trim

Trim 至少支持：

```text
Left
Right
Both
```

应用范围：

```text
Selected Columns
All Text Columns
All Columns
```

必须明确。

---

# 33. Case Normalization

支持：

```text
lowercase
UPPERCASE
Title Case
```

但必须明确：

> Title Case 对 Unicode 和不同语言不一定具有完美语言学语义。

因此：

```text
simple deterministic case transform
```

优先于：

```text
假装完整自然语言标题大小写系统
```

---

# 34. Find / Replace

必须支持：

```text
Selected Column
All Columns
Exact
Case Sensitive
```

必要时：

```text
Regex
```

但如果启用 Regex：

必须复用 M4 的：

```text
Regex Safety
Resource Limits
Diagnostics
```

不要重新实现。

---

# 35. Duplicate Row

Data Cleaner 必须支持：

```text
Duplicate Row Detection
```

默认定义：

```text
two rows are duplicates when all compared columns are equal
```

必须允许：

```text
All Columns
Selected Columns
```

例如：

```text
email
```

相同时判定重复。

---

# 36. Deduplicate Semantics

必须明确：

```text
Keep First
Keep Last
```

不能：

```text
随机保留
```

并且结果必须：

```text
deterministic
```

---

# 37. Duplicate Rows 与 M3 的区别

M3：

```text
file-level duplicate
```

M5：

```text
row-level duplicate
```

绝不能复用错误的数据模型。

但可以复用：

```text
deterministic grouping
```

等底层原则。

---

# 38. Date Detection

日期检测必须保守。

例如：

```text
2026-10-03
2026/10/03
03/10/2026
10/03/2026
```

后两者存在：

```text
locale ambiguity
```

必须：

```text
明确 locale
```

或者：

```text
报告 ambiguous
```

不能猜。

---

# 39. Date Normalization

必须支持：

```text
Input Pattern
Output Pattern
Locale
Timezone policy where relevant
```

若无法可靠确定：

```text
FAIL / NOT VERIFIED
```

而不是：

```text
自动猜测
```

---

# 40. Numeric Normalization

需要明确：

```text
decimal separator
thousands separator
sign
currency-like symbols
whitespace
```

例如：

```text
1,234.56
1.234,56
```

存在语义差异。

因此：

```text
locale-sensitive parsing
```

必须显式配置。

禁止：

```text
global comma replacement
```

---

# 41. Anomaly Discovery

至少可以检测：

```text
Mixed types
Unexpected empty ratio
Duplicate concentration
Malformed dates
Malformed numbers
Rare values
Unexpected delimiter patterns
Unexpected column counts
```

但所有：

```text
anomaly
```

必须带：

```text
reason
scope
confidence
```

---

# 42. Anomaly 不等于 Error

必须严格区分：

```text
Parse Error
Validation Error
Anomaly
Warning
Info
```

例如：

```text
"abc" in numeric-looking column
```

通常是：

```text
Anomaly
```

而不是：

```text
Parse Error
```

---

# 43. Data Inspector

Data Inspector 的核心职责：

> **告诉用户数据是什么，而不是替用户修改数据。**

至少输出：

```text
Format
Encoding
Rows
Columns
Schema
Null Rate
Unique Rate
Potential Type
```

---

# 44. Inspector Statistics

每一项统计必须说明：

```text
Exact
Sampled
Estimated
Unavailable
```

例如：

```text
Rows: 4,821,221 [Exact]

Unique Rate: unavailable
Reason: dataset exceeds exact cardinality memory limit
```

这比：

```text
Unique Rate: 37%
```

但实际上是近似猜测更可靠。

---

# 45. Null Rate

公式：

```text
null_rate = null_or_empty_count / total_rows
```

但具体：

```text
null definition
```

必须遵循用户选择或当前 schema policy。

不能：

```text
把 "0"
```

误认为：

```text
null
```

---

# 46. Unique Rate

必须明确：

```text
unique distinct values / eligible rows
```

需要考虑：

```text
empty values
null values
case sensitivity
normalization
```

这些必须写进统计规则。

---

# 47. Exact Unique

如果采用 exact distinct set：

必须有：

```text
resource limit
```

达到上限后：

不要：

```text
继续无限增长
```

也不要：

```text
悄悄切成 approximate
```

应明确：

```text
Unavailable because exact cardinality limit was reached
```

---

# 48. Sampled Profiling

大型文件允许：

```text
first N rows
uniform sample
bounded sample
```

但报告必须显示：

```text
Sampling Method
Sample Size
Dataset Size if known
```

例如：

```text
Potential Type:
DateTime

Basis:
sampled 50,000 / 4,821,221 rows
```

不能假装：

```text
4,821,221 rows fully inspected
```

---

# 49. JSON Inspector

JSON Inspector 至少支持：

```text
Object
Array
Primitive
Null
```

并且对对象数据建立：

```text
JSON Path
Potential Type
Presence Rate
Null Rate
```

例如：

```text
$.id
$.name
$.profile.age
$.profile.email
```

---

# 50. JSON Nested Structure

必须能够识别：

```text
nested object
nested array
mixed structure
missing property
null property
```

例如：

```json
[
  {
    "id": 1,
    "name": "A"
  },
  {
    "id": 2,
    "name": "B",
    "meta": {
      "age": 18
    }
  }
]
```

必须允许报告：

```text
meta.age
Presence Rate < 100%
Potential Type = Integer
```

而不是：

```text
Schema = fixed
```

---

# 51. JSON Root Semantics

必须明确支持或拒绝：

```text
Root Object
Root Array
Root Primitive
```

对于：

```text
JSON → CSV
```

如果 root 不是：

```text
array-like records
```

不能假装转换成功。

应：

```text
NOT SUPPORTED
```

或提供明确的：

```text
flatten / single-record
```

模式。

不得 silently wrap 数据而改变语义。

---

# 52. JSON → CSV

默认推荐语义：

```text
JSON root = array
each item = object
each object field = column
```

例如：

```json
[
  {"id":1,"name":"A"},
  {"id":2,"name":"B"}
]
```

转换为：

```csv
id,name
1,A
2,B
```

必须定义：

```text
missing field
null
nested object
nested array
```

行为。

---

# 53. Nested JSON → CSV

不能直接：

```text
nested object → "[object Object]"
```

这是禁止的。

必须选择明确策略，例如：

```text
Keep as JSON cell
```

或：

```text
Flatten
```

如果实现 Flatten：

必须明确：

```text
separator
array representation
collision handling
```

例如：

```text
profile.name
profile.age
```

不能：

```text
随机拼字段名
```

---

# 54. Array Cell

例如：

```json
{"tags":["a","b","c"]}
```

CSV 中可以使用：

```text
JSON cell
```

例如：

```csv
tags
["a","b","c"]
```

这是合理的。

但必须：

```text
documented
previewed
```

不能隐式变成：

```text
a,b,c
```

导致列数变化。

---

# 55. CSV → JSON

默认推荐：

```text
one row = one JSON object
```

例如：

```csv
id,name
1,A
2,B
```

输出：

```json
[
  {
    "id": "1",
    "name": "A"
  },
  {
    "id": "2",
    "name": "B"
  }
]
```

注意：

> CSV 本质是文本，因此默认不要自动把 `"1"` 变成数字 `1`。

---

# 56. CSV → JSON Typed Mode

可以提供：

```text
Preserve Strings
Infer Types
```

如果：

```text
Infer Types
```

必须在 Preview 明确：

```text
"1" → 1
"true" → true
"null" → null
```

哪些变化发生了。

---

# 57. Type Inference 安全原则

禁止：

```text
"001"
→
1
```

如果这会丢失：

```text
leading zeros
```

除非用户明确选择：

```text
Numeric interpretation
```

---

# 58. CSV → JSON Duplicate Headers

如果 CSV：

```csv
name,name
A,B
```

JSON 不能简单变成：

```json
{
  "name": "B"
}
```

因为：

```text
A
```

已经丢失。

必须：

```text
reject
```

或者：

```text
explicit duplicate-header strategy
```

例如：

```text
name
name_2
```

但该行为必须：

```text
visible
deterministic
previewed
```

---

# 59. JSONL

JSONL 定义：

```text
one JSON value per line
```

至少支持：

```text
object per line
```

典型：

```json
{"id":1,"name":"A"}
{"id":2,"name":"B"}
```

必须逐行：

```text
stream parse
```

不要：

```text
whole file read into one giant string
```

---

# 60. JSONL → JSON

默认：

```text
each valid JSONL record
→
array item
```

例如：

```json
{"id":1}
{"id":2}
```

输出：

```json
[
  {"id":1},
  {"id":2}
]
```

---

# 61. JSON → JSONL

仅在：

```text
root is array
```

且：

```text
each item is serializable JSON value
```

时默认逐项输出。

例如：

```json
[
  {"id":1},
  {"id":2}
]
```

→

```json
{"id":1}
{"id":2}
```

---

# 62. JSONL Malformed Line

例如：

```json
{"id":1}
broken
{"id":3}
```

必须提供：

```text
line number
diagnostic
```

并支持定义：

```text
Fail Fast
Collect Errors
```

如果：

```text
Collect Errors
```

也必须明确：

```text
valid records
invalid records
skipped records
```

禁止把：

```text
invalid
```

偷偷删除掉，然后显示：

```text
Success
```

---

# 63. CSV Malformed Records

必须处理：

```text
Malformed quoting
Unexpected column count
Invalid encoding
Broken record
Invalid byte sequence
```

优先依赖成熟 parser 的：

```text
position-aware error reporting
```

对于能够安全恢复的错误：

允许：

```text
Collect Diagnostics
Continue
```

对于：

```text
继续会导致数据错位
```

的错误：

必须：

```text
Stop
```

而不是冒险继续。

---

# 64. “一个坏字段不能让整个任务崩溃”

必须理解为：

> **能安全隔离的局部数据问题，应记录并继续处理；不能安全恢复的数据损坏，必须停止并明确失败原因。**

不是：

```text
永远继续
```

也不是：

```text
遇到任何问题 panic
```

---

# 65. Encoding

必须优先复用 M4：

```text
encoding detection
BOM detection
TextDocument
```

至少验证：

```text
UTF-8
UTF-8 BOM
UTF-16 LE BOM
UTF-16 BE BOM
```

显式支持 GB18030 / GBK / Latin-1 fallback（charter #27）；复用 M4 实现，若 M4 未覆盖则在本里程碑补齐并记录 DECISIONS。

如果某种编码无法可靠支持：

必须：

```text
NOT SUPPORTED
```

不能：

```text
mojibake
```

然后显示：

```text
Success
```

---

# 66. Line Ending

支持：

```text
LF
CRLF
CR
```

读取时：

```text
normalize internally
```

输出时必须支持：

```text
preserve
LF
CRLF
```

不能无提示地改变用户数据文件的：

```text
line-ending convention
```

---

# 67. Delimiter Detection

对：

```text
CSV / TSV
```

可以：

```text
detect delimiter
```

候选至少：

```text
,
\t
;
|
```

但必须有：

```text
confidence / evidence
```

遇到：

```text
ambiguous
```

应：

```text
ask / expose choice
```

而不是：

```text
偷偷选择
```

---

# 68. Quote Detection

如果 parser 支持：

```text
quote character
```

必须允许明确显示：

```text
Quote = "
```

必要时允许：

```text
custom quote
```

但不要为了覆盖极端格式引入复杂配置地狱。

---

# 69. Export Semantics

用户点击：

```text
Export
```

之前必须明确：

```text
Destination
Format
Encoding
Delimiter
Quote
Line Ending
Header
Type Conversion
```

默认：

```text
no silent overwrite
```

必须复用：

```text
M2 collision detection
Path Safety
Safe Write
Transaction
History
Undo
```

---

# 70. 原始文件原则

打开：

```text
data.csv
```

然后执行：

```text
Filter
Sort
Trim
Delete Column
```

默认：

> **不修改原始文件。**

这些操作先改变：

```text
in-memory / session state
```

最终：

```text
Export
```

才生成：

```text
new file
```

---

# 71. Apply vs Export

必须定义清晰语义：

```text
Apply
```

表示：

```text
apply transforms to current dataset session
```

而：

```text
Export
```

表示：

```text
write transformed dataset to filesystem
```

不要把：

```text
Apply
```

理解成：

```text
立即覆盖 source file
```

除非未来另有明确的 overwrite command。

---

# 72. Source Overwrite

M5 可以支持：

```text
Save / Replace Source
```

但如果实现：

必须完整复用：

```text
M2 Safe Write
TOCTOU Revalidation
Atomic Write
Transaction
History
Undo
```

不能：

```rust
std::fs::write(path, content)
```

直接覆盖。

---

# 73. TOCTOU

用户打开：

```text
data.csv
```

之后，外部程序修改了它。

用户再：

```text
Save
```

必须检测：

```text
source identity changed
```

至少检查：

```text
mtime
size
expected fingerprint where appropriate
```

最好复用已有文件 identity / hash 能力。

如果变化：

```text
do not overwrite automatically
```

必须：

```text
Conflict
```

并要求用户明确处理。

---

# 74. Undo

Undo 只对：

```text
actual file mutations
```

生效。

例如：

```text
Export new file
```

是一个实际文件操作。

但：

```text
Filter
Sort
```

只是 session state：

不需要：

```text
filesystem Undo
```

可以有：

```text
session undo
```

但不要与：

```text
M2 History / filesystem Undo
```

混淆。

---

# 75. History

History 至少记录：

```text
operation id
tool
source path if relevant
destination path if relevant
format
summary
timestamp
actual result
```

不得记录：

```text
整个 CSV 内容
完整 JSON 内容
大量敏感数据
```

除非项目当前安全设计明确要求且有充分理由。

默认：

> **History 记录操作事实，不记录文件内容。**

---

# 76. Preview

Data Cleaner 和 Converter 必须：

```text
Preview-first
```

Preview 至少展示：

```text
Rows affected
Columns affected
Before
After
Warnings
Diagnostics
Conversion decisions
```

不要只显示：

```text
Preview available
```

然后没有实际结果。

---

# 77. Preview 数据边界

大型数据集不能：

```text
渲染整个 before / after
```

必须展示：

```text
sample rows
changed rows
summary
```

例如：

```text
12,482 rows affected

Preview:
showing 50 representative rows
```

并明确：

```text
preview sample
```

---

# 78. Data Transform Model

建议建立：

```text
DataTransformPlan
```

例如：

```text
Rule 1: Trim column email
Rule 2: Lowercase column email
Rule 3: Remove duplicate rows by email
Rule 4: Fill empty country = "CN"
```

必须：

```text
ordered
deterministic
serializable
previewable
```

可序列化格式必须面向 M10 Workflow 复用设计。

---

# 79. Rule Ordering

用户配置：

```text
Trim
→
Lowercase
→
Deduplicate
```

与：

```text
Deduplicate
→
Trim
→
Lowercase
```

结果可能不同。

因此：

> **规则顺序是数据语义的一部分。**

不能：

```text
backend 自动重排
```

除非规则之间数学上保证等价。

---

# 80. Rule Validation

Apply 前必须检查：

```text
column exists
column type compatible
rule parameters valid
resource limits
output format valid
destination safe
```

---

# 81. Data Operations 必须是纯逻辑

优先让：

```text
weave-data
```

中的核心变换：

```text
Pure / side-effect free
```

即：

```text
Input Dataset
+
Transform Plan
↓
Result Dataset / Stream
```

文件写入：

```text
Adapter / Application Layer
```

负责。

---

# 82. weave-data 职责

`weave-data` 负责：

```text
DataDocument
CSV / TSV parser
JSON / JSONL parser
Schema
Type Detection
Profiler
Filtering
Sorting
Transformations
Deduplication
Conversion
Export Serialization
Data Diagnostics
```

不负责：

```text
Tauri window
React components
Filesystem implementation
Path validation implementation
History persistence
OS recycle bin
```

---

# 83. 分层

目标：

```text
React UI
   ↓
Application / Command Layer
   ↓
weave-data
   ↓
weave-text / shared foundation
   ↓
filesystem abstraction
```

禁止：

```text
React
 ↓
CSV parser
```

禁止：

```text
React
 ↓
direct file write
```

禁止：

```text
React
 ↓
own encoding detection
```

---

# 84. Data IPC

IPC DTO 必须：

```text
structured
versionable
bounded
```

不要直接把：

```text
10 MB JSON result
```

通过 IPC 不加限制地一次发送。

应该采用：

```text
paged rows
summary
diagnostics
session handle
```

等机制。

---

# 85. Paging

至少为大型 CSV Viewer 提供：

```text
page size
offset / cursor
```

或等价机制。

必须有：

```text
maximum page size
```

防止：

```text
request 1,000,000 rows
```

导致 UI / IPC 爆炸。

---

# 86. Filtering Backend

如果 dataset 很大：

不要：

```text
Rust 全表处理
→
IPC 全量发送
→
React 再过滤
```

优先：

```text
Filter
→
Backend
→
Page
→
UI
```

---

# 87. Sorting Backend

同样：

```text
Sort
```

必须尽量发生在：

```text
Rust / Data layer
```

不能把：

```text
大量 rows
```

全部发送给 React 再：

```text
Array.sort()
```

然后声称支持大文件。

---

# 88. Memory Budget

所有核心数据操作必须有：

```text
resource limit
```

至少考虑：

```text
Max Input Size
Max Rows In Memory
Max Preview Rows
Max IPC Payload
Max Unique Cardinality
Max Transform Output
Max JSON Nesting Depth
Max Error Count
```

实际数值必须：

```text
基于当前仓库与测试测量
```

不要随意写一个：

```text
1 GB
```

然后没有任何证据。

---

# 89. Streaming

至少以下能力必须支持 streaming 或等效 bounded processing：

```text
CSV Scan
TSV Scan
JSONL Scan
CSV Export
JSONL Export
CSV → JSON where feasible
JSON → JSONL
Profiling
```

禁止默认：

```rust
std::fs::read(...)
```

然后：

```text
整个 dataset
→
String
→
parse
```

作为所有数据格式统一方案。

---

# 90. JSON Streaming

注意：

普通 JSON array：

```json
[
  {...},
  {...},
  {...}
]
```

要实现真正 streaming parser：

应使用：

```text
streaming / incremental JSON parser
```

如果当前依赖栈不支持：

必须：

```text
declare bounded-size limitation
```

而不是伪装成 streaming。

默认采用 bounded + 显式声明限制，除非仓库已有轻量 streaming JSON parser 可用；不为 streaming 单独引入重型依赖。

---

# 91. JSON Parser 安全

必须配置：

```text
max nesting depth
max token size
max input size
max errors
```

避免：

```text
deeply nested JSON
huge strings
pathological input
```

拖垮应用。

---

# 92. JSON Schema Profiling

M5 的 Schema 是：

```text
observed / potential schema
```

不是：

```text
formal JSON Schema standard generator
```

除非真实实现有明确需求。

例如：

```text
$.id:
Integer, presence 100%

$.name:
String, presence 98%

$.age:
Integer | Null, presence 83%
```

这种就足够。

不要为 M5 偷做完整：

```text
JSON Schema IDE
```

---

# 93. Mixed Type

例如：

```json
[
  {"age":18},
  {"age":"unknown"},
  {"age":null}
]
```

必须报告：

```text
Potential Type:
Integer | String | Null
```

不能：

```text
age = Integer
```

然后把：

```text
unknown
```

静默转换成：

```text
0
```

---

# 94. Schema Paths

必须采用稳定表示：

```text
$.name
$.profile.age
$.items[*].price
```

如果数组结构复杂：

必须明确：

```text
sampled
```

与：

```text
fully observed
```

的区别。

---

# 95. CSV Schema

CSV schema 应至少记录：

```text
ColumnId
Name
Index
PotentialType
NullRate
UniqueRate
ObservedFormats
```

必要时：

```text
MaxLength
MinLength
```

但不要无限扩展 profiling 指标。

---

# 96. CSV Studio UI

推荐整体结构：

```text
┌─────────────────────────────────────┐
│ File / Format / Encoding            │
├─────────────────────────────────────┤
│ Search / Filter / Sort              │
├─────────────────────────────────────┤
│ Column Controls / Transform         │
├─────────────────────────────────────┤
│                                     │
│            Data Grid                │
│                                     │
├─────────────────────────────────────┤
│ Rows / Columns / Warnings / Status  │
└─────────────────────────────────────┘
```

但必须服从当前项目现有 UI design system。

---

# 97. 不要做成 Excel Clone

M5 的 CSV Studio：

不是：

```text
Excel
```

也不是：

```text
Google Sheets
```

因此禁止扩展为：

```text
Formulas
Charts
Pivot Tables
Cell Styling
Merged Cells
Conditional Formatting
Macros
Spreadsheet Collaboration
```

M5 的价值是：

> **快速检查、清理、转换数据。**

---

# 98. Column Controls

列操作应集中于：

```text
Rename
Delete
Reorder
Split
Merge
Fill
Trim
Replace
```

避免在每个 cell 弹出几十个菜单。

---

# 99. Data Cleaner UI

核心流程：

```text
Select Dataset
      ↓
Analyze
      ↓
Suggested Issues
      ↓
Rules
      ↓
Preview
      ↓
Export
```

例如：

```text
Email
Whitespace detected
12,842 affected

Country
Mixed case
3,214 affected

Date
3 formats detected
```

但：

> “Suggested” 必须明确是分析建议，不是自动替用户决定。

---

# 100. Cleaner Rule Editor

每条 rule 至少：

```text
Target
Operation
Parameters
Estimated Affected Rows
Preview
```

例如：

```text
Column:
email

Operation:
Trim

Affected:
12,842 / 12,842 rows
```

---

# 101. Converter UI

推荐：

```text
Source
↓
Format
↓
Parsing Options
↓
Conversion Semantics
↓
Preview
↓
Export
```

例如：

```text
CSV → JSON

Header:
Yes

Infer Types:
Off

Empty → null:
Off

Duplicate Header Policy:
Error
```

---

# 102. 转换摘要

Preview 必须告诉用户：

```text
Input rows
Output rows
Input columns
Output fields
Skipped records
Warnings
Type conversions
Null conversions
Structural changes
```

---

# 103. Data Inspector UI

推荐：

```text
Overview
Schema
Quality
Preview
Diagnostics
```

例如：

```text
4,821,221 rows
18 columns
Encoding: UTF-8
Format: CSV
Delimiter: ,
Header: detected

Data Quality:
email null rate 4.8%
id unique rate 100%
age mixed type
```

---

# 104. Inspector 不修改数据

Data Inspector 默认：

```text
read-only
```

如果用户想清理：

通过：

```text
Open in Data Cleaner
```

进入明确 transform workflow。

---

# 105. Tool Composition

允许自然衔接：

```text
Data Inspector
   ↓
Open in CSV Studio

Data Inspector
   ↓
Open in Data Cleaner

CSV Studio
   ↓
Convert
```

但这些必须是：

```text
explicit tool transitions
```

不要实现通用：

```text
workflow graph
```

---

# 106. Drag & Drop

必须复用 M0 / M1 已有：

```text
Drag & Drop
File identification
Path safety
```

不要为 M5 单独实现：

```text
another drop handler
```

---

# 107. File Identification

至少识别：

```text
.csv
.tsv
.json
.jsonl
```

但不能只依赖扩展名。

例如：

```text
weird.txt
```

内容可能是：

```text
JSON
```

可以：

```text
Content-based detection
```

但必须：

```text
bounded
deterministic
```

且不能自动执行危险操作。

---

# 108. Detection Priority

建议：

```text
Explicit user choice
>
Strong file signature / parser evidence
>
Extension hint
>
Heuristic detection
```

并将最终结果标记：

```text
Explicit
Detected
Inferred
Unknown
```

---

# 109. Diagnostics Model

Data diagnostics 应统一：

```text
severity
code
message
location
record
field
recoverability
```

例如：

```text
Error
CSV_UNEXPECTED_FIELD_COUNT
row=1842
expected=8
actual=9
recoverable=true
```

---

# 110. Diagnostics 不得泄漏内容

例如：

不要在 log 中直接写：

```text
customer_email=user@example.com
```

优先：

```text
row=1842
column=email
value=<redacted>
```

必要时 UI 可以展示：

```text
用户当前正在查看的数据
```

但日志不应该默认复制整份敏感数据。

---

# 111. Privacy

继续保持：

```text
No Telemetry
No Upload
No Cloud
No Remote Processing
No Account
```

数据内容：

```text
never leaves machine
```

任何第三方 parser / dependency：

必须确认：

```text
pure local
```

且不会：

```text
call network
```

---

# 112. No Shell

禁止：

```text
python script
jq
csvkit
node external CLI
powershell
bash
awk
sed
prettier
```

作为 M5 的核心实现。

所有核心数据处理：

必须：

```text
Rust
or
existing project-native library
```

完成。

---

# 113. Dependencies

依赖优先级：

```text
Existing dependency
>
Small mature library
>
New dependency only when justified
```

每增加一个第三方 crate，必须说明：

```text
Why
Alternatives
License
Maintenance
Security
Bundle Impact
```

尤其避免：

```text
大型 dataframe engine
大型 spreadsheet engine
full database engine
```

除非真实需求与架构证明它们不可替代。

---

# 114. License Audit

M5 新增依赖必须更新：

```text
THIRD_PARTY_LICENSES.md
```

并检查：

```text
license
version
source
usage
```

---

# 115. Security

必须重点检查：

```text
Path Traversal
Symlink Escape
Malformed CSV
Malformed JSON
Deep JSON
Huge Field
Huge Row
Huge Column Count
Huge IPC Payload
Regex Abuse
Memory Exhaustion
Integer Overflow
Numeric Precision Loss
Encoding Abuse
Output Path Collision
TOCTOU
```

---

# 116. Huge Field

例如单个 CSV cell：

```text
500 MB
```

不能因为：

```text
一个字段很大
```

直接让整个 UI OOM。

必须：

```text
resource limit
preview truncation
diagnostic
bounded handling
```

---

# 117. Huge Row / Columns

必须设置：

```text
Max Columns
Max Row Width
Max Cell Length
```

或者通过现有资源限制体系统一定义。

达到限制：

```text
fail safely
```

不要：

```text
crash
```

---

# 118. Error Budget

对于批量处理：

可以：

```text
Collect first N errors
```

然后：

```text
Too many errors
```

停止进一步收集。

不要：

```text
一个错误 → 永远累积
```

最终：

```text
数百万条 diagnostics
```

把 UI / memory 撑爆。

---

# 119. Cancellation

必须支持：

```text
Cancel
```

尤其：

```text
large CSV
large JSONL
large profiling
large conversion
large dedup
```

Cancel 后：

```text
后台任务真正停止
```

而不是：

```text
UI 显示 Cancelled
backend 还继续运行
```

---

# 120. Progress

Progress 必须是：

```text
honest
```

例如：

```text
Scanning records
84%
```

只有有可靠分母时才显示精确百分比。

否则：

```text
Processed 1,842,193 rows
```

比：

```text
99%
```

但实际不知道总量更诚实。

---

# 121. Determinism

同一个：

```text
Input
+
Options
+
Rule Order
```

必须生成：

```text
same output
same diagnostics order
same sort result
```

不依赖：

```text
hash map iteration order
thread completion order
filesystem enumeration order
```

---

# 122. Parallelism

M5 可以使用：

```text
bounded concurrency
```

但不得因为并行导致：

```text
row ordering changed unexpectedly
diagnostic ordering random
output nondeterministic
```

---

# 123. Stable Result Ordering

例如 Data Inspector schema：

必须稳定按：

```text
first observed / column order
```

而不是：

```text
HashMap order
```

JSON schema path：

也必须：

```text
deterministic
```

---

# 124. Sorting Stability

必须测试：

```text
equal keys
null values
mixed case
Unicode
large dataset
```

确保：

```text
stable sort
```

或者：

```text
明确 documented tie-break rule
```

---

# 125. Unicode

重点测试：

```text
中文
日文
韩文
emoji
combining marks
full-width characters
surrogate-sensitive data where applicable
Unicode whitespace
```

不能：

```text
ASCII-only assumptions
```

---

# 126. CSV Unicode Tests

至少：

```csv
姓名,城市,备注
张三,重庆,"你好 🌱"
李四,東京,"こんにちは"
```

必须：

```text
parse correctly
display correctly
export correctly
```

---

# 127. JSON Unicode Tests

测试：

```json
{
  "name": "张三",
  "city": "重庆",
  "emoji": "🌱"
}
```

以及：

```text
escaped Unicode
actual Unicode
```

两者都必须正确。

---

# 128. Property Tests

至少考虑：

```text
CSV write → read roundtrip
JSON serialize → parse
TSV write → read
transform determinism
dedup determinism
sort stability
delimiter escaping
quote escaping
```

---

# 129. Roundtrip Principle

对于：

```text
CSV → parse → serialize
```

不能强求：

```text
byte-for-byte identical
```

但必须：

```text
semantic data equivalent
```

同时：

```text
formatting normalization
```

必须可解释。

例如：

```text
quote style
line ending
column quoting
```

可能变化。

无损条件显式化：在 Preserve Strings + 未配置 null token + JSON cell 策略下，CSV → JSON → CSV 必须语义无损；用 property test 固定该保证。

---

# 130. Data Equivalence

需要定义：

```text
Semantic Equality
```

至少考虑：

```text
row count
column count
column order
field values
null semantics
```

不能只比较：

```text
output string == input string
```

---

# 131. Numeric Roundtrip

必须特别测试：

```text
0
-1
1.5
0.1
large integer
large decimal
scientific notation
very long numeric strings
leading zero identifiers
```

禁止 precision loss。

---

# 132. Null Roundtrip

至少测试：

```text
CSV empty
JSON null
configured null tokens
```

并明确：

```text
empty string
```

与：

```text
null
```

的区别。

---

# 133. CSV Escaping Tests

至少：

```text
comma
quote
newline
tab
leading/trailing spaces
empty cell
```

---

# 134. Row Count Tests

必须测试：

```text
0 rows
1 row
1,000 rows
10,000 rows
100,000 rows
large realistic fixture
```

具体上限根据实际性能结果记录。

---

# 135. Column Count Tests

必须测试：

```text
0 columns
1 column
wide table
many columns
duplicate headers
empty headers
```

---

# 136. Data Cleaner Regression

必须建立真实 fixtures：

```text
dirty_basic.csv
dirty_unicode.csv
dirty_dates.csv
dirty_numbers.csv
dirty_nulls.csv
dirty_duplicates.csv
dirty_mixed_types.csv
```

禁止所有测试只使用：

```text
hello,world
```

这种玩具数据。

---

# 137. Converter Regression

至少：

```text
simple_object_array.json
nested_objects.json
arrays.json
nulls.json
mixed_types.json
jsonl_basic.jsonl
malformed_jsonl.jsonl
csv_basic.csv
csv_quoted.csv
csv_ragged.csv
tsv_basic.tsv
```

---

# 138. Inspector Regression

至少覆盖：

```text
small exact dataset
large sampled dataset
null-heavy dataset
high-cardinality dataset
mixed-type dataset
nested JSON
sparse JSON
```

---

# 139. Fault Tests

必须测试：

```text
file missing before open
file deleted during scan
file changed during export
permission denied
locked file
invalid UTF-8
unsupported encoding
malformed CSV
malformed JSON
malformed JSONL
too many rows
too many columns
too large cell
too many errors
cancel during scan
cancel during conversion
destination collision
TOCTOU
```

---

# 140. Crash Safety

测试：

```text
large export interrupted
cancel during write
destination failure
disk-full-like error where simulatable
partial write failure
history failure
```

不得把：

```text
planned output
```

错误记录成：

```text
successful output
```

---

# 141. Atomic Write

导出必须优先：

```text
temporary file
→
flush / sync as appropriate
→
atomic replacement / rename
```

具体语义遵循当前 M2 Safe Write infrastructure。

禁止：

```text
truncate original
→
half write
→
crash
→
user loses file
```

---

# 142. Existing M2 Infrastructure Reuse

必须优先复用：

```text
OperationId
OperationPlan
OperationResult
OperationTransaction
HistoryEntry
Validation
Safe Write
TOCTOU checks
Progress
Cancellation
Path Safety
Filesystem
```

如果现有基础设施不足：

只允许：

> **最小、通用、向后兼容的增强。**

禁止：

```text
rewrite M2
```

---

# 143. M3 Infrastructure Reuse

继续复用：

```text
Streaming
Hash
Deterministic ordering
Resource Limits
Cancellation
Structured diagnostics
```

不要因为：

```text
Data Cleaner
```

再建一套：

```text
Progress framework
```

---

# 144. M4 Infrastructure Reuse

必须优先复用：

```text
TextDocument
Encoding
LineEnding
Diagnostics
JSON parser infrastructure
Regex safety
Safe Text I/O
```

M5 Data Parser 与 M4 Text Parser：

可以：

```text
共享底层解析能力
```

但不能：

```text
共享错误的业务模型
```

---

# 145. M5 Application Commands

建议命令按领域组织，例如：

```text
data_detect
data_open
data_profile
data_get_rows
data_filter
data_sort
data_transform_preview
data_convert_preview
data_export
data_cancel
```

具体命名遵循当前仓库 convention。

不要为了 M5 引入：

```text
generic_command_bus
```

等与需求无关的大型架构。

---

# 146. Session Lifetime

如果建立：

```text
DataSession
```

必须定义：

```text
creation
last-used
expiration
manual close
memory release
```

避免打开几十个大文件之后：

```text
memory never released
```

---

# 147. Session Isolation

不同文件必须：

```text
separate session identity
```

不能出现：

```text
A.csv filter state
```

泄漏到：

```text
B.csv
```

---

# 148. UI State

React UI 只管理：

```text
presentation state
selection state
active filters
visible page
tool options
```

不要管理：

```text
millions of rows
```

作为单一 React state。

---

# 149. Virtualization

Data Grid 必须：

```text
row virtualization
```

必要时：

```text
column virtualization
```

至少保证：

```text
large dataset
```

不会：

```text
mount thousands of DOM rows
```

---

# 150. Accessibility

至少支持：

```text
keyboard navigation
focus order
visible focus
screen-reader meaningful labels
column action names
filter labels
error summaries
```

不能因为 Data Grid 复杂：

```text
放弃 accessibility
```

---

# 151. i18n

所有用户可见文本必须：

```text
translation key
```

不得在组件中大量写：

```text
English hardcode
Chinese hardcode
```

至少继续支持当前：

```text
zh
en
```

并遵循项目已有 fallback。

---

# 152. Error UX

错误必须分层：

```text
What happened
Why
What can user do
```

例如：

```text
CSV parsing stopped at row 1842.

The row contains an unterminated quoted field.

The file was not modified.

[Open Diagnostic]
[Cancel]
```

不要：

```text
Parse failed
```

就结束。

---

# 153. Empty States

至少：

```text
No file selected
Empty dataset
No rows match filter
No duplicate rows
No anomalies
No diagnostics
```

都必须有明确状态。

---

# 154. “No anomalies” 不等于 “Perfect Data”

不要显示：

```text
Your data is perfect
```

如果只是：

```text
current rules found no anomalies
```

应该显示：

```text
No issues detected by the current checks.
```

---

# 155. Data Quality Language

必须避免：

```text
Clean
Safe
Correct
Valid
```

这些词在超出实际证明范围时使用。

例如：

```text
No detected anomalies
```

比：

```text
Data is clean
```

更准确。

---

# 156. Data Detection Language

区别：

```text
Detected
Inferred
Observed
Configured
Unknown
```

例如：

```text
Encoding: UTF-8 [Detected by BOM]
```

比：

```text
Encoding: UTF-8
```

更可解释。

---

# 157. M5 Performance Targets

不要先拍脑袋写 benchmark 数字。

实施前：

先建立：

```text
Performance Baseline
```

然后选择现实数据集：

```text
10 MB
100 MB
1M rows where practical
wide table
nested JSON
large JSONL
```

真实测量：

```text
operation
input bytes
rows
columns
elapsed
peak memory
throughput
result
```

---

# 158. Performance Evidence

必须最终输出：

```text
CSV Scan:
input:
rows:
columns:
elapsed:
peak memory:

CSV Filter:
input:
matched:
elapsed:
peak memory:

CSV Export:
input:
output:
elapsed:
peak memory:

JSONL → JSON:
records:
elapsed:
peak memory:

JSON → CSV:
records:
columns:
elapsed:
peak memory:

Inspector:
dataset:
exact/sample:
elapsed:
peak memory:
```

禁止：

```text
fast
works well
large files supported
```

这种无证据结论。

---

# 159. UI Performance

至少验证：

```text
10k rows
50k rows
100k rows
```

或者当前机器可实际稳定测试的同级别数据。

观察：

```text
scroll smoothness
IPC latency
filter latency
sort latency
memory growth
cancel responsiveness
```

---

# 160. IPC Performance

不能只测：

```text
Rust function
```

然后说：

```text
application is fast
```

至少测：

```text
Rust
→
Tauri IPC
→
React
```

真实路径。

---

# 161. Conversion Performance

至少比较：

```text
small
medium
large
```

不要只用：

```text
2 KB sample.json
```

就宣称：

```text
large data supported
```

---

# 162. Property / Fuzz

CSV/JSON parser 是非常适合 fuzz 的区域。

至少考虑：

```text
random delimiters
random quoting
random unicode
random escaping
nested JSON
deep JSON
long strings
empty fields
ragged rows
```

目标：

```text
no panic
no memory blow-up beyond limits
no undefined behavior
structured failure
```

---

# 163. Parser Panic Safety

任何：

```text
malformed input
```

都不得让 Tauri command：

```text
panic
```

如果第三方 parser 能 panic：

必须：

```text
wrap / validate / isolate
```

并增加 regression test。

---

# 164. No Silent Data Loss

这是 M5 的最高优先级原则之一。

任何操作如果可能：

```text
drop field
drop row
coerce value
round number
change encoding
change newline
rename field
flatten nested value
change null semantics
```

必须：

```text
explicit
previewed
or rejected
```

---

# 165. No Silent Type Coercion

禁止：

```text
CSV:
"00123"
↓
123
```

除非：

```text
Infer Types = enabled
```

且：

```text
preview visible
```

---

# 166. No Silent Row Loss

禁止：

```text
malformed row
→ skip
→ success
```

至少必须：

```text
skipped count
diagnostic
reason
```

并且 export summary 明确：

```text
Processed:
1,000,000

Exported:
999,997

Skipped:
3
```

---

# 167. Conversion Result

每次 conversion 至少：

```text
Input Records
Output Records
Skipped Records
Errors
Warnings
Type Conversions
Structural Changes
```

---

# 168. Export Result

至少：

```text
Destination
Bytes Written
Rows Written
Columns Written
Warnings
Errors
```

并区分：

```text
Planned
Actual
```

---

# 169. Actual Mutation Tracking

如果导出失败：

```text
Transaction
```

只能记录真实成功写入状态。

不能：

```text
plan says export success
```

然后：

```text
History says success
```

---

# 170. Interrupted Export

如果 export 中断：

必须：

```text
cleanup temp file where safe
```

并记录：

```text
Interrupted / Failed
```

不能让：

```text
partial destination
```

被误认为完整结果。

---

# 171. History Summary

例如：

```text
Data Cleaner
12,842 rows transformed
→ exported to cleaned.csv
```

不要写：

```text
123 KB changed
```

但实际上没有真实 mutation semantics。

---

# 172. Undo Semantics

对于：

```text
Export to new file
```

Undo 可以：

```text
remove generated output
```

但必须：

```text
revalidate generated file identity
```

如果用户已经：

```text
modified generated file
```

不得直接覆盖 / 删除。

这必须遵守：

> **Undo never destroys a later user change.**

---

# 173. Session Undo

如果实现 Data Studio 内部：

```text
Undo
Redo
```

它属于：

```text
session transformation history
```

与 M2 persistent History：

明确分层。

---

# 174. Session Transform History

例如：

```text
Trim email
↓
Rename country
↓
Sort
↓
Filter
↓
Deduplicate
```

可以允许：

```text
Undo last transform
```

但：

```text
Filter
```

到底是否属于 undo stack：

需要在设计阶段明确。

推荐：

```text
View State
```

与：

```text
Data Transform State
```

分离。

---

# 175. View State vs Data State

```text
Filter
Sort
Search
```

属于：

```text
View State
```

而：

```text
Trim
Delete Column
Split Column
Merge Column
Deduplicate
```

属于：

```text
Data State
```

这两个不要混在同一个历史概念里。

---

# 176. Filter Does Not Delete

尤其必须防止：

```text
Filter result = 10 rows
```

然后：

```text
Export
```

用户以为：

```text
整个 dataset
```

结果只导出：

```text
10 rows
```

因此 Export UI 必须明确：

```text
Export all rows
Export filtered rows
```

并显示：

```text
N of M rows
```

---

# 177. Search Does Not Filter

Search 与 Filter：

可以：

```text
separate
```

例如：

```text
Find "Alice"
```

只是：

```text
locate matches
```

而：

```text
Filter name contains Alice
```

改变：

```text
visible row set
```

不要让两者语义混乱。

---

# 178. Sort Does Not Rewrite Source

Sort 默认只是：

```text
view / session state
```

不会：

```text
自动重新排列源文件
```

除非用户：

```text
Export
```

---

# 179. Export Scope

必须清晰：

```text
Current View
Current Filtered Rows
All Rows
```

用户必须知道导出的是哪一种。

---

# 180. Column Export Scope

同样：

```text
All Columns
Current Columns
Selected Columns
```

如支持 selected columns：

必须在 preview 显示：

```text
Exporting 8 / 18 columns
```

---

# 181. CSV Studio Data Mutations

以下：

```text
Delete Column
Split Column
Merge Column
Fill Empty
Find / Replace
Trim
Normalize
Deduplicate
```

必须：

```text
previewable
deterministic
cancelable where expensive
```

---

# 182. Batch Size / Row Processing

不要为了：

```text
“支持百万行”
```

一次构造：

```text
Vec<Row>
```

几百万项。

可以使用：

```text
streaming iterator
chunk processing
bounded buffers
```

具体实现必须基于真实性能审计。

---

# 183. Output Backpressure

当：

```text
Parser
→
Transform
→
Serializer
→
File
```

连接时：

必须避免：

```text
producer outruns consumer
```

导致：

```text
unbounded memory queue
```

---

# 184. Concurrency Limits

所有并行任务必须：

```text
bounded
```

例如：

```text
max workers
max queued tasks
max buffer
```

具体值：

由：

```text
measurement
```

决定。

---

# 185. File Lock Handling

Windows 下文件被其他进程使用时：

必须：

```text
structured error
```

而不是：

```text
generic I/O error
```

UI 至少告诉用户：

```text
file may be locked
```

---

# 186. Path Safety

所有：

```text
open
export
save
```

继续走：

```text
M1 Path Safety
```

不要：

```text
UI concatenates path
```

或：

```text
Rust data module directly trusts user path
```

---

# 187. Symlink

读取数据文件时：

必须遵守：

```text
M1 symlink policy
```

不要让 M5：

```text
follow symlink differently
```

导致安全边界不一致。

---

# 188. Temp Files

所有临时文件：

必须：

```text
inside controlled temp location
```

并保证：

```text
cleanup
```

不要把：

```text
.tmp
.parsed
.preview
```

永久留在用户目录。

---

# 189. Sensitive Data

测试中可以使用：

```text
synthetic personal-like data
```

但不要提交：

```text
真实姓名
真实邮箱
真实电话
真实医院数据
真实客户数据
真实公司数据
```

---

# 190. Test Data

推荐：

```text
generated deterministic fixtures
```

例如：

```text
seeded generator
```

避免：

```text
大量 binary fixtures
```

占仓库体积。

---

# 191. Documentation

M5 至少更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
SECURITY.md
THIRD_PARTY_LICENSES.md
```

必要时增加：

```text
DATA_FORMATS.md
```

---

# 192. DATA_FORMATS.md

如果建立该文档，至少说明：

```text
CSV semantics
TSV semantics
JSON semantics
JSONL semantics
Header rules
Type inference
Null semantics
Delimiter detection
Encoding
Line ending
Nested JSON conversion
Error handling
Sampling
Resource limits
```

---

# 193. DECISIONS.md

至少记录：

```text
Why structured data model
Why csv parser
Why no spreadsheet engine
Why no SQLite dataset cache
Why type inference is opt-in / explicit
Why exact unique may become unavailable
Why malformed records have bounded recovery
Why filter/sort are view state
Why source file is not mutated by default
```

---

# 194. PROGRESS.md

完成 M5 后：

```text
M0 = COMPLETE
M1 = COMPLETE
M2 = COMPLETE
M3 = COMPLETE
M4 = COMPLETE
M5 = COMPLETE
M6 = NEXT
```

同时写：

```text
Implemented
Tested
Known Limitations
Performance
Decisions
```

---

# 195. First Step — Baseline Audit

在任何修改之前：

必须先审计当前仓库真实状态。

至少检查：

```text
Git branch
HEAD
Working tree
M0
M1
M2
M3
M4
```

并检查：

```text
Cargo workspace
weave-core
weave-files
weave-text
weave-history
weave-testkit
Tauri commands
IPC DTOs
frontend command layer
drag & drop
i18n
design tokens
Progress
Cancellation
Path Safety
Filesystem
Safe Write
Transaction
History
Undo
Encoding
TextDocument
Diagnostics
JSON parser
```

---

# 196. Audit Classification

所有发现必须分类：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

```text
FACT:
weave-text already exposes UTF-8 / UTF-16 detection.

FACT:
M2 already owns safe export transaction.

HYPOTHESIS:
M5 can reuse the same file-write path.

INFERENCE:
Data export should use existing transaction infrastructure rather than creating a second exporter writer.
```

---

# 197. Audit 状态

每个前置能力标记：

```text
CONFIRMED
RESOLVED
WORSENED
BLOCKED
UNKNOWN
```

不要因为：

```text
Charter says it exists
```

就假设：

```text
code must exist
```

---

# 198. Gap Analysis

首先给出：

```text
Existing
Missing
Reusable
Needs Extension
Must Not Duplicate
```

例如：

```text
Encoding:
EXISTING → REUSE

JSON parser:
EXISTING → EXTEND SAFELY

CSV parser:
MISSING → ADD MATURE LIBRARY

History:
EXISTING → REUSE

Data session:
MISSING → DESIGN MINIMAL
```

---

# 199. Implementation Order

严格推荐：

```text
Phase 0
Baseline Audit

Phase 1
Data Domain Model

Phase 2
CSV / TSV Parser

Phase 3
JSON / JSONL Data Parser

Phase 4
Data Inspector

Phase 5
CSV Studio

Phase 6
Data Cleaner

Phase 7
Converters

Phase 8
Preview / Export Integration

Phase 9
UI Polish / Accessibility / i18n

Phase 10
Performance / Fault / Fuzz

Phase 11
Regression / Security / Privacy

Phase 12
Final Audit

Phase 13
Documentation / Commit
```

不要：

```text
先把全部 UI 画出来
```

再：

```text
硬塞 backend
```

---

# 200. Phase 1 — Data Domain

先建立：

```text
DataFormat
DataValue
DataSchema
DataDocument
DataProfile
Diagnostic
TransformRule
ConversionOptions
```

写完整：

```text
unit tests
serialization tests
edge case tests
```

---

# 201. Phase 2 — CSV / TSV

实现：

```text
detect
parse
stream
diagnostics
profile
serialize
```

优先建立：

```text
CSV/TSV common parser abstraction
```

而不是：

```text
CSV parser
TSV parser
```

两套重复实现。

---

# 202. Phase 3 — JSON / JSONL

实现：

```text
JSON structured reader
JSONL streaming reader
schema inspection
profile
serialization
```

复用 M4：

```text
JSON parser foundations
Encoding
Diagnostics
```

---

# 203. Phase 4 — Data Inspector

先完成：

```text
Format
Encoding
Rows
Columns
Schema
Null Rate
Unique Rate
Potential Type
```

然后：

```text
exact / sampled / unavailable
```

标签。

---

# 204. Phase 5 — CSV Studio

按：

```text
Open
View
Search
Filter
Sort
Columns
```

再：

```text
Transform
Preview
Export
```

不要一次提交：

```text
50 个功能
```

---

# 205. Phase 6 — Data Cleaner

实现：

```text
Trim
Case
Empty/Null
Deduplicate
Date
Numeric
Find/Replace
Anomaly
```

全部采用：

```text
Rule
→
Preview
→
Apply
```

---

# 206. Phase 7 — Converter

实现：

```text
JSON → CSV
CSV → JSON
CSV → TSV
TSV → CSV
JSONL → JSON
JSON → JSONL
```

每个方向：

必须有：

```text
golden fixtures
roundtrip tests where meaningful
loss detection
diagnostics
```

---

# 207. Phase 8 — Export

统一：

```text
Preview
Validate
Export
Progress
Cancel
Result
History
Undo
```

不得：

```text
Converter own write layer
Cleaner own write layer
CSV Studio own write layer
```

---

# 208. Phase 9 — UI

必须最终形成：

```text
Drop
↓
Detect
↓
Inspect
↓
Choose Tool
↓
Configure
↓
Preview
↓
Export
↓
Result
```

体验一致。

---

# 209. UI 不得暴露内部实现

用户不要看到：

```text
DataSessionId
OperationPlanId
Rust task handle
```

除非是：

```text
debug build
```

---

# 210. Result Screen

必须展示：

```text
What happened
Rows affected
Columns affected
Output path
Warnings
Errors
Skipped records
```

并提供：

```text
Open File
Open Folder
Reuse Operation
History
```

其中哪些动作已经存在，就复用，不要重复实现。

---

# 211. Testing Layers

必须至少包含：

```text
Unit
Integration
Fault
Property
Fuzz
Performance
UI Smoke
```

---

# 212. Unit Tests

覆盖：

```text
Parser
Type Detection
Filtering
Sorting
Transforms
Dedup
Conversion
Profiling
Diagnostics
```

---

# 213. Integration Tests

真实：

```text
filesystem
Tauri IPC
React command layer
export
history
```

至少覆盖关键闭环。

---

# 214. Real Filesystem Tests

至少：

```text
open CSV
export CSV
export TSV
export JSON
export JSONL
overwrite conflict
path safety
cancel
```

---

# 215. UI Smoke

至少真实启动：

```text
Tauri application
```

验证：

```text
drag file
open
inspect
view
filter
sort
transform
preview
export
result
```

---

# 216. No Mock Success

禁止：

```text
hardcoded 1000 rows
fake schema
fake type detection
fake export success
fake progress
```

测试中可以使用 mock：

但必须：

```text
integration path also verified with real parser/filesystem
```

---

# 217. No Suppressed Assertions

禁止：

```text
unwrap blindly
ignored test
disabled test
commented assertion
TODO test
```

除非：

```text
reason documented
issue tracked
known limitation explicit
```

---

# 218. No Fake Parser

禁止：

```text
split(',')
regex parse JSON
manual quote parser
string replace based conversion
```

除非：

```text
the implementation is proven semantically sufficient
```

通常：

```text
CSV → mature parser
JSON → real parser
```

---

# 219. No Fake Type Detection

禁止：

```text
if value.parse::<i64>().is_ok()
then Integer
```

就直接宣布：

```text
column = Integer
```

必须考虑：

```text
sample
consistency
leading zeros
null
mixed values
format evidence
```

---

# 220. Final Acceptance Criteria

以下全部满足才能：

```text
M5 COMPLETE
```

## Data Foundation

```text
[ ] Data domain model exists
[ ] DataValue semantics are explicit
[ ] stable ColumnId exists
[ ] stable Row identity exists where needed
[ ] CSV parser works
[ ] TSV parser works
[ ] JSON parser works
[ ] JSONL parser works
[ ] encoding reuse works
[ ] diagnostics exist
```

## CSV Studio

```text
[ ] Open works
[ ] View works
[ ] Virtualized / paged UI works
[ ] Filter works
[ ] Sort works
[ ] Search works
[ ] Column Rename works
[ ] Column Delete works
[ ] Column Reorder works
[ ] Type Detection works
[ ] Encoding Detection works
[ ] Deduplicate works
[ ] Trim works
[ ] Normalize works
[ ] Split Column works
[ ] Merge Column works
[ ] Fill Empty works
[ ] Find / Replace works
```

## Data Cleaner

```text
[ ] Detect works
[ ] Preview works
[ ] Rules work
[ ] Apply works
[ ] Export works
[ ] Whitespace rules work
[ ] Case rules work
[ ] Empty / Null semantics work
[ ] Duplicate row handling works
[ ] Date normalization works within declared scope
[ ] Numeric normalization works within declared scope
[ ] Anomaly discovery works
[ ] Anomalies are distinguished from hard errors
```

## Converter

```text
[ ] JSON → CSV works
[ ] CSV → JSON works
[ ] CSV → TSV works
[ ] TSV → CSV works
[ ] JSONL → JSON works
[ ] JSON → JSONL works
[ ] Nested objects are handled explicitly
[ ] Arrays are handled explicitly
[ ] Null semantics are explicit
[ ] Type conversion is explicit
[ ] Duplicate headers are handled
[ ] Malformed input is diagnosed
[ ] No silent row loss
[ ] No silent field loss
[ ] No silent numeric precision loss
```

## Data Inspector

```text
[ ] Format works
[ ] Encoding works
[ ] Rows works
[ ] Columns works
[ ] Schema works
[ ] Null Rate works
[ ] Unique Rate works
[ ] Potential Type works
[ ] Exact vs Sampled is explicit
[ ] Unavailable statistics are explicit
[ ] JSON nested inspection works
```

## Preview / Export

```text
[ ] Preview works
[ ] Preview has no filesystem side effect
[ ] Export requires explicit action
[ ] Export scope is explicit
[ ] Output format is explicit
[ ] Encoding is explicit
[ ] Delimiter is explicit
[ ] Header policy is explicit
[ ] Safe write is reused
[ ] TOCTOU protection works
[ ] Atomic write works where applicable
[ ] Transaction is reused
[ ] History is reused
[ ] Undo is safe
```

## Performance

```text
[ ] Large CSV test exists
[ ] Large TSV test exists
[ ] Large JSONL test exists
[ ] Large JSON test exists within declared limits
[ ] Streaming / bounded processing exists where required
[ ] UI remains responsive
[ ] IPC payloads are bounded
[ ] Memory limits exist
[ ] Cancellation works
[ ] Progress is honest
```

## Security

```text
[ ] Path Traversal protection
[ ] Symlink policy reused
[ ] No arbitrary shell
[ ] No network dependency
[ ] No telemetry
[ ] No upload
[ ] No secret logging
[ ] Malformed data cannot trivially crash app
[ ] Deep JSON bounded
[ ] Huge fields bounded
[ ] Huge columns bounded
[ ] Huge errors bounded
[ ] Numeric precision preserved
[ ] TOCTOU protection
```

## Tests

```text
[ ] Unit tests
[ ] Integration tests
[ ] Fault tests
[ ] Property tests
[ ] Fuzz coverage or equivalent malformed-input stress
[ ] Performance tests
[ ] Real filesystem tests
[ ] UI smoke tests
[ ] M0 regression
[ ] M1 regression
[ ] M2 regression
[ ] M3 regression
[ ] M4 regression
```

## Quality

```text
[ ] Rust formatting passes
[ ] cargo fmt --check
[ ] cargo clippy
[ ] cargo test passes
[ ] Frontend typecheck passes
[ ] frontend test passes
[ ] Frontend lint passes
[ ] Frontend build passes
[ ] Tauri build passes
[ ] CI passes
[ ] Documentation updated
[ ] Dependency/license audit updated
[ ] Git hygiene passes
```

---

# 221. M5 Final Audit

实施结束后：

> **不要直接宣布 M5 COMPLETE。**

必须先执行：

```text
Final Audit
```

---

## A. Baseline

输出：

```text
Branch:
HEAD:
Working Tree:
M0:
M1:
M2:
M3:
M4:
```

---

## B. Architecture

检查：

```text
weave-core
weave-files
weave-text
weave-data
weave-history
weave-testkit
Tauri
IPC
React
```

确认：

```text
No duplicated filesystem layer
No duplicated encoding layer
No duplicated history layer
No duplicated preview layer
No duplicated path-safety layer
```

---

## C. Data Model

检查：

```text
DataDocument
DataValue
DataSchema
ColumnId
Row identity
Diagnostics
Profile
Transform Plan
Conversion Options
```

---

## D. CSV / TSV

逐项：

```text
Parser
Delimiter
Quote
Escaping
Header
Ragged Rows
Encoding
Line Ending
Diagnostics
Streaming
Export
```

结果只能：

```text
PASS
FAIL
NOT SUPPORTED
NOT VERIFIED
```

---

## E. JSON / JSONL

检查：

```text
JSON parse
JSONL parse
Nested Object
Nested Array
Null
Mixed Type
Streaming
Depth Limit
Malformed Input
Serialization
```

---

## F. CSV Studio

检查：

```text
Open
View
Virtualization
Filter
Sort
Search
Column Rename
Column Delete
Column Reorder
Type Detection
Encoding
Dedup
Trim
Normalize
Split
Merge
Fill
Find/Replace
```

---

## G. Data Cleaner

检查：

```text
Detect
Preview
Rules
Apply
Export
Whitespace
Case
Null
Duplicates
Date
Numeric
Anomaly
```

---

## H. Converter

检查：

```text
JSON → CSV
CSV → JSON
CSV → TSV
TSV → CSV
JSONL → JSON
JSON → JSONL
```

以及：

```text
Nested
Null
Numbers
Booleans
Strings
Duplicate headers
Malformed records
```

---

## I. Inspector

检查：

```text
Format
Encoding
Rows
Columns
Schema
Null Rate
Unique Rate
Potential Type
Sampling
Exactness
Diagnostics
```

---

## J. Preview / Mutation

确认：

```text
Preview
Plan
Validation
Export
Safe Write
TOCTOU
Atomic Write
Transaction
History
Undo
```

---

## K. Performance

必须输出真实：

```text
Dataset
Rows
Columns
Input Size
Operation
Elapsed
Peak Memory
Output
```

不得写：

```text
works fine
fast enough
```

---

## L. Security

直接输出：

```text
Path Traversal: PASS
Symlink Boundary: PASS / PLATFORM-LIMITED
Malformed CSV Safety: PASS
Malformed JSON Safety: PASS
JSON Depth Limit: PASS
Huge Field Handling: PASS
Huge Column Handling: PASS
IPC Bounds: PASS
Regex Safety: PASS / REUSED
No Shell: PASS
No Network: PASS
Secret Logging: PASS
TOCTOU: PASS
Atomic Write: PASS
```

---

## M. Privacy

确认：

```text
No Telemetry
No Upload
No Cloud Requirement
No Remote Processing
No Account
```

---

## N. Regression

必须确认：

```text
M0 PASS
M1 PASS
M2 PASS
M3 PASS
M4 PASS
```

尤其重点检查：

```text
Filesystem
Path Safety
History
Undo
Preview
Safe Write
TextDocument
Encoding
JSON foundations
```

---

## O. Documentation

检查：

```text
README
ARCHITECTURE
DECISIONS
PROGRESS
PRIVACY
SECURITY
THIRD_PARTY_LICENSES
DATA_FORMATS where applicable
CHANGELOG where applicable
```

---

## P. Known Limitations

必须诚实记录：

```text
Not Implemented
Not Supported
Platform-specific
Size-limited
Parser-specific
Encoding-specific
Sampling-based
Dialect / Locale-specific
Not Verified
```

尤其不能隐藏：

```text
JSON root limitations
nested conversion limitations
exact unique limitations
date ambiguity
numeric locale limitations
large JSON limits
malformed CSV recovery limits
```

---

# 222. M5 禁止“假完成”

以下任一情况存在：

```text
CSV Studio 有界面，但 backend 不能真实读取 CSV

CSV parser 实际是 split(',')

JSONL 实际一次性 read_to_string

Type Detection 随便猜

Unique Rate 是假的

Schema 由 mock 数据生成

Preview 与实际 Export 走不同逻辑

Export 成功但部分行被静默丢弃

Numeric precision 丢失但没有警告

Nested JSON 变成 [object Object]

Duplicate Header 覆盖旧字段

Malformed row 被偷偷跳过

Filter 在 UI，百万行全部 IPC 到前端

Sort 在 React Array.sort 上完成

Virtualized grid 实际渲染全部 rows

Cancel 只改变 UI 状态

Progress 是假百分比

History 没有记录真实 mutation

Undo 会删除用户后续修改

TOCTOU 未验证

Save 会直接覆盖原文件

使用 shell CLI

使用网络 API

加入巨大 dataframe engine 但没有必要

为了 M5 重写 M2

```

都不能宣布：

```text
M5 COMPLETE
```

---

# 223. M5 Final Output Format

完成整个 milestone 后，最终输出必须为：

# M5 Implementation Report

## 1. Baseline

```text
Branch:
HEAD:
Working Tree:
M0:
M1:
M2:
M3:
M4:
```

## 2. Implemented

```text
Data Foundation
CSV / TSV
JSON / JSONL
Data Inspector
CSV Studio
Data Cleaner
Converters
Preview
Export
History
Undo
UI
```

逐项说明：

```text
Implemented
Reused
Extended
Not Supported
Known Limitations
```

---

## 3. Verification

必须填写真实：

```text
Unit:
Integration:
Fault:
Property:
Fuzz:
Performance:
Filesystem:
UI:
Build:
CI:
```

每项必须包含：

```text
command
result
count
status
```

---

## 4. Data Correctness Evidence

至少输出：

```text
Roundtrip Tests
Malformed Input Results
Duplicate Header Results
Numeric Precision Tests
Unicode Tests
Null Semantics Tests
Nested JSON Tests
Ragged CSV Tests
```

---

## 5. Performance Evidence

至少：

```text
CSV:
Input:
Rows:
Elapsed:
Peak Memory:

JSONL:
Input:
Records:
Elapsed:
Peak Memory:

JSON:
Input:
Records:
Elapsed:
Peak Memory:

Conversion:
Input:
Output:
Elapsed:
Peak Memory:

Inspector:
Dataset:
Exact / Sampled:
Elapsed:
Peak Memory:
```

---

## 6. Security Evidence

直接列：

```text
Path Traversal:
Symlink:
Malformed CSV:
Malformed JSON:
Huge Input:
Huge Cell:
Huge Columns:
Regex:
IPC:
TOCTOU:
Atomic Write:
Shell:
Network:
Secret Logging:
```

---

## 7. Known Limitations

必须诚实。

只写真实存在的：

```text
parser limitations
encoding limitations
locale limitations
size limits
sampling limits
platform limitations
conversion limitations
```

---

## 8. Git

输出：

```text
Branch:
HEAD:
Commit:
Origin:
Working Tree:
```

并执行：

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
personal data
temporary reports
cache
build output
large generated files
```

---

# 224. Commit Strategy

只有：

```text
Final Audit PASS
```

之后才允许 commit。

遵循当前仓库 Git convention。

推荐：

```text
feat(data): implement data tools
```

或者：

```text
feat(data): add csv json data workflows
```

不要：

```text
update
misc
work
stuff
fix
```

---

# 225. M5 Handoff

M5 完成后必须向 M6 / 后续阶段明确交接：

```text
DataDocument
DataValue
DataSchema
Column Identity
Row Identity
DataTransformPlan (serializable)
CSV Parser
TSV Parser
JSON Parser
JSONL Parser
Data Diagnostics
Data Profile
Type Detection
Sampling Infrastructure
CSV Studio
Data Cleaner
Conversion Engine
Export Serialization
Preview Integration
Safe Write
Transaction
History
Undo
Progress
Cancellation
Resource Limits
```

特别明确：

> **M6 Image 必须继续复用已有 File Core、Preview、Progress、Cancellation、Safe Write、Transaction、History、Undo 与资源限制体系，不得重新建立第二套通用文件操作基础设施。**

---

# 226. M5 最终产品闭环

M5 完成后，用户应该已经能够：

```text
拖入 CSV / TSV / JSON / JSONL
        ↓
Weave Detect
        ↓
Inspector
        ↓
View
        ↓
Filter / Search / Sort
        ↓
Clean / Transform
        ↓
Preview
        ↓
Convert / Export
        ↓
Progress
        ↓
Result
        ↓
History
        ↓
Undo where applicable
```

用户最终感受到的不是：

> “Weave 又加了一个复杂的数据处理引擎。”

而是：

> **“我手里有一个乱七八糟的 CSV / JSON 文件，扔进 Weave，我能马上看懂它、发现问题、快速整理，再安全地导出去。”**

---

# 227. M5 核心原则

整个 M5 始终遵守：

```text
Correctness > Convenience
Data Integrity > Feature Count
Explicit Semantics > Magic
Preview > Mutation
Streaming > Whole-file Memory
Bounded > Unlimited
Fact > Guess
Stable Identity > Index Assumptions
Semantic Equality > Byte Equality
Reuse > Rebuild
Local > Cloud
Actual Result > Planned Result
Real Tests > Green Theater
```

尤其牢记：

> **数据工具最大的价值不是“能处理”，而是“不会悄悄改变用户的数据”。**

以及：

> **任何可能造成数据丢失、类型改变、字段丢失、行丢失、精度损失或语义变化的行为，都必须显式、可预览、可解释；否则就拒绝执行。**

---

# 228. 执行纪律

整个 M5 严格采用：

```text
Audit
 ↓
Gap Analysis
 ↓
Design
 ↓
Minimal Implementation
 ↓
Unit Tests
 ↓
Integration Tests
 ↓
Fault Tests
 ↓
Property / Fuzz
 ↓
Performance
 ↓
UI Smoke
 ↓
Security
 ↓
Privacy
 ↓
Regression
 ↓
Final Audit
 ↓
Documentation
 ↓
Commit
```

任何阶段都不要因为：

```text
“差不多能用了”
```

而跳过验证。

任何发现都要先回答：

```text
这是 FACT？
还是 HYPOTHESIS？
还是 INFERENCE？
```

任何问题都优先：

```text
P0
P1
```

只处理真正阻塞可靠性、安全性、数据正确性、架构完整性的问题。

---

# 229. 最终完成条件

只有当：

```text
M5 Acceptance Criteria
+
Final Audit
+
Regression
+
Performance Evidence
+
Security Evidence
+
Documentation
+
Git Hygiene
```

全部满足后，才能输出：

```text
M5 COMPLETE
```

否则必须输出：

```text
M5 NOT COMPLETE
```

并准确列出：

```text
BLOCKED
FAIL
NOT VERIFIED
NOT SUPPORTED
KNOWN LIMITATION
```

禁止通过：

```text
降低测试标准
隐藏数据丢失
关闭错误
放宽断言
跳过大文件测试
伪造性能
伪造 UI
硬编码结果
mock 冒充真实实现
```

来获得“完成”状态。

---

# 230. M5 的终极目标

M5 不是：

```text
CSV editor
+
JSON viewer
+
converter
```

三个孤立功能。

真正目标是建立：

```text
                    ifuyo Weave
                         │
                    Structured Data
                         │
          ┌──────────────┼──────────────┐
          ▼              ▼              ▼
      Inspector       Studio          Cleaner
          │              │              │
          └──────────────┼──────────────┘
                         ▼
                    Transform Plan
                         │
                         ▼
                       Preview
                         │
                  ┌──────┴──────┐
                  ▼             ▼
               Convert        Export
                  │             │
                  └──────┬──────┘
                         ▼
                       Result
                         │
                   History / Undo
```

最终：

> **M5 要让 Weave 从“能处理文本”，真正进入“能处理结构化数据”的阶段。**

并且整个 Data 体系必须继续保持 Weave 的核心产品哲学：

```text
Drop
→
Understand
→
Preview
→
Weave
→
Done
```

不是：

```text
Import
→
Magic
→
Hope
```