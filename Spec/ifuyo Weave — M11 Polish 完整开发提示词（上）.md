# ifuyo Weave — M11 Polish 完整开发提示词（上）

> 本文件是原《ifuyo Weave — M11 Polish 完整开发提示词》的 **第 1/2 部分**，
> 覆盖原第 0–175 节：M11 核心定义、范围与原则、M11→M12 边界与绝对禁止范围、Baseline/Git/产品表面审计、Issue 分类与产品目标、UX 信息层级与 Design Token，以及 Command Palette、Quick Drop、Favorites、Recent Operations、Shortcuts、Search、Error UX、Empty States、Accessibility、i18n、导航/工具页一致性、Workflow UX Polish、Result/Loading/State、Settings 与 Privacy、Security Audit、Performance Audit、Visual Polish、Home Page、Error Boundary、Logging 与 Diagnostics 等实现规范。
> **必须与（下）一起阅读执行**：E2E 与测试矩阵、回归与架构审计、文档、Theme 与跨平台、Final UX Audit、各类 Checklist、Final Audit A–S、验收标准、提交纪律与 M12 交接条款均在（下）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---

# ifuyo Weave
# M11 — Polish
## Command Palette、Quick Drop、Favorites、Recent Operations、Shortcuts、Search、Error UX、Empty States、Accessibility、i18n、整体产品化

你现在负责实现：

> **ifuyo Weave M11 — Polish**

这是建立在：

```text
M0 Foundation
M1 File Core
M2 Rename / Organizer
M3 Duplicate Finder
M4 Text
M5 Data
M6 Image
M7 Batch Engine
M8 Documents
M9 Utilities
M10 Workflow Composition
```

之上的：

> **产品化 / 使用体验完善阶段**

M11 的任务不是继续增加大量能力。

而是：

> **把已经存在的能力组织成一个真正顺手、稳定、低摩擦、容易发现、容易操作、错误可理解、键盘可用、可国际化的桌面工具。**

M11 完成后，用户应该不再觉得：

```text
“这里功能很多，但我要找功能。”
```

而应该感觉：

```text
“我把东西扔进去，
Weave 很快就知道我能做什么，
我能用鼠标，也能用键盘，
出错时我知道发生了什么，
下一次再做同样的事情也很快。”
```

---

# 0. M11 核心定义

严格遵守：

```text
M11 = Product Polish
```

不是：

```text
M11 = New Feature Expansion
M11 = Architecture Rewrite
M11 = New Tool Collection
M11 = Workflow Engine v2
M11 = Release Engineering
M11 = Installer
M11 = Updater
M11 = Steam Integration
```

M11 的核心目标：

```text
Discoverability
↓
Speed
↓
Consistency
↓
Recoverability
↓
Accessibility
↓
Clarity
↓
Localization
↓
Product Cohesion
```

---

# 1. M11 的范围

本阶段必须覆盖 Charter 定义的：

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

并将这些能力真正接入：

```text
Files
Text
Data
Documents
Media
Utilities
Batch
Workflow
History
```

形成统一产品体验。

---

# 2. M11 最重要的原则

始终遵守：

```text
Reuse > Rebuild
Clarity > Decoration
Speed > Animation
Consistency > Local Cleverness
Real State > Mock State
Keyboard + Mouse > Mouse Only
Recoverable Error > Silent Failure
Helpful Empty State > Blank Screen
Explicit Action > Magic
Local-first > Cloud
Privacy > Convenience
Accessibility > Decorative UI
```

以及：

```text
Correctness
>
Safety
>
Usability
>
Performance
>
Visual Polish
```

---

# 3. M11 与前面阶段的关系

必须严格保持：

```text
M1 = Filesystem Truth
M2 = Safe Mutation
M3 = Duplicate Semantics
M4 = Text Semantics
M5 = Data Semantics
M6 = Image Semantics
M7 = Reliable Execution
M8 = Document Semantics
M9 = Utility Semantics
M10 = Workflow Composition
M11 = Product Polish
M12 = Release
M13 = Final Audit
```

M11：

> **只消费前面已经建立的能力。**

不要重新实现：

```text
Filesystem
Safe Write
History
Undo
Batch Engine
Tool Runtime
Workflow Runtime
Document Parser
Image Processor
Data Engine
Hash Engine
Regex Engine
```

---

# 4. M11 → M12 边界

M11 不做：

```text
Windows Installer
MSI
NSIS
Auto Update
Updater Service
GitHub Release
Code Signing
Release Channel
Publishing
Store Packaging
Steam
Steam Cloud
Steam Overlay
Telemetry
Accounts
Payment
Cloud Sync
```

这些属于：

```text
M12 Release
```

M11 可以：

```text
优化产品结构
优化用户体验
准备截图所需的稳定 UI
完善 README 中会涉及的产品行为文档
```

但不得进入：

> **正式 Release Engineering。**

---

# 5. M11 绝对禁止范围

禁止新增：

```text
大量新工具
AI Assistant
LLM Operations
Natural Language Workflow
Plugin Marketplace
Plugin SDK
Cloud Storage
Online Search
Online Conversion
Remote Processing
Account System
Collaboration
Team Workspace
Automation Scheduler
Cron
Background Automation
RPA
Full File Manager
Full IDE
Full Office Editor
Photoshop-like Editor
Video Editor
Audio Editor
DAG Workflow
Workflow Branching
Workflow Loop
Recursive Workflow
Arbitrary Code Execution
Shell Execution
Script Runner
Macro System
```

尤其禁止：

