# ifuyo Weave — M6 Image 完整开发提示词（下）

> 本文件是原《ifuyo Weave — M6 Image 完整开发提示词》的 **第 2/2 部分**，
> 覆盖原第 150–331 节：Image Quality Tests 与 Pixel/Semantic Equality、Lossless/Lossy/Resize/Upscale/Alpha/Orientation/Metadata 测试、Format Test Matrix、Real Image Fixtures 与禁止提交真人照片、Deterministic Fixtures、Golden Image、Codec Regression、Fuzz Testing 与资源上限、Crash Safety，Partial Batch Results/Retry Boundary/Progress，UI 不阻塞、IPC/Large Preview Transfer、UI State，Preview Component/Checkerboard/Zoom/Orientation/Metadata UI 与 Image Tool UI（Resize/Compress/Convert/Metadata/Multi-image），User Must Know Scope、Export Scope/Collision、Safe Source Replacement/Hash、Reversible-first/No Destructive Default/File Deletion、Batch Folder Safety/Input Snapshot/External Change，Disk Full/File Locked/Read-only/Symlink/Temp File Safety，错误分类与 Metadata 失败隔离、Color Conversion/ICC/EXIF Orientation 与各类 Metadata Policy、Metadata Loss Report，History 不存 Metadata/Pixel，Image Result/Single Result/Output Open/Reuse Operation、Command Layer/IPC DTO/Preview Handle，Accessibility/Preview Accessibility、i18n 与单位/维度格式化，Performance Baseline 与各类 Benchmark、Memory/Leak、Fault Injection、Security/Privacy Audit、M5/M6 Regression、Dependency Isolation，Documentation 与 IMAGE_FORMATS/DECISIONS/SECURITY/PRIVACY/PROGRESS，Baseline Audit、Gap Analysis、Implementation Order、Phase 1–12，Unit/Property/Integration/UI Smoke/Real Preview/Image Diff 测试层。
> **执行与收尾条款在本部分**：资源限制文档与各类 UX、Batch Summary/Retry Failed/Transaction Honesty、Commit（提交纪律）、Final Audit、M6 不算完成的情况、Final Implementation Report、Git Hygiene、M6 Handoff（交接）、最终产品状态、核心原则、终极目标、执行纪律、最终完成条件与 M6 → M7 的最终边界。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---
# 150. Image Quality Tests

不要只做：

```text
dimensions equal
```

必须视操作测试：

```text
alpha preserved
orientation correct
format correct
metadata policy correct
```

对于 lossy：

可以验证：

```text
structural validity
reasonable size
```

但不要伪装：

```text
perceptual quality score
```

除非真正实现并有标准。

---

# 151. Pixel Equality

对于：

```text
PNG lossless
```

可以测试：

```text
pixel equality
```

但注意：

```text
metadata
color profile
orientation
```

可能不同。

因此：

```text
pixel equality
```

与：

```text
byte equality
```

必须分开。

---

# 152. Semantic Image Equality

至少区分：

```text
Byte Equal
Pixel Equal
Visual-equivalent within transform
Metadata Equal
```

不要把：

```text
same dimensions
```

当作：

```text
same image
```

---

# 153. Lossless Tests

至少验证：

```text
PNG → PNG
```

在：

```text
lossless mode
```

下的：

```text
decoded pixels
```

保持一致。

---

# 154. Lossy Tests

至少验证：

```text
JPEG quality 95
JPEG quality 80
JPEG quality 50
```

真实 output：

```text
size
decode success
dimensions
```

并记录：

```text
quality parameter semantics
```

---

# 155. Resize Tests

至少：

```text
4000×3000 → 2000×1500
4000×3000 → 1920 fit
4000×3000 → 1920 fill
4000×3000 → 1920×1080 exact
```

---

# 156. Upscale Tests

至少：

```text
800×600 → 1600×1200
```

验证：

```text
prevent upscale on
prevent upscale off
```

---

# 157. Alpha Tests

至少：

```text
transparent PNG
semi-transparent PNG
PNG → JPEG
PNG → WebP
```

---

# 158. Orientation Tests

至少覆盖：

```text
EXIF orientation 1
3
6
8
```

确保：

```text
preview
resize
convert
strip metadata
```

结果一致。

---

# 159. Metadata Tests

至少：

```text
EXIF present
GPS present
ICC present
No metadata
Malformed metadata
Large metadata
```

---

# 160. Strip Metadata Tests

验证：

```text
strip EXIF
strip GPS
strip all removable metadata
preserve ICC
```

每项：

必须检查：

```text
actual output metadata
```

不是：

```text
assume encoder did it
```

---

# 161. Format Test Matrix

建立真实测试矩阵：

```text
                Decode Encode Resize Compress Metadata Convert
PNG
JPEG
WebP
GIF
BMP
TIFF
```

每个格子：

```text
PASS
FAIL
NOT SUPPORTED
NOT VERIFIED
```

---

# 162. Real Image Fixtures

测试数据必须包括：

```text
small PNG
large PNG
transparent PNG
JPEG camera-like
JPEG orientation variants
WebP
GIF static
GIF animated
BMP
TIFF
```

并包括：

```text
corrupted files
truncated files
wrong extension
huge dimension headers
```

---

# 163. Do Not Commit Real Personal Photos

不要提交：

```text
真实用户照片
真实家庭照片
真实 GPS 数据
真实身份证照片
真实医院照片
真实客户照片
```

测试 fixtures：

优先：

```text
synthetic/generated
```

或者：

```text
license-compatible public fixture
```

---

# 164. Deterministic Fixtures

能程序生成的：

优先：

```text
seeded deterministic generation
```

例如：

```text
solid color
checkerboard
gradient
alpha circles
geometric shapes
```

这样：

```text
test repeatable
```

---

# 165. Golden Image Tests

可以使用：

```text
golden pixel fixtures
```

但：

必须考虑：

```text
platform differences
codec version differences
floating point differences
```

因此不能盲目：

```text
byte-for-byte golden
```

对于：

```text
lossy codecs
```

尤其如此。

---

# 166. Codec Regression

升级依赖后：

必须重新验证：

```text
decode
encode
dimensions
metadata
alpha
```

防止：

```text
crate update
→
behavior changed
→
tests still superficial
```

---

# 167. Fuzz Testing

Image parser 非常适合：

```text
fuzz
```

至少关注：

```text
PNG headers
JPEG markers
WebP chunks
GIF blocks
BMP headers
TIFF IFD
EXIF
```

