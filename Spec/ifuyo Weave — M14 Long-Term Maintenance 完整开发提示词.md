# ifuyo Weave — M14 Long-Term Maintenance
## 长期维护 / 补丁 / 兼容性维护完整开发提示词

> 项目：ifuyo Weave  
> 阶段：M14  
> 阶段性质：Post-M13 Maintenance / Patch / Compatibility  
> 产品类型：Local-first Desktop Utility Suite  
> 技术栈：Tauri 2 + React + TypeScript + Vite + Rust  
> 主平台：Windows  
> 架构：UI → Application Layer → Domain/Core → Adapters → OS/File System

---

# 0. M14 身份

你现在进入：

**M14 — Long-Term Maintenance / Patch / Compatibility**

M14 不是产品扩张阶段。

M14 的唯一任务：

> **维护已经完成并发布的 Weave，修复真实问题，处理安全问题，处理依赖和操作系统变化，保持兼容性、稳定性、可恢复性与发布质量。**

不要因为进入 M14 就重新设计产品。

不要默认增加任何新工具。

不要默认增加 AI。

不要默认增加云功能。

不要默认增加插件系统。

不要默认增加账户、同步、协作、自动化、脚本系统或复杂工作流。

运行模式：在 M12 / M13（发布工程）完成之前，M14 以检查纪律模式运行——采用本提示词的 #61（No False Green）、#80（CI 门清单）、#94（Regression Matrix）、#112（停止规则）作为每个开发里程碑的收尾检查项；完整 Patch Release 循环待首次发布后启用。

---

# 1. M14 与 M0–M13 的关系

Weave 的主产品开发生命周期已经结束：

```text
M0  Foundation
M1  File Core
M2  Rename / Organizer
M3  Duplicate Finder
M4  Text
M5  Data
M6  Image
M7  Batch Engine
M8  Documents
M9  Utilities
M10 Workflow Composition
M11 Polish
M12 Release
M13 Final Audit
--------------------------------
        PRODUCT COMPLETE
--------------------------------
M14 Maintenance
```

其中：

```text
M0–M11 = 产品开发
M12    = 发布工程
M13    = 最终验证 / 最终加固
M14    = 生命周期维护
```

M14 不意味着：

```text
M13 → M14 → M15 → M16 → M17 → 无限功能开发
```

而是：

```text
Release
   ↓
Observe / Receive Issue
   ↓
Assess
   ↓
Patch
   ↓
Verify
   ↓
Release Patch
   ↓
Return to Maintenance
```

---

# 2. M14 核心原则

严格遵循以下原则：

1. **Maintenance, not expansion**
2. **Real issue before code**
3. **Smallest safe change**
4. **Preserve architecture**
5. **Preserve local-first**
6. **Preserve privacy-first**
7. **Preserve security boundaries**
8. **Preserve deterministic behavior**
9. **Preserve Safe Write / Preview / Undo / History**
10. **Preserve batch safety**
11. **Preserve backwards compatibility where practical**
12. **No speculative refactor**
13. **No opportunistic feature creep**
14. **No fake verification**
15. **No silent behavior change**
16. **No unnecessary dependency**
17. **No unnecessary network**
18. **No telemetry**
19. **No cloud dependency**
20. **No breaking architecture change without explicit product-level decision**

---

# 3. 最重要规则：先判断是否真的值得改

任何 M14 任务都必须先回答：

```text
What happened?
Is it reproducible?
Who is affected?
What version is affected?
Is it security-critical?
Is it data-loss related?
Is it a regression?
Is there a workaround?
Does current architecture already support the fix?
What is the minimum safe change?
```

如果没有足够证据：

**不要直接改代码。**

如果只是个人偏好：

**不要自动当作 bug。**

如果只是“以后可能有用”：

**不要自动开发。**

如果是架构洁癖：

**不要自动重构。**

---

# 4. Issue 分类

所有 M14 问题必须先分类。

## P0 — Critical

包括但不限于：

- 数据丢失
- 文件破坏
- 错误覆盖用户文件
- Safe Write 失效导致不可逆损坏
- Undo / Recovery 失效造成严重后果
- 路径逃逸
- 任意路径写入
- 任意代码执行
- 高危 IPC 安全漏洞
- 高危权限绕过
- 严重隐私泄漏
- 严重供应链问题
- 应用无法启动且无恢复路径
- 核心发布版本大面积不可用

P0 必须优先处理。

---

# 5. P1 — High

包括：

- 核心功能明显错误
- Batch 执行错误
- Cancel 不可靠
- Resume/Retry 错误
- Undo 错误
- History 数据损坏
- Windows 关键兼容问题
- 大量用户会遭遇的回归
- 严重性能退化
- 安全问题但未达到 P0
- 更新流程可能导致安装损坏
- 重要文档与真实产品行为严重不一致

---

# 6. P2 — Low

包括：

- 小型视觉问题
- 非关键 UX 瑕疵
- 非关键文案
- 边缘情况下的不影响数据安全的问题
- 低影响兼容问题
- 非关键性能问题

默认：

**M14 不主动处理 P2。**

除非修复成本极低且不会扩大风险。

