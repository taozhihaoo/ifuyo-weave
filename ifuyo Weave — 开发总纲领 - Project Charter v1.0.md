# ifuyo Weave
## 开发总纲领 / Project Charter v1.0

> **核心定位：**
>
> **Weave 是一个 local-first 的通用桌面工具集，用一组高质量的小工具，解决文件、文本、数据、文档、媒体与批处理中的高频杂事。**
>
> 它不追求“功能最多”，而追求：
>
> **打开就能用、拖进去就能处理、批量操作真正省时间、结果透明且可撤销。**

---

# 1. 产品定位

## 1.1 Weave 解决的问题

电脑上的大量工作不是复杂工作，而是：

- 一批文件想统一重命名
- 一堆图片想压缩、转换、整理
- CSV / JSON 想清洗、转换、查看
- 文本想格式化、比较、提取
- 文件夹想找重复文件
- 一堆文件想批量处理
- 两份文档想比较
- JSON / XML / YAML 想互转
- 某个目录想快速统计和分析
- 一批文件想按规则整理
- 某个临时任务不值得安装一个完整的软件

Weave 的目标就是：

> **把这些“小麻烦”集中到一个干净的软件里。**

---

# 2. 产品哲学

产品原则按以下顺序优先。

## 2.1 Local-first

默认：

```text
文件
 ↓
本地处理
 ↓
本地结果
```

不要求账号。

不要求云端。

不要求上传。

不默认联网。

---

## 2.2 Utility-first

Weave 首先是工具。

不是：

- 社交软件
- SaaS
- 云盘
- AI 聊天工具
- 工作流平台
- 项目管理软件

视觉可以精致，但：

> **不能为了视觉牺牲效率。**

---

## 2.3 Batch-first

很多工具必须支持：

```text
1 个文件
+
100 个文件
+
10,000 个文件
```

用户应该能够：

```text
拖一个文件
拖整个目录
选择多个文件
复制路径
粘贴列表
```

然后统一处理。

---

## 2.4 Preview-first

所有具有潜在破坏性的操作：

```text
Analyze
 ↓
Preview
 ↓
Confirm
 ↓
Apply
```

默认：

> **先告诉用户将发生什么，再真正修改文件。**

---

## 2.5 Reversible-first

能撤销就必须支持撤销。

不能真正撤销的操作：

- 明确警告
- 显示目标
- 显示数量
- 显示覆盖行为
- 提供备份/复制选项

禁止：

> **静默覆盖用户原始文件。**

---

## 2.6 Composable

Weave 的工具不是一堆孤岛。

目标是最终可以形成：

```text
输入
 ↓
分析
 ↓
过滤
 ↓
处理
 ↓
转换
 ↓
输出
```

例如：

```text
选择文件
→ 重命名
→ 转换格式
→ 压缩
→ 输出到新目录
```

---

# 3. Weave 与 ifuyo Prism 的边界

必须明确：

```text
Prism
=
理解软件项目

Weave
=
处理文件、资料与本地工作
```

Prism 关心：

```text
Code
Git
Architecture
Dependency
Health
Verification
Context
```

Weave 关心：

```text
Files
Folders
Text
Data
Documents
Images
Media
Batch Operations
```

---

## 3.1 不允许功能漂移

以下功能原则上归 Prism：

- Git 分析
- 代码结构分析
- Symbol Graph
- Architecture Graph
- Code Health
- Build / Test
- AI Context
- Project Risk

以下功能归 Weave：

- 文件整理
- 批量重命名
- 文件转换
- CSV / JSON 数据处理
- 文本处理
- 图片处理
- PDF / 文档处理
- 压缩 / 解压
- 重复文件
- 批量工作流

---

# 4. 产品结构

Weave 第一版划分为：

```text
Weave
├── Files
├── Text
├── Data
├── Documents
├── Media
├── Batch
└── Utilities
```

其中：

