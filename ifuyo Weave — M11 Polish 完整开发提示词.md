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

# 176. E2E

M11 必须建立或完善真实 E2E / UI smoke。

至少覆盖：

```text
Launch
Quick Drop
Tool Search
Command Palette
Favorite
Recent
Tool Execution
Workflow Open
Workflow Run
Error
Cancel
Undo where applicable
Settings
Language
Keyboard
```

允许为此新增一个 E2E 测试依赖与一个 a11y 自动化依赖（如 tauri-driver / Playwright、axe 类工具），必须通过 §199 License Audit 并记录 DECISIONS.md。

---

# 177. E2E 不允许 Mock 核心结果

禁止：

```text
UI test inject fake result
```

然后宣布：

```text
real product works
```

UI smoke 必须尽量走：

```text
React
↓
Tauri IPC
↓
Rust
↓
real capability
```

---

# 178. Test Fixtures

继续复用：

```text
weave-testkit
```

以及现有：

```text
fixture
golden
fault
integration
```

---

# 179. Accessibility Tests

至少增加自动化检查适用项：

```text
accessible name
role
label
keyboard
focus
```

结合人工实际验证。

自动化工具：

> 不是人工体验的替代品。

---

# 180. i18n Tests

至少验证：

```text
all required locale keys
no missing key
no undefined
no accidental English leakage in Chinese locale
no accidental Chinese leakage where English expected
```

具体根据支持语言决定。

---

# 181. Shortcut Tests

必须测试：

```text
duplicate
conflict
scope
dialog
input field
palette
navigation
```

---

# 182. Search Tests

必须测试：

```text
empty query
exact
prefix
fuzzy
Chinese
English
case handling
no result
duplicate results
stale objects
large catalog
```

---

# 183. Favorites Tests

必须测试：

```text
add
remove
order
persist
reload
stale item
duplicate
corrupt config
```

---

# 184. Recent Tests

必须测试：

```text
success
failure
cancel
partial
reload
clear
repeat
history linkage
```

---

# 185. Quick Drop Tests

至少：

```text
single file
multiple files
folder
unsupported input
mixed types
empty drop
cancel
navigation
```

---

# 186. Command Palette Tests

必须测试：

```text
open
close
query
result
keyboard
execution
disabled command
missing command
localized command
focus restore
```

---

# 187. Error UX Tests

必须验证：

```text
human-readable message
correct severity
action availability
partial result
retry
cancel
details
```

---

# 188. Regression

M11 完成前运行：

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

对应：

```text
unit
integration
fault
golden
E2E
```

只报告真实结果。

---

# 189. No Regression by Polish

特别检查：

```text
Rename
Organizer
Duplicate Finder
Text
Data
Image
PDF
Documents
Utilities
Batch
Workflow
```

任何一个原有工具：

不能因为 M11 UI 改造而：

```text
功能退化
输入 contract 改变
undo 消失
preview 消失
batch 行为改变
```

---

# 190. Architecture Audit

M11 完成后仍必须保持：

```text
UI
 ↓
Application
 ↓
Domain/Core
 ↓
Adapters
 ↓
OS
```

不能因为：

```text
Quick Drop
Search
Command Palette
Favorites
```

而变成：

```text
React
 ↓
direct filesystem
```

---

# 191. Search Architecture Boundary

Search：

```text
Search capability
```

不能变成：

```text
general purpose file crawler
```

---

# 192. Favorite Architecture Boundary

Favorites：

```text
user references
```

不能变成：

```text
duplicate tool registry
```

---

# 193. Recent Architecture Boundary

Recent：

```text
index / view over real operation data
```

不能变成：

```text
second history database
```

---

# 194. Command Architecture Boundary

Command Palette：

```text
command orchestration
```

不能变成：

```text
business logic
```

例如：

错误：

```text
Command:
rename files
↓
implements rename internally
```

正确：

```text
Command
↓
open/execute existing Rename tool
```

---

# 195. Quick Drop Boundary

Quick Drop：

```text
detect
discover
route
```

不是：

```text
execute arbitrary operation
```

---

# 196. UI State Normalization

M11 最终必须检查：

```text
duplicate state
stale state
derived state
server state
persistent state
```

目标：

> **减少状态来源，而不是增加新的全局 store。**

---

# 197. Dependency Audit

M11 原则：