```text
“趁现在顺手再加几个功能”
```

如果发现产品缺少某个大型能力：

```text
记录为 DEFERRED
```

不要偷偷塞入 M11。

---

# 6. M11 第一原则：先 Audit

开始任何修改之前：

```text
Audit
↓
Baseline
↓
Gap Analysis
↓
Design
↓
Implementation
↓
Test
↓
Integration
↓
UX Audit
↓
Security Audit
↓
Accessibility Audit
↓
i18n Audit
↓
Regression
↓
Final Audit
↓
Commit
```

禁止：

```text
看到 UI 不漂亮
↓
直接重写
```

必须先确认真实代码。

---

# 7. Baseline Audit

首先读取：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
THIRD_PARTY_LICENSES.md
```

以及：

```text
package.json
Cargo.toml
Cargo.lock
src-tauri/
src/
ui/
```

实际目录结构以仓库为准。

不得假设目录一定存在。

---

# 8. Git Baseline

记录：

```text
Branch
HEAD
Remote
Working Tree
Untracked Files
Recent Commits
```

确认：

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
M9
M10
```

真实状态。

如果某个 milestone 实际没有完成：

```text
FACT = NOT COMPLETE
```

不要因为 PROGRESS.md 写了 COMPLETE 就直接认定完成。

源码事实优先。

---

# 9. 当前产品表面审计

实际运行应用。

检查：

```text
Home
Tool Discovery
Drag & Drop
Tool Pages
Workflow
History
Settings
Navigation
Dialogs
Errors
Loading
Empty States
Keyboard
Theme
Language
```

记录真实问题。

每一个发现都标记：

```text
FACT
HYPOTHESIS
INFERENCE
```

---

# 10. M11 Issue Classification

只处理：

```text
P0
P1
```

## P0

包括：

```text
功能无法访问
核心操作无法完成
会导致错误执行
状态错误
数据错误
操作结果误导
无法恢复的重要错误
严重键盘可用性问题
严重 i18n 错乱
严重 UI blocker
```

## P1

包括：

```text
明显交互摩擦
功能发现困难
导航不一致
常用操作步骤过多
错误信息难以理解
空状态无帮助
快捷键冲突
焦点管理异常
搜索结果不完整
Favorites 无法可靠工作
Recent Operations 不正确
Quick Drop 不稳定
```

P2 / P3：

```text
记录
不在本阶段展开
```

---

# 11. M11 产品目标

最终用户路径应尽可能接近：

```text
Open Weave
    ↓
Drop / Search / Command
    ↓
Recognize
    ↓
Choose
    ↓
Configure
    ↓
Preview
    ↓
Execute
    ↓
Result
    ↓
Repeat / Undo / Reveal
```

用户不应该需要记住：

```text
Weave 内部模块名称
Rust crate 名称
Tool Registry implementation
Workflow implementation details
IPC command names
```

---

# 12. UX 信息层级

整个应用必须形成统一层级：

```text
App
├── Home
├── Tools
├── Workflows
├── Recent
├── Favorites
└── Settings
```

具体结构以真实代码为准。

不要为了“看起来像完整软件”引入大量空页面。

---

# 13. Design Language Audit

首先检查当前 UI 是否已经存在设计系统。

包括：

```text
Typography
Spacing
Radius
Buttons
Inputs
Panels
Cards
Menus
Dialogs
Tabs
Tables
Badges
Tooltips
Toasts
Progress
Error
Empty
Loading
```

如果已有：

> **优先扩展现有设计系统。**

不要引入第二套 UI 语言。

---

# 14. Design Token 原则

必要时统一：

```text
font sizes
font weights
spacing
radii
border widths
focus ring
control heights
icon sizing
animation timing
```

不要创建大量无意义变量。

例如不要：

```text
space-1
space-2
space-3
space-4
space-5
...
space-37
```

保持简单、稳定、可维护。

---

# 15. Command Palette

M11 必须完成真正可用的：

```text
Command Palette
```

目标：

> **用户可以通过键盘快速找到并执行 Weave 中的常用动作。**

不是：

```text
一个漂亮的搜索框
```

而是：

```text
Keyboard
↓
Command
↓
Action
```

---

# 16. Command Palette 必须支持

至少考虑：

```text
Open Tool
Run Tool
Open Workflow
Run Workflow
Open Recent Operation
Open Favorite
Navigate
Undo
Redo where actually supported
Open Settings
Change Language
Change Theme where actually supported
```

具体命令必须根据当前真实能力注册。

不能注册不存在的功能。

---

# 17. Command Registry

Command Palette 必须有单一事实来源：

```text
CommandRegistry
```

Command 至少拥有：

```text
id
label
description
category
keywords
availability
action
shortcut
```

必要时：

```text
enabled
disabledReason
context
```

---

# 18. Command ID

Command ID 必须：

```text
稳定
唯一
机器可读
适合测试
适合持久化
```

例如：

```text
app.open-command-palette
navigation.home
tool.open.files.rename
workflow.run
history.undo
settings.open
```

实际命名以项目 convention 为准。

---

# 19. Command Palette 搜索

搜索应该支持：

```text
名称
描述
关键词
工具类别
工作流名称
```

例如用户输入：

```text
rename
```

能够找到：

```text
Rename
```

用户输入：

```text
img
```

能够找到相关图片工具。

用户输入中文：

```text
重命名
```

也应该在当前语言环境中找到对应命令。

---

# 20. Command Ranking

不要建立复杂 AI 排序。

第一版采用确定性排序：