```text
Files
=
文件与目录

Text
=
文本

Data
=
结构化数据

Documents
=
PDF / Office / 文档

Media
=
图片 / 音视频

Batch
=
批量处理与流水线

Utilities
=
小型高频工具
```

---

# 5. Files

## 5.1 File Rename

批量重命名。

支持：

```text
Prefix
Suffix
Replace
Regex
Sequence
Date
Counter
Case Conversion
Extension
Template
```

例如：

```text
IMG_001.jpg
IMG_002.jpg
IMG_003.jpg

→

vacation-001.jpg
vacation-002.jpg
vacation-003.jpg
```

必须支持：

```text
Preview
Collision Detection
Undo
Dry Run
```

---

## 5.2 File Organizer

按规则整理文件：

```text
Extension
Date
Size
Pattern
Name
Folder
```

例如：

```text
Downloads/
├── image/
├── document/
├── archive/
├── video/
└── other/
```

---

## 5.3 Duplicate Finder

查找：

```text
Exact Duplicate
Same Size
Hash Match
Near Duplicate（后期）
```

第一阶段只实现：

> **可靠的 Exact Duplicate。**

算法：

```text
size
 ↓
partial hash
 ↓
full hash
```

避免无意义地读取所有大文件。

---

## 5.4 File Inspector

显示：

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
Encoding（文本）
Metadata
```

支持：

> **一眼看懂“这个文件到底是什么”。**

---

## 5.5 Directory Analyzer

显示：

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

输出：

```text
Tree
Table
Chart
Report
```

---

# 6. Text

## 6.1 Formatter

至少支持：

```text
JSON
XML
YAML
SQL
JavaScript
CSS
Markdown
```

功能：

```text
Format
Minify
Validate
Sort
Normalize
```

---

## 6.2 Text Compare

两个文本：

```text
A
vs
B
```

显示：

```text
Added
Removed
Changed
Moved
```

支持：

```text
Side-by-side
Unified Diff
Whitespace Ignore
Case Ignore
```

---

## 6.3 Text Extractor

从文本中提取：

```text
URLs
Emails
File Paths
Numbers
IPs
JSON
Markdown Links
```

支持正则模式。

---

## 6.4 Text Transformer

高频处理：

```text
Trim
Deduplicate Lines
Sort Lines
Add Prefix/Suffix
Case Conversion
Line Numbering
Find/Replace
Regex Replace
```

所有操作：

> **必须可以在几秒内完成。**

---

# 7. Data

## 7.1 CSV Studio

这是 Weave 的重点工具之一。

支持：

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

以及：

```text
Deduplicate
Trim
Normalize
Split Column
Merge Column
Fill Empty
Find/Replace
```

---

## 7.2 CSV Cleaning

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

例如：

```text
空白字符清理
大小写统一
空值处理
重复行删除
日期格式统一
数字格式统一
异常值发现
```

---

## 7.3 JSON / CSV / TSV Conversion

支持：

```text
JSON → CSV
CSV → JSON
CSV → TSV
TSV → CSV
JSONL → JSON
JSON → JSONL
```

必须处理：

- nested object
- nested array
- null
- encoding
- delimiter
- quoting

不能因为 1 个异常字段直接崩溃。

---

## 7.4 Data Inspector

用户拖入：

```text
.csv
.json
.jsonl
.tsv
```

Weave 自动判断：

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

# 8. Documents

Documents 模块强调：

> **处理，而不是成为 Office。**

---

## 8.1 PDF Utilities

优先：

```text
Merge
Split
Extract Pages
Reorder
Rotate
Metadata Inspect
Page Count
File Size
```

后期：

```text
Images → PDF
PDF → Images
Text Extraction
```

---

## 8.2 Document Inspector

检测：

```text
PDF
DOCX
XLSX
PPTX
TXT
Markdown
```

展示：

```text
File Type
Size
Pages / Sheets / Slides
Metadata
Embedded Images
Fonts
Creation / Modification Metadata
```

---

## 8.3 Office Utilities

第一阶段避免做复杂编辑器。

先支持：

```text
DOCX Metadata
XLSX Inspect
PPTX Inspect
Office → PDF（条件允许时）
```

不把 Word / Excel / PowerPoint 完整编辑能力塞进 Weave。

---

# 9. Media

Media 的原则：

> **常用、轻量、明确，不做视频剪辑软件。**

---

## 9.1 Image Utilities

优先：

```text
Resize
Compress
Convert
Crop
Rotate
Strip Metadata
Rename
```

格式：

```text
PNG
JPEG
WebP
GIF
BMP
TIFF
```

---

## 9.2 Image Batch Processor

支持：

```text
100 张图片
 ↓