```text
Prefer existing dependencies
```

新增依赖只有在：

```text
real problem
clear benefit
maintained
compatible license
small maintenance cost
```

全部满足时才允许。

---

# 198. 不得为了 UI 引入大框架

例如，不得因为：

```text
Dialog
Command Palette
Search
```

就无条件引入：

```text
large UI framework
large state framework
large command framework
```

优先复用当前项目能力。

---

# 199. License Audit

任何新增依赖检查：

```text
name
version
license
transitive dependency
source
```

更新：

```text
THIRD_PARTY_LICENSES.md
```

如项目已有自动生成机制：

优先复用。

---

# 200. Documentation

M11 至少更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
```

必要时：

```text
docs/UX.md
docs/SHORTCUTS.md
docs/SEARCH.md
docs/DIAGNOSTICS.md
```

但不要为了文档数量创建空文件。

---

# 201. SHORTCUTS 文档

如果快捷键达到正式产品规模：

建立：

```text
docs/SHORTCUTS.md
```

说明：

```text
command
shortcut
scope
```

并以实际代码为唯一事实来源。

---

# 202. Search 文档

如需要：

记录：

```text
What is searchable
What is not indexed
How ranking works
Privacy constraints
```

重点说明：

> Weave 不默认后台扫描用户文件内容建立搜索索引。

---

# 203. UX Decisions

任何以下取舍：

```text
global vs local shortcut
recent retention
favorite persistence
search ranking
empty-state behavior
error severity
navigation
```

如果不是显而易见的实现选择：

记录：

```text
DECISIONS.md
```

---

# 204. User-facing wording

文案原则：

```text
Simple
Concrete
Actionable
Non-technical
```

例如不要：

```text
Filesystem transaction failed due to TOCTOU.
```

用户界面可以：

```text
The file changed before Weave could finish the operation.
Please review the file and try again.
```

内部 diagnostics 保留技术细节。

---

# 205. Error Tone

错误文案不要：

```text
blame user
panic
dramatic language
technical jargon without explanation
```

应该：

```text
what happened
what was affected
what next
```

---

# 206. Confirmation Wording

破坏性确认必须描述：

```text
action
count
destination
reversibility
```

不能只：

```text
Are you sure?
```

---

# 207. Unsaved / Pending State

检查：

```text
Workflow edits
Tool configuration
Data transforms
Preview state
```

用户离开页面时：

必须避免：

```text
silent loss
```

但也不要：

```text
每次导航都强制弹窗
```

只有存在真实未保存变更时才需要。

---

# 208. Dirty State

如果编辑型页面存在：

使用：

```text
dirty / clean
```

明确状态。

---

# 209. Clipboard Paste

M11 可以改善：

```text
paste paths
paste text
```

但必须遵守当前 Tool 输入 contract。

不允许：

```text
paste
→ arbitrary shell
```

---

# 210. Drag & Drop Consistency

统一：

```text
hover
active
accepted
rejected
```

状态。

不要：

```text
每个工具自己设计完全不同的 drop UI
```

---

# 211. Drop Rejection

必须明确告诉用户：

```text
Unsupported input
```

必要时：

```text
Why
```

而不是：

```text
无响应
```

---

# 212. Batch UX

所有 M7-powered batch 页面检查：

```text
queued
running
success
failed
cancelled
retry
```

状态视觉必须一致。

---

# 213. Queue UX

如果 M7 有 queue：

M11 仅改善：

```text
visibility
labels
controls
```

不重写 queue engine。

---

# 214. History UX

History 页面检查：

```text
filter
sort
status
operation
timestamp
details
undo
rerun
```

只实现当前真实 capability。

不要在 M11 重做历史存储。

---

# 215. History / Recent Relationship

最终体验：

```text
Recent
↓
Open Operation
↓
History Detail
```

而不是两个完全孤立的页面。

---

# 216. Workflow / Recent Relationship

最终体验：

```text
Recent Workflow Run
↓
Open Workflow
↓
Run Again
```

但必须基于真实保存的 Workflow。

---

# 217. Workflow / Favorites Relationship

用户可以：

```text
Favorite Workflow
↓
Home
↓
Run
```

但是运行仍进入：

```text
M7
```

---

# 218. Command Palette / Favorites Relationship

Command Palette 可以提供：

```text
Favorite Tools
Favorite Workflows
```

但必须引用已有对象。

---

# 219. Search / Favorites Relationship

搜索结果可以标记：

```text
Favorite
```

但不要复制 Favorite data。

---

# 220. Search / Recent Relationship

Recent 可以参与搜索：

```text
Tool name
Workflow name
Operation summary
```

但不得把完整用户文件内容作为默认搜索文档。

---

# 221. Accessibility + i18n 联动

重点检查：

```text
localized label
localized aria-label
localized tooltip
localized error
localized shortcut description
```

不能：

```text
界面翻译了
但 accessibility label 还是英文
```

---

# 222. Theme

如果当前已有：

```text
Light
Dark
System
```

M11 统一检查。

重点：

```text
contrast
disabled
focus
error
table
dialog
drop zone
```

不要新增大量主题功能。

---

# 223. Theme Persistence

复用现有 config。

如果用户选择：

```text
Dark
```

重启后保持。

---

# 224. Theme Fallback

配置损坏时：

```text
fallback to safe default
```

不能启动崩溃。

---

# 225. Cross-platform Preparation

M11 不需要完成跨平台发布。

但：

> 不要把 UI 逻辑硬编码到 Windows API。

特别是：

```text
shortcuts
path labels
OS reveal
dialog semantics
```

这些应该通过：

```text
existing adapter boundary
```

处理。

---

# 226. Windows Reality Test

因为 M12 首发目标是 Windows：

实际在 Windows 验证：

```text
drag/drop
keyboard
shortcut
file reveal
dialogs
window sizing
font rendering
IME
```

尤其中文输入法：

```text
text field
search
workflow parameters
regex
data editor
```

不能因为：

```text
Ctrl+K
```

或：

```text
Enter
```

破坏 IME 输入行为。

---

# 227. IME Audit

重点测试：

```text
中文输入
英文输入
切换输入法
搜索
文本框
数据表格
工作流参数
```

特别防止：

```text
global key handler
```

拦截 IME composition。

---

# 228. Native Window Behavior

检查：

```text
maximize
restore
resize
minimize
close
reopen
```

M11 不增加：

```text
tray
background mode
single instance
```

除非项目真实 baseline 已经拥有，并仅需修复。

Release-level native integration 属于后续阶段。

---

# 229. Startup UX

启动时：

避免：

```text
空白白屏
长时间无反馈
```

如果初始化需要时间：

提供真实：

```text
loading / ready
```

状态。

但不要制造虚假百分比。

---

# 230. Crash-like UI Failure

如果 UI 组件发生异常：

至少保证：

```text
app shell survives where possible
```

而不是：

```text
single component exception
→ entire UI unusable
```

---

# 231. Final Product Cohesion Audit

最终打开应用，从用户角度完整走一遍：

```text
Launch
↓
Home
↓
Drop file
↓
Select tool
↓
Preview
↓
Execute
↓
Result
↓
Recent
↓
Favorite
↓
Search
↓
Command Palette
↓
Workflow
↓
History
↓
Undo
↓
Settings
↓
Language
↓
Restart
```

检查整个体验是否像：

> **一个产品**

而不是：

> **十个阶段拼起来的工程 Demo。**

---

# 232. Product Consistency Matrix

建立内部审计表：

| Capability | Home | Search | Command | Favorite | Recent | Shortcut | Error | Empty | Accessibility | i18n |
|---|---|---|---|---|---|---|---|---|---|---|
| Tools | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | N/A | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL |
| Workflows | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL |
| History | PASS/FAIL | PASS/FAIL | PASS/FAIL | N/A | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL | PASS/FAIL |

实际报告以当前真实模块为准。

---

# 233. P0 / P1 修复顺序

严格：

```text
P0 Correctness
↓
P0 Safety
↓
P0 Accessibility blocker
↓
P1 Discoverability
↓
P1 Navigation
↓
P1 Error UX
↓
P1 Keyboard
↓
P1 i18n
↓
P1 Visual Consistency
↓
Performance
```

不要：

```text
先做颜色
再修错误
```

---

# 234. 不允许大规模 UI 重写

如果当前代码已经可以工作：

优先：

```text
incremental refactor
```

只有存在：

```text
P0/P1 architectural blocker
```

才允许较大的结构调整。

---

# 235. Refactor 规则

任何重构必须回答：

```text
Why
What
Risk
Tests
```

并避免：

```text
rename everything
move everything
rewrite everything
```

---

# 236. No Feature Creep

开发期间如果发现：

```text
“顺便可以做……”
```

默认：

```text
DEFERRED
```

特别是：

```text
new tool
new workflow semantics
new platform
new service
new cloud capability
```

---

# 237. M11 Milestone Breakdown

严格按以下顺序执行。

## M11.0

```text
Baseline Audit
```

---

## M11.1

```text
Product / UX Gap Analysis
```

输出：

```text
P0
P1
P2
```

---

## M11.2

```text
Design System / UX Consistency
```

统一：

```text
buttons
forms
dialogs
status
spacing
typography
focus
```

---

## M11.3

```text
Command Registry
Command Palette
```

---

## M11.4

```text
Quick Drop
```

---

## M11.5

```text
Favorites
```

---

## M11.6

```text
Recent Operations
```

---

## M11.7

```text
Shortcut Registry
Keyboard Navigation
```

---

## M11.8

```text
Search
```

---

## M11.9

```text
Error UX
```

---

## M11.10

```text
Empty States
Loading States
Partial States
```

---

## M11.11

```text
Accessibility
```

---

## M11.12

```text
i18n
```

---

## M11.13

```text
Workflow UX Polish
```

仅：

```text
UI
Validation presentation
Preview presentation
Run experience
Saved workflow experience
```

不增加 Workflow semantics。

---

## M11.14

```text
Home / Navigation / Cross-feature Integration
```

---

## M11.15

```text
Performance Audit
```

---

## M11.16

```text
Security / Privacy Audit
```

---

## M11.17

```text
E2E / Accessibility / Regression
```

---

## M11.18

```text
Final Product UX Audit
```

---

# 238. M11.0 Baseline Deliverable

输出：

```text
CURRENT UI MAP
CURRENT COMMAND MODEL
CURRENT SEARCH MODEL
CURRENT SETTINGS MODEL
CURRENT HISTORY MODEL
CURRENT RECENT MODEL
CURRENT TOOL REGISTRY
CURRENT WORKFLOW REGISTRY
CURRENT i18n MODEL
CURRENT ACCESSIBILITY STATUS
```

---

# 239. M11.1 Gap Analysis

每个问题：

```text
ID
Severity
Area
FACT
Expected
Actual
Risk
Proposed Fix
```

例如：

```text
P1-UX-004
Area: Command Palette