---

# 7. M14 的证据体系

所有结论必须使用：

```text
FACT
HYPOTHESIS
INFERENCE
```

其中：

### FACT

代码、测试、日志、构建输出、真实复现直接证明的事实。

### HYPOTHESIS

目前合理但尚未完全验证的解释。

### INFERENCE

基于多个事实推导出的判断。

禁止：

```text
“应该没问题”
“理论上没问题”
“看起来已经修好”
“应该兼容”
```

作为最终 PASS 依据。

---

# 8. M14 开始时必须先建立 Baseline

执行任何代码修改之前：

**完整审计当前真实状态。**

至少检查：

```text
Git
Branches
HEAD
Working Tree
Tags
Version
Package Versions
Rust Toolchain
Node Version
Tauri Version
Build Configuration
Test Configuration
Release Configuration
Installer Configuration
Updater Configuration
Capabilities
CSP
Permissions
Documentation
CHANGELOG
License
```

同时确认：

```text
当前发布版本
当前开发版本
上一稳定版本
是否存在未发布改动
是否存在未提交改动
是否存在本地临时文件
是否存在 release artifact
```

---

# 9. M14 绝对禁止先改后审计

执行顺序必须是：

```text
AUDIT
↓
REPRODUCE
↓
CLASSIFY
↓
LOCATE
↓
PLAN
↓
PATCH
↓
TEST
↓
REGRESSION
↓
BUILD
↓
VERIFY
↓
RELEASE
```

禁止：

```text
发现问题
↓
直接开始重构
```

---

# 10. Git 安全规则

默认：

- 不 force push
- 不重写历史
- 不删除用户已有分支
- 不删除 release tag
- 不覆盖远程 release
- 不执行危险 Git 操作
- 不把临时文件提交进仓库

除非用户明确要求：

**不要自动 push。**

---

# 11. 架构冻结

M14 默认：

**Architecture Frozen**

不得因为普通 bug 而修改：

```text
UI
Application
Domain/Core
Adapters
OS/File System
```

之间已经确定的依赖方向。

不得把逻辑重新集中成 God Object。

不得绕过现有 abstraction。

不得直接从 UI 操作文件系统底层。

不得绕过 Application Layer。

不得绕过 Safe Write。

不得绕过 History。

不得绕过 Batch Engine。

---

# 12. 现有核心边界必须继续保持

继续保持：

```text
UI
 ↓
Application
 ↓
Domain/Core
 ↓
Adapters
 ↓
OS / Filesystem
```

核心原则：

> UI 不应该自己实现文件安全逻辑。

> UI 不应该自己实现批处理执行逻辑。

> UI 不应该自己管理事务。

> UI 不应该自己实现 Undo。

> UI 不应该绕过 Domain/Core。

---

# 13. Local-first 永久约束

M14 不得破坏：

```text
Local-first
Privacy-first
Offline-first
```

除非已有功能本身明确要求网络，否则：

```text
Network = unnecessary
```

不得：

- 增加遥测
- 增加远程统计
- 增加云端日志
- 增加用户行为上报
- 增加在线账户
- 增加强制联网
- 增加后台网络请求

---

# 14. 网络行为审计

每次新增或升级依赖后重新检查：

```text
Does this dependency initiate network traffic?
Does startup contact remote services?
Does update check contact remote services?
Does analytics exist?
Does crash reporting exist?
Does any library send data?
```

如果存在：

必须明确：

```text
Purpose
Endpoint
Trigger
Data Sent
User Control
Offline Behavior
Privacy Impact
```

不能因为“依赖默认开启”而放任。

---

# 15. 文件安全永远优先

M14 对文件相关问题保持最高敏感度。

必须重点验证：

```text
Absolute Path
Relative Path
Traversal
UNC Path
Symlink
Junction
Reparse Point
Long Path
Unicode
Spaces
Non-ASCII
Reserved Windows Names
Invalid Filename
Readonly File
Locked File
Missing File
Directory/File Collision
Case Sensitivity
Case-only Rename
Cross-volume Move
Drive Root
Network Path
```

---

# 16. 路径验证

任何文件操作必须确保：

```text
Target
```

经过规范化之后仍然属于允许操作范围。

禁止：

```text
../../
..\..\ 
UNC escape
Symlink escape
Junction escape
```

必须有真实测试。

---

# 17. Safe Write 永久保护

所有危险写入操作继续优先使用：

```text
OperationPlan
Transaction
Safe Write
History
Undo
Recovery
```

禁止为了“简单”而重新引入：

```text
直接覆盖
直接 rename
直接 delete
```

尤其对于批量操作。

---

# 18. Rename / Organizer 维护

重点验证：

```text
Name Collision
Extension Preservation
Unicode Name
Case Rename
Existing Target
Readonly
Locked File
Cross-volume Behavior
Undo
History
Failure Recovery
Partial Failure
```

对于批量 rename：

必须保持：

```text
Plan first
Preview
Execute
Record
Recover
```

---

# 19. Duplicate Finder 维护

重点验证：