目标：

```text
no panic
no UB
bounded resource use
structured failure
```

---

# 168. Fuzz Resource Limits

Fuzz 不是：

```text
无限输入
```

必须：

```text
bounded corpus
bounded execution
bounded allocations
```

避免：

```text
CI hang
```

---

# 169. Crash Safety

至少测试：

```text
decode failure
encode failure
output disk failure
cancel during encode
cancel before commit
source changed
destination collision
metadata parsing failure
```

---

# 170. Partial Batch Results

如果：

```text
100 images
```

其中：

```text
98 success
2 fail
```

Result 页面必须：

```text
告诉用户 98/100 完成
```

并允许：

```text
查看失败项
```

如果当前架构支持：

```text
Retry Failed
```

可以复用 M2/M7-compatible operation abstraction，但不要因此实现通用 retry engine。

---

# 171. Retry Boundary

M6 可以允许：

```text
Retry Failed Images
```

但只能：

```text
same Image Operation
same options
failed inputs
```

不能变成：

```text
arbitrary retry workflow
```

Pause / Resume 属 M7 Batch Engine 范围，M6 仅提供 Cancel + Retry Failed（对齐 charter #26 的口径由 M7 统一补齐）。

---

# 172. Progress

单图：

```text
Decode
Transform
Encode
Commit
```

多图：

```text
12 / 100
```

如果内部阶段有可靠进度：

可以：

```text
Decode 40%
Encode 60%
```

否则：

不要伪造：

```text
57%
```

---

# 173. Progress Granularity

多图：

至少：

```text
Processed
Remaining
Succeeded
Failed
Skipped
```

必要时：

```text
Current File
Current Stage
```

---

# 174. Estimated Time

可以显示：

```text
ETA
```

但只能在：

```text
reliable throughput estimate
```

存在时使用。

不要：

```text
第一次处理
→
ETA 00:01
```

但实际：

```text
10 minutes
```

---

# 175. UI 不阻塞

以下全部不能冻结 React：

```text
large image decode
large resize
large export
multi-image processing
metadata parsing
```

---

# 176. IPC

不要：

```text
RGBA pixels
→
JSON stringify
→
IPC
→
React
```

传输完整图像数据。

Preview：

优先：

```text
bounded binary / asset URL / controlled preview handle
```

遵循当前 Tauri architecture。

---

# 177. Large Preview Transfer

如果必须跨 IPC：

必须：

```text
bounded
```

并避免：

```text
base64 huge image
```

造成：

```text
memory amplification
```

---

# 178. UI State

React 只保存：

```text
Image Facts Summary
Preview Handle
Current Options
Selection State
Progress State
Result Summary
```

不要长期保存：

```text
decoded megapixel buffers
```

---

# 179. Image Preview Component

至少支持：

```text
Fit
Actual Size where feasible
Zoom
Pan
Background
```

但不要扩展成：

```text
full image editor canvas
```

---

# 180. Transparency Checkerboard

透明图片 Preview 应有：

```text
checkerboard background
```

避免：

```text
transparent = white
```

导致用户误判。

同时提供：

```text
Background:
Checkerboard
White
Black
```

用于预览。

---

# 181. Zoom Limits

必须：

```text
bounded
```

例如：

```text
25%
50%
100%
200%
400%
```

根据实际实现。

不能：

```text
无限 zoom
```

---

# 182. Orientation Preview

Preview 必须：

```text
honor orientation
```

否则：

```text
camera JPEG
```

可能显示横竖错误。

---

# 183. Metadata UI

推荐：

```text
Overview
Dimensions
Color
Metadata
Privacy
```

例如：

```text
Privacy
GPS location: Present
Camera information: Present
Creation time: Present

[Strip GPS]
[Strip All Metadata]
```

---

# 184. Image Tool UI

建议：

```text
Image
├── Inspector
├── Resize
├── Compress
├── Convert
└── Metadata
```

但必须遵循当前项目现有 UI shell。

---

# 185. Resize UI

建议：

```text
Width
Height
Lock Aspect Ratio
Mode
Resampling
Prevent Upscale
Background
```

实时显示：

```text
Output Dimensions
```

---

# 186. Compress UI

建议：

```text
Format
Lossless / Lossy
Quality / Compression
Estimated Size
Metadata Policy
```

---

# 187. Convert UI

建议：

```text
Output Format
Resize
Alpha Policy
Metadata Policy
Quality
Advanced
```

但默认：

```text
只显示必要选项
```

高级配置：

```text
collapsed
```

---

# 188. Metadata UI

建议：

```text
Detected Metadata
Privacy-sensitive fields
Removal options
Preview changes
Apply / Export
```

---

# 189. Multi-image UI

建议：

```text
Files: 100

Selected Operation:
Resize

Settings:
1920 max width
Keep Aspect Ratio

Preview:
12 representative images

Expected:
100 outputs

[Preview]
[Process]
```

---

# 190. User Must Know Scope

多图处理必须显示：

```text
100 files selected
```

不要：

```text
UI 只显示 “Batch”
```

却不告诉用户：

```text
到底有多少文件
```

---

# 191. Mixed Supported/Unsupported

例如：

```text
100 selected
96 supported
4 unsupported
```

UI：

必须显示：

```text
96 ready
4 unsupported
```

---

# 192. Export Scope

多图输出必须明确：

```text
Input Scope
Output Folder
Output Format
Overwrite Policy
```

---

# 193. Same Extension Collision

例如：

```text
photo.jpg
```

执行：

```text
Resize
Output = same folder
```

目标：

```text
photo.jpg
```

默认：

必须视为：

```text
collision
```

不能假设：

```text
“这是同一个文件”
```

然后直接覆盖。

---

# 194. Safe Source Replacement

如果用户选择：

```text
Replace Source
```

必须：

```text
read source
→
verify source unchanged
→
write temp
→
verify output
→
atomic replace
```

---

# 195. Source Identity Verification

至少检查：

```text
Mtime
Size
Expected identity
```

必要时：

```text
Hash
```

不能：

```text
仅靠 path
```

---

# 196. Image Hash

可以在 output verification 使用：

```text
SHA-256
```

复用：

```text
M1 Hash abstraction
```

但不要：

```text
为 M6 建立 duplicate detection
```

---

# 197. Image Metadata Hash

不要：

```text
hash only pixels
```

就宣布：

```text
image identical
```

因为：

```text
metadata
```

不同。

---

# 198. Reversible-first

默认：

```text
Export New
```

