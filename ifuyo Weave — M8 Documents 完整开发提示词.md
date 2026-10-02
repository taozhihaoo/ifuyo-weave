# ifuyo Weave
# M8 — Documents
## PDF / DOCX / XLSX / PPTX 文档检查、页面操作、合并拆分与统一文档处理能力

你现在进入：

> **ifuyo Weave 的 M8：Documents**

项目：

```text
ifuyo Weave
```

当前阶段基线：

```text
M0 Foundation
M1 File Core
M2 Rename / Organizer
M3 Duplicate Finder
M4 Text
M5 Data
M6 Image
M7 Batch Engine
```

现在进入：

```text
M8 Documents
```

M8 的目标不是把 Weave 做成：

```text
Microsoft Office
Adobe Acrobat
LibreOffice
专业排版软件
专业 PDF 编辑器
```

而是：

> **建立一个可靠、轻量、本地优先、可预览、可批处理的 Document Processing 层。**

核心思想：

```text
Understand
    ↓
Inspect
    ↓
Plan
    ↓
Preview
    ↓
Execute
    ↓
Result
    ↓
History / Undo
```

M8 要解决的是：

```text
“我手里有一批 PDF / DOCX / XLSX / PPTX，
我想快速知道它是什么、里面有什么、
有哪些页面/工作表/幻灯片，
以及完成一些常见的结构性处理。”
```

而不是：

```text
“我想在 Weave 里完整编辑 Word / Excel / PowerPoint。”
```

---

# 0. M8 总体硬边界

## 0.1 本阶段必须完成

### PDF

至少实现：

```text
PDF Inspect
PDF Metadata Inspect
PDF Page Count
PDF File Size
PDF Merge
PDF Split
PDF Extract Pages
PDF Reorder
PDF Rotate
PDF Page Preview / Summary
```

### Document Inspector

至少识别：

```text
PDF
DOCX
XLSX
PPTX
TXT
Markdown
```

其中：

```text
TXT
Markdown
```

已经由 M4 提供基础能力。

M8 主要负责：

```text
Document-level representation
Office-document inspection
PDF processing
```

### DOCX

至少：

```text
DOCX Inspect
DOCX Metadata
Paragraph Count / Approximate Structure
Heading Detection where reliably available
Tables Count
Images Count
Sections
Core Properties
Document Statistics
```

### XLSX

至少：

```text
XLSX Inspect
Workbook Metadata
Sheet List
Sheet Count
Visible / Hidden Sheet State
Used Range
Row / Column Statistics
Cell Count Estimates
Formula Presence
Merged Cell Information where supported
Workbook Properties
```

### PPTX

至少：

```text
PPTX Inspect
Presentation Metadata
Slide Count
Slide List
Hidden Slide State
Text Presence / Statistics
Image Count
Shape Count
Speaker Notes Presence where supported
Presentation Properties
```

### 通用能力

M8 必须接入：

```text
M1 File Core
M2 Safe Write
M2 OperationPlan
M2 Transaction
M2 History
M2 Undo
M4 Text
M5 Data
M6 Image
M7 Batch Engine
```

---

# 0.2 本阶段明确禁止

不得实现：

```text
Word Full Editor
Excel Full Editor
PowerPoint Full Editor
PDF Full Annotation Editor
PDF Form Builder
PDF Form Designer
Spreadsheet Formula Engine
Spreadsheet Recalculation Engine
Word Layout Engine
Presentation Layout Engine
Office Clone
Desktop Publishing System
OCR Platform
AI Document Agent
LLM Document Editing
Semantic Document Assistant
Cloud Document Editing
Remote Document Processing
Online Document Conversion Service
Document Collaboration
Realtime Collaboration
Account System
Telemetry
Cloud Upload
Remote Execution
```

尤其禁止偷做：

```text
M10 Workflow Editor
```

以及：

```text
M9 Utility System
```

与：

```text
M11 Polish
```

中的大规模 UI 系统。

---

# 1. M8 最重要的产品原则

M8 必须遵守：

```text
Local-first
Utility-first
Batch-first
Preview-first
Reversible-first
Composable
```

用户体验继续保持：

```text
Drop
 ↓
Understand
 ↓
Preview
 ↓
Weave
 ↓
Done
```

M8 不应变成：

```text
Open
 ↓
Wait
 ↓
Enter giant editor
 ↓
Search toolbar for 10 minutes
```

而应该尽量：

```text
Drop document
 ↓
Recognize
 ↓
Show useful facts
 ↓
Choose operation
 ↓
Preview
 ↓
Execute
```

---

# 2. M8 的核心产品定义

M8 的职责：

```text
Understand documents
Inspect documents
Perform structural document operations
Process PDF pages
Batch document operations through M7
Export safely
```

M8 不负责：

```text
author documents
design documents
deep interactive editing
full document rendering compatibility
full office application compatibility
```

必须明确：

> **Weave 处理文档，而不是替代 Office。**

---

# 3. M8 架构位置

目标架构：

```text
                         ifuyo Weave
                              │
                              ▼
                       Application Layer
                              │
                              ▼
                         M7 Batch Engine
                              │
                ┌─────────────┼─────────────┐
                ▼             ▼             ▼
             PDF Tool     DOCX Tool     Office Inspector
                │             │             │
                └─────────────┼─────────────┘
                              ▼
                       Document Domain
                              │
              ┌───────────────┼────────────────┐
              ▼               ▼                ▼
          Document        Page Model       Metadata
          Identity        Structure         Facts
                              │
                              ▼
                        Safe File I/O
                              │
                    ┌─────────┴─────────┐
                    ▼                   ▼
                 M1 Files            M2 History
```

严格禁止：

```text
React
 ↓
直接解析 DOCX
```

或：

```text
React
 ↓
直接操作 filesystem
```

或：

```text
React
 ↓
直接调用 PDF library
```

UI 只负责：

```text
State
Intent
Preview display
Command invocation
Progress
Result
Error display
```

真正 document processing 必须在 Rust/domain/application 层。

---

# 4. M8 开发第一原则：先审计真实仓库

开始任何实现前：

> **不得假设 M0–M7 已经按照提示词理想状态完成。**

必须先检查真实仓库。

至少审计：

```text
git status
git branch
git log
Cargo.toml
Cargo.lock
package.json
package-lock.json
workspace
crates/
src-tauri/
ui/
docs/
tests/
```

重点确认：

```text
M1 filesystem abstraction
M1 path safety
M1 structured errors

M2 OperationPlan
M2 Transaction
M2 SafeWrite
M2 History
M2 Undo

M4 TextDocument
M4 Encoding
M4 Diagnostics

M5 Data models

M6 Image abstractions

M7 BatchJob
M7 JobPlan
M7 Pipeline
M7 Stage
M7 Tool capability
M7 Preview
M7 Execute
M7 Retry
M7 Resume
M7 Journal
M7 Progress
M7 Cancellation
M7 Resource Limits
```

所有上述能力必须：

```text
inspect actual implementation
```

而不是：

```text
assume implementation exists
```

---

# 5. 审计结果分类规则

审计发现必须分类：

```text
FACT
HYPOTHESIS
INFERENCE
```

同时标记：

```text
RESOLVED
CONFIRMED
WORSENED
BLOCKED
NOT VERIFIED
```

不要把：

```text
设计文档写了
```

当成：

```text
代码已经实现
```

不要把：

```text
接口存在
```

当成：

```text
功能可用
```

不要把：

```text
测试文件存在
```

当成：

```text
测试通过
```

---

# 6. M8 依赖选择原则

M8 很可能需要引入：

```text
PDF parser / writer
DOCX parser
XLSX parser
PPTX parser
archive / XML parsing
```

但：

> **依赖不是越多越好。**

必须逐项评估：

```text
Need
Maturity
Maintenance
License
Security
Windows compatibility
Rust compatibility
WASM irrelevant / not required
Performance
Memory behavior
Error handling
Format support
```

不得因为：

```text
“这个库很流行”
```

就直接加入。

PDF 库选型评估（Capability Matrix 定稿前完成）：列出候选（如 lopdf、pdf-writer 等），逐项评估 Merge / Split / Extract Pages / Reorder / Rotate 能力、license、维护状态、纯 Rust 与否，结论记入 DECISIONS.md。

---

# 7. 不要伪装格式支持

这是 M8 最重要的规则之一。

UI 不得写：

```text
DOCX ✓
XLSX ✓
PPTX ✓
```

除非真实测试证明：

```text
解析器能够读取目标字段
```

同样：

```text
PDF Merge
PDF Split
PDF Rotate
```

必须是真正执行：

```text
parse
transform
write
```

不能：

```text
rename extension
copy bytes
placeholder output
```

也不得为了通过测试创建：

```text
fake PDF
fake DOCX
fake XLSX
fake PPTX
```

除非测试明确属于：

```text
synthetic fixture
```