```text
Hash Correctness
Partial Scan
File Mutation During Scan
Cancelled Scan
Large Files
Permission Errors
Symlink Loops
Duplicate Groups
Deletion Safety
Undo
```

禁止因为性能问题直接牺牲正确性。

---

# 20. Data / Documents / Media 维护

对每一种现有工具：

必须确认：

```text
Input validation
Malformed input
Large input
Empty input
Encoding
Unicode
Unsupported format
Corrupted input
Permission failure
Cancellation
Error reporting
Output correctness
```

任何 parser/library 升级后都必须重新执行兼容性测试。

---

# 21. Batch Engine 永久保护

M7 建立的统一执行语义必须继续保持：

```text
Job
Preview
Execute
Cancel
Retry
Undo
History
```

M14 不得偷偷引入不同的执行模型。

---

# 22. Batch Cancellation

必须确认：

```text
Cancellation requested
↓
new work stops
↓
active operation reaches safe point
↓
state becomes consistent
↓
history recorded
↓
UI informed
```

禁止：

```text
Cancel button changes UI only
```

而后台仍继续修改大量文件。

---

# 23. Concurrency

任何并发问题修复后必须重新检查：

```text
bounded concurrency
resource budgets
queue behavior
backpressure
deadlock
starvation
race
duplicate execution
duplicate history
```

禁止无限并发。

---

# 24. Retry

Retry 必须是：

**safe retry**

必须分析：

```text
Is operation idempotent?
Has target already changed?
Was previous attempt partially committed?
Can retry cause duplicate output?
Can retry overwrite user data?
```

不能简单：

```text
catch error
↓
run again
```

---

# 25. Undo

Undo 必须继续满足：

```text
User intent
Correct target
Correct previous state
Ordering
Partial failure handling
Persistence
Restart recovery
```

不能假设：

```text
undo state = memory only
```

如果历史需要持久化，则必须验证重启后的恢复。

---

# 26. History

必须检查：

```text
Creation
Persistence
Corruption
Recovery
Versioning
Migration
Partial write
Concurrent update
Cleanup
```

History corruption 不允许静默吞掉。

---

# 27. Workflow Composition 永久保持线性语义

M10 定义：

```text
Input
→ Filter
→ Tool
→ Tool
→ Export
```

M14 不得将 Workflow 偷偷扩展为：

```text
DAG
Branch
Loop
Trigger
Scheduler
Sub-workflow
Event automation
```

M14 只修复已有 workflow 的真实 bug。

---

# 28. Search 维护

适用条件：V1 的 Search 是基于 Tool / Command / Workflow 注册表的本地对象搜索，无持久倒排索引。本节中索引一致性 / 增量更新 / 删除文件处理 / 陈旧索引 / 索引损坏等检查，仅在未来引入持久索引后适用；V1 仅需检查注册表数据的实时性与排序确定性。

检查：

```text
Index consistency
Incremental update
Deleted file handling
Renamed file handling
Unicode search
Case behavior
Large repository scan
Cancellation
Stale index
Index corruption
```

特别注意：

```text
file changed while indexed
```

---

# 29. Command Palette / Quick Drop / Shortcuts

重点检查：

```text
Shortcut collision
Focus handling
Keyboard accessibility
Quick Drop path handling
Palette result correctness
Recent Operations correctness
Favorites persistence
```

不得为了修一个快捷键问题而重新设计整个交互系统。

---

# 30. Windows Compatibility

作为主要目标平台，持续检查：

```text
Windows 10 compatibility
Windows 11 compatibility
Path semantics
Drive letters
UNC
Long paths
UAC
Installer
Uninstaller
Permissions
File locking
Shell integration
Startup
Updater
Desktop shortcut
AppData behavior
```

发现兼容性问题时：

必须明确具体：

```text
OS version
Build
Hardware if relevant
Reproduction steps
Observed
Expected
Impact
```

---

# 31. Tauri 维护

每次 Tauri / plugin / Rust dependency 更新：

必须检查：

```text
API compatibility
Capability changes
Permission model
IPC behavior
CSP
Window behavior
Updater
Packaging
Platform behavior
```

不得只看到：

```text
cargo build = success
```

就认为升级完成。

---

# 32. IPC Security

继续保持：

```text
minimum command exposure
input validation
output validation
capability restriction
least privilege
```

任何新 command：

必须解释：

```text
Why required
Who can call
Arguments
Validation
Filesystem impact
Security impact
```

---

# 33. Capability Security

不得因为一个功能而直接扩大：

```text
filesystem permissions
shell permissions
process permissions
network permissions
```

必须遵守：

**Least Privilege**

---

# 34. Shell / Process Execution

M14 默认：

**No arbitrary shell execution**

不得为了调试方便增加：

```text
cmd.exe
powershell.exe
bash
arbitrary executable
```

除非某个已经存在并明确属于产品范围的功能确实依赖它，并经过单独安全评估。

---

# 35. 安全漏洞处理

发现安全问题时：

先判断：

```text
P0?
P1?
P2?
```

然后：

```text
Reproduce
Contain
Patch
Regression Test
Security Test
Release
Document
```

不要为了“隐藏问题”删除测试。

---

# 36. Dependency Maintenance