优先：

```text
Replace Source
```

---

# 199. No Destructive Default

默认不：

```text
overwrite
delete
replace
```

---

# 200. File Deletion

M6 不需要自己的：

```text
Delete
```

如果输出需要清理：

使用：

```text
M2 existing safe filesystem semantics
```

不能新增：

```text
permanent delete fallback
```

---

# 201. Batch Folder Safety

如果输入：

```text
folder
```

输出：

```text
same folder
```

特别注意：

不要：

```text
enumerate output file
→
process output again
→
infinite loop
```

必须：

```text
input snapshot
```

在开始处理前确定。

---

# 202. Input Snapshot

多文件任务必须：

```text
snapshot inputs
```

然后：

```text
process snapshot
```

而不是：

```text
实时扫描文件夹
```

导致：

```text
new output enters same job
```

---

# 203. Stable Input Identity

每个 input 至少：

```text
Path
Size
Mtime
```

最好复用：

```text
M1 file identity
```

---

# 204. External File Change

处理前或提交前如果：

```text
source changed
```

应：

```text
Conflict
```

而不是：

```text
process latest silently
```

---

# 205. Output Folder Changed

如果目标：

```text
deleted
```

或者：

```text
permissions changed
```

必须：

```text
structured failure
```

---

# 206. Disk Full

必须测试：

```text
output write fails due to insufficient storage
```

实际环境难以强制模拟时：

可：

```text
mock filesystem adapter
```

但必须：

```text
at least one real write-path integration test
```

---

# 207. File Locked

Windows：

```text
source locked
destination locked
```

必须：

```text
structured error
```

---

# 208. Read-only

如果：

```text
source readonly
```

执行：

```text
export new file
```

通常仍然可能成功。

但：

```text
replace source
```

必须：

```text
blocked
```

或：

```text
permission error
```

明确说明。

---

# 209. Symlink

遵守：

```text
M1 symlink policy
```

不要：

```text
M6 follows symlink differently
```

---

# 210. Temp File Safety

临时输出：

必须：

```text
controlled temp location
unique
```

避免：

```text
name collision
```

---

# 211. Temporary Cleanup

成功：

```text
temp removed
```

失败：

```text
cleanup attempted
```

如果 cleanup 失败：

必须：

```text
warning
```

而不是：

```text
ignore
```

---

# 212. Image Processing Errors

至少定义：

```text
UnsupportedFormat
DecodeFailed
EncodeFailed
MetadataReadFailed
MetadataWriteFailed
DimensionsTooLarge
PixelBudgetExceeded
InvalidOptions
AlphaUnsupported
AnimationUnsupported
ColorSpaceUnsupported
BitDepthUnsupported
PermissionDenied
FileMissing
FileChanged
AlreadyExists
PathUnsafe
Cancelled
WriteFailed
VerificationFailed
```

---

# 213. Error Classification

错误至少：

```text
Fatal
Recoverable
PerFile
Global
Warning
```

---

# 214. Per-file Error vs Global Error

例如：

```text
one image corrupt
```

是：

```text
Per-file
```

而：

```text
output root unavailable
```

是：

```text
Global
```

两者不能：

```text
相同 UI 处理
```

---

# 215. Metadata Parse Failure

如果：

```text
pixels decode successfully
metadata corrupted
```

允许：

```text
image processing continue
```

但必须：

```text
Warning
```

前提：

> metadata failure 不影响像素操作的安全性。

---

# 216. Metadata Failure Must Not Become Pixel Failure

例如：

```text
EXIF malformed
```

不应该默认：

```text
image decode failed
```

除非 codec 必须依赖 metadata 才能正确解释 pixel data。

---

# 217. Unsupported Color

例如：

```text
CMYK JPEG
```

如果当前 codec 不支持：

必须：

```text
UnsupportedColorSpace
```

而不是：

```text
interpret as RGB
```

---

# 218. Color Conversion

如实现：

```text
CMYK → RGB
```

必须：

```text
explicit
```

并测试：

```text
expected color handling
```

不要宣称：

```text
color-perfect
```

除非有严谨验证。

---

# 219. ICC Preservation

如果：

```text
input has ICC
```

而 output：

```text
drops ICC
```

必须：

```text
warning
```

或者：

```text
unsupported preservation
```

---

# 220. Output Color Policy

必须定义：

```text
Preserve profile
Convert to sRGB
Drop profile
```

如果只支持其中一种：

明确。

默认优先：

```text
Preserve when possible
```

而不是：

```text
always drop
```

---

# 221. EXIF Orientation Policy

必须在最终实现文档中写清：

```text
Preview
Resize
Convert
Strip Metadata
```

各自：

```text
how orientation is handled
```

---

# 222. Metadata Privacy Policy

M6 UI 默认建议：

```text
Preserve metadata:
ON
```

但当：

```text
Strip Metadata
```

时：

必须明确提醒：

```text
GPS / camera / time data may be removed
```

不要偷偷：

```text
remove metadata
```

---

# 223. Compression Metadata Policy

压缩默认：

```text
preserve metadata
```

或者：

```text
explicit current behavior
```

不能：

```text
PNG→PNG compression
```

顺便：

```text
GPS disappeared
```

而用户不知道。

---

# 224. Convert Metadata Policy

格式转换：

必须明确：

```text
Preserve where supported
Strip
Custom
```

如果目标格式不支持某 metadata：

必须：

```text
report loss
```

---

# 225. “Preserve Metadata” 不是保证

对于：

```text
JPEG EXIF
→
WebP
```

可能存在：

```text
format-specific metadata support
```

因此：

> **Preserve = preserve where representable and actually implemented.**

不支持的字段：

必须：

```text
reported
```

---

# 226. Metadata Loss Report

转换完成后：

如果：

```text
Camera Make preserved
GPS removed
ICC preserved
```

应报告：

```text
Metadata:
3 preserved
1 removed
2 unsupported
```

不要：

```text
Metadata preserved
```

这种模糊结论。

---

# 227. Image Inspector Metadata Detail

点击：

```text
GPS
```

可以显示：

```text
Present
```

是否显示具体：

```text
latitude / longitude
```

视隐私 UX 设计。

但默认：

> 在非必要界面不要扩大暴露。

---

# 228. No Metadata in History

History：

不保存：

```text
GPS
EXIF contents
camera serial
```

只保存：

```text
metadata policy
```

例如：

```text
Strip GPS
```

---

# 229. No Pixel Data in History

不得：

