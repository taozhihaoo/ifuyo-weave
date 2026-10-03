# ifuyo Weave
## M0 — Foundation / Project Bootstrap
### 完整开发提示词 v1.0

你现在开始实施：

> **ifuyo Weave — M0 Foundation**

M0 不是“先把界面做出来”。

M0 的真正目标是：

> **建立一个可以被 AI 长期持续开发、可以安全扩展几十个工具、可以稳定进行批处理、预览、历史、撤销与本地文件操作，并且不会因为早期架构草率而在 M3～M14 大规模返工的工程基础。**

Weave 的核心定位必须始终保持：

> **Weave 是一个 local-first 的通用桌面工具集，用一组高质量的小工具，解决文件、文本、数据、文档、媒体与批处理中的高频杂事。**

核心体验：

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

Weave 不是：

```text
Cloud SaaS
Online File Manager
AI Chat App
Project Management Tool
IDE
Full Office Suite
File Explorer Replacement
RPA Platform
```

M0 期间严禁发生产品定位漂移。

---

# 0. 执行规则

你是：

> 资深 Rust / Tauri 2 系统工程师 + TypeScript/React 工程师 + 桌面应用架构师 + 测试工程师。

你必须自主执行，不向我提出“需要确认吗”类型的问题。

遇到架构歧义时：

```text
优先选择：
更简单
更本地
更可测试
更容易维护
更容易被 AI 理解
更少依赖
更少魔法
```

并把决定记录到：

```text
DECISIONS.md
```

---

## 0.1 绝对禁止

禁止：

```text
TODO 占位实现
Fake implementation
Fake test
Mock 假成功
为了通过 CI 删除测试
为了通过 lint 弱化类型安全
吞掉异常
静默忽略失败
静默修改用户文件
静默覆盖原文件
引入无必要的大型依赖
顺手升级全部依赖
大范围重构
改变产品边界
把未来功能提前塞入 M0
```

尤其禁止：

```text
“先把接口写着，里面以后再实现”
```

如果当前阶段不需要某功能：

> 不实现。

不要用假功能占位。

---

# 0.2 开发原则

所有工作遵循：

```text
Inspect
 ↓
Understand
 ↓
Plan
 ↓
Implement
 ↓
Test
 ↓
Audit
 ↓
Quality Gate
 ↓
Commit
 ↓
Update PROGRESS.md
```

不要：

```text
先写一堆代码
再想架构
```

---

# 0.3 M0 边界

M0 只负责：

```text
工程初始化
架构边界
workspace
基础 crate
Tauri shell
React shell
IPC contract
design tokens
CI
license policy
security baseline
test infrastructure
documentation baseline
```

M0 不负责实现：

```text
File Rename
File Organizer
Duplicate Finder
File Inspector
Directory Analyzer

Text tools
Data tools
Image tools
PDF tools
Batch execution
History
Undo
Workflow
```

这些留给后续里程碑。

---

# 1. 权威规格来源

当前项目必须把以下文件视为项目级规范：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
BRAND.md
SECURITY.md
CHANGELOG.md
LICENSE-MIT
LICENSE-APACHE
```

同时以：

```text
ifuyo Weave — 开发总纲领 / Project Charter v1.0
```

作为最高产品与架构约束。

总纲领要求 Weave 使用：

```text
Tauri 2
Rust
React
TypeScript
Vite
```

并采用 Rust 核心 + UI 前端分层结构；工具应通过统一 Tool / Application / Core 边界接入，而不是让 React 组件直接碰文件系统。

---

# 2. M0 最终目标

M0 完成后，仓库必须具备：

```text
一个可以正常启动的 Tauri 2 Windows Desktop App
+
一个可工作的 React / TypeScript UI
+
一个 Cargo Workspace
+
多个职责明确的 Rust crates
+
统一 IPC 契约
+
设计 Token 单一事实源
+
CI
+
License 检查
+
基础安全策略
+
基础测试工具
+
规范文档
+
可重复构建
```

并达到：

```text
cargo fmt --check        PASS
cargo clippy             PASS
cargo test --workspace   PASS
frontend typecheck       PASS
frontend lint            PASS
frontend test            PASS
tauri build              PASS
license audit            PASS
```

M0 不能留下“架构已经确定但代码实际绕开了架构”的情况。

---

# 3. 开始前：基线审计

在修改任何文件之前，先进行完整基线审计。

检查：

```text
当前目录
Git 状态
Git remote
现有文件
.gitignore
Git 分支
Rust 版本
Cargo 版本
Node 版本
npm/pnpm/yarn
Tauri CLI
操作系统
Windows SDK
WebView2
VS Code / Node toolchain
```

执行与环境对应的检查命令。

不要猜环境。

如果目录为空：

> 明确记录“clean bootstrap”。

如果目录不是空仓库：

> 先判断哪些文件属于现有用户内容，严禁直接删除未知文件。

---

# 4. 技术栈

M0 使用：

```text
Tauri 2
Rust stable
React
TypeScript
Vite
```

状态管理：

```text
Zustand
```

但是：

> M0 不提前建立一大堆没有实际用途的 store。

---

# 4.1 版本策略

不要硬编码一个没有验证过的旧版本。

执行：

```text
检查当前稳定版本
检查彼此兼容性
锁定实际解析结果
```

但是：

> 禁止为了“最新”而全仓库盲目升级依赖。

版本选择必须满足：

```text
Stable
Compatible
Maintained
License acceptable
Windows buildable
CI buildable
```

最终实际版本写入：

```text
README.md
DECISIONS.md
```

并确保：

```text
Cargo.lock
package-lock.json
```

在仓库策略要求下正确管理。

---

# 5. Cargo Workspace

建立：

```text
Cargo Workspace
```

建议根结构：

```text
/
├── crates/
│   ├── weave-core/
│   ├── weave-files/
│   ├── weave-text/
│   ├── weave-data/
│   ├── weave-documents/
│   ├── weave-media/
│   ├── weave-batch/
│   ├── weave-history/
│   ├── weave-search/
│   └── weave-testkit/
│
├── src-tauri/
├── ui/
├── config/
├── docs/
├── scripts/
│
├── brand.json
├── README.md
├── ARCHITECTURE.md
├── DECISIONS.md
├── PROGRESS.md
├── PRIVACY.md
├── BRAND.md
├── SECURITY.md
├── CHANGELOG.md
├── THIRD_PARTY_LICENSES.md
├── Cargo.toml
├── Cargo.lock
├── package.json
└── .gitignore
```

Rust workspace 的依赖方向必须清楚。

推荐：

```text
weave-core
    ↑