持续维护：

```text
Rust crates
npm packages
Tauri plugins
Build tools
CI actions
Installer dependencies
```

但：

**不要为了保持“最新”而无条件升级。**

升级的依据必须是：

```text
Security
Bug Fix
Compatibility
Required API
Performance
Build Reliability
```

而不是：

```text
newer = better
```

---

# 37. Dependency Change Protocol

每次依赖变化必须记录：

```text
Old Version
New Version
Reason
Security / Bug / Compatibility Motivation
Breaking Changes
Affected Modules
Test Coverage
Build Result
Runtime Verification
Rollback Option
```

---

# 38. Dependency Minification

如果发现：

```text
dependency no longer used
duplicate dependency
unnecessary plugin
unnecessary transitive dependency
```

可以清理。

但必须：

```text
remove
↓
build
↓
test
↓
runtime check
```

---

# 39. Supply Chain Audit

检查：

```text
Lockfile
Unexpected package
Suspicious dependency
Typosquatting
Unmaintained package
Unexpected install script
Unexpected network
CI action drift
```

尤其注意：

```text
package.json
Cargo.toml
Cargo.lock
package-lock.json
pnpm-lock / yarn.lock if applicable
GitHub Actions
Tauri plugins
```

---

# 40. License Maintenance

每次新增/升级依赖：

重新检查：

```text
License
Copyright
NOTICE
Attribution
Distribution requirements
```

不得因为“只是升级 patch version”而跳过。

---

# 41. Privacy Maintenance

确认：

```text
No telemetry
No analytics
No unnecessary logging
No user file content upload
No personal data collection
No unnecessary network
```

错误日志不得意外包含：

```text
file contents
tokens
passwords
credentials
private environment data
```

路径本身也应谨慎处理。

---

# 42. Sensitive Logging

日志输出必须避免：

```text
API Keys
Tokens
Passwords
Cookies
Private Content
Authentication Data
```

调试模式也不能默认泄露敏感信息。

---

# 43. Performance Maintenance

只有发现真实性能问题时才进行针对性优化。

重点观察：

```text
Startup
File Scanning
Search
Hashing
Large File Operations
Batch Processing
Memory
CPU
UI Responsiveness
IPC Overhead
Rendering
History Persistence
```

继续维护 M0–M13 的性能目标。

---

# 44. 性能退化检测

如果新版本相较稳定版本明显退化：

必须记录：

```text
Baseline Version
Current Version
Machine
OS
Dataset
Operation
Method
Measurement
Difference
```

不要使用：

```text
感觉变慢
```

作为唯一证据。

---

# 45. 不允许为了微优化破坏稳定性

例如：

```text
1% CPU improvement
```

不能成为：

```text
transaction safety reduction
```

的理由。

优先顺序：

```text
Correctness
Safety
Security
Recoverability
Stability
Performance
Micro-optimization
```

---

# 46. Crash / Startup Failure

发现启动崩溃：

必须优先确认：

```text
Can user start app?
Can user recover settings?
Can user bypass corrupted state?
Does old version still start?
Can state be migrated?
```

不要默认：

```text
delete all app data
```

作为唯一解决方案。

---

# 47. Persistence Compatibility

如果数据结构发生变化：

必须设计：

```text
Version
Migration
Fallback
Recovery
Corruption handling
```

禁止：

```text
new version silently interprets old data incorrectly
```

---

# 48. Release Patch 规则

M14 每次修复可以形成：

```text
Patch Release
```

例如：

```text
1.0.0
↓
1.0.1
↓
1.0.2
```

版本号必须与真实产品状态一致。

---

# 49. Release Hygiene

每次 patch release 检查：

```text
Version
Changelog
README
Installer
Executable
Updater
Documentation
License
Checksums if used
Git tag
Release notes
```

禁止：

```text
binary = v1.0.2
package metadata = v1.0.1
```

这种版本漂移。

---

# 50. Changelog

每个 patch release 必须明确：

```text
Fixed
Security
Compatibility
Performance
Dependencies
Breaking Changes
```

没有实际变化的内容不要编造。

---

# 51. Regression Testing

任何 bug fix 必须尽量形成：

```text
Regression Test
```

确保：

```text
Bug exists before fix
↓
Test fails or reproduces
↓
Fix implemented
↓
Test passes
↓
Regression suite passes
```

不允许只测试修复后的结果。

---

# 52. Test Priority

测试优先级：

```text
Security
Data Safety
Core Domain
Filesystem
Transactions
Batch
Undo/History
IPC
Persistence
Compatibility
UI
Cosmetic
```

---

# 53. 不允许删除失败测试来获得绿色

禁止：

```text
test fails
↓
delete test
```

或者：

```text
weaken assertion
```

来制造 PASS。

---

# 54. Determinism

继续保持：

```text
deterministic core behavior
stable tests
reproducible operations
controlled randomness where applicable
```

不能因为测试偶发失败就：

```text
sleep(1000)
retry 10 times
```

来掩盖 race condition。

---

# 55. E2E

涉及 UI → IPC → Rust → Filesystem 的 bug：

必须尽可能进行真实 E2E 验证。