FACT:
No keyboard command discovery exists.

Expected:
User can find core commands.

Actual:
Only toolbar navigation exists.

Risk:
High discoverability friction.

Fix:
Add CommandRegistry + Palette.
```

---

# 240. Final UX Audit

必须实际执行：

```text
Mouse-only pass
Keyboard-only pass
Fresh-user pass
Repeat-user pass
Error-path pass
Cancellation pass
Accessibility pass
Localization pass
Dark-theme pass if supported
```

---

# 241. Fresh-user Pass

假设用户第一次打开 Weave：

不能依赖：

```text
用户阅读 README
```

测试：

```text
能否找到工具
能否完成一次简单操作
能否理解结果
```

---

# 242. Repeat-user Pass

模拟用户已经知道工具：

检查：

```text
Favorite
Recent
Shortcut
Command Palette
Search
```

是否真的降低重复操作成本。

---

# 243. Mouse-only Pass

完整操作：

```text
Drop
Select
Configure
Preview
Execute
Result
```

不得要求：

```text
必须记快捷键
```

---

# 244. Keyboard-only Pass

完整操作：

```text
Open
Search
Select
Execute
Cancel
Navigate
```

重点检查：

```text
focus
tab order
enter
escape
arrow
shortcut
```

---

# 245. Error-path Pass

模拟：

```text
unsupported
collision
permission denied
invalid input
cancel
partial failure
malformed document
```

要求：

```text
understandable
recoverable
```

---

# 246. Accessibility Pass

实际使用：

```text
keyboard
screen reader where available
high zoom
long labels
long translations
```

---

# 247. Localization Pass

实际切换：

```text
Chinese
English
```

逐页检查：

```text
Home
Tool
Workflow
History
Search
Palette
Settings
Dialogs
Errors
Empty States
```

---

# 248. Performance Pass

记录实际：

```text
startup
palette open
search
recent load
favorites load
history load
workflow load
large result render
```

---

# 249. Performance Regression

M11 不能因为：

```text
Search
Recent
Favorites
Command Palette
```

导致：

```text
startup slower
memory growth
UI jank
```

需要比较：

```text
Before M11
After M11
```

适用时写入：

```text
docs/PERF.md
```

---

# 250. Memory Audit

重点观察：

```text
long-running app
open/close palette repeatedly
search repeatedly
run workflow repeatedly
history repeatedly
```

检查：

```text
memory growth
event listener leaks
subscription leaks
```

---

# 251. Event Listener Audit

特别检查：

```text
keydown
drop
resize
navigation
store subscription
IPC listener
progress listener
```

确保：

```text
mount
→ subscribe