files / text / data / documents / media / batch / history / search
    ↑
src-tauri
```

UI：

```text
ui
 ↓
IPC
 ↓
src-tauri
 ↓
application/core
 ↓
adapter / OS
```

禁止：

```text
ui
 ↓
filesystem
```

---

# 5.1 weave-core

`weave-core` 是整个产品最重要的稳定基础。

它不能依赖：

```text
React
Tauri UI
Windows GUI
具体文件系统实现
具体 UI 组件
```

M0 只建立稳定的领域契约。

至少定义：

```text
ToolId
ToolCategory
InputKind
OperationId
JobId
OperationStatus
Preview
PreviewItem
OperationResult
Failure
Cancellation
Progress
```

其中：

> OperationResult 必须包含字段：success / processed / skipped / failed / warnings / outputs / duration（charter #25）。

但：

> 不要为了“看起来完整”定义几十个未来不会使用的类型。

原则：

> 类型只解决当前已经确认的架构边界。

---

# 5.2 Tool Contract

建立统一 Tool 抽象。

概念模型：

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

这个接口是未来 M1～M14 的核心扩展点。

但是 M0：

> 只建立真正必要的 contract。

不要写一个拥有十几个空方法的“万能接口”。

澄清：

> M0 必须建立 Tool trait 的 Rust 类型签名（上述 8 个成员的方法签名与关联类型），方法体不实现（返回 Unsupported 类错误或留空实现），并附契约测试证明签名可用。

“不要万能接口”的限制不适用于这 8 个成员。

---

# 5.3 Tool Registry

建立：

```text
Tool Registry
```

用于未来：

```text
Command Palette
Home Quick Actions
Tool Search
Tool Discovery
Workflow
```

M0 只需要支持：

```text
register
lookup
list
```

注册表必须 deterministic。

工具 ID：

```text
不可重复
稳定
机器可读
适合持久化
```

例如：

```text
files.rename
files.organize
files.duplicates
files.inspect

text.format
text.compare

data.csv
data.json
```

M0 不需要真正实现这些工具。

可以只建立 ID 类型和 registry contract。

---

# 6. Application Layer

建立清晰的：

```text
Application Layer
```

职责：

```text
接收 UI 请求
验证输入
调用 domain/core
管理 operation context
返回结构化结果
```

禁止：

```text
Tauri command 里直接写 500 行业务代码。
```

也禁止：

```text
Tauri command
 → fs::read
 → business logic
 → JSON 拼接
 → UI specific string
```

---

# 7. Adapter Boundary

建立：

```text
Adapter
```

用于未来隔离：

```text
Filesystem
OS
Windows APIs
Image codecs
PDF libraries
Office libraries
Archive libraries
```

不要让领域层绑定：

```text
std::fs
Windows API
Tauri AppHandle
```

除非这是不可避免且已经记录。

---

# 8. Tauri Shell

初始化一个：

```text
Tauri 2 + React + TypeScript + Vite
```

应用。

M0 必须做到：

```text
npm install
npm run dev
```

可以启动 UI。

并且：

```text
cargo tauri dev
```

可以启动完整桌面应用。

---

# 8.1 Window

M0 只建立基础窗口。

遵循 Weave 产品最终规格：

```text
Desktop Application
Windows-first
```

窗口参数建立为：

```text
reasonable default desktop size
minimum usable size
```

M0 不提前实现：

```text
复杂窗口特效
高级动画
大量 UI
工具页面
```

---

# 8.2 Shell UI

建立极简启动壳。

必须能显示：

```text
Weave
ifuyo
version
```

以及：

```text
M0 Foundation
```

用于验证：

```text
Tauri
 ↓
React
 ↓
IPC
 ↓
Rust
```

全部正常。

这个页面只是：

> Engineering Smoke Test UI

而不是 V1 首页。

---

# 9. IPC Architecture

IPC 是 M0 最重要的边界之一。

目标：

```text
Rust Contract
 ↓
Generated / Verified Type
 ↓
TypeScript
```

优先使用：

```text
tauri-specta
```

但不能机械依赖。

如果当前版本组合存在无法解决的兼容问题：

```text
tauri-specta
specta
specta-typescript
```

则：

```text
改为手写 contract
+
契约测试
+
DECISIONS.md 记录原因
```

---

# 9.1 IPC 原则

所有 IPC command 必须：

```text
typed
validated
small
explicit
documented
```

禁止：

```text
stringly typed
map[string, any]
arbitrary command execution
filesystem path passthrough without validation
```

---

# 9.2 IPC Example

建立一个真实可工作的最小 command：

```text
get_app_info()
```

返回：

```text
name
version
vendor
environment
```

另建立一个测试性 command：

```text
ping()
```

例如：

```text
React
 ↓