不能仅：

```text
mock IPC
```

然后宣布：

```text
real path verified
```

---

# 56. Mock 的正确边界

Mock 可以用于：

```text
unit isolation
failure simulation
rare OS conditions
network isolation
```

但：

**最终发布验证必须尽可能使用真实环境。**

---

# 57. Reproduction Matrix

对于兼容性问题，建立：

```text
OS
Build
Input
Operation
Expected
Actual
```

如果无法复现：

状态标记：

```text
UNREPRODUCED
```

而不是：

```text
FIXED
```

---

# 58. Issue 状态

统一使用：

```text
OPEN
CONFIRMED
IN PROGRESS
FIXED
VERIFIED
REGRESSED
WONTFIX
DEFERRED
UNREPRODUCED
BLOCKED
N/A
```

---

# 59. WONTFIX

只有在明确理由存在时：

```text
documented
supported by design
risk > benefit
external limitation
platform limitation
```

才能标记：

```text
WONTFIX
```

不得因为“不想做”就标记 WONTFIX。

---

# 60. BLOCKED

如果外部条件无法验证：

例如：

```text
specific Windows build unavailable
specific hardware unavailable
external store unavailable
signing credential unavailable
```

必须：

```text
BLOCKED
```

不得伪装：

```text
PASS
```

---

# 61. No False Green

以下都不允许作为 PASS：

```text
没报错
应该能运行
本机没问题
编译通过
单元测试通过
```

如果没有验证完整路径：

只能写：

```text
PARTIAL
```

或者：

```text
UNVERIFIED
```

---

# 62. M14 Change Budget

每个维护任务优先：

```text
1 root cause
1 smallest safe fix
1 regression suite
```

避免：

```text
while fixing bug
↓
refactor entire subsystem
↓
rename hundreds of files
↓
replace architecture
```

---

# 63. Refactor 规则

允许 refactor 的条件：

必须同时满足：

```text
Directly required for the bug fix
AND
Small enough
AND
Behavior-preserving
AND
Testable
AND
No broader architecture migration
```

否则：

**DEFER**

---

# 64. Feature Request 处理

M14 经常会收到：

```text
“顺便加个功能”
“这里再加个选项”
“能不能增加一个工具”
```

默认：

**不进入 M14。**

将其标记：

```text
FEATURE REQUEST
```

不允许借维护之名扩大产品范围。

---

# 65. UX Bug

可以修：

```text
button broken
wrong state
incorrect error message
keyboard failure
accessibility regression
layout blocking function
workflow impossible to complete
```

不应借此重新设计整个产品。

---

# 66. Accessibility Maintenance

保持：

```text
Keyboard navigation
Focus visibility
Semantic labels
Accessible names
Contrast
Error announcement
Dialog semantics
Shortcut discoverability
```

只有真实问题才改。

---

# 67. i18n Maintenance

检查：

```text
Missing key
Wrong fallback
Broken interpolation
Overflow
Encoding
Placeholder mismatch
Language persistence
```

不得因为一个字符串问题大规模重构文本系统。

---

# 68. Error UX

错误应该回答：

```text
What happened?
Why?
What was affected?
Can retry?
Can undo?
What should user do next?
```

避免：

```text
Unknown Error
```

这种没有行动信息的错误。

---

# 69. Data-loss 防线

任何涉及文件修改的 patch：

必须进行：

```text
Positive Test
Negative Test
Collision Test
Failure Test
Cancel Test
Undo Test
Restart Test
Recovery Test
```

---

# 70. Batch Failure

必须明确：

```text
Success
Partial Success
Failure
Cancelled
Rollback
```

不能把：

```text
50 success + 10 failure
```

显示为：

```text
Success
```

---

# 71. Crash During Operation

至少针对高风险操作验证：

```text
Start
Partial execution
Process termination
Restart
Recover
History consistency
Filesystem consistency
```

---

# 72. Updater Maintenance

更新器变化时：

验证：

```text
Version detection
Package integrity
Signature if configured
Download failure
Interrupted update
Offline behavior
Rollback / recovery
Restart
Post-update launch
```

禁止：

```text
download anything
replace executable blindly
```

---

# 73. Installer Maintenance

检查：

```text
Install
Upgrade
Repair if supported
Uninstall
Shortcut
Start Menu
File Association if applicable
Permissions
Existing user data
Existing installation
```

---

# 74. Backward Compatibility

优先保持：

```text
Existing user data
Existing settings
Existing history
Existing workflows
Existing saved configuration
```

升级不能无理由清空。

---

# 75. Downgrade Awareness

如果新版本修改持久化数据：

必须考虑：

```text
Can old version still read it?
```

如果：

```text
No
```

必须明确记录：

```text
downgrade limitation
```

---

# 76. User Data Is More Important Than App State

排序：

```text
User Files
>
User History / Recovery
>
User Configuration
>
Application Cache
>
Temporary State
```

任何恢复逻辑不得优先保护 cache 而牺牲用户文件。

---

# 77. Cache Maintenance

缓存应允许：

```text
rebuild
invalidate
delete
recover
```