```text
store original pixels
store output pixels
store full thumbnails
```

除非未来另有明确架构。

---

# 230. Image Result

结果页至少：

```text
Operation
Input count
Output count
Success
Failed
Skipped
Cancelled
Input size
Output size
Reduction
Warnings
Output location
```

---

# 231. Single Result

单文件：

```text
Input
Output
Format
Dimensions
Size
Metadata
Warnings
```

---

# 232. Output Open

完成后可提供：

```text
Open Image
Open Folder
```

复用已有 OS integration。

---

# 233. Reuse Operation

如果当前项目：

已有：

```text
Reuse Operation
```

可展示：

```text
same Resize settings
same Compression settings
```

不要重新开发：

```text
M7 workflow storage
```

---

# 234. Command Layer

建议：

```text
image_inspect
image_preview
image_resize_preview
image_compress_preview
image_convert_preview
image_metadata_preview
image_execute
image_batch_preview
image_batch_execute
image_cancel
```

具体命名：

遵循当前仓库 convention。

---

# 235. IPC DTO

必须：

```text
structured
bounded
versionable
```

不要：

```text
serialize full raw image buffers
```

到 React。

---

# 236. Preview Handle

推荐：

```text
PreviewHandle
```

而不是：

```text
Base64 full image in command response
```

如果当前架构已有：

```text
asset protocol
```

优先复用。

---

# 237. UI Command Safety

React：

不允许直接：

```text
fs.writeFile
```

或：

```text
decode image
```

核心数据和文件操作：

必须经过：

```text
Application layer
```

---

# 238. Accessibility

Image tool 至少支持：

```text
keyboard navigation
focus visibility
button labels
form labels
error announcements
progress announcements
preview alternative description where relevant
```

Data / Text 阶段已有规范继续复用。

---

# 239. Preview Accessibility

图片本身：

可以：

```text
alt summary
```

例如：

```text
1920 × 1080 JPEG preview
```

不要：

```text
AI-generated scene description
```

M6 没有 AI vision scope。

---

# 240. i18n

所有 UI 文案：

必须：

```text
translation key
```

尤其：

```text
Format
Quality
Lossless
Lossy
Metadata
GPS
ICC
Orientation
Unsupported
Estimated
Actual
```

---

# 241. No Hardcoded Units

避免：

```text
1920px
5MB
300DPI
```

直接拼在组件里。

使用统一：

```text
formatters
```

---

# 242. Unit Formatting

必须正确支持：

```text
B
KB
MB
GB
```

并采用项目统一标准。

---

# 243. Dimension Formatting

例如：

```text
1920 × 1080
```

保持：

```text
locale independent
```

必要时数字 formatting 使用统一工具。

---

# 244. Performance Baseline

所有基准测量结果写入 docs/PERF.md（charter #68）。

必须先测：

```text
small
medium
large
```

例如：

```text
1 MP
12 MP
24 MP
48 MP
```

实际 fixtures 根据当前机器。

---

# 245. Resize Benchmark

至少：

```text
12 MP JPEG → 1920
24 MP JPEG → 1920
48 MP JPEG → 1920
```

记录：

```text
Elapsed
Peak Memory
Throughput
```

---

# 246. PNG Benchmark

至少：

```text
PNG decode
PNG encode
PNG resize
```

测：

```text
small
medium
large
```

---

# 247. WebP Benchmark

同样：

```text
Decode
Encode
Resize
```

---

# 248. Multi-image Benchmark

至少：

```text
100 images
500 images
1000 images
```

或者：

```text
当前机器可稳定测试的实际规模
```

记录：

```text
Total Bytes
Total Files
Elapsed
Peak Memory
Success
Failed
```

---

# 249. No “Fast” Claims Without Numbers

禁止：

```text
Fast
Lightning-fast
Handles thousands easily
Low memory
```

如果没有：

```text
benchmark evidence
```

---

# 250. Memory

至少记录：

```text
Peak RSS / process memory where practical
Decoded peak
Batch peak
Preview peak
```

---

# 251. Memory Regression

同一输入：

```text
100 images
```

重新执行：

必须：

```text
memory returns / bounded
```

避免：

```text
preview cache leak
```

---

# 252. Batch Memory

重点验证：

```text
1000 images
```

不是：

```text
1000 × decoded memory
```

---

# 253. Preview Cache Leak

测试：

```text
open preview A
preview B
preview C
...
```

最终：

```text
memory stable / bounded
```

---

# 254. Session Close

用户关闭 Image tool：

必须：

```text
release preview resources
release decoded buffers
cancel outstanding work where appropriate
```

---

# 255. Fault Injection

建议 filesystem adapter 支持模拟：

```text
ReadFail
WriteFail
FlushFail
RenameFail
PermissionDenied
FileChanged
FileMissing
DiskFull
Locked
```

以测试 transaction path。

---

# 256. Codec Faults

可以通过：

```text
mock codec
```

模拟：

```text
decode failed
encode failed
metadata failed
unexpected output
```

并确保：

```text
application result honest
```

---

# 257. No Panic Boundary

所有用户图片：

都视为：

```text
untrusted input
```

因此：

```text
malformed input
```

不能：

```text
panic app
```

---

# 258. Fuzz Outcome

最终至少报告：

```text
Corpus
Iterations / Time
Crashes
Panics
OOM
Timeouts
Interesting Cases
```

---

# 259. Security Audit

至少输出：

```text
Image Parser:
PASS

Dimension Guard:
PASS

Pixel Budget:
PASS

Metadata Limits:
PASS

Path Safety:
PASS

Symlink:
PASS

TOCTOU:
PASS

Safe Write:
PASS

Output Collision:
PASS

No Shell:
PASS

No Network:
PASS

Secret Logging:
PASS
```

---

# 260. Privacy Audit

至少：

```text
GPS:
No upload

EXIF:
No upload

Image Pixels:
No upload

Metadata:
Not stored in history

Logs:
No raw image data
```

---

# 261. M6 Regression

必须保持：

```text
M0 PASS
M1 PASS
M2 PASS
M3 PASS
M4 PASS
M5 PASS
```

尤其：

```text
Filesystem
Path Safety
Preview
Safe Write
Transaction
History
Undo
Progress
Cancellation
Encoding
Diagnostics
```

---

# 262. M5 Regression

需要确认：

```text
Data tools still work
```

尤其：

```text
CSV export
JSON export
JSONL streaming
```

不得因为：

```text
media dependency
```

破坏：

```text
weave-data
```