unmount
→ unsubscribe
```

---

# 252. Timer Audit

检查：

```text
setInterval
setTimeout
polling
debounce
```

确保页面离开后：

```text
no unnecessary timers
```

---

# 253. IPC Listener Audit

重点：

```text
M7 progress
M7 completion
M7 cancellation
history
workflow events
```

防止：

```text
duplicate listeners
```

导致：

```text
one event
→ executes twice
```

---

# 254. M11 Security Checklist

必须真实检查：

```text
[ ] No arbitrary shell execution
[ ] No arbitrary process execution
[ ] No command injection
[ ] No unsafe HTML rendering
[ ] No unsafe workflow execution
[ ] No path bypass
[ ] No secret leakage in logs
[ ] No file-content upload
[ ] No telemetry
[ ] No cloud dependency
[ ] No unsafe persistence parsing
[ ] No shortcut command injection
[ ] Search input is treated as data
```

---

# 255. M11 Privacy Checklist

```text
[ ] Favorites local
[ ] Recent local
[ ] Search local
[ ] Command Palette local
[ ] No user file upload
[ ] No telemetry
[ ] No account
[ ] No cloud sync
[ ] Logs bounded
[ ] Sensitive paths minimized
[ ] No file contents in generic logs
```

---

# 256. M11 Accessibility Checklist

```text
[ ] Keyboard navigation
[ ] Visible focus
[ ] Logical tab order
[ ] Dialog focus trap
[ ] Focus restore
[ ] Accessible labels
[ ] Semantic controls
[ ] Error association
[ ] Progress semantics
[ ] Table semantics
[ ] Icon labels
[ ] No color-only state
[ ] Reduced motion 必测
```

a11y 量化验收：

```text
Dialog 必须响应 Escape 键关闭并恢复焦点
High Contrast 模式下验证可读性与焦点可见性
90%–140% UI Scaling 实测（至少 100% / 125% / 150%）
Reduced Motion 必测
对比度满足 WCAG AA（正文 ≥ 4.5:1，大字 ≥ 3:1）
```

---

# 257. M11 i18n Checklist

```text
[ ] Visible strings
[ ] Error strings
[ ] Validation
[ ] Tool names
[ ] Search metadata
[ ] Command metadata
[ ] Favorite UI
[ ] Recent UI
[ ] Settings
[ ] Empty states
[ ] Accessibility labels
[ ] Tooltips
[ ] Dialogs
[ ] Long string layout
[ ] Fallback behavior
```

---

# 258. M11 Functional Checklist

```text
[ ] Command Palette
[ ] Quick Drop
[ ] Favorites
[ ] Recent Operations
[ ] Shortcuts
[ ] Search
[ ] Error UX
[ ] Empty States
[ ] Loading States
[ ] Accessibility
[ ] i18n
[ ] Navigation
[ ] Home polish
[ ] Workflow UX polish
```

---

# 259. M11 Architecture Checklist

```text
[ ] Existing Tool Registry reused
[ ] Existing Workflow registry reused
[ ] Existing History reused
[ ] Existing M7 execution reused
[ ] Existing M2 mutation reused
[ ] Existing Search capability reused
[ ] No duplicate business engine
[ ] No UI direct filesystem access
[ ] No arbitrary shell
[ ] No duplicate persistence systems
[ ] No unnecessary state managers
[ ] No architecture drift
```

---

# 260. M11 Dependency Checklist

```text
[ ] Existing dependencies reused
[ ] New dependencies justified
[ ] No unnecessary UI framework
[ ] No duplicate search library
[ ] No duplicate state library
[ ] No duplicate shortcut library without reason
[ ] License checked
[ ] Cargo lock updated intentionally
[ ] Package lock updated intentionally
```

---

# 261. M11 Documentation Checklist

```text
[ ] README updated where behavior changed
[ ] ARCHITECTURE updated
[ ] DECISIONS updated
[ ] PROGRESS updated
[ ] PRIVACY updated if needed
[ ] SHORTCUTS documented if needed
[ ] SEARCH documented if needed
[ ] UX decisions recorded
```

---

# 262. Git Hygiene

最终检查：

```text
git status
git diff --stat
git diff
git ls-files
```

不得提交：

```text
node_modules
target
dist
logs
cache
database
temporary files
screenshots containing user data
private files
.env
secrets
keys
credentials
```

---

# 263. No Local Path Leakage

扫描：

```text
C:\
D:\
/Users/
 /home/