统一尺寸
 ↓
统一格式
 ↓
统一命名
 ↓
输出目录
```

必须实时显示：

```text
Processed
Remaining
Failed
Output Size
```

---

## 9.3 Image Metadata

显示：

```text
EXIF
Dimensions
Color Space
DPI
Creation Time
Camera
GPS（如存在）
```

支持：

> **一键删除隐私元数据。**

---

## 9.4 Audio / Video

仅提供轻量级工具：

```text
Media Info
Duration
Codec
Resolution
Bitrate
Audio Channels
Frame Rate
Container
```

转换、压缩等能力作为后续 [B]。

不得让 Weave 演化成完整视频编辑器。

---

# 10. Batch Engine

这是 Weave 最重要的底层能力之一。

所有工具尽量共享：

```text
Batch Engine
```

而不是每个工具自己实现批处理。

---

## 10.1 Batch Job

统一模型：

```text
Job
├── Input
├── Operation
├── Options
├── Preview
├── Progress
├── Result
└── Rollback
```

---

## 10.2 Pipeline

未来支持：

```text
Input
 ↓
Filter
 ↓
Transform
 ↓
Transform
 ↓
Export
```

例如：

```text
Folder
 ↓
Images only
 ↓
Resize 1920px
 ↓
Convert WebP
 ↓
Strip EXIF
 ↓
Rename
 ↓
output/
```

---

## 10.3 Failure Handling

1000 个文件中：

```text
997 Success
3 Failed
```

不能：

> 997 个成功后整个 Job 被一个失败打断。

应该：

```text
Success
Failed
Skipped
Cancelled
```

分别统计。

失败项可重新运行。

---

# 11. Preview System

Weave 的所有破坏性工具必须尽可能支持：

```text
Before
After
```

例如重命名：

```text
old_name.png
→
new_name.png
```

整理：

```text
Downloads/a.zip
→
Downloads/archive/a.zip
```

图片：

```text
3840×2160 / 8.2MB
→
1920×1080 / 1.6MB
```

CSV：

```text
Before rows: 10240
After rows: 9842
Removed duplicates: 398
```

---

# 12. Undo / Transaction

Weave 应该设计统一的：

```text
OperationTransaction
```

至少保存：

```text
Original Path
New Path
Original Metadata
Generated File
Changed File
Operation Type
Timestamp
```

优先支持：

```text
Rename Undo
Move Undo
Copy Cleanup
Generated Output Tracking
```

对无法安全回滚的操作明确标记：

```text
Not Reversible
```

---

# 13. Workspace

Weave 不强迫用户建立 Project。

用户可以直接：

```text
Open App
 ↓
Drop Files
 ↓
Do Something
 ↓