---

# 263. Dependency Isolation

如果：

```text
image crate
```

升级或新增：

必须尽量：

```text
isolated dependency boundary
```

避免：

```text
global architecture coupling
```

---

# 264. Documentation

M6 至少更新：

```text
README.md
ARCHITECTURE.md
DECISIONS.md
PROGRESS.md
PRIVACY.md
SECURITY.md
THIRD_PARTY_LICENSES.md
```

必要时：

```text
IMAGE_FORMATS.md
```

---

# 265. IMAGE_FORMATS.md

至少记录：

```text
Supported Formats
Decode
Encode
Alpha
Animation
Metadata
Color
Bit Depth
Limitations
```

---

# 266. DECISIONS.md

必须记录：

```text
Why selected image library
Why not ImageMagick CLI
Why no Photoshop-like editor
Why no general batch engine
Why static GIF scope if applicable
Why TIFF scope
Why metadata policy
Why pixel budget
Why preview uses bounded representation
Why default export is non-destructive
```

---

# 267. SECURITY.md

至少新增：

```text
Image Parsing Threat Model
Decompression Bomb
Dimension Limit
Pixel Memory Limit
Metadata Limit
Codec Isolation
Output Safety
```

---

# 268. PRIVACY.md

至少写：

```text
Image pixels remain local
EXIF remains local
GPS remains local
No telemetry
No upload
No cloud processing
History does not store image content
```

---

# 269. PROGRESS.md

完成后：

```text
M0 = COMPLETE
M1 = COMPLETE
M2 = COMPLETE
M3 = COMPLETE
M4 = COMPLETE
M5 = COMPLETE
M6 = COMPLETE
M7 = NEXT
```

并注明：

```text
Implemented
Tested
Verified
Not Verified
Known Limitations
```

---

# 270. Baseline Audit

开始实施前：

必须首先审计：

```text
Git
Cargo workspace
weave-core
weave-files
weave-text
weave-data
weave-history
weave-testkit
Tauri
React
IPC
Preview
Progress
Cancellation
Path Safety
Safe Write
Transaction
History
Undo
```

---

# 271. Audit Output

先输出：

```text
FACT
HYPOTHESIS
INFERENCE
```

例如：

```text
FACT:
weave-files already exposes Safe Write.

FACT:
M2 already persists History.

HYPOTHESIS:
Image export can reuse M2 transaction path.

INFERENCE:
No second image-specific filesystem mutation system should be introduced.
```

---

# 272. Status

每项：

```text
CONFIRMED
RESOLVED
WORSENED
BLOCKED
UNKNOWN
```

---

# 273. Gap Analysis

必须先列：

```text
Existing
Reusable
Missing
Needs Extension
Must Not Duplicate
```

---

# 274. Implementation Order

严格按：

```text
Phase 0
Baseline Audit

Phase 1
Image Domain Model

Phase 2
Codec / Decode / Inspect

Phase 3
Image Inspector

Phase 4
Resize

Phase 5
Compression

Phase 6
Format Conversion

Phase 7
Metadata / Privacy

Phase 8
Preview Engine

Phase 9
Multi-Image Processing

Phase 10
Safe Export / History / Undo

Phase 11
UI / Accessibility / i18n

Phase 12
Fault / Fuzz / Performance

Phase 13
Security / Privacy / Regression

Phase 14
Final Audit

Phase 15
Documentation

Phase 16
Commit
```

---

# 275. Phase 1 — Domain

先建立：

```text
ImageDocument
ImageFormat
ImageCapabilities
ImageMetadata
ImageOperationPlan
ResizeOptions
CompressionOptions
ConversionOptions
MetadataOptions
ImageResult
BatchImageResult
```

---

# 276. Phase 2 — Codec

先实现：

```text
probe
decode
thumbnail decode
encode
capabilities
```

然后才做：

```text
UI
```

---

# 277. Phase 3 — Inspector

先做到：

```text
Format
Size
Dimensions
Color
Bit Depth
Alpha
Orientation
Metadata
Animation
```

---

# 278. Phase 4 — Resize

先：

```text
Fit
Exact
Scale
Width
Height
```

再：

```text
Fill
```

---

# 279. Phase 5 — Compression

先：

```text
JPEG quality
PNG compression
WebP quality
```

再根据 capability：

```text
lossless / lossy
```

---

# 280. Phase 6 — Conversion

按照：

```text
Capability Matrix
```

逐项实现。

不要：

```text
一次宣布所有格式完成
```

---

# 281. Phase 7 — Metadata

先：

```text
Inspect
Strip EXIF
Strip GPS
Strip All
```

然后：

```text
ICC
Orientation
```

---

# 282. Phase 8 — Preview

Preview：

必须调用：

```text
actual transform pipeline
```

只是：

```text
bounded output
```

而不是：

```text
fake UI
```

---

# 283. Phase 9 — Multi-image

只支持：

```text
one operation
+
many inputs
```

例如：

```text
Resize 100 images
Convert 100 images
Strip metadata 100 images
```

---

# 284. Phase 10 — Export

统一：

```text
Plan
Preview
Validation
Execute
Verify
Result
History
Undo
```

---

# 285. Phase 11 — UI

形成：

```text
Drop
→
Detect
→
Inspect
→
Tool
→
Options
→
Preview
→
Export
→
Progress
→
Result
```

---

# 286. Phase 12 — Test

必须：

```text
Unit
Integration
Fault
Property
Fuzz
Performance
Filesystem
UI Smoke
```

全部真实执行。

---

# 287. Unit Tests

至少覆盖：

```text
dimension calculations
aspect ratio
fit
fill
scale
upscale rules
compression options
format capabilities
metadata policy
orientation
```

---

# 288. Property Tests

至少考虑：

```text
resize keeps aspect when configured
dimensions never exceed target when using Fit
output extension matches format
metadata policy is deterministic
same plan + same input = same structural result
```

---

# 289. Property — Idempotence

某些操作应测试：

```text
Strip Metadata
```

是否：

```text
再次 Strip
```

不会继续产生意外变化。

即：

```text
Strip(Strip(image))
≈
Strip(image)
```

如果格式编码本身存在变化：

需明确其边界。

---

# 290. Property — Resize

例如：

```text
Fit 1920
```

执行两次：

```text
image
→
1920
→
1920
```

第二次不应：

```text
继续缩小
```

如果输入已经符合限制。

---

# 291. Integration — Real Decode

至少真实打开：

