# ifuyo Weave — M11 Polish 完整开发提示词（下）

> 本文件是原《ifuyo Weave — M11 Polish 完整开发提示词》的 **第 2/2 部分**，
> 覆盖原第 176–299 节：E2E 与 Test Fixtures、各功能测试与回归（No Regression by Polish）、Architecture Audit 与架构边界、Documentation、UX Decisions 与文案措辞、Theme 与跨平台/Windows/IME、Final Product Cohesion Audit 与 P0/P1 修复顺序、Milestone Breakdown、Final UX Audit 与 Fresh-user/Keyboard-only 等各类 Pass、Memory/Listener/Timer/IPC 审计、Security/Privacy/Accessibility/i18n/Functional/Architecture/Dependency/Documentation Checklist、Git Hygiene、Final Test Matrix、Build 与 Final Regression、Final Audit A–S、Final Acceptance Criteria、Final Implementation Report、Known Limitations、M11→M12 Handoff、Final Git、提交前最后检查、最终停止条件与核心原则最终确认。
> **必须与（上）一起阅读执行**：M11 核心定义、硬边界与原则、各功能域（Command Palette、Quick Drop、Favorites、Recent、Shortcuts、Search、Error UX、Empty States、a11y、i18n 等）实现规范均在（上）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

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