并明确标识。

---

# 8. capability-first

建立：

```text
DocumentCapabilities
```

不要把所有格式假装拥有相同能力。

例如：

```text
PDF
├── Inspect
├── PageCount
├── Merge
├── Split
├── Reorder
├── Rotate
└── Metadata

DOCX
├── Inspect
├── Metadata
├── Structure
└── Statistics

XLSX
├── Inspect
├── Workbook
├── Sheets
├── Dimensions
└── Statistics

PPTX
├── Inspect
├── Slides
├── Metadata
└── Statistics
```

能力矩阵必须根据真实代码生成。

---

# 9. DocumentIdentity

建立统一：

```text
DocumentIdentity
```

至少包含：

```text
DocumentId
Path
FileName
Extension
DetectedFormat
MimeType
Size
Created
Modified
```

注意：

```text
Extension
```

不是最终事实。

必须区分：

```text
extension
```

与：

```text
detected format
```

例如：

```text
foo.pdf
```

不一定真的是：

```text
PDF
```

---

# 10. DocumentFormat

至少定义：

```text
PDF
DOCX
XLSX
PPTX
TXT
Markdown
Unknown
Unsupported
Malformed
```

必要时：

```text
LegacyOffice
Encrypted
Corrupted
Partial
```

但只有实际能力能够判断时才加入。

---

# 11. DocumentFacts

建立统一：

```text
DocumentFacts
```

可包括：

```text
Format
Size
Created
Modified
Pages
Sheets
Slides
Metadata
EmbeddedImages
EmbeddedFonts
TextStatistics
DocumentStructure
Warnings
```

字段必须允许：

```text
Known
Unknown
Unavailable
Unsupported
Failed
Estimated
```

不要把：

```text
解析失败
```

变成：

```text
0
```

例如：

```text
Pages = 0
```

不能同时表示：

```text
zero-page document
```

与：

```text
parser failed
```

---

# 12. Document Inspector

核心流程：

```text
Input
 ↓
Path Validation
 ↓
Format Detection
 ↓
Capability Check
 ↓
Safe Read
 ↓
Parse
 ↓
Inspect
 ↓
Facts
 ↓
Diagnostics
```

输出：

```text
DocumentInspectionResult
```

---

# 13. Inspector 必须结构化

不要输出一大段字符串：

```text
“PDF, 12MB, 42 pages...”
```

必须建立结构化领域对象。

例如：

```text
DocumentInspectionResult
├── identity
├── format
├── metadata
├── structure
├── statistics
├── capabilities
├── diagnostics
└── warnings
```

这样未来：

```text
UI
Batch
Search
Workflow
Export
```

都可以复用。

---

# 14. Inspector UI

默认视图建议：

```text
Overview
Metadata
Structure
Statistics
Capabilities
Warnings
```

避免一开始就设计：

```text
40 个 tabs
```

---

# 15. PDF Domain

PDF 是 M8 的主力能力。

至少建立：

```text
PdfDocument
PdfPage
PdfMetadata
PdfPageOperation
PdfPageRange
PdfMergePlan
PdfSplitPlan
PdfInspection
```

---

# 16. PDF Document Model

至少考虑：

```text
PageCount
PageSizes
PageRotation
Metadata
Encrypted
Linearized
PdfVersion
FileSize
```

只报告：

```text
library actually exposes
```

的字段。

---

# 17. PDF Metadata

可能包含：

```text
Title
Author
Subject
Keywords
Creator
Producer
CreationDate
ModificationDate
PdfVersion
Fonts
```

不要把：

```text
missing metadata
```

显示为：

```text
Unknown Author
```

除非 UI 明确区分：

```text
Not Present
Not Readable
Not Supported
```

---

# 18. PDF Merge

流程必须是：

```text
Input PDFs
 ↓
Validate
 ↓
Inspect
 ↓
Build Merge Plan
 ↓
Preview
 ↓
Execute
 ↓
Validate Output
 ↓
History
```

必须支持：

```text
multiple input PDFs
ordered merge
```

输出顺序必须：

```text
deterministic
```

---

# 19. PDF Merge Preview

Preview 必须明确：

```text
Input files
Page counts
Final page count
Output path
Potential warnings
```

例如：

```text
A.pdf — 10 pages
B.pdf — 7 pages
C.pdf — 4 pages

Output
21 pages
```

不能只显示：

```text
Ready to merge
```

---

# 20. PDF Merge 安全性

必须检测：

```text
missing file
duplicate input path
locked file
corrupt PDF
unsupported encryption
output collision
same input/output path
permission denied
disk full
```

---

# 21. PDF Split

至少支持：

```text
Split by page range
Split every N pages
Extract selected pages
```

核心领域模型：

```text
PdfPageRange
```

支持：

```text
1
1-5
7-9
12
```

必须：

```text
validate
normalize
sort
deduplicate
```

除非用户显式要求保留重复页。

---

# 22. Page Range Parser

输入：

```text
1-3,5,8-10
```

必须解析为：

```text
1
2
3
5
8
9
10
```

对：

```text
0
negative
reverse range
out of bounds
malformed token
```

必须给出结构化错误。

---

# 23. Split Every N Pages

例如：

```text
100 pages
N = 20
```

结果：

```text
1–20
21–40
41–60
61–80
81–100
```

必须保证：

```text
no missing pages
no duplicate pages
deterministic naming
```

---

# 24. PDF Reorder

建立：

```text
PdfReorderPlan
```

例如：

```text
[3,1,2,5,4]
```

必须：

```text
validate all page numbers
detect omissions
detect duplicates
preview final order
```

默认不允许：

```text
implicit page loss
```

---

# 25. PDF Rotate

支持：

```text
90°
180°
270°
```

可以针对：

```text
all pages
selected pages
page ranges
```

必须确认 PDF library 的真实语义：

```text
page rotation metadata
```

还是：

```text
physical content rotation
```

两者不能混淆。

---

# 26. PDF Rotation Preview

必须在能力允许时显示：

```text
before
after
```

如果只能展示：

```text
rotation metadata change
```

则必须明确说明。

不要假装做了视觉重渲染。

---

# 27. PDF Output Validation

任何写出的 PDF 必须至少：

```text
exists
readable
non-zero size
parseable
expected page count
```

必要时还要：

```text
expected page order
expected rotation
```

不得以：

```text
file exists
```

作为成功标准。

---

# 28. PDF Split Output Naming

命名必须：

```text
deterministic
collision-safe
```

例如：

```text
report.part-001.pdf
report.part-002.pdf
report.part-003.pdf
```

不得默认：

```text
overwrite existing
```

---

# 29. DOCX 定位

M8 的 DOCX 重点：

```text
Inspect
Metadata
Structure Statistics
```

不是：

```text
Full Edit
```

---

# 30. DOCX 本质

DOCX 必须被视为：

```text
ZIP package
+
XML parts
+
relationships
```

不要直接：

```text
read docx bytes as plain text
```

---

# 31. DOCX Inspector

至少尝试获取：

```text
Paragraph count
Heading count
Table count
Image count
Section count
Hyperlink count
Word/document properties
```

但必须基于实际解析能力。

---

# 32. DOCX Metadata

可能包含：

```text
Title
Subject
Creator
Keywords
Description
LastModifiedBy
Created
Modified
Revision
Fonts
```

只报告：

```text
actual parsed values
```

---

# 33. DOCX Structure

建议：

```text
Document
 ├── Body
 │   ├── Paragraphs
 │   ├── Tables
 │   └── Other Blocks
 ├── Headers
 ├── Footers
 ├── Sections
 └── Relationships
```

M8 不需要构建完整 Word Layout Model。

---

# 34. DOCX Statistics

可以提供：

```text
Paragraphs
Characters
Words
Tables
Images
Sections
Headings
Hyperlinks
```

统计必须标注：

```text
Exact
Approximate
Unavailable
```

尤其：

```text
Word Count
```

不能假设：

```text
simple whitespace split
```

一定等于 Word 的真实统计。

---

# 35. DOCX Embedded Images

如果 parser 能可靠获取：

```text
image count
```

则报告：

```text
Embedded Images
```

必要时：

```text
Image Type
Image Size
```

但不要为了一个 image inspector 偷偷重新实现 M6。

---

# 36. DOCX 与 M6 的边界

M8 可以：

```text
发现 DOCX 中嵌入图片
```

但不要在 M8 重新实现：

```text
Image Resize
Image Convert
Image Compress
Image Metadata pipeline
```

未来应该：

```text
Document
 ↓
Extract embedded asset
 ↓
M6
```

但这属于未来组合能力。

---

# 37. XLSX 定位

XLSX：

```text
Inspect
```

而不是：

```text
Excel Clone
```

---

# 38. XLSX Domain

建议：

```text
XlsxWorkbook
XlsxSheet
XlsxCellSummary
XlsxDimension
XlsxMetadata
```