```text
PNG
JPEG
WebP
GIF
BMP
TIFF
```

能够支持多少：

必须由：

```text
Capability Matrix
```

决定。

---

# 292. Integration — Real Export

真实执行：

```text
Resize
Compress
Convert
Strip Metadata
```

并检查真实文件。

---

# 293. UI Smoke

至少：

```text
Drag Image
Inspect
Resize
Preview
Export
Open Output
```

多图：

```text
Drag 10 images
Select Operation
Preview
Process
Result
```

---

# 294. UI Negative Tests

至少：

```text
unsupported format
corrupt image
oversized image
cancelled operation
existing destination
permission denied
source changed
```

---

# 295. No Mock Success

禁止：

```text
UI 显示缩略图
backend 其实没处理
```

禁止：

```text
fake output file
```

禁止：

```text
fake metadata
```

禁止：

```text
fake progress
```

---

# 296. Real Preview

至少有：

```text
actual transformed image
```

而不是：

```text
“预计会调整大小”
```

的静态文案。

---

# 297. Image Diff Testing

对于 resize：

可以：

```text
check output dimensions
```

对于 metadata：

可以：

```text
parse output metadata
```

对于 lossless：

可以：

```text
pixel compare
```

对于 lossy：

不要：

```text
pixel exact equality
```

误判失败。

---

# 298. Performance Acceptance

不要定义：

```text
“必须 1 秒”
```

除非真实环境已经建立 baseline。

先：

```text
benchmark
```

再：

```text
record baseline
```

---

# 299. Benchmark Table

最终至少：

```text
Operation | Input | Dimensions | Format | Elapsed | Peak Memory | Output
```

例如：

```text
Resize | 24 MB | 6000×4000 | JPEG | ... | ... | ...
```

---

# 300. Multi-image Benchmark

至少：

```text
10
100
500
1000
```

真实可行范围。

记录：

```text
files
total input bytes
elapsed
peak memory
throughput
success
```

---

# 301. CPU vs Memory

至少观察：

```text
CPU-heavy
memory-heavy
```

操作差异。

不要：

```text
只看 wall time
```

就宣布性能优秀。

---

# 302. Preview Benchmark

单图：

```text
large source
→
bounded preview
```

必须：

```text
UI responsive
memory bounded
```

---

# 303. Batch Preview Benchmark

多图：

必须验证：

```text
1000 files
```

Preview 不会：

```text
decode 1000 full-resolution images
```

---

# 304. Resource Limits Documentation

所有限制最终记录：

```text
Max Image Pixels
Max Width
Max Height
Max Metadata
Max Preview Size
Max Concurrent Images
Max Batch Inputs
```

以及：

```text
Why
```

---

# 305. User-visible Limit Errors

例如：

```text
This image is 12000 × 12000 px and exceeds the current safety limit.

No files were modified.
```

比：

```text
Out of memory
```

更有帮助。

---

# 306. Image Format Mismatch UX

例如：

```text
File extension:
.jpg

Detected:
PNG

The file content does not match its extension.
```

提供：

```text
Open as PNG
```

而不是：

```text
Rename extension silently
```

---

# 307. Corrupt Image UX

例如：

```text
The image could not be decoded.

No output was created.

Reason:
Unexpected end of JPEG stream.
```

---

# 308. Metadata Warning UX

例如：

```text
GPS metadata detected.

Exporting with “Preserve Metadata” will keep location information.
```

这比：

```text
Metadata: ON
```

更明确。

---

# 309. Conversion Warning UX

例如：

```text
PNG → JPEG

Transparency will be composited onto:
White

JPEG is lossy.
```

---

# 310. Resize Warning UX

例如：

```text
Exact 1920 × 1080

Aspect ratio will change from:
4:3
to
16:9
```

---

# 311. Quality UX

例如：

```text
JPEG
Quality:
82

This is an encoder quality setting, not a guaranteed visual-quality percentage.
```

---

# 312. Result Honesty

结果：

```text
Actual output:
2.1 MB
```

不要：

```text
Estimated output:
2.0 MB
```

混用。

---

# 313. Metadata Result Honesty

例如：

```text
EXIF:
Removed

GPS:
Removed

ICC:
Preserved

XMP:
Not verified
```

---

# 314. Image Batch Summary

例如：

```text
100 files processed

98 succeeded
1 skipped
1 failed

Input:
1.8 GB

Output:
642 MB

Reduction:
64.3%
```

所有数值都来自：

```text
actual results
```

---

# 315. Failure Summary

例如：

```text
1 file failed

broken.jpg
Reason:
Decode failed
```

---

# 316. Retry Failed

如实现：

```text
Retry Failed
```

必须：

```text
仅重试 failed inputs
```

不要：

```text
重新处理所有 inputs
```

除非用户明确。

---

# 317. Existing Output Handling

如果 output exists：

必须：

```text
Collision
```

支持哪种策略：

依据：

```text
M2 infrastructure
```

统一。

---

# 318. Batch Atomicity

不要承诺：

```text
all-or-nothing
```

除非真的做到。

默认：

```text
per-file success/failure
+
aggregate summary
```

更符合图像批处理。

---

# 319. Transaction Honesty

History：

必须记录：

```text
actual outputs
```

不是：

```text
planned outputs
```

---

# 320. Commit

最终仅当：

```text
Final Audit PASS
```

后：

允许：

```text
commit
```

遵循当前仓库 Git convention。

推荐：

```text
feat(image): implement image tools
```

或者：

```text
feat(media): add image processing tools
```

不要：

```text
update
misc
work
stuff
```

---

# 321. Final Audit

完成后：

不要直接说：

```text
M6 Complete
```

必须执行完整审计。

质量门（charter #45，无条件必过）：

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

无法执行的项必须标 BLOCKED，不得标 PASS。

---

## A. Baseline

```text
Branch
HEAD
Working Tree
M0
M1
M2
M3
M4
M5
```

---

## B. Architecture

检查：

```text
weave-media
weave-files
weave-core
weave-history
weave-testkit
Tauri
React
IPC
```

确认：

```text
No duplicate filesystem layer
No duplicate transaction layer
No duplicate preview layer
No duplicate path safety
No duplicate history
No second batch engine
```

---

## C. Codec

检查：

```text
PNG
JPEG
WebP
GIF
BMP
TIFF
```

逐项输出：

```text
Decode
Encode
Resize
Compress
Convert
Metadata
Animation
Alpha
```

---

## D. Resize

检查：

