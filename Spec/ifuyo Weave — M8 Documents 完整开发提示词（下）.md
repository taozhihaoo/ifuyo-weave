# ifuyo Weave — M8 Documents 完整开发提示词（下）

> 本文件是原《ifuyo Weave — M8 Documents 完整开发提示词》的 **第 2/2 部分**，
> 覆盖原第 119–244 节：UI 信息架构与全部 UI/UX 及 a11y/i18n、崩溃恢复/Resume/孤儿输出清理、Test Fixture 与 Golden/Fuzz/集成测试、性能基准与并发调度、安全/隐私/许可证审计、应用分层与 Documentation、回归与反模式、推荐实施顺序（Stage 1–10）、Final Audit A–P、验收标准与提交纪律、M9/M10 Handoff 与最终执行要求。
> **必须与（上）一起阅读执行**：硬边界、产品原则、文档领域模型与实现规范均在（上）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

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