---

# 39. XLSX Workbook Inspector

至少报告：

```text
Workbook Name
Sheet Count
Sheet Names
Visible Sheets
Hidden Sheets
Workbook Properties
Fonts
```

---

# 40. XLSX Sheet Inspector

每个 Sheet 可以报告：

```text
Name
Visibility
Used Range
Row Count
Column Count
Merged Cells
Formula Presence
Hyperlink Presence
```

必须区分：

```text
Worksheet dimension
```

与：

```text
actually populated cells
```

两者不是总能完全等价。

---

# 41. XLSX Used Range

对：

```text
A1:Z500
```

不能简单假设：

```text
500 * 26
```

就是：

```text
500*26 populated cells
```

必须明确：

```text
Range
```

与：

```text
Populated Cell Count
```

的区别。

---

# 42. XLSX Formula

M8 可以检测：

```text
Formula Present
```

但不要自己构建：

```text
Excel Formula Engine
```

例如不要自行实现：

```text
SUM
VLOOKUP
XLOOKUP
INDEX
MATCH
Pivot
Array Formula Engine
```

除非未来另立阶段。

---

# 43. XLSX Cached Values

如果 library 暴露：

```text
cached formula result
```

可以读取。

但：

> 不得声称 Weave 已重新计算 Excel 公式。

必须区分：

```text
Stored Formula
Cached Result
Recalculated Result
```

---

# 44. XLSX Hidden Sheets

必须显示：

```text
Visible
Hidden
VeryHidden
```

仅当底层格式/库能够真实区分。

不能把所有非 visible 都叫：

```text
Hidden
```

---

# 45. XLSX Data Boundaries

M5 已负责：

```text
CSV
TSV
JSON
JSONL
Data Tables
```

M8 只负责：

```text
XLSX workbook inspection
```

不要重新实现：

```text
DataDocument
DataTable
CSV Parser
```

---

# 46. PPTX 定位

PPTX：

```text
Inspect
```

不是：

```text
PowerPoint Clone
```

---

# 47. PPTX Domain

建议：

```text
PptxPresentation
PptxSlide
PptxShapeSummary
PptxMetadata
```

---

# 48. PPTX Presentation Inspector

至少报告：

```text
Slide Count
Slide Names / Titles where available
Hidden Slides
Text Presence
Image Count
Shape Count
Notes Presence
Presentation Properties
Fonts
```

---

# 49. PPTX Slide Statistics

每页可报告：

```text
Text Blocks
Images
Shapes
Tables
Charts
Notes
```

只有：

```text
parser actually identifies
```

时才能展示。

---

# 50. PPTX Text Extraction

M8 可以提供：

```text
slide text summary
```

但必须明确：

```text
text extraction
```

不等于：

```text
visual rendering equivalence
```

复杂：

```text
SmartArt
embedded charts
grouped shapes
text boxes
animations
```

可能存在支持边界。

---

# 51. PPTX Rendering

默认不要自己开发：

```text
PowerPoint renderer
```

如果没有可靠 renderer：

```text
do not promise visual preview
```

可以只展示：

```text
structured summary
```

---

# 52. Office Preview 的原则

对于：

```text
DOCX
XLSX
PPTX
```

优先：

```text
Structured Preview
```

而不是假装提供：

```text
pixel-perfect preview
```

---

# 53. PDF Preview

PDF 如果底层具备可靠渲染能力，可以：

```text
thumbnail
page preview
```

否则：

```text
page structure preview
```

也可以。

必须在 capability matrix 中真实反映。

---

# 54. Document Preview 的统一模型

建立：

```text
DocumentPreview
```

可包括：

```text
Summary
Thumbnail
Pages
Sheets
Slides
Metadata
Warnings
```

同时允许：

```text
Unavailable
```

---

# 55. Preview 与 Execute 必须共享语义

这是 M8 的硬规则：

```text
Preview
```

与：

```text
Execute
```

必须来自：

```text
同一个 Domain Operation
```

禁止：

```text
Preview implementation A
Execute implementation B
```

尤其禁止为了 UI：

```text
mock result
```

---

# 56. Generic Operation Plan

M8 必须复用：

```text
M2 OperationPlan
```

而不是再造：

```text
DocumentOperationPlan
```

作为完全独立体系。

必要时扩展：

```text
typed payload
```

但保持：

```text
common operation lifecycle
```

统一。

---

# 57. M8 PDF Operation

例如：

```text
PdfMerge
PdfSplit
PdfExtract
PdfReorder
PdfRotate
```

都应进入：

```text
OperationPlan
```

---

# 58. M8 与 M7 Batch Engine

必须复用：

```text
M7 Batch Engine
```

例如：

```text
100 PDFs
→ inspect
```

或者：

```text
20 PDFs
→ rotate 90°
```

或者：

```text
10 PDFs
→ inspect metadata
```

都应通过：

```text
BatchJob
JobPlan
Pipeline
Stage
```

执行。

---

# 59. 不要建立第二套 Document Batch Engine

禁止：

```text
DocumentBatchProcessor
DocumentBatchManager
PdfBatchManager
OfficeBatchExecutor
```

如果这些类的职责实际上已经属于：

```text
M7
```

则不得重复建设。

---

# 60. Tool Adapter

M8 应向 M7 暴露：

```text
DocumentTool
PdfTool
DocxInspector
XlsxInspector
PptxInspector
```

但必须遵守：

```text
Tool = What
Batch Engine = How
```

---

# 61. Capability Contract

M8 每个 Tool 必须声明：

```text
Input Types
Output Types
Options
Supports Preview
Supports Execute
Supports Batch
Supports Undo
Supports Metadata Preservation
Resource Constraints
```

---

# 62. Example

例如：

```text
PdfRotate
```

可以声明：

```text
Input:
PDF

Output:
PDF

Supports Preview:
Yes

Supports Batch:
Yes

Supports Undo:
Yes when output replacement is used through transaction

Requires Decode:
No / according to actual implementation

Resource Limits:
...
```

所有字段必须基于实际代码。

---

# 63. Document Operation Pipeline

标准流程：

```text
Input
 ↓
Identify
 ↓
Validate
 ↓
Inspect
 ↓
Build Plan
 ↓
Preview
 ↓
Confirm
 ↓
Execute
 ↓
Verify
 ↓
Result
 ↓
History
```

---

# 64. Read-only Inspector

Inspector 不修改原文件。

例如：

```text
PDF Inspect
DOCX Inspect
XLSX Inspect
PPTX Inspect
```

都应：

```text
read only
```

默认：

```text
No Transaction needed
No Undo needed
```

但如果被 M7 批处理，则仍然要有：

```text
Job
Progress
Cancellation
Result
```

---

# 65. Mutation Operations

以下属于 mutation：

```text
PDF Merge
PDF Split
PDF Extract
PDF Reorder
PDF Rotate
```

必须进入：

```text
Plan
Preview
Confirm
Safe Write
Transaction
History
```

---

# 66. Output-first 默认策略

默认：

```text
source.pdf
```

处理后：

```text
source.rotated.pdf
```

而不是直接：

```text
overwrite source.pdf
```

---

# 67. Source Replace

若允许：

```text
Replace Original
```

必须：

```text
explicit user action
```

并经过：

```text
SafeWrite
TOCTOU
Transaction
History
```

---

# 68. TOCTOU

必须防止：

```text
Preview
 ↓
user changes file
 ↓
Execute
```

导致：

```text
old plan
applied to new file
```

执行前必须重新验证：

```text
path
existence
size
mtime where useful
content identity where required
```

---

# 69. Input Snapshot

Operation plan 应尽可能记录：

```text
Path
Size
Modified Time
Identity
Hash when appropriate
Format
```

不要对所有大文档强制做完整 hash。

应根据成本与风险设计：

```text
cheap identity checks
→ stronger verification when necessary
```

---

# 70. Output Validation

任何 document mutation 后：

```text
output exists
output readable
output parseable
output size reasonable
operation-specific invariant satisfied
```

例如 Merge：

```text
output page count
=
sum input page counts
```

除非 PDF library 的实际语义造成特殊例外，必须解释。

---

# 71. Failure Isolation

M7 必须保证：

```text
one item failed
```

不自动意味着：

```text
whole job failed
```

例如：

```text
100 PDFs
```

其中：

```text
97 success
2 failed
1 skipped
```

应该得到：

```text
Job = CompletedWithFailures
```

而不是：

```text
Job = Success
```

也不是：

```text
Job = Total Failure
```

---

# 72. Structured Result

M8 Result 至少区分：

```text
Success
Failed
Skipped
Cancelled
```

同时提供：

```text
Created Outputs
Warnings
Diagnostics
Errors
```

---

# 73. PDF Partial Failure

例如：

```text
20 PDFs
merge
```

其中一个：

```text
corrupt
```

必须根据操作语义决定：