缓存损坏不应该导致永久不可用。

---

# 78. Observability

Weave 默认不发送 telemetry。

因此本地 debug 能力必须尽量做到：

```text
local logs
error context
operation IDs
history records
diagnostic mode if already supported
```

同时注意隐私。

---

# 79. Diagnostic Bundle

若已有诊断系统，可以提供：

```text
version
OS
architecture
feature state
error summary
operation metadata
```

但：

**不要默认收集用户文件内容。**

---

# 80. CI Maintenance

持续保持：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
frontend typecheck
frontend lint
frontend test
tauri build
license check
```

具体命令以仓库真实配置为准。

不得假定配置一定存在。

---

# 81. CI Failure 规则

CI 失败：

先分类：

```text
Product bug
Test bug
Environment failure
Dependency failure
Infrastructure failure
Flaky test
```

不要第一时间修改产品代码。

---

# 82. Flaky Test

发现 flaky：

必须先确认：

```text
reproduction frequency
environment
parallelism
timing
shared state
cleanup
```

不能简单增加：

```text
retry
sleep
timeout
```

来隐藏。

---

# 83. Build Reproducibility

尽量确保：

```text
Clean checkout
Clean dependency install
Clean build
```

能够稳定生成 release artifact。

---

# 84. Artifact Verification

Patch release 至少验证：

```text
Installer exists
Executable exists
Package version correct
Artifact launches
Core operation works
No accidental debug build
```

---

# 85. Security Release

安全 patch：

必须单独记录：

```text
Issue
Severity
Affected versions
Fixed version
Mitigation
Regression test
Release status
```

公开 release note 时：

不要泄漏不必要的可利用细节。

---

# 86. M14 不负责重新开发核心模块

以下内容默认禁止重写：

```text
File Core
Batch Engine
History
Search
Workflow
UI shell
Architecture
```

除非：

**真实 P0/P1 问题明确证明当前实现无法安全修复。**

---

# 87. 何时可以做较大修复

只有出现：

```text
P0
P1
Repeated severe regression
Security vulnerability
Compatibility break
Data corruption
Unmaintainable dependency blocker
```

才可以考虑扩大修改范围。

而且必须先输出：

```text
Root Cause
Options
Chosen Approach
Why Minimal Patch Is Insufficient
Regression Risk
Rollback Plan
```

---

# 88. M14 标准工作流

每一个 maintenance cycle：

```text
1. Baseline
2. Issue intake
3. Reproduce
4. Severity
5. Root cause
6. Minimal design
7. Implement
8. Unit test
9. Integration test
10. E2E test
11. Regression
12. Security check
13. Performance check
14. Build
15. Artifact verify
16. Documentation
17. Release
```

---

# 89. 每轮开始时必须输出 Maintenance Brief

格式：

```text
M14 Maintenance Brief

Release:
Version:

Issue:
Status:

Severity:
P0 / P1 / P2

FACT:
-

HYPOTHESIS:
-

INFERENCE:
-

Affected Components:
-

User Impact:
-

Security Impact:
-

Data Safety Impact:
-

Proposed Minimal Fix:
-

Regression Risk:
-

Required Tests:
-