```text
Exact Match
>
Prefix Match
>
Token Match
>
Keyword Match
>
Fuzzy Match
```

Tie-break：

```text
priority
category
label
id
```

保证 deterministic。

---

# 21. Command Palette 键盘行为

必须支持：

```text
Open
Close
Arrow Up
Arrow Down
Enter
Escape
```

并保证：

```text
focus remains controlled
```

不能出现：

```text
打开 Palette
↓
焦点跑到后台页面
```

---

# 22. Command Palette 空结果

不能：

```text
空白
```

应该明确：

```text
No matching commands
```

并提供有意义的恢复方式，例如：

```text
Clear search
```

---

# 23. Command Palette Error

Command 执行失败时：

```text
Palette close/open behavior
Error reporting
Focus restoration
```

必须明确。

不能：

```text
用户按 Enter
↓
什么都没发生
```

---

# 24. Command Palette 测试

至少测试：

```text
open
close
search
exact match
fuzzy match
keyboard navigation
execute
disabled command
no result
error
focus restore
i18n
duplicate command id
```

---

# 25. Quick Drop

M11 必须真正打磨：

```text
Quick Drop
```

目标：

> **让用户打开 Weave 后，不需要先想“应该点哪个工具”。**

核心：

```text
Drop file/folder
↓
Detect
↓
Identify likely capabilities
↓
Show useful actions
```

---

# 26. Quick Drop 不应该“自作聪明”

不能：

```text
AI Guess
Magic Transformation
Automatic Mutation
Automatic Delete
Automatic Export
```

Quick Drop 应该帮助：

```text
发现
```

而不是直接：

```text
执行
```

---

# 27. Quick Drop 输入范围

至少支持当前已经真实实现的：

```text
file
multiple files
folder
```

具体支持什么，以 M1–M10 当前真实输入 contract 为准。

---

# 28. Quick Drop 行为

建议统一：

```text
Drop
↓
Detect
↓
Inspect
↓
Show Capabilities
↓
Choose Tool
```

例如：

```text
image.jpg
```

展示真实支持：

```text
Inspect
Resize
Compress
Convert
Metadata
Hash
```

只能显示：

> **当前 Tool Registry 确认存在且 capability 支持的操作。**

---

# 29. Quick Drop 能力发现

不得维护第二套：

```text
image → tools
pdf → tools
csv → tools
```

必须优先通过：

```text
Tool Registry
Capability Model
Input Types
```

动态获取。

---

# 30. Quick Drop 文件夹

拖入目录后：

```text
Directory Facts
↓
Relevant Tools
```

可以展示：

```text
Rename
Organizer
Duplicate Finder
Directory Analyzer
Batch / Workflow
```

但：

> 不得未经用户选择就修改目录。

---

# 31. Quick Drop 多文件

多文件输入应明确：

```text
Selected Files: N
```

并基于真实 capability 显示：

```text
Common Actions
```

不能因为其中一个文件支持某功能，就假装所有文件都支持。

---

# 32. Quick Drop 类型冲突

例如：

```text
image.jpg
+
data.csv
```

如果工具只接受：

```text
Image
```

UI 应明确：

```text
Some inputs are incompatible
```

而不是静默跳过。

---

# 33. Quick Drop Empty State

首页没有输入时：

不要只是：

```text
Drop files here
```

应该至少能够告诉用户：

```text
Drop files here
or
Choose files
or
Search a tool
```

如果 Command Palette 已存在：

允许：

```text
Press shortcut to search tools
```

但不要堆太多文字。

---

# 34. Favorites

M11 必须实现：

```text
Favorites
```

Favorites 是：

> **用户主动收藏的工具和 Workflow 快速入口。**

---

# 35. Favorites 类型

至少支持：

```text
Tool
Workflow
```

不要提前支持：

```text
Folder
Search Query
Complex Smart Collection
```

除非当前真实需求已经存在。

---

# 36. Favorite Persistence

Favorites 应持久化到：

```text
local configuration
```

而不是：

```text
cloud
```

必须遵守：

```text
No account
No sync
No telemetry
```

---

# 37. Favorites Schema

持久化数据至少需要：

```text
type
id
order
```

必要时：

```text
addedAt
```

但不要保存冗余的：

```text
tool label
tool description
tool capability
```

这些应该从当前 Registry 重新解析。

---

# 38. Favorite Validation

启动时如果：

```text
favorite id no longer exists
```

不能崩溃。

应该：

```text
mark stale
remove safely
or migrate
```

具体按现有 persistence policy。

---

# 39. Favorite Ordering

必须 deterministic。

例如：

```text
explicit user order
```

而不是：

```text
random
```

---

# 40. Favorite Toggle

用户应该可以：

```text
Add Favorite
Remove Favorite
```

并获得明确反馈。

---

# 41. Favorites 与 Command Palette

Command Palette 应能够找到：

```text
Favorites
```

例如：

```text
Favorites > Image Resize
```

但不要复制 Tool 实体。

---

# 42. Favorites 与首页

首页可以展示：

```text
Favorites
```

但：

> Favorite 只是引用，不是另一套 Tool Registry。

---

# 43. Recent Operations

M11 必须完善：

```text
Recent Operations
```

数据来源必须是真实：

```text
M7 Job
M2 History
Workflow Run
Utility Execution
```

不要手工在前端维护一份：

```text
fakeRecent[]
```

---

# 44. Recent Operation 与 History 的边界

必须明确：

```text
History
=
详细操作历史 / 可恢复信息
```