```text
stop-before-write
```

或者：

```text
item-level failure
```

绝不能：

```text
silently skip corrupt PDF
```

---

# 74. Composite Operations

例如：

```text
Merge A + B + C
```

这是：

```text
single logical operation
```

不是：

```text
three independent item operations
```

因此失败策略必须按：

```text
operation semantics
```

设计。

---

# 75. Transaction Semantics

必须诚实描述：

```text
atomic
best-effort
partial
non-atomic
```

不能把：

```text
multi-file operation
```

自动称为：

```text
fully atomic
```

---

# 76. Undo

能够安全撤销时：

```text
Undo
```

必须：

```text
revalidate
```

并且：

```text
never overwrite later user modifications
```

如果无法安全 undo：

```text
CanUndo = false
```

不能骗用户：

```text
Undo available
```

---

# 77. History

每个 mutation 至少记录：

```text
OperationId
Tool
Input Paths
Output Paths
Timestamp
Result Summary
Undo Capability
```

不得记录：

```text
document content
```

除非用户未来明确启用内容快照功能。

---

# 78. History Privacy

默认：

```text
No content snapshots
No document upload
No telemetry
No remote metadata analytics
```

History 主要记录：

```text
what happened
```

而不是：

```text
what was inside document
```

---

# 79. Metadata Preservation

文档处理必须考虑：

```text
metadata
```

例如：

```text
PDF author
PDF title
DOCX properties
XLSX workbook properties
PPTX properties
```

必须明确：

```text
preserved
modified
removed
unsupported
unknown
```

不能默认宣传：

```text
metadata preserved
```

除非测试证明。

---

# 80. PDF Merge Metadata

合并多个 PDF 时：

```text
Which metadata wins?
```

必须有明确策略。

至少：

```text
deterministic
documented
testable
```

例如：

```text
use output-level metadata
```

而不是：

```text
randomly inherited
```

---

# 81. PDF Page Labels / Bookmarks

第一版不强制实现复杂：

```text
Bookmarks
Page Labels
Named Destinations
Attachments
```

但如果底层库在 Merge/Split 时：

```text
preserves
drops
rewrites
```

必须进行测试并记录。

---

# 82. PDF Annotations

默认：

```text
do not edit annotations
```

如果操作可能：

```text
preserve
or
drop
```

必须真实验证并记录。

---

# 83. PDF Forms

默认：

```text
No form editing
```

对于：

```text
AcroForm
```

必须避免：

```text
silent flattening
silent data loss
```

如果无法安全处理：

```text
block operation
```

或明确：

```text
known limitation
```

---

# 84. Encrypted PDF

需要识别：

```text
Encrypted
```

如果无法操作：

```text
structured error
```

不要：

```text
retry forever
```

不要：

```text
remove password protection magically
```

加密的 DOCX / XLSX / PPTX：至少检测并如实报告 Encrypted 状态（参照加密 PDF 的处理方式），不得伪装为可解析。

---

# 85. Password Input

M8 第一版不需要建立复杂：

```text
Credential Vault
```

但必须确保：

```text
password
```

不会被：

```text
logs
history
error message
telemetry
```

泄漏。

---

# 86. Malformed Documents

以下必须有测试：

```text
truncated PDF
invalid ZIP
broken XML
invalid relationships
malformed workbook
malformed presentation
```

系统不得：

```text
panic
```

UI 不应：

```text
freeze indefinitely
```

---

# 87. ZIP Bomb / Resource Abuse

DOCX/XLSX/PPTX 本质上涉及 ZIP。

必须考虑：

```text
compressed size
uncompressed size
entry count
entry size
compression ratio
```

必要时建立：

```text
DocumentResourceLimits
```

---

# 88. XML Safety

对于 Office XML：

```text
XXE
entity expansion
malicious external reference
unexpected external resource
```

必须：

```text
disabled
sandboxed
or rejected
```

根据所选 parser 的真实行为实现。

---

# 89. External Relationships

Office 文档可能包含：

```text
external links
external relationships
embedded resources
```

M8 默认：

```text
no network fetch
```

不能因为文档声明：

```text
http://...
```

就主动联网。

---

# 90. Local-first 强制规则

文档处理：

```text
完全本地
```

禁止：

```text
upload
remote parse
remote rendering
external API
telemetry
```

---

# 91. Shell 禁止

不得通过：

```text
cmd
powershell
bash
libreoffice CLI
python subprocess
external converter
```

偷偷实现核心 document processing。

除非：

1. 经明确架构决策批准；
2. 明确属于稳定、可控、可分发的 adapter；
3. 安全、许可证、Windows 兼容性都通过；
4. 没有更合适的 native implementation。

默认优先：

```text
native Rust library
```

---

# 92. Office → PDF

Project Charter 允许：

```text
Office → PDF（条件允许时）
```

M8 对此必须非常保守。

只有当实际环境证明：

```text
可靠
可分发
许可证可接受
Windows 一致可运行
不依赖用户安装 Office
```

才考虑实现。

否则：

```text
明确 NOT SUPPORTED
```

不得依赖：

```text
“用户电脑安装了 Word”
```

作为默认产品行为。

---

# 93. 不允许隐藏系统依赖

不能：

```text
works on developer machine
```

却：

```text
fresh machine fails
```

任何系统依赖必须：

```text
detected
documented
validated
```

---

# 94. Document Scanner

可以建立：

```text
DocumentDetector
```

输入：

```text
Path
```

输出：

```text
DetectedFormat
Confidence / DetectionReason
Capability
```

检测优先级：

```text
content signature
container structure
extension
```

不要仅靠：

```text
extension
```

---

# 95. Extension Mismatch

例如：

```text
foo.pdf
```

实际是：

```text
ZIP
```

必须显示：

```text
Format mismatch
```

而不是强行当 PDF。

---

# 96. Unsupported vs Malformed

必须区分：

```text
Unsupported
```

和：

```text
Malformed
```

例如：

```text
valid PDF 2.x feature not supported
```

可能是：

```text
Unsupported
```

而：

```text
broken cross-reference table
```

可能是：

```text
Malformed
```

---

# 97. Diagnostic Model

建立：

```text
DocumentDiagnostic
```

至少：

```text
Severity
Code
Message
Context
Recoverability
```

Severity：

```text
Info
Warning
Error
```

---

# 98. 用户错误信息

用户看到：

```text
无法读取文档
```

之外，应尽量给出：

```text
原因
位置
建议
```

例如：

```text
The PDF appears encrypted and this version does not support password-protected input.
```

而不是：

```text
Unknown error
```

---

# 99. 错误不要泄漏内部细节

不要把：

```text
Rust panic
crate stack trace
absolute temp path
secret
password
```

直接展示给用户。

开发日志可以：

```text
debug details
```

但必须经过：

```text
sanitization
```

---

# 100. Document Resource Limits

必须建立可配置或至少集中定义：

```text
MaxFileSize
MaxArchiveEntries
MaxArchiveEntrySize
MaxDecompressedSize
MaxXmlSize
MaxPages
MaxSheets
MaxSlides
MaxEmbeddedAssets
MaxPreviewPixels
MaxConcurrentDocuments
```

实际值：

> **必须基于实际测试和所选库确定。**

禁止拍脑袋写：

```text
1GB
10000 pages
```

然后声称：

```text
secure
```

---

# 101. Large PDF

必须测试：

```text
100 pages
500 pages
1000 pages
```

根据实际能力。

关注：

```text
memory
latency
cancellation
output size
```

---

# 102. Large Office Documents

至少设计真实 fixture：

```text
large DOCX
large XLSX
large PPTX
```

测试：

```text
inspect
cancel
memory behavior
```

不要只用：

```text
3KB sample.docx
```

宣称：

```text
large document ready
```

---

# 103. Streaming

凡是能够：

```text
stream
```

就不要：

```text
whole-file buffer
```

但：

> 不要为了“看起来高级”而强行流式。

必须根据底层 parser 的实际 API 设计。

---

# 104. Memory Budget

至少测量：

```text
baseline
document inspect
PDF merge
PDF split
batch processing
```

并记录：

```text
peak memory
```

在：

```text
docs/PERF.md
```

中。

---

# 105. Cancellation

M8 所有长操作：

```text
PDF Merge
PDF Split
PDF Inspect large files
Batch Inspection
```

必须支持：

```text
Cancellation
```

取消不得仅仅是：

```text
UI says cancelled
```

后端必须真正停止：

```text
future work
or
safe cancellation boundary
```

---

# 106. Cancellation Semantics

必须区分：

```text
Cancelled before start
Cancelled while processing
Cancelled after output creation
```

如果取消发生在：

```text
output partially written
```

必须：

```text
cleanup
or
mark partial artifact
```

不能留下：

```text
report-final.pdf
```

却声明：

```text
Cancelled safely
```

---

# 107. Progress