Done
```

同时提供：

```text
Workspace
```

让连续任务可以保留：

```text
Input Files
Rules
Recent Operations
Output Directory
```

---

# 14. Recent Operations

首页应该显示最近操作：

```text
Today
├── Rename 83 files
├── Clean CSV
├── Convert 21 images
└── Merge 5 PDFs
```

用户能够：

```text
Open Again
Repeat
Undo
Reveal Output
```

---

# 15. Search

Weave 工具越来越多之后，不能让用户翻目录。

因此提供：

```text
Ctrl + K
```

搜索：

```text
Rename
CSV
JSON
PDF
Image
Duplicate
Compare
Compress
Metadata
```

命令面板也是：

> **Weave 的核心交互入口。**

---

# 16. Quick Drop

用户把文件拖到 Weave 窗口时：

系统根据输入自动推荐工具。

例如：

```text
example.csv
```

出现：

```text
Open CSV
Clean CSV
Convert CSV
Inspect
```

例如：

```text
100 JPG files
```

出现：

```text
Resize
Compress
Convert
Rename
Strip Metadata
```

目标：

> **用户不用先想“工具叫什么”，只需要把东西扔进来。**

---

# 17. 文件安全

Weave 是直接处理真实文件的软件。

因此安全优先级极高。

必须：

```text
Path Validation
Traversal Protection
Symlink Handling
Permission Errors
Collision Detection
Read-only Detection
Locked File Detection
```

不得：

- 随意删除目录
- 静默覆盖文件
- 修改用户未选择的目录
- 修改系统目录
- 默认批量永久删除

---

# 18. 删除策略

删除属于高风险操作。

默认：

```text
Move to Recycle Bin
```

而不是：

```text
permanent delete
```

永久删除必须二次确认。

Duplicate Finder 默认行为：

```text
Find
 ↓
Select
 ↓
Review
 ↓
Recycle
```

而不是自动删除。

---

# 19. Privacy

Weave 默认：

```text
No Telemetry
No Cloud
No Account
No Upload
```

本地文件内容不自动发送到任何服务器。

尤其是：

```text
CSV
Excel
PDF
Images
Documents
```

默认：

> **100% 本地处理。**

---

# 20. AI 原则

Weave 可以具有 AI 能力，但 AI：

> **绝不能成为普通工具运行的必要条件。**

例如：

```text
CSV Clean
```

不应该要求 AI。

```text
JSON Format
```

不应该要求 AI。

```text
Rename
```

不应该要求 AI。

AI 可以作为增强功能：

```text
自然语言批处理
智能规则生成
数据异常解释
文件分类建议
```

但核心操作仍由确定性代码执行。

---

# 21. 自然语言操作

未来可以支持：

> “把这个文件夹里的图片都压到 2MB 以下，然后按拍摄日期重新命名。”

AI 应该生成：

```text
Plan
├── Filter: image
├── Compress: <= 2MB
├── Sort: captureDate
└── Rename: YYYY-MM-DD-###
```

然后：

```text
Preview
 ↓
User Confirm
 ↓
Deterministic Execution
```

禁止：

> AI 直接获得无限文件系统写权限。

---

# 22. 技术架构

推荐：

```text
Tauri 2
├── React
├── TypeScript
├── Vite
└── Rust
```

核心：

```text
Rust
├── weave-core
├── weave-files
├── weave-text
├── weave-data
├── weave-documents
├── weave-media
├── weave-batch
├── weave-history
├── weave-search
└── weave-testkit
```

前端：

```text
ui/
├── components
├── features
├── pages
├── stores
├── commands
├── design
└── generated
```

---

# 23. 架构原则

必须：

```text
UI
 ↓
Application Layer
 ↓
Domain / Core
 ↓
Adapters
 ↓
OS / File System
```

禁止：

```text
React Component
 ↓