而：

```text
Recent
=
用户快速重新访问最近使用过的东西
```

两者可以关联，但不能重复建立完全相同的数据系统。

---

# 45. Recent Operation 信息

至少显示：

```text
Operation type
Target summary
Status
Timestamp
```

适用时：

```text
Success count
Failed count
Workflow name
Tool name
```

禁止显示敏感文件内容。

---

# 46. Recent 状态

必须区分：

```text
Success
Failed
Cancelled
Partial
```

不得：

```text
Failed → treated as Success
```

---

# 47. Recent Actions

根据真实 capability 提供：

```text
Open
Repeat
Reveal Output
Open History
Undo
```

只有真正支持的 action 才显示。

---

# 48. Repeat Operation

Repeat 必须建立在：

```text
real persisted operation/workflow definition
```

上。

不得：

```text
根据 UI 文本猜参数
```

---

# 49. Recent 清理

如果已有清理机制：

复用。

如果没有：

M11 可以提供必要的：

```text
Clear Recent
```

但不得删除：

```text
History
```

除非用户明确操作。

---

# 50. Shortcuts

M11 必须建立统一快捷键体系。

至少检查：

```text
Command Palette
Quick Drop related actions
Undo
Redo where supported
Open
Save/Export where applicable
Search
Navigation
Close Dialog
Cancel
```

---

# 51. Shortcut Registry

不要把快捷键散落在几十个组件：

```text
onKeyDown(...)
```

应该建立统一：

```text
ShortcutRegistry
```

或复用当前已有 equivalent。

---

# 52. Shortcut Contract

至少考虑：

```text
id
keys
scope
action
enabledWhen
```

必要时：

```text
description
```

---

# 53. Shortcut Scope

必须区分：

```text
Global App
Page
Dialog
Text Input
Table
Workflow Builder
```

例如：

```text
Ctrl+Z
```

在文本输入框里：

> 不能被 Weave 全局 Undo 硬抢。

---

# 54. Shortcut Conflict

启动或注册时检查：

```text
duplicate key
same scope
conflicting actions
```

不能让：

```text
Ctrl+K
```

同时：

```text
Open Palette
+
Delete File
```

---

# 55. Shortcut 与平台

必须实际考虑：

```text
Windows
```

因为 M12 首发目标是 Windows。

同时避免写死：

```text
Ctrl
```

导致未来跨平台不可扩展。

推荐抽象：

```text
Mod
```

具体映射由平台决定。

---

# 56. Global Hotkey 边界

除非当前产品已经明确需要：

> 不要把普通快捷键升级为系统级全局热键。

禁止：

```text
background global shortcut listener
```

作为“Polish”顺手加入。

原因：

```text
security
privacy
user surprise
lifecycle complexity
```

---

# 57. Search

M11 必须真正激活并打磨：

```text
Search
```

但必须明确：

> **Search 是产品发现与本地对象搜索，不是文件内容搜索引擎。**

---

# 58. Search 第一版范围

优先搜索：

```text
Tools
Commands
Workflows
Favorites
Recent Operations
```

适用时：

```text
History
```

---

# 59. Search 不默认搜索文件内容

禁止默认：

```text
扫描整个硬盘
建立巨大内容索引
读取所有用户文件
后台持续索引
上传内容
```

特别是：

```text
PDF
DOCX
XLSX
Images
Secrets
```

不得因为 Search 而偷偷建立内容上传或云端索引。

---

# 60. Search Architecture

优先复用：

```text
weave-search
Tool Registry
Workflow Registry
History/Recent store
```

不要重新建立：

```text
SearchEngineV2
GlobalIndexerV2
ToolSearchService2
```

---

# 61. Search Ranking

采用 deterministic strategy：

```text
Exact
>
Prefix
>
Token
>
Fuzzy
```

Tie-break：

```text
priority
category
label
id
```

保证结果稳定。

---

# 62. Search Categories

结果可以分类：

```text
Tools
Commands
Workflows
Recent
Favorites
```

避免用户不知道：

> “这个结果到底是什么？”

---

# 63. Search Empty State

例如：

```text
No results for "abc"
```

同时可提示：

```text
Try a different keyword
```

不能显示没有意义的空白。

---

# 64. Search Performance

搜索应该：

```text
instant for normal catalog
bounded
deterministic
non-blocking
```

不要：

```text
每次输入一个字符
↓
重新扫描整个磁盘
```

---

# 65. Search i18n

搜索应该尽量支持：

```text
Localized Name
Localized Description
Stable Keywords
```

例如：

```text
Rename
重命名
rename files
```

均可以命中同一个能力。

---

# 66. Error UX

这是 M11 的核心。

目标不是：

> “错误越来越少”

而是：

> **错误发生时，用户知道发生了什么、哪些东西没有完成、接下来可以怎么办。**

---

# 67. Error 四层模型

错误 UI 至少区分：

```text
What happened
Why it happened
What was affected
What can I do next
```

例如：

```text
3 files could not be moved.

Reason:
destination already exists.

Affected:
a.png
b.png
c.png

Next:
Review
Retry
Open details
```

---

# 68. 禁止 Error UX

禁止：

```text
Something went wrong.
```

作为唯一信息。

也禁止：

```text
Error: E_IO_17
```

直接展示内部错误编号。

内部 code：

```text
可用于 diagnostics
```

但用户界面应该显示：

```text
human-readable message
```

---

# 69. Structured Error Mapping

保持：

```text
Domain Error
↓
Application Error
↓
UI Error Presentation
```