M8 接入 M7 Progress。

不得自己再建立：

```text
ProgressManager2
```

或：

```text
DocumentProgressService
```

---

# 108. Progress Precision

Progress 只能报告：

```text
measurable progress
```

如果 PDF library 无法暴露内部 parse progress：

可以显示：

```text
Processing document 3 of 10
```

但不要伪造：

```text
63.7%
```

---

# 109. IPC

禁止：

```text
每个 page 一次 IPC
```

或：

```text
每个 cell 一次 IPC
```

必须：

```text
aggregate
paginate
stream bounded events
```

---

# 110. XLSX Sheet Preview

不要一次性将：

```text
1,000,000 cells
```

全部发送到 React。

必须：

```text
bounded preview
pagination
virtualization
summary first
```

---

# 111. DOCX Preview

默认优先：

```text
summary
```

如需文本：

```text
bounded extraction
```

避免：

```text
whole document dump
```

---

# 112. PPTX Preview

默认：

```text
slide summaries
```

而不是：

```text
entire XML
```

---

# 113. Document Batch Inspection

通过 M7：

```text
Input 100 documents
 ↓
Detect
 ↓
Inspect
 ↓
Aggregate result
```

UI 显示：

```text
PDF: 62
DOCX: 18
XLSX: 11
PPTX: 9
Unsupported: 3
Malformed: 1
```

这类统计必须是真实数据。

---

# 114. Batch Capability Validation

如果 Job 是：

```text
PDF Rotate
```

输入混入：

```text
DOCX
XLSX
```

不能：

```text
silently ignore
```

应明确：

```text
unsupported input
```

并遵循 M7 failure semantics。

---

# 115. Filter Stage

M7 已有：

```text
Filter
```

M8 可以定义：

```text
FormatFilter
PageCountFilter
SheetCountFilter
SlideCountFilter
SizeFilter
```

但：

> Filter 逻辑必须是 Document capability/data，而不是另建一套 batch filter engine。

---

# 116. Example Pipeline

允许：

```text
Source
 ↓
Filter: PDF
 ↓
Inspect PDF
 ↓
Rotate
 ↓
Export
```

执行方式：

```text
M7
```

不是：

```text
M8 own pipeline engine
```

---

# 117. M8 不做 Workflow Editor

不得实现：

```text
drag node
connect nodes
save graph
branch
loop
condition builder
visual workflow canvas
```

这是：

```text
M10
```

---

# 118. M8 与 M10 边界

M8：

```text
Document Tools
```

M10：

```text
Workflow Composition
```

例如：

```text
M8:
PDF Split
```

可以作为：

```text
Tool
```

但：

```text
“PDF Split → Rename → Organize → Image Convert → Export”
```

作为用户可自由设计的工作流：

```text
M10
```

---

# 119. M8 UI 信息架构

建议：

```text
Home
Documents
 ├── Inspect
 ├── PDF
 └── Office
```

PDF：

```text
Inspect
Merge
Split
Extract
Reorder
Rotate
```

Office：

```text
Inspect
```

---

# 120. 不做工具海洋

不要创建：

```text
PDF Tool 1
PDF Tool 2
PDF Tool 3
```

而应该让：

```text
Document
```

成为统一入口。

---

# 121. Quick Drop

M8 可以复用已有 Drop infrastructure。

用户拖入：

```text
report.pdf
```

系统展示：

```text
PDF
42 pages
18.4 MB
```

然后：

```text
Inspect
Merge
Split
Extract
Reorder
Rotate
```

---

# 122. Multi-selection

支持：

```text
多文档选择
```

对于：

```text
PDF Merge
```

直接进入：

```text
ordered input list
```

---

# 123. Ordering UX

用户必须能：

```text
drag reorder
```

或：

```text
move up/down
```

并且最终顺序：

```text
fully deterministic
```

---

# 124. Duplicate Input Detection

例如：

```text
A.pdf
A.pdf
```

或：

```text
same file selected twice
```

必须：

```text
detect
warn
```

不能无意生成：

```text
A + A
```

---

# 125. Same Input / Output Path

例如：

```text
input = report.pdf
output = report.pdf
```

必须：

```text
explicit replace path
```

不得默默覆盖。

---

# 126. Collision Policy

默认：

```text
No Silent Overwrite
```

支持策略可考虑：

```text
Fail
Auto-suffix
Replace Explicitly
```

实际策略必须与 M2 保持一致。

---

# 127. Temporary Files

Document processing 常常需要：

```text
temp output
```

必须确保：

```text
temp path safe
unique
inside controlled temp dir
cleanup
```

不得：

```text
写入项目仓库
写入用户 Downloads 随机位置
```

---

# 128. Crash Recovery

如果：

```text
PDF Merge
```

过程中应用崩溃：

必须依靠 M7：

```text
Journal
```

与 M2：

```text
Transaction
```

定义恢复行为。

---

# 129. Resume

不要：

```text
blind replay
```

必须验证：

```text
input unchanged
output state
job journal
operation stage
```

再决定：

```text
resume
retry
skip
rollback
restart
```

---

# 130. Output Orphan Cleanup

如果：

```text
output created
```

但：

```text
job crashes
```

必须能识别：

```text
orphan output
```

并根据安全策略：

```text
cleanup
recover
or mark pending
```

---

# 131. Test Fixtures

建立：

```text
tests/fixtures/documents/
```

建议至少：

```text
valid/
malformed/
encrypted/
large/
edge/
```

---

# 132. PDF Fixtures

至少准备：

```text
1-page PDF
multi-page PDF
rotated page PDF
metadata PDF
large-ish PDF
malformed PDF
```

如许可证允许：

```text
real-world PDFs
```

否则：

```text
synthetic fixtures
```

并明确来源。

---

# 133. DOCX Fixtures

至少：

```text
minimal.docx
metadata.docx
tables.docx
images.docx
headings.docx
multiple-sections.docx
malformed.docx
```

---

# 134. XLSX Fixtures

至少：

```text
minimal.xlsx
multiple-sheets.xlsx
hidden-sheets.xlsx
formulas.xlsx
merged-cells.xlsx
large-ish.xlsx
malformed.xlsx
```

---

# 135. PPTX Fixtures

至少：

```text
minimal.pptx
multi-slide.pptx
hidden-slide.pptx
images.pptx
notes.pptx
malformed.pptx
```

---

# 136. Golden Tests

对于：

```text
Document Inspector
PDF page order
PDF page count
metadata extraction
XLSX sheet listing
PPTX slide statistics
DOCX counts
```

建立：

```text
golden expected result
```

确保：

```text
deterministic
```

---

# 137. Determinism

相同：

```text
Input
Options
Environment constraints
```

必须尽可能得到：

```text
same Plan
same Result
same ordering
same naming
```

---

# 138. Property Tests

适合：

```text
page range parser
filename generation
ordering normalization
split partitioning
capability serialization
```

例如：

```text
split partitions cover pages exactly once
```

---

# 139. Fuzz Tests

重点：

```text
PDF page range input
malformed XML
malformed ZIP structure
metadata strings
document detector
path inputs
```

目标：

```text
no panic
no undefined behavior
no infinite loop
```

---

# 140. Fault Injection

必须测试：

```text
permission denied
input deleted
input modified
output collision
disk write failure
temp file failure
parser error
cancellation
process interruption
journal failure
history write failure
```

---

# 141. TOCTOU Integration Test

场景：

```text
Inspect
 ↓
modify input externally
 ↓
Execute
```

预期：

```text
DetectedChangedSincePlan
```

而不是：

```text
blindly process
```

---

# 142. PDF Merge Test

真实场景：

```text
A.pdf 3 pages
B.pdf 5 pages
C.pdf 2 pages
```

执行：

```text
Merge
```

验证：

```text
10 pages
```

并验证：

```text
order = A1..A3 B1..B5 C1..C2
```

---

# 143. PDF Split Test

例如：

```text
10 pages
range = 2-4,8-10
```

验证：

```text
2,3,4,8,9,10
```

---

# 144. PDF Reorder Test

例如：

```text
[4,1,3,2]
```

验证输出：

```text
4,1,3,2
```

而不是：

```text
sorted
```

---

# 145. PDF Rotate Test

验证：

```text
selected pages only
```

以及：

```text
unselected pages unchanged
```

---

# 146. DOCX Inspector Test

必须验证：

```text
known fixture
```

能够得到：

```text
paragraph count
table count
image count
metadata
```

并与：

```text
fixture facts
```

对齐。

---

# 147. XLSX Inspector Test

验证：

```text
sheet count
sheet names
hidden state
dimensions
formula presence
```

---

# 148. PPTX Inspector Test

验证：

```text
slide count
hidden state
text presence
image count
notes presence
```

---

# 149. Integration Test

必须有：

```text
real filesystem
real documents
real parser
real output
real reopen/validate
```