直接操作文件系统
```

---

# 24. Tool Adapter

每一个工具尽量实现统一接口：

```text
Tool
├── id
├── category
├── inputTypes
├── options
├── preview()
├── execute()
├── canUndo()
└── undo()
```

例如：

```text
RenameTool
ImageResizeTool
CsvCleanTool
PdfMergeTool
DuplicateFinderTool
JsonFormatterTool
```

这样未来增加工具不会污染 Core。

---

# 25. Result Model

所有工具统一返回：

```text
OperationResult
├── success
├── processed
├── skipped
├── failed
├── warnings
├── outputs
└── duration
```

错误统一：

```text
Error
├── Code
├── Message
├── Location
├── Recoverability
└── Suggestion
```

禁止只返回：

```text
Something went wrong.
```

---

# 26. Progress System

批量操作必须显示：

```text
Current
Total
Percentage
Rate
ETA
Failed
```

支持：

```text
Pause
Resume
Cancel
Retry Failed
```

---

# 27. Encoding Strategy

Weave 必须认真处理编码问题。

至少识别：

```text
UTF-8
UTF-8 BOM
UTF-16 LE
UTF-16 BE
GB18030 / GBK（Windows 中文环境常见）
Latin-1 fallback
```

文本工具不得因为一个非 UTF-8 文件直接崩溃。

转换时显示：

```text
Detected Encoding
Target Encoding
```

---

# 28. Large File Strategy

不得把所有文件全部读进内存。

对于：

```text
100MB
1GB
10GB
```

的大文件，应优先采用：

```text
Streaming
Chunking
Memory Limits
Progressive Processing
```

尤其：

```text
CSV
JSONL
Text
Hash
```

必须支持流式处理。

---

# 29. 性能原则

Weave 的体验应该是：

> **工具启动很快，操作马上开始。**

原则：

```text
Lazy Initialization
Streaming
Parallel Processing
Cancellation
Incremental Preview
Background Workers
```

不能为了一个小工具把所有运行时一次性加载。

---

# 30. UI 设计

Weave 应该继承 ifuyo 的整体气质：

```text
干净
克制
现代
有一点情绪
```

但不能复制 Aura 的音乐视觉。

---

## 30.1 UI 层级

推荐：

```text
Home
├── Quick Actions
├── Recent
├── Favorites
└── Drop Zone