不得让 React 直接：

```text
parse arbitrary Rust error string
```

---

# 70. Error Taxonomy

至少检查：

```text
Invalid Input
Permission Denied
File Not Found
Collision
Unsupported Format
Malformed Document
Invalid Parameter
Cancelled
Resource Limit
Disk Full
I/O Failure
Internal Error
```

具体集合以当前真实 Error Model 为准。

---

# 71. Partial Failure UX

对于：

```text
100 files
97 success
2 failed
1 cancelled
```

UI 必须展示：

```text
Success: 97
Failed: 2
Cancelled: 1
```

不能只显示：

```text
Operation Failed
```

---

# 72. Error Details

用户能够查看必要详情。

但必须避免泄露：

```text
secrets
tokens
full sensitive file contents
credential values
```

路径展示也应遵守：

```text
minimum necessary disclosure
```

---

# 73. Error Recovery

根据真实 capability 提供：

```text
Retry
Retry Failed
Open Input
Reveal Output
Open History
Undo
```

只能显示实际支持的动作。

---

# 74. Retry

Retry 必须依赖：

```text
M7 Retry
```

或现有对应 capability。

M11 不创建：

```text
UI-only retry engine
```

---

# 75. Cancel UX

取消必须明确状态：

```text
Cancelling
Cancelled
Completed Before Cancel
Partially Completed
```

不要：

```text
点击 Cancel
↓
按钮消失
↓
用户不知道有没有取消
```

---

# 76. Progress UX

长操作至少显示：

```text
Current State
Processed
Remaining if known
Success
Failed
Cancelled
```

必要时：

```text
current item summary
```

---

# 77. 不确定进度

如果某个操作无法计算精确百分比：

不要伪造：

```text
73%
```

应该使用：

```text
indeterminate
```

或：

```text
processed count
```

原则：

> **真实状态 > 虚假百分比。**

---

# 78. Toast / Notification

Toast 只用于：

```text
small transient feedback
```

例如：

```text
Copied
Added to Favorites
Workflow Saved
```

重大错误不要只放 Toast。

---

# 79. Modal Error

Modal 适合：

```text
destructive confirmation
critical failure requiring decision
```

不要把所有错误都 Modal 化。

---

# 80. Empty States

M11 必须全面审查：

```text
Home
Favorites
Recent
History
Search
Workflow List
Workflow Builder
Tool Result
File List
Data Table
Document View
```

---

# 81. Empty State 三类

至少区分：

```text
No Data Yet
No Results
Unavailable
```

例如：

```text
Favorites:
还没有收藏
```

和：

```text
Search:
没有匹配结果
```

以及：

```text
Feature unavailable for this file
```

不能混为一谈。

---

# 82. Empty State 必须有下一步

适用时提供：

```text
Add Favorite
Drop File
Create Workflow
Clear Search
Open Tool
```

不能：

```text
No data
```

然后让用户猜。

---

# 83. Empty State 不要过度设计

禁止：

```text
巨大插画
长篇品牌文案
装饰性动画
```

优先：

```text
State
Explanation
Next Action
```

---

# 84. Accessibility 总体目标

M11 必须进行真实 Accessibility Audit。

至少覆盖：

```text
Keyboard
Focus
Labels
Semantic Structure
Screen Reader
Contrast
Focus Visibility
Dialogs
Menus
Tables
Forms
Errors
Progress
```

---

# 85. Keyboard Accessibility

核心流程必须不依赖鼠标：

```text
Open App
Open Command Palette
Search
Navigate Results
Open Tool
Configure
Preview
Execute
Cancel
Close
```

---

# 86. Focus Management

检查：

```text
Dialog open
Dialog close
Palette open
Palette close
Error modal
Navigation
Page transition
Workflow builder
```

必须明确：

```text
focus target
focus restore
```

---

# 87. Focus Trap

Modal / Dialog 应当：

```text
trap focus
```

但普通页面：

> 不得错误地限制键盘移动。

---

# 88. Visible Focus

禁止：

```text
focus outline: none
```

然后没有替代方案。

必须保证：

> 用户能明显看到当前焦点在哪里。

---

# 89. Semantic Buttons

不能：

```html
<div onClick=...>
```

冒充：

```text
button
```

除非有充分语义处理且确实必要。

优先使用：

```text
button
input
select
dialog
nav
main
```

等语义元素。

---

# 90. Form Accessibility

每一个：

```text
input
select
checkbox
radio
```

必须有可识别 label。

错误状态应关联到对应控件。

---

# 91. Error Accessibility

错误不能：

```text
仅通过颜色表示
```

必须同时有：

```text
text
icon or semantic state
```

适用时：

```text
aria-live
```

但不要滥用。

---

# 92. Progress Accessibility

长任务应该让辅助技术能够知道：

```text
running
progress
completed
failed
cancelled
```

具体实现遵循当前前端栈最佳实践。

---

# 93. Table Accessibility

检查：

```text
headers
row/column relationships
sort state
selected state
keyboard navigation
```

特别是：

```text
CSV
Directory Analyzer
History
Workflow
```

等页面。

---

# 94. Dialog Accessibility

每个 dialog 必须有：

```text
name/title
description when needed
close path
focus behavior
keyboard support
```

---

# 95. Contrast

检查：

```text
text
secondary text
disabled state
border
focus
error
warning
success
```

不能只有“看起来差不多”。

必须实际检查。

---

# 96. Motion

M11 不新增复杂动画。

现有动画应考虑：

```text
reduced motion
```