禁止整个 M8 只测试 mock interfaces。

---

# 150. UI Smoke Test

至少：

```text
Launch
Drop PDF
Inspect
Merge two PDFs
Preview
Execute
Show result
Open History
Undo where supported
```

以及：

```text
Drop DOCX
Inspect
```

```text
Drop XLSX
Inspect
```

```text
Drop PPTX
Inspect
```

---

# 151. Accessibility

M8 UI 至少保证：

```text
keyboard reachable
visible focus
semantic buttons
readable errors
no color-only status
```

不要在 M8 大规模建设完整 accessibility framework。

---

# 152. Internationalization

所有新增 UI 文案：

```text
must use existing i18n mechanism
```

不得：

```text
hardcode English only
```

或：

```text
hardcode Chinese only
```

---

# 153. Error UX

至少区分：

```text
Unsupported
Malformed
Permission Denied
Encrypted
Changed
Output Collision
Cancelled
Resource Limit
Unknown
```

---

# 154. Empty States

例如：

```text
No document selected
No pages selected
No output yet
No metadata available
No sheets found
No slide statistics available
```

避免：

```text
blank screen
```

---

# 155. Progress UX

显示：

```text
Current operation
Processed
Remaining
Failed
Cancelled
```

对于可计算阶段：

```text
percentage
```

对于不可计算阶段：

```text
indeterminate
```

---

# 156. Result UX

例如：

```text
Merge complete

3 files
24 pages
1 output
```

失败：

```text
2 succeeded
1 failed
```

并支持：

```text
Open output
Show details
Retry
View history
```

---

# 157. Batch Result Aggregation

M7 已定义 Job Result。

M8 不要复制：

```text
DocumentBatchResult
```

除非只是：

```text
document-specific payload
```

---

# 158. History Aggregation

M7 Job 可以拥有：

```text
one Job history entry
```

必要时：

```text
item-level details
```

但 UI 不要把：

```text
100-item job
```

渲染成：

```text
100 independent history rows
```

默认应优先：

```text
one logical operation
```

---

# 159. Reuse Semantics

History 中允许：

```text
Reuse Operation
```

例如：

```text
Repeat PDF Rotate
```

但必须基于：

```text
stored options
```

重新验证当前输入。

不要保存：

```text
stale absolute assumptions
```

---

# 160. No Content Snapshot

M8 默认不保存：

```text
whole document copy
```

仅为了：

```text
History
```

而把用户硬盘复制一遍。

---

# 161. License Audit

M8 新增每个 parser / codec / archive dependency：

必须记录：

```text
Package
Version
License
Source
Usage
Compatibility
Redistribution concerns
```

统一进入：

```text
THIRD_PARTY_LICENSES.md
```

---

# 162. Security Audit

重点：

```text
Path Safety
ZIP Safety
XML Safety
PDF Parsing
Malformed Input
Resource Limits
External References
Temporary Files
File Permissions
Output Collisions
```

---

# 163. Security 不等于“没有崩溃”

必须考虑：

```text
memory exhaustion
CPU exhaustion
disk exhaustion
decompression bombs
parser pathological input
```

---

# 164. Privacy Audit

必须确认：

```text
No upload
No telemetry
No cloud
No remote parser
No content analytics
No document content in logs
No document content in history
```

---

# 165. Logging

日志最多记录：

```text
operation id
tool
input count
output count
duration
status
error code
```

避免默认记录：

```text
full document text
full XML
full path
metadata content
GPS-like information
personal document fields
```

---

# 166. Absolute Path Logging

默认：

```text
sanitize
```

例如：

```text
C:\Users\Alice\Documents\...
```

不要轻易进入：

```text
public logs
```

---

# 167. Performance Benchmarks

至少测量：

```text
PDF inspect
PDF merge
PDF split
PDF reorder
PDF rotate
DOCX inspect
XLSX inspect
PPTX inspect
batch document inspection
```

---

# 168. Batch Benchmarks

至少测试：

```text
10 documents
100 documents
1000 documents
```

在实际数据允许时。

记录：

```text
Throughput
Peak Memory
Latency
CPU
Cancellation responsiveness
```

---

# 169. Concurrency

通过：

```text
M7 Scheduler
```

控制并发。

禁止：

```text
for each document
    spawn unlimited task
```

---

# 170. Backpressure

大批量 Office/PDF 检查必须：

```text
bounded queue
bounded IPC
bounded memory
```

---

# 171. Resource-aware Scheduling

例如：

```text
PDF parse = medium
XLSX inspect = medium
PPTX inspect = medium-high
```

实际权重必须依据：

```text
measurement
```

而不是纯主观。

---

# 172. Mixed Document Batch

测试：

```text
PDF
DOCX
XLSX
PPTX
```

混合进入：

```text
Document Inspector
```

验证：

```text
correct detection
correct capabilities
correct result aggregation
```

---

# 173. Unsupported Mixed Batch

加入：

```text
ZIP
EXE
JPG
MP4
```

验证：

```text
Unsupported / routed elsewhere
```

不得：

```text
crash
```

---

# 174. Wrong Extension Tests

例如：

```text
real.pdf → wrong.docx
real.docx → wrong.pdf
```

验证：

```text
detector behavior
```

---

# 175. Unicode Tests

文件名：

```text
中文
日本語
한국어
emoji
accented names
```

文档内容：

```text
Unicode
CJK
emoji
RTL where parser supports it
```

验证：

```text
no corruption
```

---

# 176. Long Path Tests

验证：

```text
long directory
long filename
nested document
```

必须服从：

```text
M1 Path Safety
```

---

# 177. Symlink / Junction

文档扫描不得因为：

```text
symlink
junction
reparse point
```

逃出既定 root。

继续复用：

```text
M1 Path Safety
```

---

# 178. Atomic Output

输出应优先：

```text
write temp
 ↓
flush
 ↓
validate
 ↓
atomic replace / move
```

具体是否真正 atomic：

> 必须根据 Windows/filesystem semantics 和实际实现验证。

---

# 179. Partial Output

任何失败：

```text
must not leave apparently valid but incomplete final output
```

例如：

```text
report.pdf
```

已经存在但只写了前 30%。

必须：

```text
temporary output
```

与：

```text
final output promotion
```

分离。

---

# 180. M8 Application Layer

建议建立：

```text
DocumentApplicationService
```

职责：

```text
orchestrate
validate capabilities
build plans
invoke tools
bridge UI
```

不是：

```text
parse XML itself
```

---

# 181. M8 Domain Layer

至少：

```text
DocumentIdentity
DocumentFormat
DocumentFacts
DocumentCapabilities
DocumentInspectionResult

PdfDocument
PdfPage
PdfPageRange
PdfOperation

OfficeDocument
DocxInspection
XlsxInspection
PptxInspection
```

---

# 182. M8 Adapter Layer

依赖：

```text
actual PDF library
actual Office parser
filesystem
```

Adapter 负责：

```text
library-specific behavior
```

Domain 不应被：

```text
crate-specific API
```

污染。

---

# 183. Error Boundary

底层：

```text
library error
```

必须映射到：

```text
Weave structured error
```

不要让 UI 直接看到：

```text
anyhow chain
serde XML parser internals
zip crate internals
```

---

# 184. Version Drift

M8 完成时必须记录：

```text
actual resolved dependency versions
```

不要只写：

```text
latest
```

---

# 185. Documentation