Tools
├── Files
├── Text
├── Data
├── Documents
├── Media
└── Utilities
```

---

## 30.2 首页

核心视觉：

> **一个大的“把东西拖进来”区域。**

下面：

```text
Quick Tools
Recent
Favorites
```

不要做传统软件那种：

> 50 个小图标铺满整个首页。

---

# 31. 视觉原则

```text
一个页面一个焦点
```

避免：

- 重阴影
- 过度毛玻璃
- 大量彩色按钮
- 密集表格边框
- 不必要动画
- 大量渐变
- 为了“科技感”牺牲可读性

工具页面应该明显：

> **比 Prism 更实用，比 Aura 更中性。**

---

# 32. 快捷键

统一：

```text
Ctrl+K
```

命令面板。

基础：

```text
Ctrl+O
Ctrl+S
Ctrl+Z
Ctrl+Shift+Z
Delete
Escape
Enter
```

批处理：

```text
Ctrl+Enter
```

执行。

---

# 33. Drag & Drop

Weave 的核心交互之一。

必须支持：

```text
File → Window
Folder → Window
Multiple Files → Window
Text → Input
URL → Input（仅工具实际支持时）
```

拖入后自动识别类型。

---

# 34. 国际化

首发：

```text
zh-CN
en
```

所有 UI 文案资源化。

禁止业务代码散落：

```text
"开始处理"
"文件错误"
```

---

# 35. Accessibility

必须支持：

```text
Keyboard Navigation
Focus Visible
Screen Reader Labels
Dialog Escape
High Contrast Compatibility
90%~140% UI Scaling
Reduced Motion
```

---

# 36. Logging

日志：

```text
%APPDATA%/ifuyo/Weave/logs
```

记录：

```text
Operation
Error
Warning
Performance
```

禁止：

```text
File Content
Passwords
Tokens
Sensitive Data
```

进入日志。

---

# 37. Settings

设置必须非常克制。

建议：

```text
General
Appearance
Files
Privacy
Advanced
```

不要把每一个工具的几十个参数塞进全局设置。

工具参数：

> **跟随工具本身。**

---

# 38. Storage

Weave 可以使用 SQLite 保存：

```text
Recent Operations
Favorites
Tool Settings
History
Workspace
```

但：

> **删除数据库不能影响用户文件。**

---

# 39. 项目数据原则

任何内部数据库只能保存：

```text
Metadata
Configuration
History
Indexes
```

不应该偷偷复制整个用户文件库。

---

# 40. Dependency Strategy

总体原则：

> **少依赖、成熟依赖、许可证清晰、能替换。**

优先：

```text
MIT
Apache-2.0
BSD
ISC
Zlib
CC0
```

对于 GPL / AGPL / LGPL / 商业限制依赖：

> 必须单独评估，不允许 AI 随手加入。

所有第三方组件进入：

```text
THIRD_PARTY_LICENSES.md
```

并自动检查。

---

# 41. 测试策略

核心模块优先测试：

```text
Core
↓
Tools
↓
Batch
↓
Filesystem
↓
IPC
↓
UI
```

---

# 42. Fixture

必须拥有：

```text
fixtures/
├── files/
├── text/
├── csv/
├── json/
├── images/
├── documents/
├── duplicates/
├── encodings/
├── malformed/
└── large-files/
```

测试不依赖用户真实文件。

---

# 43. Fault Testing

尤其对文件操作：

```text
Permission Denied
Disk Full
File Locked
Rename Collision
Partial Write
Interrupted Operation
Cancelled Operation
Malformed Input
```

注入测试。

目标：

> **失败时也不能把用户文件搞坏。**

---

# 44. Golden Test

对于：

```text
JSON
CSV
Text Transform
Document Metadata
Image Processing
```

建立：

```text
Input
+
Expected Output
```

作为 Golden Fixture。

---

# 45. Release Quality

每个里程碑必须：

```text
cargo fmt --check
cargo clippy
cargo test
frontend typecheck
frontend lint
frontend test
tauri build
license check
```

全部通过。

---

# 46. AI Coding 开发规则

Weave 本身必须适合 AI 持续开发。

每一个工具都遵循：

```text
Specification
 ↓
Test Fixture
 ↓
Implementation
 ↓
Integration
 ↓
UI
 ↓
Audit
```

禁止：

```text
先做 UI
 ↓
再猜逻辑
```

---

# 47. AI 修改原则

AI 每次修改之前必须：

```text
Inspect
Understand
Plan
Change
Test
Audit
```

禁止：

- 大范围重构
- 顺手升级所有依赖
- 顺手改 UI
- 删除测试
- 用假数据通过测试
- 把异常吞掉
- 添加 TODO 作为完成状态
- 把失败操作伪装成成功

---

# 48. 工具优先级

## [A] 核心

第一阶段必须拥有：

```text
File Rename
File Organizer
Duplicate Finder
File Inspector
Directory Analyzer

Text Formatter
Text Compare
Text Transformer
Text Extractor

CSV Viewer
CSV Cleaner
CSV/JSON Converter
JSON Formatter

Image Resize
Image Compress
Image Convert
Image Metadata

Batch Engine
Preview
History
Undo
Command Palette
Drag & Drop
```

---

## [B] 应当

```text
PDF Merge/Split
PDF Inspect
DOCX/XLSX/PPTX Inspect
Archive Utilities
Advanced Search
Advanced Regex
Image Strip Metadata
Media Inspector
Workflow Pipeline
Large File Tools
```

---

## [C] 加分

```text
Natural Language Batch
Near Duplicate
OCR
Advanced PDF Processing
Audio Conversion
Video Conversion
Advanced Workflow Automation
Plugin / Tool SDK
```

---

# 49. 明确不做

Weave 不做：

```text
完整 Office
完整 Photoshop
完整 Premiere
完整 DAW
完整文件管理器
完整网盘
完整压缩软件
完整下载器
完整 IDE
完整项目管理
完整 RPA 平台
```

更不做：

```text
账号体系
强制云端
广告
遥测
应用内付费墙
```

---

# 50. 推荐的首发工具集合

首发不要做 50 个工具。

建议 V1 只打磨：

```text
01  Batch Rename
02  File Organizer
03  Duplicate Finder
04  File Inspector
05  Directory Analyzer