username
private paths
```

排除合理：

```text
test fixture
documentation example
intentional platform example
```

但必须判断真实情况。

---

# 264. No Secret Leakage

检查：

```text
API keys
tokens
passwords
private keys
credentials
.env
```

以及：

```text
git diff
git history
logs
test fixtures
```

---

# 265. Final Test Matrix

最终必须实际执行项目已有质量门禁，包括适用项：

```bash
cargo fmt --check
cargo clippy --all-targets --all-features
cargo test --workspace
```

前端：

```bash
npm run typecheck
npm run lint
npm run test
```

或者严格遵循仓库当前真实 scripts。

---

# 266. Build

必须实际执行：

```text
tauri build
```

或仓库当前等价正式 build 命令。

不能只：

```text
cargo test
```

就宣布完成。

---

# 267. E2E

必须实际运行：

```text
real UI smoke
real Tauri IPC
real filesystem fixture
real workflow
```

如果环境限制导致某项无法执行：

写：

```text
BLOCKED
```

不能写：

```text
PASS
```

---

# 268. Final Regression

必须重新确认：

```text
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

核心测试没有因为 M11 UI 改造而退化。

---

# 269. M11 Final Audit A — Baseline

输出：

```text
Branch:
HEAD:
Working Tree:
Baseline Commit:
```