```text
Fit
Fill
Exact
Scale
Width
Height
Aspect
Upscale
Resampling
Alpha
```

---

## E. Compression

检查：

```text
JPEG
PNG
WebP
Lossy
Lossless
Quality
Compression
Size
```

---

## F. Conversion

检查：

```text
format matrix
alpha policy
metadata policy
color handling
orientation
lossy warning
```

---

## G. Metadata

检查：

```text
EXIF
GPS
ICC
Orientation
DPI
Camera
Creation Time
Strip
Preserve
Loss Report
```

---

## H. Preview

检查：

```text
Real transform
Before / After
Bounded decode
Actual dimensions
Estimated size
Warnings
```

---

## I. Multi-image

检查：

```text
Input snapshot
Deterministic order
Bounded concurrency
Progress
Cancellation
Partial failure
Result summary
```

---

## J. Safe Write

检查：

```text
Path Safety
Collision
TOCTOU
Atomic Write
Transaction
History
Undo
```

---

## K. Performance

必须输出真实：

```text
Small
Medium
Large
Multi-image
Preview
Memory
```

---

## L. Security

必须输出：

```text
Malformed Image
Huge Dimensions
Pixel Budget
Metadata Limit
Path Safety
Symlink
TOCTOU
Collision
Atomic Write
No Shell
No Network
Secret Logging
```

---

## M. Privacy

必须确认：

```text
No Upload
No Cloud
No Telemetry
No Account
No Remote Processing
No Image Content in History
No GPS in Logs
```

---

## N. Regression

必须：

```text
M0 PASS
M1 PASS
M2 PASS
M3 PASS
M4 PASS
M5 PASS
```

---

## O. Documentation

必须：

```text
README
ARCHITECTURE
DECISIONS
PROGRESS
PRIVACY
SECURITY
THIRD_PARTY_LICENSES
IMAGE_FORMATS where applicable
```

---

## P. Known Limitations

必须明确：

```text
Unsupported codecs
Animation limits
TIFF limits
Color limitations
Bit-depth limitations
Metadata limitations
Size limits
Platform limitations
Preview limitations
```

---

# 322. M6 不算完成的情况

以下任何一种存在：

```text
Image resize UI exists
but backend returns fake preview

Conversion only renames extension

JPEG output is corrupted but marked success

PNG transparency silently becomes black

EXIF GPS is removed unexpectedly

Orientation is wrong after stripping metadata

TIFF is advertised but actual decoder is missing

Animated GIF becomes first frame silently

4 GB decode can OOM application

1000 images are all decoded simultaneously

Cancel only changes UI

Progress is fake

Batch operation uses unbounded tasks

Output collision silently overwrites

Undo deletes a user-modified output

History records planned output instead of actual mutation

Preview path differs materially from execute path

Metadata parser logs sensitive values

Image data is sent to cloud

Image processing uses shell CLI without architecture approval

M7 generic Batch Engine sneaks into M6

M1/M2/M5 regressions occur

Tests are mock-only

Fuzz is absent for parser boundary

Large-image performance is unverified

```

都不能宣布：

```text
M6 COMPLETE
```

---

# 323. Final Implementation Report

完成后最终输出：

# M6 Implementation Report

## 1. Baseline

```text
Branch:
HEAD:
Working Tree:
M0:
M1:
M2:
M3:
M4:
M5:
```

---

## 2. Architecture

```text
New Modules:
Reused Modules:
Extended Modules:
Dependencies Added:
Why:
```

---

## 3. Image Capability Matrix

```text
Format
Decode
Encode
Alpha
Animation
Metadata Read
Metadata Write
Resize
Compress
Convert
Status
```

---

## 4. Implemented Features

```text
Image Inspector
Resize
Compress
Convert
Metadata
Strip Metadata
Preview
Multi-Image Processing
Progress
Cancellation
History
Undo
```

逐项：

```text
Implemented
Reused
Extended
Not Supported
Not Verified
Known Limitation
```

---

## 5. Tests

```text
Unit:
Integration:
Fault:
Property:
Fuzz:
Performance:
Filesystem:
UI Smoke:
Build:
CI:
```

必须有：

```text
command
result
count
status
```

---

## 6. Format Test Evidence

至少：

```text
PNG
JPEG
WebP
GIF
BMP
TIFF
```

并列出：

```text
Decode
Encode
Resize
Compress
Convert
Metadata
Animation
Alpha
```

---

## 7. Metadata Evidence

必须输出：

```text
EXIF
GPS
ICC
Orientation
DPI
Strip
Preserve
Loss
```

---

## 8. Image Correctness Evidence

至少：

```text
Resize dimensions
Aspect ratio
Alpha
Orientation
Pixel equality for applicable lossless cases
Decode-after-encode
Format correctness
Metadata policy
```

---

## 9. Performance Evidence

至少：

```text
Single Image:
Input
Dimensions
Format
Elapsed
Peak Memory
Output

Multi Image:
Files
Total Input
Elapsed
Peak Memory
Success
Failed
Skipped
```

---

## 10. Security Evidence

```text
Malformed Images:
PASS / FAIL

Huge Dimensions:
PASS / FAIL

Pixel Budget:
PASS / FAIL

Metadata Limits:
PASS / FAIL

Path Traversal:
PASS / FAIL

Symlink:
PASS / FAIL

TOCTOU:
PASS / FAIL

Collision:
PASS / FAIL

Atomic Write:
PASS / FAIL

No Shell:
PASS / FAIL

No Network:
PASS / FAIL
```

---

## 11. Privacy Evidence

```text
GPS:
Not Logged

Image Pixels:
Not Uploaded

Metadata:
Not Stored in History

Telemetry:
None

Cloud:
None
```

---

## 12. Known Limitations

只写真实：

```text
Not Implemented
Not Supported
Not Verified
Platform-specific
Codec-specific
Animation-specific
Color-specific
Metadata-specific
Size-limited
```

---

# 324. Git Hygiene

执行：

```powershell
git status
git diff --stat
git diff
git diff --cached
git ls-files
```

检查：

```text
.env
secrets
private keys
certificates
personal photos
GPS-bearing fixtures
local paths
temp previews
cache
build output
large generated images
```

---

# 325. M6 Handoff

M6 完成后向 M7 交接：

```text
ImageDocument
ImageFormat
ImageCapabilities
ImageMetadata
Image Inspector
Resize
Compress
Convert
Metadata Strip
Preview
ImageOperationPlan
Multi-Image Processing
Progress
Cancellation
Safe Write
Transaction
History
Undo
Path Safety
Resource Limits
Codec Capability Matrix
```

