# ifuyo Weave — M8 Documents 完整开发提示词（上）

> 本文件是原《ifuyo Weave — M8 Documents 完整开发提示词》的 **第 1/2 部分**，
> 覆盖原第 0–118 节：总体硬边界与明确禁止、产品原则与核心定义、架构位置与审计优先、依赖选择与 capability-first、DocumentIdentity/Format/Facts 与 Inspector、PDF/DOCX/XLSX/PPTX 各领域规范、Preview 统一模型、与 M7 Batch Engine 的集成、Pipeline 与变更操作、TOCTOU/失败隔离/Undo/History、加密与恶意文档防护、错误诊断与资源限制、取消/进度/IPC 等实现规范。
> **必须与（下）一起阅读执行**：Test Fixture 与测试矩阵、UI/UX 与 a11y/i18n、审计与性能基准、文档、Final Audit、验收标准与提交纪律、M9/M10 交接条款均在（下）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---

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