Release Impact:
-
```

---

# 90. 修改前必须给出 Change Plan

内容：

```text
Files to modify
Why each file
Expected behavior change
Tests to add/change
Migration impact
Security impact
Performance impact
Release impact
Rollback plan
```

---

# 91. 不要修改无关文件

禁止借机：

```text
format entire repository
rename unrelated variables
rewrite README
reorganize folders
upgrade all dependencies
```

除非该动作是当前 issue 的必要部分。

---

# 92. 修改后必须重新审计

至少检查：

```text
git diff
git status
changed files
changed dependencies
changed permissions
changed capabilities
changed build config
changed release config
```

---

# 93. Diff 审查

人工逻辑检查：

```text
Did we change more than required?
Did any security boundary widen?
Did any file operation bypass Safe Write?
Did any async path become unbounded?
Did any error disappear?
Did any test weaken?
Did any dependency change unexpectedly?
```

---

# 94. Regression Matrix

必须至少覆盖受影响区域：

| Area | Before | After | Result |
|---|---|---|---|
| Core | | | |
| Filesystem | | | |
| Batch | | | |
| Undo/History | | | |
| IPC | | | |
| UI | | | |
| Security | | | |
| Performance | | | |
| Build | | | |

---

# 95. Release Matrix

| Check | Result |
|---|---|
| Version | |
| Git | |
| Tests | |
| Lint | |
| Clippy | |
| Build | |
| Installer | |
| Updater | |
| Security | |
| Privacy | |
| License | |
| Performance | |
| Documentation | |

---

# 96. Release Decision

只允许：

```text
RELEASE READY
```

或者：

```text
NOT RELEASE READY
```

不能用：

```text
basically ready
probably okay
looks fine
```

---

# 97. RELEASE READY 条件

必须：

```text
P0 = 0
P1 = 0
Required tests = PASS
Build = PASS
Affected regression = PASS
Security check = PASS or documented accepted state
Privacy = PASS
License = PASS
Artifact = VERIFIED
Documentation = CONSISTENT
```

---

# 98. BLOCKED 条件

任何重要验证无法完成：

例如：

```text
required OS unavailable
signing unavailable
release infrastructure unavailable
external dependency unavailable
```

必须显示：

```text
BLOCKED
```

不能变成：

```text
PASS
```

---

# 99. Commit 规则

建议：

```text
fix:
fix(security):
fix(files):
fix(batch):
fix(search):
fix(ui):
fix(build):
fix(deps):
fix(windows):
```

一个逻辑问题尽量一个清晰 commit。

---

# 100. Patch Release 规则

当一次维护完成并验证：

```text
commit
↓
tag
↓
artifact
↓
release
```

具体操作以当前仓库和发布策略为准。

默认：

**不要自动 push。**

---

# 101. M14 文档要求

维护记录至少包括：

```text
Issue
Root Cause
Fix
Tests
Affected Versions
Fixed Version
Compatibility
Security
Migration
Release Notes
```

---

# 102. CHANGELOG 要真实

禁止：

```text
Fixed several bugs
```

这种无法解释的内容。

应该记录实际影响：

```text
Fixed batch cancellation leaving incomplete history records.
Fixed Windows path normalization for ...
Fixed ...
```

---

# 103. 不允许虚构兼容性

例如没有验证 Windows 10：

不能写：

```text
Windows 10 supported
```

应该写：

```text
Not revalidated on Windows 10 in this maintenance cycle.
```

---

# 104. 不允许虚构性能

没有 benchmark：

不能写：

```text
Performance improved by 20%
```

必须有：

```text
dataset
method
measurement
baseline
result
```

---

# 105. User-visible Behavior Changes

任何用户可见变化：

必须记录：

```text
Before
After
Reason
Impact
```

尤其：

```text
file naming
sort
search
shortcut
dialog
error
batch behavior
undo
```

---

# 106. Compatibility Contract

对于已有用户：

优先确保：

```text
Existing files
Existing operations
Existing history
Existing workflow definitions
Existing settings
```

继续工作。

---

# 107. Migration Contract

如果必须迁移：

必须：

```text
detect
validate
backup if necessary
migrate
verify
recover on failure
```

---

# 108. Migration Failure

失败时：

不能：

```text
delete old data
```

必须尽量：

```text
preserve original
report error
provide recovery
```

---

# 109. Security Regression Matrix

至少检查：

```text
Path traversal
Path validation
Symlink/junction
IPC
Capabilities
CSP
File permissions
Sensitive logs
Dependency vulnerabilities
Updater integrity
Installer behavior
```

---

# 110. Privacy Regression Matrix

检查：

```text
network
telemetry
analytics
crash reporting
logs
file contents
paths
settings
diagnostics
```

---

# 111. Performance Regression Matrix

至少：

```text
startup
scan
search
large file
batch
memory
UI responsiveness
```

---

# 112. M14 的“停止规则”

维护工作完成后：

**停止。**

不要继续：

```text
“顺便优化一下”
“顺便重构一下”
“顺便统一一下”
“顺便增加一个设置”
“顺便增加一个工具”
```

除非发现新的：

```text
P0
P1
security issue
compatibility blocker
```

---

# 113. Feature Creep 防线

以下内容自动标记：

```text
FEATURE
```

而不是：

```text
BUG FIX
```

包括：

```text
新工具
新工作流能力
新格式
新自动化
新插件机制
新 AI
新云服务
新账户
新协作
新调度器
新脚本语言
```

---

# 114. M14 与未来版本的关系

未来如果真的需要新产品能力：

应该建立一个：

```text
NEW PRODUCT DEVELOPMENT CYCLE
```

而不是偷偷把它塞进 M14。

例如：

```text
M14 = maintenance

Future Product Version:
2.0 Planning
```

两者必须分开。

---

# 115. Maintenance Debt

可以记录：

```text
Known Issues
Deferred Improvements
Technical Debt
Platform Debt
Documentation Debt
```

但：

**记录 ≠ 立即开发。**

---

# 116. Deferred Issue

每个延期问题必须有：

```text
Reason
Impact
Risk
Trigger for Revisit
```

---

# 117. Known Limitations

对于无法修复的问题：

必须记录：

```text
Limitation
Affected Users
Workaround
Risk
```

---

# 118. Third-party Breakage

如果依赖方变化导致 Weave 出问题：

先判断：

```text
Can pin?
Can patch?
Can upgrade?
Can remove dependency?
Can isolate?
```

优先选择风险最小的方案。

---

# 119. OS Breakage

如果 Windows 更新导致行为变化：

必须分辨：

```text
Weave bug
OS behavior change
API change
permission change
filesystem change
driver/hardware behavior
```

不要默认所有问题都是应用 bug。

---

# 120. External Constraints

涉及：

```text
Microsoft
Tauri
Rust
Node
GitHub
third-party library
certificate/signing
```

必须明确：

```text
external dependency
```

不要把外部限制写成内部代码错误。

---

# 121. Reproducibility Pack

对于严重 bug：

保留：

```text
steps
input setup
version
environment
expected
actual
logs
test
fix
```

使之后能够重新验证。

---

# 122. M14 最重要的工程纪律

永远遵循：

> **Don't fix what is not broken.**

以及：

> **Don't redesign what can be patched safely.**

以及：

> **Don't claim what you did not verify.**

---

# 123. M14 最终审计

每次 maintenance cycle 结束前必须执行：

### A. Git

```text
status
diff
branch
commit
tags
```

### B. Code

```text
changed modules
dead code
unexpected changes
```

### C. Test

```text
unit
integration
E2E
regression
```

### D. Security

```text
IPC
filesystem
capabilities
dependencies
logging
updater
```

### E. Privacy

```text
network
telemetry
logs
data
```

### F. Performance

```text
startup
core operation
batch
memory
UI
```

### G. Compatibility

```text
Windows
existing data
existing configuration
existing workflows
```

### H. Release

```text
version
installer
artifact
documentation
changelog
```

---

# 124. Final Maintenance Report

最终必须输出：

```text
# M14 Maintenance Report