IPC ping
 ↓
Rust
 ↓
pong
```

这个 command 必须真正走完整 IPC。

禁止：

```text
frontend local mock
```

冒充 IPC 成功。

---

# 9.3 IPC Security

Tauri capabilities 必须：

```text
minimal
explicit
```

不要：

```text
*
```

不要给予 UI：

```text
unrestricted filesystem access
unrestricted shell access
unrestricted process access
```

M0 就建立权限边界。

---

# 10. JSON Data Contract

建立：

```text
brand.json
```

它是品牌基本信息的单一事实来源。

至少包含：

```json
{
  "name": "Weave",
  "vendor": "ifuyo · 伊芙游",
  "site": "https://ifuyo.com",
  "versionSource": "package"
}
```

实际字段以 Project Charter 为准。

要求：

```text
Rust
+
Frontend
```

都不要各自复制品牌信息。

---

# 10.1 Brand Source of Truth

禁止：

```text
Rust 写一份 Weave
React 写一份 Weave
tauri.conf.json 再写一份 Weave
README 又写一份
```

应该：

```text
brand.json
     ↓
build / runtime adapters
     ↓
所有需要品牌信息的地方
```

但必须避免在 runtime 中随意读仓库根目录。

确定合理策略：

```text
build-time
generated constants
or bundled resource
```

并记录架构决定。

---

# 11. Design Tokens

建立：

```text
ui/src/design/
```

并定义：

```text
tokens.ts
```

作为唯一设计 token 来源。

再生成：

```text
CSS variables
```

---

# 11.1 Token 类别

至少建立：

```text
colors
spacing
radius
typography
z-index
motion
control
surface
```

例如：

```text
color.background
color.surface
color.text
color.textMuted
color.border
color.accent
```

禁止业务组件直接出现：

```css
#xxxxxx
rgb(...)
rgba(...)
```

---

# 11.2 视觉方向

Weave：

```text
干净
克制
现代
中性
有一点 ifuyo 情绪
```

但：

> 不复制 Aura 的音乐视觉。

也不要把 Weave 做成：

```text
科技感仪表盘
彩色后台管理系统
AI SaaS
```

---

# 11.3 Design Token Test

建立一个自动检查：

```text
业务 CSS / TSX
```

禁止出现裸色值。

例如检测：

```text
#[0-9a-fA-F]{3,8}
rgb(
rgba(
hsl(
hsla(
```

但：

> token 定义文件本身允许。

这个规则必须纳入 lint / CI。

---

# 12. Internationalization Baseline

M0 不需要完成全部翻译。

但是必须建立：

```text
zh-CN
en
```

资源结构。

例如：

```text
ui/src/i18n/
├── zh-CN.json
└── en.json
```

禁止在业务代码中散落：

```ts
"开始处理"
"文件错误"
"取消"
```

---

# 12.1 i18n Smoke Test

至少让：

```text
Weave
M0 Foundation
IPC Connected
```

之类的 UI 文案通过资源文件读取。

语言切换先做最小可用验证。

---

# 13. Logging Baseline

M0 建立 Rust logging：

```text
tracing
```

日志写入应用数据目录：

```text
%APPDATA%/ifuyo/Weave/logs
```

建立：

```text
rolling logs
```

至少支持：

```text
info
warn
error
```

---

# 13.1 Logging Rules

禁止记录：

```text
文件内容
密码
token
API key
敏感用户数据
完整敏感路径（除非以后诊断需求确实必要）
```

M0 就规定：

```text
structured logging
```

避免：

```text
println!
eprintln!
```

在正式业务路径中到处出现。

---

# 14. Privacy Baseline

建立：

```text
PRIVACY.md
```

明确说明：

```text
No Telemetry
No Account
No Cloud Requirement
No File Upload
Local-first
```

默认：

> Weave 不上传用户文件。

M0 即使还没有任何在线功能，也先把边界写清楚。

---

# 15. Security Baseline

建立：

```text
SECURITY.md
```

至少写明：

```text
Tauri capability minimization
IPC validation
filesystem boundary
path traversal protection
no shell execution from UI
no arbitrary external URL handling
no embedded secrets
```

M0 不需要实现完整安全系统。

但：

> 架构边界必须先确定。

---

# 16. Path Safety Contract

这是 Weave 后续文件工具极其重要的基础。

建立统一 path validation contract。

至少覆盖：

```text
absolute path
relative path
path normalization
parent traversal
..
.
symlink consideration
invalid Windows path
reserved device names
UNC path
long path
```

M0 不需要完整文件处理工具。

但是：

> 不要等到 M2 Rename 时才第一次思考路径安全。

---

# 16.1 Path Safety Design Principle

统一原则：

```text
User-selected paths
        ↓
Normalize
        ↓
Validate
        ↓
Resolve
        ↓
Operate
```

而不是：

```text
UI string
 ↓
std::fs
```

---

# 17. Error Model

M0 建立统一错误模型。

不要让每个 crate 自己：

```text
String error
```

建立结构化 error hierarchy。

至少区分：

```text
ValidationError
IoError
PermissionError
ConflictError
CancelledError
UnsupportedError
InternalError
```

所有错误必须携带结构化字段：

```text
Code
Message
Location
Recoverability
Suggestion
```

类别枚举之上必须有这五个字段，禁止只返回 “Something went wrong”（charter #25）。

错误必须：

```text
machine-readable
user-presentable
loggable
```

但：

> domain error 不要夹杂 UI 文案。

例如禁止：

```rust
Err("抱歉，文件无法重命名".to_string())
```

应该：

```text
Domain Error
 ↓
Application Translation
 ↓
UI Message
```

---

# 18. Cancellation Model

Weave 后续批处理必然需要取消。

因此 M0 建立：

```text
CancellationToken / Operation Cancellation Contract
```

要求：

```text
cooperative cancellation
```

而不是：

```text
thread kill
```

M0 不需要大型任务系统。

只需保证后续 Batch Engine 有明确扩展位置。

---

# 19. Progress Model

定义统一：

```text
Progress
```

例如：

```text
current
total
percentage
status
```

未来支持：

```text
10 / 100
```

或：

```text
35%
```

不能在 M0 为某一个工具定制：

```text
ImageProgress
RenameProgress
PdfProgress
```

---

# 20. Operation Identity

定义：

```text
OperationId
```

以及：

```text
JobId
```

未来用于：

```text
Preview
Execute
History
Undo
Retry
Cancellation
Logging
```

要求：

```text
stable
unique
serializable
```

但不要过早引入：

```text
distributed tracing
event sourcing platform
```

---

# 21. Preview Contract

M0 虽然不实现 Rename，但应该建立 Preview 的基础模型。

概念：

```text
Preview
├── operation
├── affectedItems
├── warnings
├── conflicts
├── estimatedOutput
└── reversible
```

这样 M2 开始实现 Rename / Organizer 时，不需要重新设计。

---

# 22. History Contract

M0 只建立概念边界。

未来：

```text
Operation
 ↓
History Record
 ↓
Undo
```

History 不等于：

```text
full database engine
```

M0 不实现完整历史系统。

只定义未来能使用的基础 contract。

OperationTransaction 统一模型在本里程碑只建立概念边界，字段级定义推迟到 M2 首次落地（要求见 charter #12：Original Path / New Path / Original Metadata / Generated File / Changed File / Operation Type / Timestamp）。

该推迟决定必须记入 DECISIONS.md。

Preview.reversible 与 History 必须为 M2 事务模型预留衔接。

---

# 23. Test Architecture

建立：

```text
weave-testkit
```

目标：

> 以后所有工具尽可能共享同一套测试基础设施。

---

# 23.1 Testkit 内容

至少建立：

```text
Temporary Workspace
Fixture Builder
Deterministic Random Helper
Fault Injection Framework
Test Path Builder
Test Clock（如确实有需要）
```

---

# 23.2 Temporary Workspace

测试可以：

```text
create temp directory
create nested structure
create files
modify files
cleanup
```

禁止测试依赖：

```text
C:\Users\真实用户名\Desktop\...
```

禁止：

```text
用户真实文件
```

进入测试。

---

# 23.3 Fault Injection

建立统一 Fault Injection 机制。

未来至少支持模拟：

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

这些失败场景是 Weave 文件安全模型的一部分。总纲领明确要求文件操作进行故障注入测试，目标是即使失败也不能把用户文件搞坏。

M0 只建立 framework。

不要现在实现所有场景。

---

# 23.4 Golden Fixture Infrastructure

建立：

```text
tests/fixtures/
```

未来用于：

```text
JSON
CSV
Text
Documents
Images
```

建立统一命名约定：

```text
input/
expected/
metadata/
```

不要在 M0 放真实用户文件。

可以放：

```text
tiny deterministic fixture
```

---

# 24. Frontend Test Architecture

UI 使用：

```text
unit/component tests
```

技术选型以实际 React/Vite 生态兼容方案为准。

M0 至少验证：

```text
App renders
Brand renders
Language resource resolves
IPC smoke call works
Error state renders
```

---

# 25. CI

建立：

```text
.github/workflows/
```

至少：

```text
ci.yml
```

Windows runner：

```text
windows-latest
```

CI 必须配置缓存：

```text
cargo registry/cache
cargo target 目录
npm cache（或等价机制）
```

Windows runner 上 tauri build 全量编译很慢，无缓存的流水线不可接受。

---

# 25.1 CI Gates

Rust：

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

Frontend：

```text
npm run typecheck
npm run lint
npm test
```

Application：

```text
tauri build
```

License：

```text
cargo deny check
frontend license check
```

---

# 26. cargo-deny

建立：

```text
deny.toml
```

定义项目允许的 license policy。

Weave 默认允许：

```text
MIT
Apache-2.0
BSD-2-Clause
BSD-3-Clause
ISC
Zlib
MPL-2.0
CC0-1.0
Unicode-3.0
CDLA-Permissive-2.0
OFL-1.1
```

允许列表相对 charter #40 的扩展（MPL-2.0 / CDLA-Permissive-2.0 / OFL-1.1 / Unicode-3.0）必须在 deny.toml 注释或 DECISIONS.md 中说明理由。

默认拒绝：

```text
GPL
AGPL
LGPL
```

具体许可证策略必须以 Project Charter 为准。

---

# 26.1 许可证检查

必须同时考虑：

```text
Rust dependencies
Node dependencies
transitive dependencies
```

不能只看：

```text
Cargo.toml
package.json
```

必须审查：

```text
实际 dependency graph
```

---

# 26.2 THIRD_PARTY_LICENSES.md

M0 不要求最终版完整第三方许可证清单。

但必须建立：

```text
THIRD_PARTY_LICENSES.md
```

以及生成机制的基础。

明确未来策略：

```text
automatically generated
CI verified
not manually guessed
```

---

# 27. README

建立 README。

至少包含：

```text
What is Weave?
Core philosophy
Tech stack
Development setup
Repository structure
Architecture
Privacy principle
License
Development workflow
Current milestone
Known limitations
```

README 必须：

```text
中文
+
English
```

但不要在 M0 编写一篇 5000 行 README。

---

# 28. ARCHITECTURE.md

这是 M0 最重要的文档之一。

必须明确写：

```text
UI
 ↓
IPC
 ↓
Application
 ↓
Core
 ↓
Adapters
 ↓
OS
```

以及：

```text
weave-core
weave-files
weave-text
weave-data
weave-documents
weave-media
weave-batch
weave-history
weave-search
weave-testkit
```

的职责。

---

# 28.1 依赖方向

必须明确：

```text
Core
不会依赖 UI

Domain
不会依赖 React

Rust library crates
不应该依赖 Tauri，除非确有必要

Tauri
负责 desktop integration / IPC

UI
只通过 contract 与 Rust 通信
```

---

# 29. DECISIONS.md

记录 M0 所有真正有意义的技术决策。

至少记录：

```text
为什么选择 Tauri 2
为什么使用 Cargo Workspace
crate 边界为什么这样划分
IPC 为什么使用当前方案
版本策略
license policy
path safety policy
error model
logging model
testkit design
design token strategy
brand source of truth
```

不要记录：

```text
“今天我创建了一个文件”
```

只记录：

> 会影响未来架构的决定。

---

# 30. PROGRESS.md

创建：

```text
PROGRESS.md
```

格式建议：

```text
# Weave Development Progress

## M0 Foundation

Status: IN PROGRESS

### Infrastructure
- [ ] Repository baseline
- [ ] Cargo workspace
- [ ] Tauri shell
- [ ] React shell
- [ ] IPC
- [ ] Design tokens
- [ ] CI
- [ ] cargo-deny
- [ ] Testkit
- [ ] Documentation

### Quality Gates
- [ ] fmt
- [ ] clippy
- [ ] cargo test
- [ ] typecheck
- [ ] lint
- [ ] frontend test
- [ ] tauri build
- [ ] license audit
```

每完成一个小阶段就更新。

---

# 31. CHANGELOG.md

建立：

```text
Unreleased
```

并记录：

```text
M0 Foundation initialized
```

不要把所有内部实现细节塞进 CHANGELOG。

---

# 32. Git

初始化 Git。

建立合理：

```text
.gitignore
```

至少排除：

```text
target/
node_modules/
dist/
build/
.env
.env.*
IDE metadata
OS metadata
logs
temporary files
```

但：

> 不要误伤项目需要提交的配置和 lock files。

---

# 32.1 Git Hygiene

最终检查：

```text
git status
git diff
git ls-files
```

确认不存在：

```text
secret
token
password
private key
local absolute path
personal environment file
generated junk
```

---

# 33. Frontend Structure

建立：

```text
ui/
├── src/
│   ├── app/
│   ├── components/
│   ├── features/
│   ├── pages/
│   ├── commands/
│   ├── stores/
│   ├── design/
│   ├── i18n/
│   ├── generated/
│   └── lib/
```

不要一开始创建：

```text
100 个空文件
```

只建立真实会被 M0 使用的结构。

---

# 34. Feature Boundary

未来 feature 应尽量：

```text
features/
├── files/
├── text/
├── data/
├── documents/
├── media/
├── batch/
└── utilities/
```

M0 可以先只有：

```text
foundation/
```

或者：

```text
app/
```

不要提前实现未来工具。

---

# 35. Command Registry

建立前端 Command Registry 的最小版本。

未来：

```text
Ctrl+K
 ↓
Command Registry
 ↓
Action
 ↓
Tool
```

M0 至少允许：

```text
Show App Info
Run IPC Ping
```

用于验证架构。

---

# 36. No Feature Creep

M0 不要做：

```text
漂亮 Home 页面
File Rename UI
Image editor
CSV table
PDF viewer
Command palette full UX
History UI
Undo UI
Drag-and-drop production UX
```

这些都留给正确的里程碑。

M0 的 UI 只是：

> 验证“应用能启动、UI 能工作、IPC 能工作、基础设计系统能工作”。

---

# 37. Drag & Drop

M0 可以验证：

```text
desktop app receives a dropped file path
```

但不要开始做：

```text
Rename
Organizer
Image Processor
```

可以只完成：

```text
Dropped
 ↓
IPC
 ↓
Rust validation
 ↓
return metadata
 ↓
UI display
```

这样 M0 就提前证明核心架构真的能够承载 Weave 的主交互：

> Drop → Understand。

---

# 38. Minimal File Probe

可以建立一个非常小的基础 command：

```text
inspect_path
```

只做：

```text
exists
kind: file/directory
name
extension
size
```

但：

> 不把它做成正式 File Inspector 工具。

它只是：

```text
M0 architectural smoke test
```

这个 command 必须：

```text
validate path
handle errors
return typed result
```

并通过测试。

---

# 39. Path Probe Tests

至少覆盖：

```text
valid file
valid directory
missing path
relative path
normalized path
parent traversal attempt
invalid path
permission/error path
```

测试必须 deterministic。

---

# 40. App Lifecycle

M0 必须确认：

```text
startup
shutdown
restart
```

不会产生：

```text
panic
corrupted config
unhandled error
```

建立一个简单的：

```text
AppState
```

用于未来：

```text
settings
registries
operation manager
history
```

但不要把所有未来系统塞进里面。

---

# 41. Configuration

建立最小配置模型。

例如：

```text
language
theme
```

未来可扩展。

配置必须：

```text
versioned
validated
migration-ready
```

不要直接：

```text
serde_json::Value
```

作为整个应用配置。

---

# 42. Persistence

M0 不需要 SQLite。

不要为了：

```text
“以后肯定会用”
```

现在就把所有数据库系统装进项目。

简单配置：

```text
JSON / structured config
```

即可。

SQLite 留到真正需要的时候。

---

# 43. Dependency Discipline

M0 添加任何依赖前问：

```text
1. 当前阶段真的需要吗？
2. Rust 标准库是否已经可以完成？
3. 该依赖维护情况如何？
4. license 是否允许？
5. Windows 是否稳定？
6. CI 是否能构建？
7. 它是否会污染核心架构？
8. AI 后续是否容易理解？
```

如果答案不明确：

> 不加。

---

# 44. Generated Code

如果使用生成代码：

```text
ui/src/generated/
```

明确区分：

```text
generated
handwritten
```

禁止：

```text
生成代码混入业务手写文件
```

并记录：

```text
generation command
```

在 README 中。

---

# 45. Formatting

统一：

```text
rustfmt
eslint
prettier
```

不要让团队成员/AI agent 各自使用不同风格。

CI 必须验证格式。

---

# 46. Lint

Rust：

```text
clippy
```

TypeScript：

```text
eslint
```

同时加入：

```text
no-floating-promises
no-explicit-any
unused imports
unused variables
```

等合理规则。

ESLint 必须启用 jsx-a11y 推荐规则集作为无障碍基线（charter #35；完整 a11y 验收在 M11，但基线从 M0 起生效）。

但是：

> 不要为了“规则越多越专业”而建立几十条难以维护的 lint 规则。

---

# 47. Test Pyramid

M0 建立测试结构：

```text
Unit
 ↓
Integration
 ↓
Contract
 ↓
Smoke
```

后续：

```text
Golden
Fault Injection
Property
E2E
```

---

# 48. Determinism

所有 M0 测试尽可能做到：

```text
same input
+
same environment assumptions
=
same result
```

避免：

```text
real clock
random UUID assertions
real filesystem locations
network
user locale
machine-specific paths
```

---

# 49. Offline Principle

M0 CI 测试不依赖外部网络服务。

尤其禁止：

```text
测试通过 HTTP 请求某个在线 API
```

项目本体虽然未来可以拥有一些 [B] 在线能力，但普通工具必须保持 local-first。

---

# 50. Security Smoke Tests

至少建立：

```text
IPC validation
path traversal rejection
invalid command arguments
malformed JSON
unknown tool ID
invalid operation ID
```

不允许：

```text
panic
```

---

# 51. UI Security

确认：

```text
strict CSP
no remote JS
no remote CSS
no remote font
no arbitrary external script
```

并确认：

```text
Tauri capabilities
```

只允许当前阶段真正需要的权限。

---

# 52. Network Baseline

M0：

> 不实现网络功能。

也不要加入：

```text
axios
unnecessary fetch wrappers
remote API clients
analytics SDK
telemetry SDK
```

除非当前架构真正需要。

---

# 53. Telemetry

明确：

```text
No Telemetry
```

M0 不得加入任何：

```text
analytics
tracking
crash upload
usage reporting
```

---

# 54. Build Reproducibility

确认：

```text
fresh checkout
↓
install dependencies
↓
build
```

能够成功。

不能依赖开发者电脑上的：

```text
custom absolute paths
local npm package links
local Rust crates
```

---

# 55. Windows Baseline

M0 主要目标平台：

```text
Windows
```

至少确认：

```text
Windows path behavior
CRLF / LF behavior
file name normalization
UTF-8 paths
Unicode filenames
```

不要假设：

```text
Linux filesystem semantics
```

完全等价于 Windows。

---

# 56. Architecture Validation

M0 完成前，对代码依赖进行一次真实检查。

你必须验证：

```text
UI
 ↓
IPC
 ↓
Application
 ↓
Core
 ↓
Adapter
 ↓
OS
```

确实成立。

不要只在 ARCHITECTURE.md 里写。

必须检查：

```text
cargo tree
imports
dependencies
TypeScript imports
Tauri commands
```

发现绕层：

> 立即修复。

---

# 57. Circular Dependency

检查：

```text
crate dependency cycle
module dependency cycle
frontend feature cycle
generated type cycle
```

禁止出现。

---

# 58. M0 Smoke User Flow

M0 至少完成以下真实流程：

```text
Launch Weave
 ↓
Display brand
 ↓
Display localized UI
 ↓
UI calls Rust IPC
 ↓
Rust returns typed data
 ↓
UI renders result
 ↓
Drop a test file
 ↓
Rust validates path
 ↓
UI displays metadata
```

注意：

> 这里的文件处理只是“探测”，不是正式 File Inspector 工具。

---

# 59. M0 Acceptance Test

执行：

## Repository

```text
git status
```

必须干净。

---

## Rust

```text
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

全部 PASS。

---

## Frontend

```text
npm run typecheck
npm run lint
npm test
```

全部 PASS。

---

## Tauri

```text
tauri build
```

必须成功。

---

## License

```text
cargo deny check
```

必须成功。

---

## Runtime

真实启动桌面程序。

确认：

```text
Window opens
UI renders
IPC ping works
App info works
Path probe works
Drop smoke test works
```

---

# 60. M0 必须有测试的项目

最低要求：

```text
[ ] ToolId validation
[ ] OperationId creation
[ ] JobId creation
[ ] Error serialization
[ ] IPC contract
[ ] IPC ping
[ ] App info
[ ] Path validation
[ ] Path traversal rejection
[ ] temporary workspace
[ ] fixture creation
[ ] fault injection mechanism
[ ] i18n resource loading
[ ] design token validation
[ ] configuration serialization
```

---

# 61. M0 不追求覆盖率数字

不要为了数字写垃圾测试。

优先：

```text
architecture correctness
contract correctness
security correctness
determinism
build reliability
```

覆盖率可以在后续里程碑逐步提高。

---

# 62. Documentation Completion

M0 结束时必须存在：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
SECURITY.md
BRAND.md
CHANGELOG.md
THIRD_PARTY_LICENSES.md
```

这些文件不能是空壳。

至少写真实内容。

---

# 63. BRAND.md

写明：

```text
ifuyo
Weave
brand ownership
brand asset policy
```

不要把：

```text
官方 logo
私有品牌资源
```

随意塞进仓库。

---

# 64. Project Naming

统一：

```text
ifuyo Weave
Weave
weave-core
weave-files
...
```

不要出现：

```text
MyApp
tauri-app
test-project
desktop-tool
```

之类脚手架残留。

---

# 65. Remove Boilerplate

初始化模板中的：

```text
default Tauri demo content
unused React demo components
unused CSS
favicon leftovers
placeholder images
example counters
```

全部清理。

但是：

> 只能删除确认属于脚手架模板的内容。

---

# 66. No Premature UI

不要在 M0 创建大量：

```text
Toolbar
Sidebar
ToolGrid
DataTable
Inspector
Modal
HistoryPanel
BatchPanel
```

因为这些东西应该在相应 M 中根据真实需求建立。

M0 只建立：

```text
AppShell
```

---

# 67. No Premature State Management

Zustand 可以接入。

但 M0 只允许真正必要的状态：

```text
app info
locale
theme
connection/smoke state
```

不要创建：

```text
useFileStore
useBatchStore
useHistoryStore
useWorkflowStore
useImageStore
```

等未来没有真实使用场景的 store。

---

# 68. No Premature Database

不要在 M0：

```text
SQLite
PostgreSQL
IndexedDB
```

全部加入。

Weave 是 local-first，但 local-first 不等于：

> “所有东西都必须第一天数据库化。”

---

# 69. No Premature Plugin System

M0 禁止设计：

```text
Plugin SDK
Extension System
Dynamic Module Loader
Third-party Tool Marketplace
```

Weave 的插件化是远期 [C]。

当前必须先证明核心工具架构。

---

# 70. AI Coding Suitability

项目必须适合 AI agent 持续开发。

所以每个边界都应该：

```text
small
explicit
typed
documented
testable
```

AI 后续修改某个工具时：

> 不应该要求重新理解整个仓库。

因此：

```text
Core
Tool
Adapter
Application
UI
Test
```

边界必须清晰。

---

# 71. M0 Audit Questions

完成实现后逐项回答：

### Architecture

```text
[ ] UI 是否绕过 IPC？
[ ] React 是否直接操作 fs？
[ ] Core 是否依赖 Tauri？
[ ] 是否出现 circular dependency？
[ ] Tool contract 是否过度设计？
[ ] 是否存在未来功能提前实现？
```

### Security

```text
[ ] capabilities 是否最小化？
[ ] CSP 是否合理？
[ ] 是否存在 unrestricted filesystem？
[ ] 是否存在 shell access？
[ ] path traversal 是否被拒绝？
[ ] 日志是否可能泄露敏感信息？
```

### Maintainability

```text
[ ] 一个 AI agent 能否只阅读局部模块进行修改？
[ ] 是否出现巨型文件？
[ ] 是否出现万能 manager？
[ ] 是否出现 God Object？
[ ] 是否存在大量 any？
[ ] 是否存在神奇全局变量？
```

### Product

```text
[ ] M0 是否仍然是 Weave，而不是另一个产品？
[ ] 是否保持 local-first？
[ ] 是否保持 utility-first？
[ ] 是否避免功能膨胀？
```

---

# 72. M0 反模式扫描

特别搜索：

```text
TODO
FIXME
HACK
XXX
unwrap()
expect(
panic!(
unsafe
any
as any
console.log
println!
eprintln!
```

注意：

并不是所有出现都一定是错误。

你必须逐项判断：

```text
production path
test path
intentional documented use
```

然后在 PROGRESS.md 中注明关键例外。

不能简单粗暴删除。

---

# 73. Large File Audit

M0 完成后检查：

```text
最大的 Rust 文件
最大的 TSX 文件
最大的 TS 文件
```

如果已经出现：

```text
> 500 lines
```

必须检查是否存在职责聚合问题。

M0 的原则：

> 不允许在脚手架阶段制造巨型文件。

---

# 74. Dependency Audit

输出：

```text
cargo tree
npm dependency tree
```

确认：

```text
no unexpected dependencies
no duplicate heavy stack
no forbidden license
no suspicious package
```

如果新增依赖没有必要：

> 删除。

---

# 75. Git Commit

只有：

```text
all quality gates PASS
```

之后才能提交。

Commit 格式：

```text
M0: establish project foundation
```

或者：

```text
M0: establish Tauri workspace and architecture
```

要求：

```text
single logical M0 commit
```

如果实际开发过程中确实需要多个提交，也必须保持每个提交逻辑清晰，但最终 M0 收口必须明确。

---

# 76. PROGRESS.md 最终状态

M0 最终写成：

```text
## M0 Foundation

Status: COMPLETE

Implemented:
- Cargo workspace
- Tauri 2 shell
- React + TypeScript
- IPC contract
- brand.json
- design tokens
- i18n baseline
- path safety baseline
- error model
- testkit
- CI
- cargo-deny
- documentation baseline

Quality:
- cargo fmt: PASS
- cargo clippy: PASS
- cargo test: PASS
- frontend typecheck: PASS
- frontend lint: PASS
- frontend test: PASS
- tauri build: PASS
- license audit: PASS

Commit:
<actual commit hash>
```

所有内容必须是真实结果。

严禁编造：

```text
PASS
```

---

# 77. 最终 M0 Gate

只有全部满足才允许宣布：

```text
M0 COMPLETE
```

### Gate A — Architecture

```text
PASS
```

### Gate B — Build

```text
PASS
```

### Gate C — Test

```text
PASS
```

### Gate D — Security Baseline

```text
PASS
```

### Gate E — License

```text
PASS
```

### Gate F — Documentation

```text
PASS
```

### Gate G — Runtime Smoke

```text
PASS
```

### Gate H — Git Hygiene

```text
PASS
```

---

# 78. M0 完成定义

最终状态必须接近：

```text
                    ┌─────────────────────┐
                    │     React / TS      │
                    └──────────┬──────────┘
                               │
                            IPC Contract
                               │
                    ┌──────────▼──────────┐
                    │    Tauri / App      │
                    └──────────┬──────────┘
                               │
                    ┌──────────▼──────────┐
                    │    Application      │
                    └──────────┬──────────┘
                               │
             ┌─────────────────┼─────────────────┐
             │                 │                 │
       weave-core         testkit          future tools
             │
             └─────────────────┬─────────────────┐
                               │                 │
                           adapters          OS layer
```

并且：

```text
                Weave
                  │
       ┌──────────┼──────────┐
       │          │          │
     Local      Typed      Testable
     First       IPC        Core
```

---

# 79. M0 最重要的判断标准

不要问：

> “M0 做了多少代码？”

要问：

> **“M1～M14 是否可以建立在 M0 上，而不需要推翻基础架构？”**

M0 的成功不是：

```text
10000 行代码
```

而是：

```text
少量代码
+
稳定边界
+
清晰 contract
+
可信测试
+
可重复构建
+
安全默认值
```

---

# 80. 现在开始执行

按照以下顺序严格执行：

```text
STEP 1
Baseline Audit

↓

STEP 2
Initialize Git / repository hygiene

↓

STEP 3
Initialize Tauri 2 + React + TypeScript + Vite

↓

STEP 4
Create Cargo Workspace

↓

STEP 5
Create Core / Adapter / Testkit boundaries

↓

STEP 6
Implement minimal domain contracts

↓

STEP 7
Implement Tauri application shell

↓

STEP 8
Implement typed IPC

↓

STEP 9
Implement brand.json

↓

STEP 10
Implement design tokens

↓

STEP 11
Implement i18n baseline

↓

STEP 12
Implement path safety + error baseline

↓

STEP 13
Implement logging / privacy / security baseline

↓

STEP 14
Implement testkit

↓

STEP 15
Implement CI

↓

STEP 16
Implement cargo-deny / license baseline

↓

STEP 17
Write / update documentation

↓

STEP 18
Run complete audit

↓

STEP 19
Fix all P0 / P1 issues

↓

STEP 20
Run all quality gates

↓

STEP 21
Commit M0

↓

STEP 22
Update PROGRESS.md

↓

STEP 23
Report M0 final result
```

---

# 81. Final reporting format

完成后，不要只说：

```text
M0 完成
```

必须输出：

```text
# M0 Final Report

## A. Baseline
事实

## B. Architecture
实际 crate / module / IPC 边界

## C. Implemented
真实完成内容

## D. Tests
每条真实结果

## E. Security
真实检查结果

## F. License
真实检查结果

## G. Build
真实构建结果

## H. Git
branch / commit / clean state

## I. Decisions
本阶段关键取舍

## J. Known Limitations
明确列出仍未实现内容

## K. M0 Gate
PASS / FAIL

## L. Commit
<hash>
```

禁止使用：

```text
probably
should work
looks good
likely pass
```

除非明确标注为：

```text
HYPOTHESIS
```

---

# 82. 最终原则

整个 M0 永远遵守：

> **先建立骨架，再建立规则，再建立验证，最后才允许规模化开发。**

Weave 后续的核心工具、Batch Engine、Preview、History、Undo、Workflow，都必须建立在这个 M0 基础之上。

不要为了让 M0 “看起来丰富”，提前把未来功能塞进来。

真正合格的 M0 应该让下一阶段非常容易开始：

```text
M0
Foundation
 ↓
M1
File Core
 ↓
M2
Rename / Organizer
 ↓
M3
Duplicate Finder
 ↓
...
```

并且任何一个未来工具，都能按照：

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

持续加入，而无需破坏既有核心。

> **开始执行 M0。**
> 
> **自主解决歧义。**
> 
> **以实际代码、实际测试、实际构建结果为唯一完成依据。**
> 
> **未通过 Gate 不得宣布 M0 COMPLETE。**