06  Text Formatter
07  Text Compare
08  Text Transformer
09  Text Extractor

10  CSV Studio
11  Data Cleaner
12  JSON/CSV Converter

13  Image Resize
14  Image Compress
15  Image Convert
16  Image Metadata

17  Batch Engine
18  History / Undo
```

这 18 个工具已经可以形成完整产品。

---

# 51. 后续可以自然扩展的工具

在核心架构稳定之后，再逐渐加入：

```text
PDF Tools
Archive Tools
Media Info
Encoding Converter
Checksum Tool
Base64 Tool
Hash Generator
UUID Generator
Timestamp Converter
URL Encoder
Color Converter
Regex Tester
Cron Helper
Markdown Tools
```

原则：

> **只有真实高频、独立、有价值的小工具才加入。**

---

# 52. 工具加入标准

任何新工具加入 Weave 必须至少满足：

```text
1. 能解决明确问题
2. 本地使用价值明确
3. 独立工具软件体验不够好或太分散
4. 处理流程足够简单
5. 可自动化测试
6. 不污染核心架构
7. 不引入不必要的大型依赖
```

禁止：

> “因为看起来很酷，所以加入。”

---

# 53. V1 完整工作流

Weave V1 至少完整支持：

```text
拖入文件
 ↓
识别类型
 ↓
推荐工具
 ↓
调整规则
 ↓
Preview
 ↓
Execute
 ↓
Progress
 ↓
Result
 ↓
History
 ↓
Undo / Reuse
```

这才算真正完成产品闭环。

---

# 54. M0 — Foundation

建立：

```text
Tauri 2
Rust
React
TypeScript
Vite
Workspace
Core Error Model
IPC Model
Logging
Config
Theme
Testkit
CI
License Audit
```

同时建立：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
THIRD_PARTY_LICENSES.md
```

---

# 55. M1 — File Core

完成：

```text
File Inspector
Directory Analyzer
Hash
Path Safety
Metadata
Filesystem Abstraction
```

---

# 56. M2 — Rename / Organizer

完成：

```text
Batch Rename
Preview
Collision Detection
Organizer
Undo
History
```

---

# 57. M3 — Duplicate Finder

完成：

```text
Size Filter
Partial Hash
Full Hash
Duplicate Groups
Preview
Recycle
```

---

# 58. M4 — Text

完成：

```text
Formatter
Comparator
Transformer
Extractor
Regex
Encoding Detection
```

---

# 59. M5 — Data

完成：

```text
CSV Viewer
CSV Cleaner
JSON Inspector
JSON Formatter
CSV ↔ JSON
CSV ↔ TSV
Streaming Processing
```

---

# 60. M6 — Image

完成：

```text
Resize
Compress
Convert
Metadata
Batch Processing
Preview
```

---

# 61. M7 — Batch Engine

将此前工具统一到：

```text
Job
Preview
Execute
Cancel
Retry
Undo
History
```

并开始建立真正的：

> **Weave Operation Engine**

---

# 62. M8 — Documents

完成：

```text
PDF Inspect
PDF Merge
PDF Split
Page Operations
DOCX Inspect
XLSX Inspect
PPTX Inspect
```

---

# 63. M9 — Utilities

加入：

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

仅保留真正高频的工具。

---

# 64. M10 — Workflow