## 1. Release
Version:

## 2. Issue
-

## 3. Severity
P0 / P1 / P2

## 4. FACT
-

## 5. HYPOTHESIS
-

## 6. INFERENCE
-

## 7. Root Cause
-

## 8. Fix
-

## 9. Files Changed
-

## 10. Tests
-

## 11. Regression
-

## 12. Security
-

## 13. Privacy
-

## 14. Performance
-

## 15. Compatibility
-

## 16. Build
-

## 17. Release Artifact
-

## 18. Documentation
-

## 19. Remaining Issues
-

## 20. Final Status
RELEASE READY / NOT RELEASE READY
```

---

# 125. M14 COMPLETE

一次 M14 maintenance cycle 可以标记：

```text
M14 CYCLE COMPLETE
```

条件：

```text
P0 = 0
P1 = 0
Required tests = PASS
Regression = PASS
Build = PASS
Security = PASS
Privacy = PASS
Release hygiene = PASS
Artifact = VERIFIED
Documentation = CONSISTENT
```

---

# 126. M14 不是“最终产品阶段”

需要明确：

```text
M13 = Product Development Complete
M14 = Maintenance Lifecycle
```

因此：

**M14 可以重复执行多个 maintenance cycle。**

例如：

```text
M14.1 Security Patch
M14.2 Windows Compatibility Patch
M14.3 Dependency Patch
M14.4 Data Safety Bug Fix
M14.5 Updater Patch
```

这些不是新的产品开发阶段。

---

# 127. M14 默认不产生新功能

最终状态应该是：

```text
Stable Product
+
Maintenance
+
Security Updates
+
Compatibility Updates
+
Bug Fixes
```

而不是：

```text
Stable Product
+
Endless Feature Expansion
```

---

# 128. 如果没有真实问题

如果进入 M14 时：

```text
No P0
No P1
No confirmed regression
No security issue
No compatibility blocker
```

那么：

**不要为了 M14 必须“有代码提交”而制造任务。**

直接输出：

```text
M14 MAINTENANCE REVIEW — NO ACTION REQUIRED
```

并结束当前 cycle。

---

# 129. M14 最终原则

牢记：

```text
M0–M11 = Build
M12     = Ship
M13     = Prove
M14     = Maintain
```

M14 的成功不是：

```text
changed many files
added many features
wrote many lines
```

而是：

```text
fixed real problems
protected user data
preserved architecture
maintained security
maintained compatibility
maintained release quality
introduced minimal risk
```

---

# 130. 执行指令

现在开始 M14。

严格按照以下顺序：

```text
1. Audit actual repository state.
2. Do not modify files before baseline audit.
3. Identify current released version.
4. Identify current open maintenance issues.
5. Reproduce confirmed issues.
6. Classify P0/P1/P2.
7. Separate FACT / HYPOTHESIS / INFERENCE.
8. Select only legitimate maintenance work.
9. Produce a minimal change plan.
10. Implement the smallest safe fix.
11. Add or strengthen regression tests.
12. Run affected unit/integration tests.
13. Run E2E where applicable.
14. Run full regression suite.
15. Run security/privacy checks.
16. Run performance checks where affected.
17. Run clean production build.
18. Verify installer/artifacts.
19. Update changelog/docs where required.
20. Perform final diff/release audit.
21. Report RELEASE READY or NOT RELEASE READY.
22. Commit only the maintenance changes.
23. Do not push unless explicitly instructed.
24. Stop after this maintenance cycle is complete.
```

---

# 131. 最终硬性规则

永远不要：

```text
fake test results
fake benchmark results
fake compatibility claims
fake security claims
fake release verification
```

永远不要：

```text
expand scope without evidence
rewrite architecture without necessity
introduce unnecessary dependencies
introduce telemetry
introduce cloud dependency
bypass Safe Write
bypass Undo
bypass History
bypass permission boundaries
```

永远遵循：

```text
Evidence
↓
Minimal Change
↓
Regression Protection
↓
Verification
↓
Release
↓
Stop
```

**M14 的最终目标不是让 Weave 变得“更大”，而是让已经完成的 Weave 能够长期稳定存在。**