特别说明：

> **M7 必须建立统一 Batch Engine，而不是把 M6 的 Multi-Image Processing 直接复制粘贴扩大。**

M7 应该把：

```text
M2 Rename
M2 Organizer
M3 Duplicate
M5 Data
M6 Image
```

逐步接入统一：

```text
Job
Preview
Execute
Progress
Cancel
Retry
Result
Rollback / Undo
History
```

---

# 326. M6 最终产品状态

完成 M6 后，用户应该能够：

```text
拖入一张图片
    ↓
Inspect
    ↓
看到：
Format
Dimensions
Color
Metadata
GPS
Orientation
    ↓
Resize / Compress / Convert / Metadata
    ↓
Preview
    ↓
确认
    ↓
Export
    ↓
Result
```

也可以：

```text
拖入 100 张图片
       ↓
Detect
       ↓
选择同一个 Image Operation
       ↓
统一设置参数
       ↓
Preview representative samples
       ↓
Process
       ↓
Progress
       ↓
997 Success
2 Failed
1 Skipped
       ↓
Result
```

整个过程中：

```text
不会默认覆盖原图
不会偷偷丢 metadata
不会悄悄改变 orientation
不会把透明背景变黑却不说
不会因为一张坏图让全部任务失去结果
不会把假的进度显示成真实进度
不会把 unsupported 当 supported
不会因为大图轻易 OOM
不会把 M6 偷偷变成 M7
```

---

# 327. M6 的核心原则

始终遵守：

```text
Pixels > Convenience
Metadata > Hidden Assumptions
Capability > Marketing
Preview > Mutation
Bounded Memory > Maximum Input
Actual Output > Estimated Output
Explicit Loss > Silent Loss
Preserve > Destroy
Safe Export > Fast Overwrite
Per-file Failure Isolation > Global Failure
Reuse > Rebuild
Real Codec > Fake Extension Conversion
Real Tests > Green Theater
```

尤其牢记：

> **图片工具最大的风险不是“图片没变漂亮”，而是用户不知道自己的图片到底被改变了什么。**

因此：

```text
Resize
Compress
Convert
Metadata
Orientation
Alpha
Color
```

任何可能发生变化的地方：

必须：

```text
显式
可预览
可验证
可解释
```

---

# 328. M6 的终极目标

M6 不是：

```text
Image editor
```

而是：

```text
ifuyo Weave
      │
     Image
      │
 ┌────┼────────────┐
 ▼    ▼            ▼
Inspect Resize   Convert
 │     │            │
 │     ▼            ▼
 │   Compress    Metadata
 │       \          /
 │        \        /
 └──────── Preview
             │
          Validate
             │
           Export
             │
          Result
             │
        History / Undo
```

最终用户应该感觉：

> **“我有一张或一堆图片，不管是尺寸太大、格式不对、文件太重，还是 EXIF/GPS 不想保留，扔给 Weave 就能快速、安全、清楚地处理完。”**

而不是：

> **“我又得打开一个复杂的图片编辑软件。”**

---

# 329. 执行纪律

整个 M6 严格执行：

```text
Audit
 ↓
Gap Analysis
 ↓
Dependency / Capability Audit
 ↓
Domain Model
 ↓
Codec
 ↓
Inspector
 ↓
Resize
 ↓
Compress
 ↓
Convert
 ↓
Metadata
 ↓
Preview
 ↓
Multi-Image
 ↓
Safe Export
 ↓
History / Undo
 ↓
UI
 ↓
Fault
 ↓
Fuzz
 ↓
Performance
 ↓
Security
 ↓
Privacy
 ↓
Regression
 ↓
Documentation
 ↓
Final Audit
 ↓
Commit
```

每一个阶段：

必须优先回答：

```text
这是 FACT？
还是 HYPOTHESIS？
还是 INFERENCE？
```

问题优先按：

```text
P0
P1
```

处理。

只修：

```text
数据正确性
安全性
隐私
资源耗尽
核心功能
架构边界
回归
发布卫生
```

真正阻塞的问题。

不要为了：

```text
“功能看起来更多”
```

而牺牲：

```text
稳定性
正确性
可维护性
```

---

# 330. 最终完成条件

只有：

```text
M6 Acceptance Criteria
+
Codec Verification
+
Image Correctness Tests
+
Metadata Tests
+
Performance Evidence
+
Security Evidence
+
Privacy Evidence
+
M0–M5 Regression
+
UI Smoke
+
Documentation
+
Git Hygiene
```

全部 PASS 后：

才能输出：

```text
M6 COMPLETE
```

否则必须：

```text
M6 NOT COMPLETE
```

并准确列出：

```text
BLOCKED
FAIL
NOT VERIFIED
NOT SUPPORTED
KNOWN LIMITATION
```

严禁：

```text
隐藏 codec 限制
隐藏 metadata loss
隐藏 orientation problem
隐藏 alpha loss
隐藏 OOM risk
关闭失败测试
屏蔽 fuzz case
伪造 progress
伪造 benchmark
mock 冒充真实 codec
extension rename 冒充 conversion
M7 capability 偷渡
```

---

# 331. M6 → M7 的最终边界

M6 交付：

```text
Reliable Image Tools
```

M7 才负责：

```text
Unified Batch Engine
```

因此：

```text
M6:
“把同一个图片操作安全地应用到很多图片上。”

M7:
“把 Weave 中不同工具组织成统一、可组合、可重试、可追踪的批处理系统。”
```

两者必须保持清晰边界。

---

# 332. 最终原则

> **M6 的完成标准不是“图片能处理”，而是“图片能被可靠地处理”。**

> **能打开 ≠ 能正确解释。**

> **能转换 ≠ 转换没有损失。**

> **能压缩 ≠ 用户知道发生了什么。**

> **能删除元数据 ≠ 所有元数据真的都删除了。**

> **能批量执行 ≠ 已经拥有通用 Batch Engine。**

> **Preview 显示了结果 ≠ Execute 一定安全。**

必须让：

```text
Detection
→
Decode
→
Transform
→
Preview
→
Validate
→
Encode
→
Verify
→
Export
```

每一步都拥有：

```text
明确语义
真实实现
结构化错误
资源边界
测试证据
```

最终让 ifuyo Weave 的 Image 能力真正成为：

> **一个“把图片处理得更简单”的工具，而不是一个“功能越来越多、风险也越来越大”的图片软件。**