---

# 270. Final Audit B — Product Surface

报告：

```text
Home
Navigation
Tools
Workflows
History
Recent
Favorites
Settings
```

---

# 271. Final Audit C — Command Palette

报告：

```text
Command count
Search behavior
Keyboard
Execution
Errors
i18n
Accessibility
```

---

# 272. Final Audit D — Quick Drop

报告：

```text
file
multi-file
folder
unsupported
mixed input
routing
```

---

# 273. Final Audit E — Favorites

报告：

```text
Tool favorites
Workflow favorites
Persistence
Ordering
Stale item handling
```

---

# 274. Final Audit F — Recent

报告：

```text
Source
Persistence
Status
Repeat
History linkage
Clear behavior
```

---

# 275. Final Audit G — Shortcuts

报告：

```text
Registry
Conflicts
Scopes
Keyboard navigation
IME safety
```

---

# 276. Final Audit H — Search

报告：

```text
Indexed objects
Ranking
Localization
Performance
Privacy
```

特别明确：

```text
What is searched
What is NOT searched
```

---

# 277. Final Audit I — Error UX

报告：

```text
Error taxonomy
Partial failure
Cancellation
Retry
Recovery
Human-readable messaging
```

---

# 278. Final Audit J — Empty / Loading

报告：

```text
Empty states
Loading
Unavailable
Partial states
```

---

# 279. Final Audit K — Accessibility

每项：

```text
PASS / FAIL / BLOCKED
```

至少：

```text
Keyboard
Focus
Labels
Screen Reader
Contrast
Dialogs
Tables
Errors
Progress
```

---

# 280. Final Audit L — i18n

每项：

```text
PASS / FAIL / BLOCKED
```

至少：

```text
Chinese
English
Fallback
Missing keys
Long strings
Accessibility labels
Errors
Dialogs
Search
Commands
```

---

# 281. Final Audit M — Security

至少：

```text
IPC
Path Safety
Shell Execution
HTML
Import
Search
Shortcuts
Logging
Persistence
```

---

# 282. Final Audit N — Privacy

至少：

```text
Telemetry
Cloud
Upload
File Content
Logs
Recent
Favorites
Search
```

---