至少更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
SECURITY.md
THIRD_PARTY_LICENSES.md
docs/PERF.md
```

必要时新增：

```text
docs/documents/
```

---

# 186. ARCHITECTURE.md

必须解释：

```text
Document Domain
PDF adapter
Office adapter
M7 integration
M2 Safe Write
M2 History/Undo
resource limits
```

---

# 187. DECISIONS.md

至少记录：

```text
why these PDF libraries
why these Office parsers
why no full editor
why native Rust
why no Office installed dependency
why no OCR
why capability matrix
```

---

# 188. README

必须告诉用户：

```text
Supported Formats
Supported Operations
Known Limitations
Privacy
Local-first behavior
```

---

# 189. Progress

必须准确：

```text
M8 COMPLETE
```

或：

```text
M8 NOT COMPLETE
```

不得：

```text
Mostly complete
Nearly done
Production ready-ish
```

---

# 190. Capability Matrix

必须生成表格：

| Format | Inspect | Metadata | Structural Stats | Merge | Split | Reorder | Rotate | Preview |
|---|---|---|---|---|---|---|---|---|
| PDF | actual | actual | actual | actual | actual | actual | actual | actual/limited |
| DOCX | actual | actual | actual | N/A | N/A | N/A | N/A | structured |
| XLSX | actual | actual | actual | N/A | N/A | N/A | N/A | structured |
| PPTX | actual | actual | actual | N/A | N/A | N/A | N/A | structured |

注意：

> 表中的每个 `actual` 都必须由真实测试证明。

---

# 191. Format Support Language

UI 文案不要使用：

```text
Fully supported
```

除非有足够证据。

更安全：

```text
Supported
Limited
Partial
Unsupported
```

---

# 192. Known Limitations

必须列出真实存在的：

```text
Unsupported
Known Limitation
Partial Support
```

例如可能包括：

```text
PDF forms not editable
Office visual rendering not available
Encrypted documents require support or are blocked
Advanced Office features not interpreted
```

但：

> 只有实际存在才写。

---

# 193. No Future-feature leakage

不要在 UI 暗示：

```text
Coming soon:
OCR
AI
Convert everything
```

M8 不应承担产品宣传页面。

---

# 194. Testing Matrix

必须至少覆盖：

```text
Unit
Integration
Real filesystem
Fault injection
Concurrency
Cancellation
Persistence
Crash recovery
Performance
UI smoke
Security
Regression
```

---

# 195. M0–M7 Regression

M8 完成前必须确认：

```text
M0 PASS
M1 PASS
M2 PASS
M3 PASS
M4 PASS
M5 PASS
M6 PASS
M7 PASS
```

如果其中任一回归失败：

```text
M8 NOT COMPLETE
```

除非：

```text
documented pre-existing issue
```

并且完整披露。

---

# 196. Regression 不允许偷偷修改旧业务

M8 为修复 M1–M7 缺陷时：

必须做到：

```text
minimal change
documented reason
new regression test
```

禁止：

```text
顺手大重构
```

---

# 197. Refactoring Policy

允许：

```text
small extraction
shared interface cleanup
dependency inversion
clear duplication removal
```

禁止：

```text
rewrite architecture
```

除非真实审计证明：

```text
M8 otherwise impossible
```

并必须记录于：

```text
DECISIONS.md
```

---

# 198. Anti-patterns

严禁出现：

```text
God DocumentManager
```

包含：

```text
PDF
DOCX
XLSX
PPTX
filesystem
history
UI
batch
```

全部逻辑混在一起。

---

# 199. Anti-pattern: extension switch explosion

不要：

```rust
match extension {
    "pdf" => ...
    "docx" => ...
    "xlsx" => ...
    "pptx" => ...
}
```

遍布整个工程。

应集中：

```text
DocumentDetector
Capabilities
Tool registry
Adapters
```

---

# 200. Anti-pattern: parser leakage

UI 不应知道：

```text
zip::ZipArchive
quick_xml::Reader
PDF crate page object
```

---

# 201. Anti-pattern: duplicated safe write

M8 不得重新实现：

```text
write_to_temp
backup
atomic replace
history
undo
```

必须复用：

```text
M2
```

---

# 202. Anti-pattern: duplicated batching

M8 不得重新实现：

```text
worker pool
queue
retry
resume
journal
progress scheduler
```

必须复用：

```text
M7
```

---

# 203. Anti-pattern: fake preview

不能：

```text
显示一个缩略图
```

然后实际 execute：

```text
另一条代码路径
```

---

# 204. Anti-pattern: fake format support

不能：

```text
“打开成功”
```

实际只是：

```text
ZIP header detected
```

---

# 205. Anti-pattern: hidden network

Office/PDF parser：

```text
must not fetch remote URLs
```

---

# 206. Anti-pattern: hidden external app

不能：

```text
调用本机 Word
```

然后 UI 写：

```text
Weave supports DOCX
```

除非整个依赖模型明确设计、检测、记录并被批准。

默认禁止。

---

# 207. Recommended implementation order

硬约束：

> **每个工具必须先写 Specification 与 Fixture（含期望输出），再写实现；禁止先 UI 后逻辑（charter #46：Specification → Test Fixture → Implementation → Integration → UI → Audit）。**

按以下顺序：

```text
1. Baseline Audit
2. Dependency Audit
3. Document Domain
4. Format Detection
5. Document Inspector
6. PDF Inspect
7. PDF Page Operations
8. Safe Output / Verification
9. DOCX Inspector
10. XLSX Inspector
11. PPTX Inspector
12. M7 Integration
13. UI
14. Tests
15. Performance
16. Security
17. Documentation
18. Final Audit
```

---

# 208. Stage 1 — Baseline Audit

先只检查：

```text
repository
architecture
M7
dependencies
tests
```

不要立即改代码。

输出：

```text
A. Repository
B. Architecture
C. Existing Document-related code
D. Reusable infrastructure
E. Missing infrastructure
F. Dependencies
G. Risks
H. Recommended minimal changes
```

---

# 209. Stage 2 — Domain

建立：

```text
DocumentIdentity
DocumentFormat
DocumentCapabilities
DocumentFacts
DocumentInspectionResult
```

先让：

```text
PDF
DOCX
XLSX
PPTX
```

拥有统一语言。

---

# 210. Stage 3 — Detection

真实识别：

```text
content
container
extension
```

处理：

```text
unknown
unsupported
malformed
mismatch
```

---

# 211. Stage 4 — PDF

先：

```text
Inspect
```

再：

```text
Merge
Split
Extract
Reorder
Rotate
```

每一步都有：

```text
plan
preview
execute
verify
```

---

# 212. Stage 5 — Office Inspection

按：

```text
DOCX
XLSX
PPTX
```

逐个建立可靠 Inspector。

不要三个同时写成：

```text
巨大 generic parser
```

---

# 213. Stage 6 — M7 Integration

把：

```text
Document Tool
```

注册进：

```text
M7 Tool Registry
```

验证：

```text
single execution
batch execution
progress
cancellation
failure isolation
retry
resume
journal
```

---

# 214. Stage 7 — UI

UI 只做：

```text
Document Selection
Inspection Display
Operation Configuration
Preview
Execution
Result
History
```

---

# 215. Stage 8 — Test

从：

```text
unit
```

到：

```text
real fixtures
```

再到：

```text
real filesystem
```

再到：

```text
batch
fault
recovery
```

---

# 216. Stage 9 — Performance

必须测：

```text
inspect
transform
batch
cancel
resume
```

---

# 217. Stage 10 — Final Audit

最终只允许：

```text
Bug Fix
Release Hygiene
Performance
Test
Security
Privacy
Dependency
License
UX corrections
```

禁止：

```text
large new feature
```

---

# 218. Final Audit A — Architecture

确认：

```text
[ ] UI never accesses filesystem directly
[ ] Application layer orchestrates
[ ] Domain is format-aware but library-agnostic
[ ] Adapters isolate external libraries
[ ] M7 owns batching
[ ] M2 owns safe write/history/undo
```

---

# 219. Final Audit B — PDF

确认：

```text
[ ] Inspect works
[ ] Merge works
[ ] Split works
[ ] Extract works
[ ] Reorder works
[ ] Rotate works
[ ] Output validation works
[ ] malformed handling works
[ ] collision handling works
[ ] cancellation works
```

---

# 220. Final Audit C — Office

确认：

```text
[ ] DOCX inspection real
[ ] XLSX inspection real
[ ] PPTX inspection real
[ ] metadata claims verified
[ ] statistics claims verified
[ ] capability matrix accurate
[ ] unsupported cases honest
```

---

# 221. Final Audit D — Preview

确认：

```text
[ ] Preview uses same domain semantics
[ ] Preview not mock
[ ] Preview clearly states limitations
[ ] output path visible
[ ] expected result visible
```

---

# 222. Final Audit E — Safety

确认：

```text
[ ] Path Safety
[ ] TOCTOU
[ ] No silent overwrite
[ ] Atomic / validated output
[ ] Temp cleanup
[ ] Resource limits
[ ] ZIP safety
[ ] XML safety
[ ] PDF parsing safety
```

---

# 223. Final Audit F — Batch

确认：

```text
[ ] M7 reused
[ ] bounded concurrency
[ ] bounded queue
[ ] failure isolation
[ ] retry semantics
[ ] resume semantics
[ ] journal
[ ] cancellation
[ ] progress
```

---

# 224. Final Audit G — History / Undo

确认：

```text
[ ] mutation operations recorded
[ ] history is local
[ ] history has no document content
[ ] undo revalidates state
[ ] later user changes are protected
```

---

# 225. Final Audit H — Performance

必须输出：

```text
docs/PERF.md
```

至少记录：

```text
Environment
Fixture
Operation
Input Size
Duration
Peak Memory
Concurrency
Result
```

不允许：

```text
fake benchmark
```

---

# 226. Final Audit I — Test

至少执行：

```text
cargo test
cargo fmt --check
cargo clippy
frontend test
frontend lint
frontend typecheck
license check
Tauri build
```

具体命令必须根据真实项目脚本调整。

不得假设脚本名。

以上全部为无条件必过项（charter #45），不允许有条件跳过。

---

# 227. Final Audit J — Build

必须验证：

```text
clean build
```

最好再验证：

```text
fresh-ish environment
```

确保不存在：

```text
developer-machine-only dependency
```

---

# 228. Final Audit K — Security

必须输出：

```text
PASS
FAIL
KNOWN LIMITATION
```

至少检查：

```text
malformed files
resource exhaustion
ZIP/XML abuse
path traversal
temp files
external links
parser crashes
```

---

# 229. Final Audit L — Privacy

确认：

```text
[ ] no telemetry
[ ] no upload
[ ] no cloud
[ ] no remote parser
[ ] no document content analytics
[ ] no document content in logs
```

---

# 230. Final Audit M — License

确认：

```text
all dependencies documented
licenses compatible
redistribution obligations understood
THIRD_PARTY_LICENSES.md updated
```

---

# 231. Final Audit N — UX

检查：

```text
Drag & Drop
Inspection
Preview
Progress
Cancellation
Error UX
Result UX
History
Undo where applicable
```

---

# 232. Final Audit O — Regression

必须确认：

```text
M0 PASS
M1 PASS
M2 PASS
M3 PASS
M4 PASS
M5 PASS
M6 PASS
M7 PASS
```

---

# 233. Final Audit P — Git Hygiene

确认：

```text
no temp files
no generated junk
no binaries
no local absolute paths
no private fixtures
no secrets
no accidental databases
no debug artifacts
```

并检查：

```text
git status
git diff
git diff --cached
```

---

# 234. M8 Acceptance Criteria

只有以下全部满足：

```text
[ ] Document domain exists
[ ] Format detection exists
[ ] Capability model exists
[ ] PDF inspector works
[ ] PDF merge works
[ ] PDF split works
[ ] PDF extract works
[ ] PDF reorder works
[ ] PDF rotate works
[ ] PDF output validation works
[ ] DOCX inspector works
[ ] XLSX inspector works
[ ] PPTX inspector works
[ ] Real fixtures exist
[ ] Real filesystem integration exists
[ ] Preview works
[ ] Preview is not mock
[ ] Safe Write reused
[ ] TOCTOU handled
[ ] History integrated
[ ] Undo integrated where applicable
[ ] M7 Batch Engine reused
[ ] Cancellation works
[ ] Progress works
[ ] Failure isolation works
[ ] Retry works where safe
[ ] Resume works where safe
[ ] Journal integrated
[ ] Resource limits exist
[ ] Security audit passed
[ ] Privacy audit passed
[ ] License audit passed
[ ] Performance measured
[ ] UI smoke passed
[ ] M0–M7 regression passed
[ ] Documentation updated
[ ] Git hygiene passed
```

全部满足之后：

```text
M8 COMPLETE
```

---

# 235. M8 不算完成的情况

以下任意一项存在：

```text
PDF operation is fake
DOCX/XLSX/PPTX support is extension-only
Preview differs semantically from Execute
M8 contains a second batch engine
M8 contains a second history/undo system
Unsafe overwrite
TOCTOU ignored
Malformed document crashes app
ZIP bomb has no resource limits
XML external entity behavior unsafe
Office processing depends silently on installed Office
UI processes documents directly
No real filesystem integration
No real document fixtures
No output validation
Cancellation is UI-only
Progress is fake
History stores full document contents
Security claims are unverified
License information is missing
M7 regression fails
```

则：

```text
M8 NOT COMPLETE
```

---

# 236. M8 Completion Report

最终必须输出正式报告：

```text
# M8 Documents Completion Audit
```

至少包括：

```text
1. Baseline
2. Architecture
3. Implemented Features
4. PDF Capability Matrix
5. DOCX Capability Matrix
6. XLSX Capability Matrix
7. PPTX Capability Matrix
8. M7 Integration
9. Safe Write / History / Undo
10. Preview Semantics
11. Tests
12. Fault Injection
13. Performance
14. Security
15. Privacy
16. License
17. Known Limitations
18. M8 → M9 Handoff
19. M8 → M10 Handoff
20. Final Status
```

---

# 237. M8 → M9 Handoff

必须明确：

M9 可以复用：

```text
DocumentDetector
DocumentInspector
DocumentCapabilities
M7 Batch Engine
Path Safety
Safe Write
History
Undo
Diagnostics
```

但：

```text
M9
```

将主要负责：

```text
Hash
Base64
UUID
Timestamp
URL Encode
Regex Tester
Checksum
Color Converter
```

不要让 M9 偷偷继续膨胀成：

```text
Document Mega Module
```

---

# 238. M8 → M10 Handoff

M8 提供：

```text
Document Tools
PDF Tools
Office Inspectors
Capabilities
Structured Operations
Batch-compatible Tools
```

M10 再负责：

```text
Input
 ↓