不能：

```text
用户无法关闭的大量动画
```

---

# 97. i18n

M11 必须全面审计国际化。

至少确认：

```text
Chinese
English
```

是否已有其他语言，则继续遵循项目实际支持范围。

---

# 98. i18n 不得有硬编码

搜索 UI 中：

```text
buttons
labels
tool names
errors
empty states
dialogs
validation
shortcuts descriptions
```

是否存在散落：

```tsx
<Button>Save</Button>
```

之类硬编码。

---

# 99. i18n Architecture

继续复用现有 i18n 系统。

禁止：

```text
create another translation framework
```

---

# 100. Translation Keys

key 必须：

```text
stable
semantic
non-duplicated
```

不要：

```text
button_1
text_29
label_new
```

更倾向：

```text
command.openPalette
favorites.empty
recent.clear
errors.fileNotFound
```

具体遵循项目现有 convention。

---

# 101. Missing Translation

必须有：

```text
fallback
```

但不能静默变成：

```text
undefined
```

---

# 102. String Expansion

测试语言切换后：

```text
short English
long Chinese
```

以及可能的长文本。

检查：

```text
button overflow
dialog overflow
table overflow
sidebar overflow
```

---

# 103. Numeric / Date Formatting

Recent / History 等界面必须避免：

```text
硬编码格式
```

日期、时间、数字应使用统一 formatter。

---

# 104. Product Terminology

M11 必须统一：

```text
Tool
Workflow
Operation
Job
Preview
Execute
Cancel
Retry
Undo
History
Favorite
Recent
```

不要同一个概念出现：

```text
Run
Execute
Process
Start
Apply
```

五种叫法而 UI 没有区别。

---

# 105. Navigation Consistency

检查：

```text
sidebar
breadcrumbs if used
back
close
home
tool navigation
workflow navigation
settings
```

确保：

> 用户永远知道自己在哪里。

---

# 106. Back / Close 语义

必须区分：

```text
Back
Close
Cancel
Exit
```

不要全部设计成：

```text
X
```

---

# 107. Tool Page Consistency

不同工具页面应该尽量共享：

```text
Input
Configuration
Preview
Execute
Result
```

但：

> 不强迫所有工具拥有完全相同 UI。

统一的是：

```text
interaction semantics
```

不是：

```text
pixel-perfect duplication
```

---

# 108. Preview Consistency

所有已有 Preview 工具检查：

```text
Before
After
Changed count
Warnings
Collision
Unsupported
```

必须使用统一视觉与语义。

---

# 109. Destructive Action Consistency

删除 / 移动 / 覆盖 / 替换等操作必须明确：

```text
What
Where
How many
Reversible?
```

---

# 110. Undo Consistency

支持 Undo 的操作应该统一：

```text
Undo
```

行为。

不能：

```text
Rename → Undo works
Organizer → Undo works
Workflow Rename → Undo does something else
```

M10 Workflow 仍必须通过：

```text
M2
M7
```

原有执行与历史语义。

---

# 111. M10 Workflow UX Polish

M11 允许：

> **改善 M10 Workflow 的 UX。**

但不允许扩展 Workflow 语义。

---

# 112. Workflow Builder 允许优化

例如：

```text
step readability
tool selection
parameter layout
validation display
preview display
run controls
error presentation
saved workflow list
```

---

# 113. Workflow Builder 禁止新增

不做：

```text
DAG
Branch
Loop
Condition Graph
Trigger
Schedule
Sub-workflow
Parallel Graph
Script Step
Code Step
AI Step
```

---

# 114. Workflow Validation UX

Validation error 必须能够定位：

```text
Step
Parameter
Binding
Cause
```

例如：

```text
Step 3 — Image Resize
Width is required.
```

而不是：

```text
Invalid workflow.
```

---

# 115. Workflow Preview UX

应该让用户看懂：

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

并显示关键参数。

不要：

```text
JSON dump
```

代替真正的人类可读 Preview。

---

# 116. Workflow Favorites

支持：

```text
Favorite Workflow
```

复用前面 Favorites 机制。

禁止再建立：

```text
workflowFavoriteStore
```

作为平行系统。

---

# 117. Workflow Recent

运行 Workflow 后：

记录到：

```text
Recent
History
```

按各自定义工作。

---

# 118. Tool Discovery

M11 必须检查：

```text
用户第一次打开 Weave
```

是否能发现：

```text
有哪些工具
这些工具能解决什么
如何启动
```

不能：

```text
只展示图标
```

而用户完全不知道是什么。

---

# 119. Tool Metadata

Tool Registry 可以补充：

```text
title
description
keywords
category
inputTypes
```

但不得复制业务逻辑。

---

# 120. Tool Search / Command Palette / Home

三者应该共享：

```text
Tool Registry
```

而不是维护：

```text
Home Tool List
Search Tool List
Command Tool List
Workflow Tool List
```

四份数据。

---

# 121. Result UX

每一个核心工具的完成结果必须回答：

```text
Did it work?
What changed?
How many?
Where is the output?
Can I undo?
Can I repeat?
```

适用的才显示对应动作。

---

# 122. Success State

不能只有：

```text
Success
```

更好的最小信息：

```text
Completed
83 files processed
82 succeeded
1 failed
```

或工具适合的真实统计。

---

# 123. Failure State

失败应该明确：

```text
Completed with errors
```

当部分成功时不要显示：

```text
Failed
```

误导用户。

---

# 124. Cancelled State

取消后的 UI 必须：

```text
Cancelled
```