# 283. Final Audit O — Performance

必须输出：

```text
startup
palette latency
search latency
recent load
favorites load
history load
large list performance
memory observations
```

只能写：

```text
Measured
```

不能：

```text
Expected
Should be fast
Looks fine
```

---

# 284. Final Audit P — Dependencies

报告：

```text
New Dependencies
Purpose
License
Alternatives Considered
```

如果没有：

```text
No new runtime dependency
```

---

# 285. Final Audit Q — Regression

必须报告：

```text
M1 = PASS / FAIL / BLOCKED
M2 = PASS / FAIL / BLOCKED
M3 = PASS / FAIL / BLOCKED
M4 = PASS / FAIL / BLOCKED
M5 = PASS / FAIL / BLOCKED
M6 = PASS / FAIL / BLOCKED
M7 = PASS / FAIL / BLOCKED
M8 = PASS / FAIL / BLOCKED
M9 = PASS / FAIL / BLOCKED
M10 = PASS / FAIL / BLOCKED
```

---

# 286. Final Audit R — Git

确认：

```text
Working tree clean
No secrets
No temporary files
No user data
No local path leakage
No generated junk
```

---

# 287. Final Audit S — Scope

确认 M11 没有偷偷引入：

```text
new tool families
DAG
loops
cloud
AI
accounts
telemetry
installer
updater
Steam
automation platform
plugin platform
```

---

# 288. Final Acceptance Criteria

只有以下全部满足，才可以：

# M11 COMPLETE

```text
[ ] Baseline audit completed
[ ] P0 issues resolved
[ ] P1 issues resolved or explicitly deferred
[ ] Command Palette works
[ ] Command Registry is centralized
[ ] Quick Drop works
[ ] Quick Drop uses real capabilities
[ ] Favorites work
[ ] Favorites persist safely
[ ] Recent Operations use real operation data
[ ] Shortcuts work
[ ] Shortcut conflicts are handled
[ ] Keyboard navigation works
[ ] Search works
[ ] Search is deterministic
[ ] Search does not become file-content crawler
[ ] Error UX is understandable
[ ] Partial failures are correctly represented
[ ] Cancellation UX is correct
[ ] Retry uses real execution capability
[ ] Empty states are useful
[ ] Loading states are correct
[ ] Workflow UX is polished
[ ] M10 semantics are unchanged
[ ] Accessibility audit passes
[ ] i18n audit passes
[ ] Home is coherent
[ ] Navigation is coherent
[ ] Privacy remains local-first
[ ] Security audit passes
[ ] Performance measured
[ ] Regression passes
[ ] Rust quality gates pass
[ ] Frontend quality gates pass
[ ] Tauri build passes
[ ] Documentation updated
[ ] Git hygiene passes
```

---

# 289. 不允许“基本完成”

最终状态只能：

```text
M11 COMPLETE
```

或者：

```text
M11 NOT COMPLETE
```

不得使用：

```text
Mostly complete
Almost done
Usable enough
85%
基本完成
差不多
```

---

# 290. Final Implementation Report

完成后必须输出：

```markdown
# M11 Polish Final Audit
```

## A. Baseline

```text
Branch:
HEAD:
Working Tree:
```

## B. Product Polish

```text
Navigation:
Home:
Tool Discovery:
Workflow:
History:
```

## C. Command Palette

```text
Commands:
Keyboard:
Search:
Execution:
```

## D. Quick Drop

```text
Single:
Multi:
Folder:
Unsupported:
```

## E. Favorites

```text
Tools:
Workflows:
Persistence:
Stale Handling:
```

## F. Recent

```text
Source:
Statuses:
Repeat:
History:
```

## G. Shortcuts

```text
Registry:
Scopes:
Conflicts:
IME:
```

## H. Search

```text
Indexed Objects:
Ranking:
Performance:
Privacy:
```

## I. Error UX

```text
Error Types:
Partial Failure:
Cancel:
Retry:
Recovery:
```

## J. Empty / Loading

```text
Empty:
Loading:
Unavailable:
Partial:
```

## K. Accessibility

```text
Keyboard:
Focus:
Labels:
Screen Reader:
Contrast:
Dialogs:
Tables:
Errors:
Progress:
```

## L. i18n

```text
Chinese:
English:
Fallback:
Missing Keys:
Long Strings:
```

## M. Security

