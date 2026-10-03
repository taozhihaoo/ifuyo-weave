# ifuyo Weave — M5 Data 完整开发提示词（上）

> 本文件是原《ifuyo Weave — M5 Data 完整开发提示词》的 **第 1/2 部分**，
> 覆盖原第 0–95 节：M5 硬边界、本阶段明确禁止、架构原则、与 M4 的边界，M5 数据模型与 DataValue、Type Detection 与 Numeric/JSON 数值安全，CSV/TSV Parse 与 Header/Duplicate Header/Ragged Rows，Data Table/Data Session 及其边界，Filter/Sort/Search 及其语义，全部列操作（Rename/Delete/Reorder/Split/Merge/Fill Empty）、Null Policy、Trim/Case/Find-Replace/Duplicate/Deduplicate，日期与数值归一化、Anomaly 语义，Data Inspector 与统计（Null Rate/Unique Rate/Sampled Profiling），JSON Inspector 与嵌套结构，JSON/CSV/JSONL 双向转换与 Malformed 处理，Encoding/Line Ending/Delimiter/Quote 检测，Export 语义与原始文件原则、Apply vs Export、TOCTOU，Undo/History/Preview，Data Transform Model 与 Rule Ordering/Validation、纯逻辑约束，weave-data 职责、分层、Data IPC/Paging/Filtering/Sorting Backend、Memory Budget 与 Streaming、JSON Parser 安全、JSON Schema Profiling 与 Schema Paths/CSV Schema。
> **必须与（下）一起阅读执行**：CSV Studio/Cleaner/Converter/Inspector 的 UI 与 UX、测试矩阵与回归、性能目标、安全与隐私、Baseline Audit、实施阶段（Phase 1–9）、Final Acceptance Criteria、M5 Final Audit、提交纪律、交接条款均在（下）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---
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