并说明：

```text
completed before cancellation
```

如果 M7 提供真实数量，则显示数量。

---

# 125. Unsupported State

如果当前能力不支持：

不要：

```text
按钮存在
↓
点击后报 unknown error
```

应该：

```text
Not supported
```

或提前隐藏。

---

# 126. Disabled Controls

Disabled control 如果用户需要理解原因：

应该有：

```text
tooltip
description
nearby explanation
```

不能只：

```text
灰掉
```

---

# 127. Tooltip

只用于：

```text
non-obvious controls
abbreviated icons
keyboard shortcuts
advanced options
```

不要每个按钮都增加 tooltip。

---

# 128. Icon-only Button

Icon-only button 必须有：

```text
accessible label
```

tooltip 适用于视觉用户。

---

# 129. Clipboard UX

Copy 操作应该：

```text
成功后明确反馈
```

失败：

```text
明确错误
```

不能假装复制成功。

---

# 130. Open / Reveal UX

如果已有：

```text
Open File
Reveal in Explorer
Open Folder
```

必须复用现有 OS adapter。

不能在前端直接：

```text
spawn
shell
```

---

# 131. No Shell Escape

M11 绝不能因为做：

```text
Reveal in Explorer
Open File
```

而在前端实现：

```text
arbitrary shell execution
```

必须使用已有受控 OS integration。

---

# 132. Loading UX

所有主要异步页面检查：

```text
first load
reload
slow operation
cancel
error
```

不要：

```text
页面空白 3 秒
```

---

# 133. Skeleton 与 Spinner

只在真正有意义的地方使用。

不要：

```text
所有页面统一 spinner
```

尤其短操作：

```text
<100ms
```

不要为了“有 loading”增加闪烁。

---

# 134. Async Race

重点检查：

```text
search
navigation
preview
tool switching
workflow execution
cancel
history refresh
```

防止：

```text
旧请求结果覆盖新请求结果
```

---

# 135. State Ownership

审计：

```text
local state
global state
server/domain state
persisted state
```

避免：

```text
同一数据
三个 store
```

并且最后不知道哪个是真的。

---

# 136. Persistence

M11 新增持久化只允许：

```text
Favorites
Relevant Preferences
Recent configuration where already designed
```

不要：

```text
无限保存 UI state
无限保存临时操作状态
```

---

# 137. Corrupted Preferences

如果本地配置损坏：

应用应该：

```text
detect
recover
fallback
```

不能：

```text
启动崩溃
```

---

# 138. Preferences

可检查：

```text
language
theme
favorite order
shortcut configuration if editable
recent retention if already supported
```

不要建立：

```text
巨大 Settings Platform
```

---

# 139. Settings UX

Settings 必须：

```text
clear categories
consistent controls
immediate feedback where safe
reset behavior if needed
```

---

# 140. Settings 不做

不增加：

```text
Account
Cloud
Telemetry
AI
Sync
Marketplace
```

---

# 141. Privacy UX

用户必须能够理解：

```text
Weave processes files locally
No upload by default
No telemetry
```

但不要做：

```text
每次启动弹隐私对话框
```

除非法律/产品要求。

---

# 142. Privacy-sensitive Error

错误日志不得在 UI 中直接展示：

```text
full environment
full path tree
secret values
tokens
credentials
file contents
```

---

# 143. Search Privacy

必须确认 Search 不会：

```text
偷偷读取大量用户文件
```

除非用户明确触发某个文件分析能力。

---

# 144. Recent Privacy

Recent 数据只记录真正必要：

例如：

```text
Tool ID
Workflow ID
Operation ID
Timestamp
Summary
```

不要保存：

```text
完整文件内容
```

---

# 145. Favorites Privacy

Favorites 不应包含：

```text
完整文件路径
文件内容
敏感元数据
```

除非实际需求明确要求。

---

# 146. Security Audit

M11 虽然是 Polish，但必须做 UI-facing Security Audit。

检查：

```text
Tauri IPC exposure
Command exposure
Path arguments
Unsafe frontend input
Deserialization
XSS / HTML rendering
Markdown rendering if used
Clipboard input
Search input
Regex input
Tooltip content
Workflow content
```

---

# 147. XSS / HTML

如果 UI 使用：

```text
dangerouslySetInnerHTML
HTML preview
Markdown rendering
```

必须审计。

禁止：

```text
user-controlled content
→
raw HTML
```

直接渲染。

---

# 148. Workflow Import

M10 已支持 Workflow Import。

M11 必须确认 UI：

```text
invalid file
malformed schema
unknown tool
unsupported version
```

时表现清晰。

绝不能因为用户导入一个恶意/损坏 JSON 而：

```text
crash
```

---

# 149. Shortcut Security

不要允许导入配置直接注入：

```text
arbitrary command
```

快捷键只能绑定：

```text
registered command
```

---

# 150. Search Injection

搜索输入必须作为：

```text
data
```

处理。

不要因为 Search：

```text
构造 SQL
构造 shell
构造 regex
```

而产生注入风险。

---

# 151. Regex

如果 Search / Command Palette 内部使用 fuzzy search：

不要直接把用户文本当：

```text
unbounded regex
```

除非明确安全处理。

---

# 152. Performance Audit

M11 必须实际测量 UI 体验。

重点：

```text
startup
home render
command palette open
search latency
favorites load
recent load
workflow list
large result list
large history list
navigation
```

---

# 153. Large List

重点测试：

```text
100
1,000
10,000
```

条数据。

如果某页面理论上可能出现大量项：