```text
IPC:
Path:
Shell:
HTML:
Import:
Search:
Logging:
Persistence:
```

## N. Privacy

```text
Telemetry:
Cloud:
Upload:
Logs:
User Content:
```

## O. Performance

```text
Startup:
Palette:
Search:
Recent:
Favorites:
History:
Large Lists:
Memory:
```

## P. Tests

必须报告：

```text
Command
Result
Count
Failures
```

不能只：

```text
Tests passed
```

---

# 291. Known Limitations

真实存在的 limitation 必须明确：

```text
KNOWN LIMITATION
```

例如：

```text
platform-specific behavior
screen-reader differences
large-history rendering
specific file-type discovery limitation
```

禁止把：

```text
未完成
```

伪装成：

```text
limitation
```

---

# 292. Deferred Features

明确列出仍然延后的：

```text
New Tools
Advanced Search
File Content Search
AI
Natural Language Workflow
DAG
Branch
Loop
Scheduler
Cloud
Sync
Accounts
Plugin SDK
Marketplace
Installer
Updater
Steam
```

只有实际 deferred 项才列。

---

# 293. M11 → M12 Handoff

M11 完成后必须明确交接：

```text
M11 provides:
- Product-level Navigation
- Command Palette
- Command Registry
- Quick Drop
- Favorites
- Recent Operations
- Shortcut Registry
- Search
- Error UX
- Empty States
- Loading States
- Accessibility
- i18n
- Product-consistent Tool UX
- Product-consistent Workflow UX
- Stable Home
```

并明确：

```text
M12 must reuse these capabilities.
```

---

# 294. M12 不得重新实现

下一阶段不得重新实现：

```text
Command Palette
Search
Favorites
Recent
Workflow Runtime
Batch Engine
Safe Write
History
Undo
Tool Registry
```

M12 应该专注：

```text
Release
Installer
Updater
GitHub Release
Documentation
Screenshots
License Audit
Privacy Audit
Security Audit
Performance Audit
```

---

# 295. M11 Final Git

只有在：

```text
all required tests pass
build pass
audit pass
working tree understood
```

之后提交。

Commit message 遵循仓库现有 convention，例如：

```text
M11: polish product UX
```

或者：

```text
feat(ui): complete product polish
```

不得使用：

```text
fix stuff
update ui
misc
```

---

# 296. Commit 前最后检查

再次执行：

```text
git diff
git diff --stat
git status
```

确认：

```text
M11 only
```

没有：

```text
M12 work
M13 work
```

---

# 297. 最终停止条件

当：

```text
M11 COMPLETE
```

之后：

> **立即停止。**

不要自动开始：

```text
M12 Release
Installer
Updater
GitHub Release
Code Signing
Steam
```

下一阶段只能通过单独下达：

> **M12 提示词**

启动。

---

# 298. 最终产品目标

M11 完成以后，ifuyo Weave 不应该只是：

```text
很多工具
+
一个 Batch Engine
+
一个 Workflow
```

而应该成为：

```text
Open Weave
      ↓
Drop
or
Search
or
Command
      ↓
Find the right capability
      ↓
Use it immediately
      ↓
Preview
      ↓
Execute safely
      ↓
Understand result
      ↓
Undo / Retry / Repeat
      ↓
Next time:
Favorite / Recent / Shortcut
```

最终用户感受到的应该是：

> **“我不需要记住 Weave 有多少功能。它总能让我用最低摩擦找到那个现在需要的功能，而且我始终知道它在做什么。”**

这才是：

> **M11 — Polish**

真正应该完成的事情。

---

# 299. M11 核心原则最终确认

整个 milestone 始终遵守：

```text
Do not add complexity to solve discoverability.

Do not add automation to solve convenience.

Do not add cloud to solve persistence.

Do not add AI to solve navigation.

Do not add a new engine to solve a UI problem.

Do not rewrite working systems merely because they can look cleaner.

Reuse the capability.

Polish the path.

Clarify the state.

Reduce friction.

Preserve safety.
```

最终目标：

```text
M0–M10
=
Capability

M11
=
Usability

M12
=
Release

M13
=
Final Assurance
```

因此：

> **M11 的成功标准不是“代码更多了”。**

而是：

> **同样的能力，用户可以更快发现、更少思考、更少出错、更容易恢复、更容易重复使用。**