Filter
 ↓
Tool
 ↓
Tool
 ↓
Export

Workflow
```

M10 v1 为线性管道（Input → Filter → Tool → Tool → Export），以 M10 提示词为准；DAG / 条件 / 循环不在 V1 范围。

M8 不提前实现这些。

---

# 239. M8 的终极目标

M8 完成之后：

```text
ifuyo Weave
```

应该从：

```text
File
Text
Data
Image
Batch
```

自然扩展到：

```text
Documents
```

完整结构：

```text
                           ifuyo Weave
                                │
               ┌────────────────┼────────────────┐
               ▼                ▼                ▼
             Files             Data          Documents
               │                │                │
               ▼                ▼                ▼
             Rename           CSV/JSON          PDF
             Organize         Cleaner            DOCX
             Duplicate        Convert            XLSX
                               Inspect            PPTX
               │                │                │
               └────────────────┼────────────────┘
                                ▼
                         M7 Batch Engine
                                │
                 ┌──────────────┼──────────────┐
                 ▼              ▼              ▼
              Planner       Scheduler        Journal
                 │              │              │
              Preview        Execute         Recovery
                 │              │              │
                 └──────────────┼──────────────┘
                                ▼
                       Safe Write / History
```

---

# 240. M8 产品闭环

用户最终感受到的是：

```text
Drop document
      ↓
Understand
      ↓
Inspect
      ↓
Choose operation
      ↓
Preview
      ↓
Confirm
      ↓
Execute
      ↓
Progress
      ↓
Verify
      ↓
Result
      ↓
History
      ↓
Undo / Reuse
```

而不是：

```text
Select file
 ↓
Magic happens
 ↓
Hope nothing broke
```

---

# 241. M8 核心工程原则

始终优先：

```text
Truth over convenience
```

```text
Actual capability over marketing
```

```text
Preview over surprise
```

```text
Safe output over destructive speed
```

```text
Reuse over duplication
```

```text
Boundaries over feature count
```

```text
Real tests over green mocks
```

---

# 242. M8 最终架构原则

牢记：

```text
M1
=
Filesystem Truth

M2
=
Safe Mutation

M3
=
Duplicate Semantics

M4
=
Text Semantics

M5
=
Data Semantics

M6
=
Image Semantics

M7
=
Reliable Execution

M8
=
Document Semantics
```

因此：

> **M8 的核心不是“支持多少种 Office 功能”，而是把文档作为一种可靠、可检查、可处理、可批量执行的真实输入类型，接入 Weave 已经建立的统一执行体系。**

---

# 243. 最终执行要求

现在开始工作时，严格遵循：

```text
AUDIT
→
CLASSIFY
→
PLAN
→
MINIMAL IMPLEMENTATION
→
TEST
→
FAULT TEST
→
PERFORMANCE TEST
→
SECURITY AUDIT
→
REGRESSION
→
DOCUMENT
→
FINAL AUDIT
```

不要跳过：

```text
Baseline Audit
```

不要：

```text
猜测已有代码
```

不要：

```text
为了赶进度降低真实能力验证
```

不要：

```text
因为某个 library 不好用就偷偷改架构
```

不要：

```text
用 mock 替代真实 document processing
```

不要：

```text
把 limitation 藏起来
```

不要：

```text
输出没有证据支持的 COMPLETE
```

最终只能：

```text
M8 COMPLETE
```

或者：

```text
M8 NOT COMPLETE
```

---

# 244. Final Report 输出要求

最终回复必须直接给出：

```text
# M8 Documents Final Audit
```

并按以下结构：

```text
A. Baseline
B. Implemented Architecture
C. PDF
D. DOCX
E. XLSX
F. PPTX
G. Preview
H. Batch Engine
I. Transaction / History / Undo
J. Safety
K. Security
L. Privacy
M. Performance
N. Tests
O. Regression
P. Known Limitations
Q. M9 Handoff
R. M10 Handoff
S. Final Status
```

每个关键结论都必须尽量给：

```text
FACT
Evidence
Command
Test
File
Commit
```

没有证据时写：

```text
NOT VERIFIED
```

而不是：

```text
PASS
```

最终目标：

> **让 ifuyo Weave 在 M8 结束后真正拥有一套可靠的本地文档处理能力，同时没有把自己变成另一个 Office、Acrobat 或 Workflow 产品。**