需要：

```text
pagination
virtualization
bounded rendering
```

但必须根据真实性能证据选择。

不要为了“专业”而无条件引入 virtualization。

---

# 154. Search Performance

测量：

```text
catalog size
query latency
render latency
```

不要只说：

```text
search is fast
```

必须有真实数据。

---

# 155. Command Palette Performance

打开 Palette 应：

```text
near immediate
```

不能每次：

```text
打开
↓
重新扫描
↓
等待
```

除非真实原因不可避免。

性能验收锚定 charter #68：冷启动 < 1.5s；Command Palette 打开与搜索返回给出明确上限（如 < 100ms / < 200ms），如实测无法达标，记录实测值与原因，不得放宽目标或含糊表述。

---

# 156. React Performance

审查：

```text
unnecessary rerender
large context provider
unstable callbacks
duplicate derived state
expensive list rendering
```

原则：

> 只修有证据的问题。

不要为了“优化”大规模重写状态管理。

---

# 157. Rust / IPC Performance

检查：

```text
repeated IPC
duplicate serialization
large payloads
unnecessary polling
```

尤其：

```text
Recent
History
Search
Workflow
Progress
```

---

# 158. No Chatty IPC

避免：

```text
每渲染一个 item
↓
一次 IPC
```

应该：

```text
batch
cache
reuse
```

具体以真实 profiling 为依据。

---

# 159. Visual Polish

视觉打磨应重点：

```text
spacing
alignment
typography
hierarchy
consistent controls
status clarity
focus
density
```

不是：

```text
大量阴影
渐变
玻璃特效
动画
3D
```

---

# 160. Density

Weave 是 Utility-first 工具。

不要把每个页面设计成：

```text
巨大卡片
巨大标题
大量留白
```

用户需要看到：

```text
更多有效信息
```

尤其：

```text
tables
file lists
history
workflow steps
data inspection
```

---

# 161. Responsive Window

因为是桌面软件，检查：

```text
small window
normal window
large window
```

确保：

```text
sidebar
content
dialogs
tables
workflow builder
```

不会明显崩坏。

---

# 162. Minimum Viable Window

根据真实 UI 测量合理：

```text
minimum usable window
```

不要：

```text
4000px minimum width
```

也不要让：

```text
table
dialog
workflow
```

在正常窗口完全不可用。

---

# 163. Scroll Behavior

统一审计：

```text
page scroll
panel scroll
table scroll
dialog scroll
sidebar scroll
```

禁止：

```text
nested scroll hell
```

即：

用户滚动鼠标时：

```text
不知道到底哪个容器在滚
```

---

# 164. Keyboard Navigation Between Regions

例如：

```text
Sidebar
Main
Dialog
Search
```

焦点迁移应：

```text
predictable
```

不能出现：

```text
Tab
↓
跳到页面完全不相关的位置
```

---

# 165. Command Palette + Shortcut Discoverability

用户在看到命令时：

适用时显示：

```text
Ctrl+K
Ctrl+Z
...
```

帮助用户学习系统。

但不要把所有快捷键同时显示到 UI。

---

# 166. Context-sensitive Commands

Command Registry 可以根据上下文判断：

```text
available
enabled
```

例如：

```text
Undo
```

在：

```text
no undoable operation
```

时应：

```text
disabled
```

或者不显示。

必须符合当前产品 convention。

---

# 167. No Magic Actions

禁止：

```text
Quick Drop
→ automatic resize

Open CSV
→ automatic clean

Open duplicate finder
→ automatic recycle
```

M11 的目标是：

> **降低操作成本，不改变产品安全哲学。**

---

# 168. Home Page Polish

Home 应至少帮助用户完成三个入口：

```text
Drop something
Find a tool
Continue previous work
```

可对应：

```text
Quick Drop
Search / Command Palette
Favorites / Recent
```

---

# 169. Home 不变成 Dashboard

禁止堆：

```text
metrics
charts
usage statistics
fake productivity score
```

Weave 不是项目管理工具。

---

# 170. Tool Launch Path

检查：

```text
Home
Search
Command Palette
Favorite
Recent
Workflow
```

最终是否都能够进入：

> **同一个真实 Tool / Workflow 实现。**

禁止：

```text
Home Tool
Search Tool
Workflow Tool
```

三个入口实际上调用三套逻辑。

---

# 171. Navigation State

切换页面时避免：

```text
unexpected reset
```

例如：

```text
Search → Tool → Back
```

适用时保留：

```text
search context
```

但不要无限保留所有 UI 状态。

---

# 172. Error Boundary

检查 React Error Boundary。

如果页面发生未预期前端异常：

不能：

```text
整个应用白屏
```

应尽可能：

```text
显示受控错误页面
允许恢复
记录本地诊断
```

注意：

> 不得自动上传错误。

---

# 173. Rust Panic Boundary

M11 不应主动制造 panic。

对：

```text
IPC
filesystem
parsing
workflow
history
search
```

尽量返回：

```text
structured error
```

不要让一个用户输入导致整个 Tauri backend 崩溃。

---

# 174. Logging

检查日志：

```text
useful
structured
bounded
privacy-safe
```

禁止：

```text
log entire file content
log full CSV
log full JSON
log secrets
log full document
```

---

# 175. Diagnostics

如果项目已有 diagnostics：

复用。

M11 可以改善：

```text
error ID
operation ID
job ID
```

之间的关系。

例如：

```text
Operation failed
Operation ID: ...
```

允许用户在本地定位问题。

---