实现：

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
```

支持保存为：

```text
Workflow
```

---

# 65. M11 — Polish

完善：

```text
Command Palette
Quick Drop
Favorites
Recent Operations
Shortcuts
Search
Error UX
Empty States
Accessibility
i18n
```

---

# 66. M12 — Release

完成：

```text
Windows Installer
Auto Update
GitHub Release
Screenshots
README
Documentation
License Audit
Privacy Audit
Security Audit
Performance Audit
```

---

# 67. M13 — Final Audit

最终只做：

```text
Bug Fix
Release Hygiene
Performance
Test
Dependency Audit
License Audit
Privacy Audit
Security Audit
UX Audit
```

禁止在 Final Audit 阶段：

```text
新增大型功能
更换架构
大规模换技术栈
```

---

# 68. 性能目标

目标以真实测试为准，不允许编造。

V1 目标：

```text
冷启动 < 1.5s
普通文件操作即时反馈
大文件采用流式处理
1 万文件扫描不阻塞 UI
批处理过程 UI 始终可交互
取消操作可快速响应
```

所有实际测试结果写入：

```text
docs/PERF.md
```

---

# 69. 隐私验收

必须确认：

```text
[ ] 无默认遥测
[ ] 无强制联网
[ ] 文件默认本地处理
[ ] 日志不包含敏感内容
[ ] 不上传用户文件
[ ] 路径经过校验
[ ] 删除默认进入回收站
[ ] 破坏性操作有 Preview / Confirm
```

---

# 70. 最终完成标准

## Product

```text
[ ] File tools 完成
[ ] Text tools 完成
[ ] Data tools 完成
[ ] Image tools 完成
[ ] Batch Engine 完成
[ ] Preview 完成
[ ] History 完成
[ ] Undo 完成
[ ] Command Palette 完成
[ ] Drag & Drop 完成
```

## Quality

```text
[ ] 所有测试通过
[ ] CI 通过
[ ] License Audit 通过
[ ] Security Audit 通过
[ ] Privacy Audit 通过
[ ] Performance Audit 完成
```

## UX

```text
[ ] 新用户无需教程即可开始
[ ] 常用操作 ≤ 3 步
[ ] 批处理有明确进度
[ ] 错误可理解
[ ] 操作结果明确
[ ] 不会静默破坏文件
```

---

# 71. Weave 的核心设计语言

Weave 最重要的一句话：

> **“把东西扔进来，Weave 帮你处理干净。”**

因此整个产品应该围绕：

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

建立。

---

# 72. 最终产品定义

> **ifuyo Weave 不是一个巨大的万能工具箱。**
>
> 它是一组被精心挑选、共享同一套本地处理引擎、预览机制、批处理机制、历史机制和设计语言的小工具。
>
> 用户不需要思考“我应该安装哪个软件”。
>
> **遇到一个小麻烦，打开 Weave，扔进去，处理完，拿走。**

---

# 73. 最终一句话

> **ifuyo Weave：把文件、文本、数据和各种琐碎工作，编织成简单、可靠、可控的本地工作流。**

---

# 74. 开发启动规则

现在开始：

```text
1. 创建 PROGRESS.md
2. 创建 DECISIONS.md
3. 创建 ARCHITECTURE.md
4. 创建 PRIVACY.md
5. 初始化 Tauri 2 + Rust + React + TypeScript
6. 建立 Core / Adapter / UI 边界
7. 从 M0 开始
8. 每个 M 必须：
   Audit
   → Implement
   → Test
   → Quality Gate
   → Commit
   → Update PROGRESS.md
```

执行时：

> **自主解决歧义，优先选择简单、可测试、可维护的方案。**

禁止：

```text
TODO placeholder
Fake implementation
Fake test
Suppressed test
Silent destructive operation
Unnecessary dependency
Architecture drift
```

任何架构取舍记录到：

```text
DECISIONS.md
```

任何已完成事项记录到：

```text
PROGRESS.md
```

最终目标不是“写了很多工具”，而是：

> **让 Weave 成为一套真正愿意每天打开的本地工具。**