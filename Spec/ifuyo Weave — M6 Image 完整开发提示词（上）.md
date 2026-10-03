# ifuyo Weave — M6 Image 完整开发提示词（上）

> 本文件是原《ifuyo Weave — M6 Image 完整开发提示词》的 **第 1/2 部分**，
> 覆盖原第 0–149 节：M6 硬边界、本阶段明确禁止、与 M7 的关键边界、M1–M5 复用与核心产品定义、图像处理的真实危险点，ImageDocument/PixelData/Image Dimensions 与 Decompression Bomb Protection，Decode Policy、Format Detection/Extension Is Not Truth/Format Capability Matrix，PNG/JPEG/WebP/GIF/BMP/TIFF 各格式规则，Transparency/Alpha Compositing，Metadata 模型与 Pixel Data 分离、EXIF/GPS Privacy/Strip Metadata/Orientation、ICC Profile/Color Space/Bit Depth/DPI，Image Inspector，Resize 全量语义（Modes/Fit/Fill/Exact/Scale/Up-Down/Resampling/Alpha/Quality），Compress（JPEG/WebP/PNG Quality、Lossless/Lossy Warning），Format Convert（Re-encoding、跨格式转换、Animation Policy），Metadata Preview/Privacy，Preview Strategy/Resolution/Quality/Before-After、Real Output Size/Compression Ratio，Metadata-Only Operation、File Mutation Boundary、Source Overwrite/Collision/Output Naming/Destination Structure，Multi-image 模型与 Batch Preview/Sampling/Result、Failure Isolation、Cancellation、No False Success、Transaction/History/Undo，Output Identity/Metadata Strip Undo/Replace Source，M1–M5 Reuse，weave-media 与 Codec Adapter/Capability API/Unsupported Handling，Dependency Strategy/No CLI/No Shell/No Network/Privacy/Logging、Security Threat Model/Metadata Bomb/Huge Dimension/内存估算与并发/Backpressure，Input Enumeration/Selection 与各类 Options、Transform Plan、Preview Safety/Cache，TOCTOU、Atomic Output、Output/Roundtrip Verification、Corrupt Output Detection。
> **必须与（下）一起阅读执行**：测试矩阵与 Real/Deterministic Fixtures、进度与 IPC、全部 UI/UX、Accessibility/i18n、性能基准与故障注入、审计、文档、Baseline Audit、实施阶段（Phase 1–12）、Commit、Final Audit、Git Hygiene、Handoff 条款均在（下）。
> 原单文件已废弃，由（上）+（下）共同替代；正文与原文件逐字一致，未做任何改写。

---
# ifuyo Weave
# M6 — Image
## Image Resize、Image Compress、Image Convert、Image Metadata、Multi-Image Processing、Preview

你现在进入：

> **ifuyo Weave 的 M6：Image**

项目：

```text
ifuyo Weave
```

当前 milestone：

```text
M0 = Foundation
M1 = File Core
M2 = Rename / Organizer
M3 = Duplicate Finder
M4 = Text
M5 = Data
M6 = Image
```

M6 的目标不是：

> “做一个 Photoshop。”

也不是：

> “做一个完整图片编辑器。”

更不是：

> “提前实现一个通用 Batch Engine。”

M6 的真正目标是：

> **让用户可以把一张或一组图片交给 Weave，快速完成尺寸调整、压缩、格式转换、隐私元数据处理与基础图像信息查看，并且整个过程可预览、可验证、可取消、可恢复。**

核心闭环：

```text
Drop / Open
    ↓
Detect
    ↓
Inspect
    ↓
Choose Image Tool
    ↓
Adjust Parameters
    ↓
Preview
    ↓
Validate
    ↓
Process
    ↓
Progress
    ↓
Result
    ↓
History / Undo where applicable
```

多图场景：

```text
Select Images
    ↓
Inspect
    ↓
Choose One Operation
    ↓
Apply Same Explicit Parameters
    ↓
Preview Representative Results
    ↓
Validate
    ↓
Process
    ↓
Progress
    ↓
Success / Failed / Skipped / Cancelled
    ↓
Result
```

始终遵守：

```text
Local-first
Utility-first
Preview-first
Reversible-first
Composable
Batch-ready
```

以及：

```text
Correctness > Feature Count
Image Integrity > Convenience
Explicit Parameters > Magic
Preview > Mutation
Streaming > Unbounded Memory
Reuse > Rebuild
Bounded > Unlimited
Actual Result > Planned Result
Privacy by Default > Metadata Leakage
Real Tests > Green Theater
```

---

# 0. M6 硬边界

## 0.1 本阶段必须完成

核心能力：

```text
Image Detection
Image Decode
Image Inspect
Image Resize
Image Compress
Image Convert
Image Metadata
Strip Metadata
Preview
Multi-Image Processing
Progress
Cancellation
Safe Export
History
Undo where applicable
```

---

## 0.2 Charter 范围

核心格式目标：

```text
PNG
JPEG
WebP
GIF
BMP
TIFF
```

但必须建立：

> **Format Capability Matrix**

因为：

```text
Decode capability
Encode capability
Animation capability
Metadata capability
Color-management capability
```

可能并不完全相同。

因此：

> **只有真实验证通过的 codec capability 才能标记为 Supported。**

禁止为了满足列表而：

```text
extension rename
fake conversion
partial decode pretending success
```

---

# 0.3 本阶段明确禁止

M6 不得实现：

```text
Full Photoshop-like Editor
Layers
Brushes
Masks
Advanced Retouching
Content-aware Fill
AI Upscaling
AI Background Removal
Generative Fill
OCR
Face Recognition
Object Detection
Image Search
Perceptual Similarity
Visual Deduplication
Video Editing
Audio Editing
GIF Timeline Editor
RAW Development Suite
Professional Color Grading
LUT Management
3D Image Processing
Cloud Processing
Remote Image Processing
Telemetry
Account
Cloud Storage
```

尤其禁止提前实现：

```text
M7 General Batch Engine
M8 Documents
M10 Workflow
```

声明：Crop / Rotate / Rename 不作为 M6 独立工具（Resize Fill 内含 crop、orientation normalize 隐含旋转）；如后续需要，须按 charter #52 工具加入标准另行评估。

---

# 1. M6 与 M7 的关键边界

M6 可以处理：

```text
1 image
10 images
100 images
```

甚至：

```text
1000 images
```

只要：

```text
所有输入执行同一种明确的 Image Operation
```

例如：

```text
100 images
→ Resize 1920px
→ WebP
→ quality 82
→ output/
```

这是：

> **M6 Multi-Image Processing**

不是：

> **M7 General Batch Engine**

---

## 1.1 M6 不得实现通用 Pipeline

禁止：

```text
Input
 ↓
Filter
 ↓
Resize
 ↓
Convert
 ↓
Rename
 ↓
Organize
 ↓
Upload
```

作为可自由编排的：

```text
Workflow / Pipeline
```

M6 只允许：

```text
One tool
+
One coherent image operation
+
Explicit multi-file application
```

---

# 1.2 M6 可以复用 M2 的 Operation Infrastructure

必须优先复用：

```text
OperationId
OperationPlan
OperationResult
Progress
Cancellation
Collision Detection
Transaction
History
Undo
Safe Write
Path Safety
```

但：

> **不要因此在 M6 创建第二套 Batch Engine。**

---

# 2. M6 核心产品定义

M6 负责：

```text
Understand image
Transform image
Protect metadata
Export image
```

不负责：

```text
Create artwork
Edit artwork interactively
Manage layers
```

---

# 3. 图像处理的真实危险点

图像工具看起来简单：

```text
Resize
Compress
Convert
```

实际上存在：

```text
Decode failure
Color profile
Alpha channel
Bit depth
Metadata
Orientation
ICC profile
EXIF GPS
Animation
Compression loss
Quality settings
Large dimensions
Large pixel count
Memory exhaustion
Malformed image
Decompression bombs
Integer overflow
Output collision
Partial write
```

所以 M6 的重点不是：

> 功能很多。

而是：

> **图像处理结果可控、可解释、不会轻易吃爆内存，也不会偷偷丢失重要信息。**

---

# 4. ImageDocument

建议建立统一：

```text
ImageDocument
```

至少包含：

```text
ImageIdentity
Format
Dimensions
ColorModel
BitDepth
Alpha
Orientation
MetadataSummary
AnimationInfo
FileFacts
```

必要时：

```text
PixelFormat
ColorProfile
FrameCount
FrameDurationSummary
Dpi
```

---

# 5. PixelData

不要默认把每张图片：

```text
整个 decode
→
RGB Vec<u8>
```

然后无论大小直接塞进内存。

必须建立：

```text
Pixel Resource Limits
```

至少考虑：

```text
Max Width
Max Height
Max Pixels
Max Decoded Bytes
Max Frames
Max Frame Pixels
Max Metadata Size
```

实际值：

> **必须通过当前环境测试决定，而不是拍脑袋。**

---

# 6. Image Dimensions

必须安全处理：

```text
width
height
```

防止：

```text
u32
→
u64
→
usize
```

转换时发生：

```text
overflow
```

例如：

```text
width * height * channels * bytes_per_channel
```

必须：

```text
checked arithmetic
```

---

# 7. Decompression Bomb Protection

必须考虑：

> **一个很小的压缩图片文件，解码后可能变成非常巨大的像素缓冲。**

例如：

```text
input:
2 MB

decoded:
12000 × 12000 RGBA
```

不能因为磁盘文件只有：

```text
2 MB
```

就认为：

```text
memory requirement = 2 MB
```

必须以：

```text
decoded pixel budget
```

控制。

---

# 8. Decode Policy

图片打开流程：

```text
Path Safety
 ↓
File Facts
 ↓
Format Detection
 ↓
Codec
 ↓
Dimension Guard
 ↓
Decode
 ↓
Metadata Parse
```

不要：

```text
read entire file
→
decode blindly
```

---

# 9. Format Detection

优先级：

```text
Explicit User Choice
>
Content Signature / Magic Bytes
>
Trusted Decoder Detection
>
Extension Hint
>
Heuristic
```

最终结果必须标记：

```text
Explicit
Detected
Inferred
Unknown
```

---

# 10. Extension Is Not Truth

例如：

```text
photo.jpg
```

内容却是：

```text
PNG
```

应报告：

```text
Extension:
.jpg

Detected Format:
PNG

Mismatch:
Yes
```

而不是：

```text
强制当 JPEG
```

---

# 11. Format Capability Matrix

实现阶段必须生成真实矩阵，例如：

```text
Format | Decode | Encode | Alpha | Animation | Metadata Read | Metadata Write
PNG    | ?      | ?      | ?     | No         | ?              | ?
JPEG   | ?      | ?      | No    | No         | ?              | ?
WebP   | ?      | ?      | ?     | ?          | ?              | ?
GIF    | ?      | ?      | ?     | ?          | ?              | ?
BMP    | ?      | ?      | ?     | No         | ?              | ?
TIFF   | ?      | ?      | ?     | ?          | ?              | ?
```

其中：

```text
?
```

必须在真实代码和测试后替换。

禁止：

```text
“理论上支持”
```

作为发布依据。

---

# 12. PNG

至少验证：

```text
Decode
Encode
Transparency
RGBA
RGB
```

必要时：

```text
8-bit
16-bit
```

如果 16-bit 暂不支持：

必须明确：

```text
Not Supported
```

而不是：

```text
silent downconvert
```

---

# 13. JPEG

至少支持：

```text
Decode
Encode
Quality Control
RGB
```

注意：

JPEG 不支持：

```text
Alpha Channel
```

因此：

```text
PNG RGBA
→
JPEG
```

必须明确处理透明区域。

不能：

```text
transparent
→
black
```

然后不告诉用户。

---

# 14. Transparency Policy

当：

```text
source has alpha
destination has no alpha
```

必须要求：

```text
Background Color
```

或者提供：

```text
Predefined background policy
```

例如：

```text
White
Black
Custom
```

默认行为必须：

```text
explicit
```

---

# 15. Alpha Compositing

不能直接：

```text
drop alpha
```

必须执行：

```text
RGBA
+
Background
→
RGB
```

并测试：

```text
semi-transparent edges
```

防止：

```text
halo
```

问题。

---

# 16. WebP

至少验证：

```text
Decode
Encode
Quality
Lossless / Lossy where library supports
Alpha
```

如果支持：

```text
Animation
```

必须单独测试。

不要因为静态图片 WebP 能处理：

就宣称：

```text
animated WebP fully supported
```

---

# 17. GIF

GIF 必须首先区分：

```text
Static GIF
Animated GIF
```

M6 默认重点：

```text
Static Image Processing
```

如果实现 animated GIF：

必须建立：

```text
Frame Count
Frame Dimensions
Duration
Loop
```

等 capability。

不能：

```text
只处理第一帧
```

然后：

```text
显示 conversion success
```

如果第一版不处理 animated GIF：

必须：

```text
Not Supported
```

而不是：

```text
静默只取首帧
```

---

# 18. BMP

至少验证：

```text
Decode
Encode
Common bit depths
```

如果只支持：

```text
24-bit / 32-bit
```

必须明确。

---

# 19. TIFF

TIFF 可能存在：

```text
multiple pages
various compressions
high bit depth
different photometric interpretations
```

因此必须特别建立：

```text
Capability Matrix
```

不要：

```text
TIFF = 普通 JPEG
```

简单处理。

如果当前依赖不适合完整 TIFF：

可以：

```text
Metadata / Inspect
Decode selected variants
```

但必须明确边界。

---

# 20. Metadata 模型

建立：

```text
ImageMetadata
```

至少区分：

```text
MetadataFamily
MetadataField
Value
Source
PrivacySensitivity
Writable
Removable
```

至少考虑：

```text
EXIF
XMP where actually supported
ICC
DPI
Orientation
Creation Time
Camera Make
Camera Model
Lens
GPS
Software
Copyright
Comment
```

---

# 21. Metadata 与 Pixel Data 分离

必须：

```text
PixelData
```

与：

```text
Metadata
```

分开。

这样可以支持：

```text
Strip Metadata
```

而不需要：

```text
重新修改 pixel
```

除非目标格式编码方式确实要求重新编码。

---

# 22. EXIF

至少读取：

```text
Camera Make
Camera Model
DateTime
Orientation
Lens
Exposure-related common fields if available
GPS
```

但：

> **只报告真实解析成功的字段。**

不要：

```text
从文件名猜相机
```

---

# 23. GPS Privacy

GPS 是高敏感元数据之一。

必须在 UI 中明确：

```text
GPS Location
Present
```

并提供：

```text
Remove GPS
```

或者：

```text
Strip All Metadata
```

---

# 24. Strip Metadata

必须至少支持：

```text
Strip All Metadata
```

必要时可以：

```text
Keep ICC
Keep Orientation if required by output pipeline
Remove GPS
Remove EXIF
```

但：

> **任何“保留哪些元数据”的策略必须明确。**

---

# 25. Metadata Removal 不能破坏显示方向

例如：

```text
JPEG
pixels physically unrotated
EXIF Orientation = 6
```

如果：

```text
Strip EXIF
```

同时不处理：

```text
pixel orientation
```

最终图片可能：

```text
方向错误
```

因此必须明确：

```text
Orientation Policy
```

---

# 26. Orientation

至少支持：

```text
EXIF Orientation
```

显示时：

必须正确。

转换时：

必须明确是否：

```text
Honor Orientation
Physically Normalize Orientation
Preserve Orientation Metadata
```

不能：

```text
显示正确
导出错误
```

---

# 27. Recommended Orientation Policy

优先推荐：

```text
Decode
→
Apply EXIF orientation to pixels when transformation requires re-encoding
→
Write normalized orientation metadata or remove it according to policy
```

但必须：

```text
基于实际 codec 行为验证
```

不能仅凭理论。

---

# 28. ICC Profile

如果输入存在：

```text
ICC profile
```

必须：

```text
detect
report
```

如果输出格式允许：

优先：

```text
preserve
```

如果发生：

```text
color conversion
```

必须：

```text
explicit
```

不能：

```text
input profile discarded silently
```

---

# 29. Color Space

至少区分：

```text
RGB
RGBA
Grayscale
```

必要时：

```text
CMYK
Indexed
```

如果当前 capability 不支持：

必须：

```text
clear diagnostic
```

不能：

```text
silently reinterpret
```

---

# 30. Bit Depth

至少检测：

```text
8-bit
```

对于：

```text
16-bit
32-bit float
```

等高位深：

必须明确：

```text
supported
converted
unsupported
```

---

# 31. DPI

读取：

```text
DPI / Resolution metadata
```

必须区分：

```text
pixel dimensions
physical resolution
```

不要因为：

```text
300 DPI
```

就把：

```text
3000×2000
```

自动 resize。

---

# 32. Pixel Dimensions vs Print Dimensions

UI 必须明确：

```text
Width: 3000 px
Height: 2000 px

Resolution:
300 DPI
```

不要把：

```text
3000
```

显示成：

```text
3000 DPI
```

---

# 33. Image Inspector

至少显示：

```text
File Name
Format
Size
Width
Height
Aspect Ratio
Color Model
Bit Depth
Alpha
Orientation
Metadata Presence
EXIF
ICC
Animation
```

必要时：

```text
DPI
Frame Count
Compression Type
```

---

# 34. Image Inspector 不修改

默认：

```text
Read-only
```

只做：

```text
Inspect
```

如需清理：

进入：

```text
Metadata Cleaner
```

---

# 35. Resize 核心定义

Resize 允许：

```text
Width
Height
```

以及：

```text
Scale %
```

必须支持：

```text
Keep Aspect Ratio
```

---

# 36. Resize Modes

至少：

```text
Fit
Fill
Exact
Width Only
Height Only
```

但不要一次塞几十个模式。

---

# 37. Fit

例如输入：

```text
4000 × 3000
```

目标：

```text
1920 × 1920
```

Fit：

应得到：

```text
1920 × 1440
```

不得：

```text
1920 × 1920
```

导致拉伸。

---

# 38. Fill

Fill：

```text
4000 × 3000
→
1920 × 1920
```

必须：

```text
crop
```

而不是：

```text
distort
```

因此 Crop Position 必须至少支持：

```text
Center
```

必要时：

```text
Top
Bottom
Left
Right
```

但不要提前做完整自由裁切编辑器。

---

# 39. Exact

Exact：

```text
1920 × 1080
```

必须：

```text
1920 × 1080
```

无论原始比例。

但 UI 必须明确：

```text
Aspect Ratio will change
```

---

# 40. Scale

例如：

```text
50%
```

必须：

```text
4000 × 3000
→
2000 × 1500
```

尺寸计算必须：

```text
deterministic
```

并明确：

```text
rounding policy
```

例如：

```text
0.5
round half up
```

或：

```text
floor
```

只允许一种一致规则。

---

# 41. Resize Up / Down

必须区分：

```text
Downscale
Upscale
```

并允许：

```text
Prevent Upscaling
```

默认可以：

```text
ON
```

但以用户体验与实际测试为准。

关键是：

> **不要让“fit to 4000px”无意中把 800px 图片放大到 4000px。**

---

# 42. Resampling

至少验证：

```text
Nearest
Bilinear
Bicubic
Lanczos
```

实际支持的算法根据所选 image crate。

不要虚构。

推荐默认使用：

```text
high-quality general-purpose filter
```

而不是：

```text
nearest-neighbor
```

除非用户明确需要。

---

# 43. Alpha Resize

测试：

```text
transparent PNG
semi-transparent edges
text over transparency
```

防止：

```text
alpha premultiplication
```

错误导致边缘出现：

```text
dark halo
white halo
```

---

# 44. Resize Quality

Resize Preview 必须同时展示：

```text
Input
Output Dimensions
Estimated Output Size
```

如果输出编码尚未执行：

```text
Estimated
```

不要说：

```text
Output Size = 1.2 MB
```

---

# 45. Compress 核心定义

Compress 必须区分：

```text
Lossless
Lossy
```

以及：

```text
Format-specific encoder settings
```

---

# 46. Compression 不是 Resize

禁止 UI 出现：

```text
Compression = smaller width
```

这种概念混淆。

Compression：

```text
改变编码
```

Resize：

```text
改变像素尺寸
```

二者可以：

```text
串联
```

但在 M6 内必须：

```text
分别理解
```

---

# 47. JPEG Quality

JPEG：

```text
Quality 0–100
```

实际 encoder 可能不是线性关系。

因此 UI 可以使用：

```text
0–100
```

但不要告诉用户：

```text
Quality 80 = 80% image quality
```

因为这不是严格数学关系。

---

# 48. WebP Quality

同理：

必须以实际 encoder semantics 为准。

UI 可以：

```text
Quality
```

但 documentation 必须：

```text
说明是 encoder quality parameter
```

而不是：

```text
视觉质量百分比
```

---

# 49. PNG Compression

PNG 通常应明确：

```text
compression level
```

不应显示为：

```text
quality 1–100
```

如果项目采用统一 UI：

必须做好：

```text
format-specific mapping
```

而不是假装所有格式拥有同一参数语义。

---

# 50. Lossless Compression

必须允许：

```text
Lossless
```

至少对实际支持的格式验证。

例如：

```text
PNG
```

不应：

```text
decode
→
JPEG
```

然后称为：

```text
lossless compression
```

---

# 51. Lossy Warning

当：

```text
PNG → JPEG
```

或者：

```text
JPEG → JPEG
quality reduced
```

必须明确：

```text
Lossy
```

不能只显示：

```text
Compression complete
```

---

# 52. Format Convert

支持：

```text
PNG ↔ JPEG
PNG ↔ WebP
JPEG ↔ WebP
BMP ↔ PNG
GIF ↔ PNG where semantics allow
TIFF ↔ supported target where verified
```

但实际方向必须由：

```text
Capability Matrix
```

决定。

---

# 53. Format Convert 的核心原则

转换不是：

```text
rename extension
```

而是：

```text
Decode
→
Interpret
→
Transform if needed
→
Encode
```

---

# 54. Re-encoding Warning

例如：

```text
JPEG
→
JPEG
```

即使尺寸不变：

也属于：

```text
re-encode
```

可能产生：

```text
additional loss
```

因此 Preview 应：

```text
显示 Re-encoding
```

---

# 55. JPEG → JPEG

如果输入已经：

```text
JPEG
```

而用户只是：

```text
Change metadata
```

不要强制：

```text
re-encode pixels
```

如果库允许：

应尽量：

```text
metadata-preserving rewrite
```

否则必须说明：

```text
pixel data may be re-encoded
```

---

# 56. PNG → JPEG

必须配置：

```text
Background
```

如果 alpha 存在。

---

# 57. JPEG → PNG

不会恢复：

```text
original JPEG information
```

因此不能说：

```text
lossless restoration
```

必须明确：

```text
Format conversion only
```

---

# 58. GIF → PNG

如果输入：

```text
animated GIF
```

而目标：

```text
PNG
```

必须：

```text
reject
```

或者明确提供：

```text
First Frame Only
```

并且 Preview：

明确显示：

```text
Animation will not be preserved
```

---

# 59. Animation Policy

M6 首版如果不完整支持：

```text
GIF animation
WebP animation
```

可以明确限制：

```text
Static Images Only
```

这是允许的。

禁止：

```text
fake support
```

---

# 60. Metadata Preview

Metadata 工具至少支持：

```text
Inspect
Remove
```

可以进一步：

```text
Remove EXIF
Remove GPS
Remove all removable metadata
```

但每一项都必须说明：

```text
What will be removed
What may remain
```

---

# 61. “Strip Metadata” 不是绝对删除一切

不同格式：

可能仍然有：

```text
format-required metadata
encoder metadata
color profile
orientation
container metadata
```

所以 UI 文案不能：

```text
“Delete absolutely all metadata”
```

除非真实 codec 能证明。

更准确：

> **Remove removable metadata supported by the selected format and encoder.**

---

# 62. Metadata Privacy Summary

可以显示：

```text
Privacy Risk:
GPS present
Camera model present
Creation time present
```

但这只是：

```text
metadata presence
```

不要说：

```text
This image is private
```

---

# 63. Preview Strategy

M6 必须提供：

```text
Before
After
```

至少包括：

```text
Image Preview
Dimensions
Format
File Size
Metadata Summary
Warnings
```

---

# 64. Preview 使用真实处理管线

禁止：

```text
preview:
fake thumbnail
```

而：

```text
execute:
different code path
```

必须：

```text
Preview uses same transformation engine
Execute uses same transformation engine
```

区别只在于：

```text
destination side effect
```

---

# 65. Preview Resolution

对于：

```text
8000×6000
```

Preview 不应：

```text
decode at full resolution
```

如果只需 UI：

应建立：

```text
bounded preview decode
```

例如：

```text
max preview pixels
```

避免：

```text
4K/8K image
→
巨大 UI memory
```

---

# 66. Preview Quality

Preview 可以：

```text
scaled representation
```

但必须：

```text
clearly labeled
```

例如：

```text
Preview
```

不需要声称：

```text
pixel-perfect at 100%
```

如果实际不是。

---

# 67. Before / After Side-by-side

推荐：

```text
Before
│
│
After
```

支持：

```text
Fit
1:1 where feasible
```

但不要提前做：

```text
professional image comparison suite
```

---

# 68. Resize Preview Summary

例如：

```text
Input:
4000 × 3000

Output:
1920 × 1440

Scale:
48%

Aspect Ratio:
Preserved

Estimated Size:
1.8 MB

Output:
WebP
Quality:
82
```

---

# 69. Compression Preview Summary

至少：

```text
Input Size
Estimated Output Size
Ratio
Format
Quality / Compression Level
Lossless / Lossy
```

如果 output size 只能在 encode 后知道：

必须：

```text
Estimated
```

---

# 70. Real Output Size

Execute 完成后：

必须报告：

```text
Actual Output Size
```

不要使用 Preview 的：

```text
Estimate
```

冒充：

```text
Actual
```

---

# 71. Compression Ratio

定义：

```text
Compression Ratio
=
output_size / input_size
```

或者：

```text
Size Reduction
=
1 - output_size/input_size
```

UI 必须选定一种。

例如：

```text
Original:
5.0 MB

Output:
2.1 MB

Reduction:
58%
```

---

# 72. Metadata-Only Operation

允许：

```text
Strip Metadata
```

本质上属于：

```text
Image rewrite
```

因此：

```text
Preview
Safe Write
Transaction
History
```

仍然适用。

---

# 73. File Mutation Boundary

打开：

```text
photo.jpg
```

执行：

```text
Resize
Compress
Convert
Strip Metadata
```

默认：

> **不修改原文件。**

默认输出：

```text
new destination
```

例如：

```text
photo.webp
```

---

# 74. Source Overwrite

可以支持：

```text
Replace Source
```

但必须：

```text
explicit
```

而且：

```text
Safe Write
TOCTOU
Atomic Write
Transaction
History
Undo
```

全部复用 M2。

---

# 75. Collision Safety

目标文件已经存在：

```text
photo.webp
```

禁止：

```text
silent overwrite
```

可提供：

```text
Replace
Skip
Rename
Cancel
```

具体支持哪些：

基于现有 M2 能力。

---

# 76. Output Naming

M6 默认可以：

```text
same stem
new extension
```

例如：

```text
photo.jpg
→
photo.webp
```

不要自动：

```text
photo_final_v2_new_2.webp
```

这种不可预测命名。

如果存在 collision：

复用：

```text
Collision Infrastructure
```

---

# 77. Multi-image Naming

多图：

```text
a.jpg
b.png
c.jpeg
```

输出：

```text
a.webp
b.webp
c.webp
```

必须：

```text
deterministic
```

---

# 78. Destination Structure

至少支持：

```text
Same Folder
Selected Output Folder
```

如果：

```text
input folder
```

含子目录：

M6 首版不要自动实现：

```text
复杂镜像目录树
```

除非实际需求明确。

---

# 79. Multi-Image Processing Model

建议：

```text
ImageBatchPlan
```

但它只能描述：

```text
same operation
same options
multiple inputs
```

例如：

```text
Operation:
Resize

Options:
1920 px
Keep Aspect

Inputs:
100 files
```

不要变成：

```text
generic pipeline
```

---

# 80. Batch Preview

多图 Preview：

不能：

```text
1000 张全部渲染
```

必须：

```text
Representative Preview
+
Aggregate Summary
```

例如：

```text
Preview:
showing 12 representative images

Planned:
100 images

All:
same resize operation
```

---

# 81. Representative Sampling

采样必须：

```text
deterministic
```

例如：

```text
first
middle
last
size-stratified sample
```

不能：

```text
random every refresh
```

导致 Preview 不稳定。

---

# 82. Batch Result

必须统计：

```text
Total
Succeeded
Failed
Skipped
Cancelled
```

以及：

```text
Input Bytes
Output Bytes
Total Reduction
```

---

# 83. Per-file Result

每个文件：

至少：

```text
Input
Output
Status
Error
Warning
Input Size
Output Size
```

状态：

```text
Success
Failed
Skipped
Cancelled
```

---

# 84. Failure Isolation

1000 张图片中：

```text
997 Success
2 Failed
1 Skipped
```

必须允许：

```text
整个任务继续
```

前提：

> 单个失败不会导致后续安全性不确定。

---

# 85. 不安全恢复必须停止

如果出现：

```text
filesystem corruption
destination root inaccessible
transaction integrity failure
unexpected global decoder failure
resource budget violation
```

不得：

```text
继续处理
```

必须：

```text
stop safely
```

---

# 86. Per-file Decode Failure

例如：

```text
003.jpg
```

损坏。

结果：

```text
Failed
Reason:
Image decode failed
```

其他：

```text
001.jpg
002.jpg
004.jpg
```

仍可处理。

---

# 87. Cancellation

用户点击：

```text
Cancel
```

必须：

```text
cancel signal
→
decoder / transform / encoder
```

尽可能传播。

不是：

```text
UI status = cancelled
backend continues
```

---

# 88. Cancellation Semantics

取消后：

必须明确：

```text
Completed
Cancelled before start
Cancelled during processing
Output already written
Output not written
```

---

# 89. Partial Batch Cancellation

例如：

```text
100 files
```

处理到：

```text
63
```

用户取消。

结果：

```text
63 processed
37 not started / cancelled
```

必须准确记录。

---

# 90. No False Success

如果：

```text
output file partially written
```

绝不能：

```text
Status = Success
```

必须：

```text
Failed / Interrupted
```

并清理：

```text
temporary output
```

如安全可行。

---

# 91. Transaction

M6 多文件处理：

必须尽量：

```text
plan
→
prepare
→
process
→
commit output
```

但必须注意：

> **“全成功事务”与“多文件独立处理”不是一回事。**

不要为了概念完整：

```text
1000 images
+
single global transaction
```

导致：

```text
1 failure
→
rollback 999 good outputs
```

M6 更合理：

```text
per-file transaction
+
batch aggregate result
```

除非实际 M2 infrastructure 已经支持更好的模型。

---

# 92. History

多图操作 History 记录：

```text
OperationId
Tool
Operation
Input Count
Success Count
Failed Count
Skipped Count
Output Folder
Summary
Timestamp
```

不要保存：

```text
image pixels
```

也不要保存：

```text
thumbnails
```

除非当前架构明确要求，并且有严格限制。

---

# 93. Undo Multi-image

Undo：

必须：

```text
revalidate
```

每个 output file：

检查：

```text
still exists
identity unchanged
not modified by user
```

只有满足条件：

才允许安全撤销。

---

# 94. Undo 不覆盖用户后续修改

如果：

```text
Weave output
→
user opens Photoshop
→
modifies output
```

然后：

```text
Undo
```

必须：

```text
Conflict
```

而不是：

```text
delete file
```

---

# 95. Output Identity

M6 最好记录：

```text
output path
+
size
+
mtime
+
hash where justified
```

至少采用：

```text
M2 existing identity mechanism
```

防止：

```text
Undo deletes wrong file
```

---

# 96. Metadata Strip Undo

如果：

```text
Replace Source
```

然后：

```text
Undo
```

必须真正恢复：

```text
original bytes
```

或者：

```text
transaction snapshot
```

必须可证明安全。

不能：

```text
重新编码
```

然后假装恢复原图。

---

# 97. Replace Source 优先级

M6 的默认推荐：

```text
Export New File
```

而不是：

```text
Replace Source
```

因为：

```text
new file
```

风险更低。

---

# 98. Image Inspector Facts

Inspector 必须把：

```text
FACT
```

与：

```text
INFERENCE
```

分开。

例如：

```text
FACT:
Format = JPEG

FACT:
Width = 4032

FACT:
EXIF GPS present

INFERENCE:
Likely captured by camera
```

默认不要展示第二类。

---

# 99. File Metadata vs Image Metadata

必须分开：

```text
Filesystem Metadata
```

例如：

```text
Path
Size
Created
Modified
```

与：

```text
Image Metadata
```

例如：

```text
EXIF
ICC
Orientation
```

不要混在一个平面数据结构中。

---

# 100. M1 Reuse

必须复用：

```text
Filesystem abstraction
Path Safety
File Metadata
Hash
Streaming file access
Cancellation
Progress
Structured Errors
Resource Limits
```

---

# 101. M2 Reuse

必须复用：

```text
OperationId
OperationPlan
Preview
Collision Detection
Safe Write
Atomic Write
Transaction
History
Undo
TOCTOU
```

---

# 102. M3 Reuse

可以复用：

```text
Streaming hash
Deterministic result ordering
bounded concurrency
resource limit patterns
```

如需要：

```text
output verification hash
```

也优先调用既有 hash abstraction。

---

# 103. M4 Reuse

如项目已有：

```text
Diagnostics
Text / Binary detection helpers
Encoding infrastructure
```

可复用。

但：

> **不要把 ImageDocument 做成 TextDocument 的子类。**

---

# 104. M5 Reuse

Data 与 Image 之间可以共享：

```text
Progress
Cancellation
Preview
Export
Result
Diagnostics
```

但：

```text
DataDocument
```

与：

```text
ImageDocument
```

必须保持领域边界。

---

# 105. weave-media

建议：

```text
weave-media
```

职责：

```text
ImageDocument
ImageFormat
ImageDecoder
ImageEncoder
ImageMetadata
Resize
Compress
Convert
MetadataStrip
Preview
ImageBatchPlan
ImageResult
```

不负责：

```text
React
Tauri windows
Filesystem policy
History storage
Path validation
OS integration
```

---

# 106. Codec Adapter

建议：

```text
ImageCodec
```

统一抽象：

```text
probe
decode
decode_thumbnail
encode
capabilities
```

但注意：

> **不要为了抽象而抽象。**

如果当前实际 codec library 已经提供合理统一接口：

优先：

```text
wrap
```

而不是：

```text
build giant codec framework
```

---

# 107. Capability API

建议：

```text
ImageCapabilities
```

例如：

```text
decode
encode
alpha
animation
metadata_read
metadata_write
lossless
lossy
bit_depth
color_space
```

UI 根据：

```text
capability
```

显示真实选项。

---

# 108. Unsupported Handling

例如用户选择：

```text
TIFF → WebP
```

但当前环境不支持：

必须：

```text
Unsupported
```

并说明：

```text
TIFF decoder unavailable
```

不要：

```text
转换按钮灰掉
```

但不解释。

---

# 109. Dependency Strategy

优先：

```text
existing dependency
>
small mature Rust image libraries
>
new dependency only with explicit justification
```

每个新增 crate：

必须记录：

```text
Purpose
Why Needed
Alternatives
License
Security
Maintenance
Bundle Impact
```

codec 选型决策（实现前强制）：

- 纯 Rust image 生态对 WebP 有损编码（含 quality 参数）支持有限，必须在 Phase 0 做 encode spike 验证 WebP / 动画 GIF / TIFF 的真实编码能力。
- spike 结论（可行/不可行/需引入 libwebp 绑定等）写入 DECISIONS.md 与 IMAGE_FORMATS.md，Capability Matrix 依据 spike 结果定稿。
- EXIF 读写候选（如 kamadak-exif）须评估 license 与维护状态并记录。

---

# 110. 禁止巨大依赖

除非真实证明必要：

不要为了 M6 引入：

```text
full image editing engine
desktop graphics suite
browser runtime
Python image stack
external CLI suite
cloud image SDK
```

---

# 111. No CLI Wrappers

禁止核心能力依赖：

```text
ImageMagick CLI
GraphicsMagick CLI
ffmpeg
Python PIL
```

通过：

```text
shell
```

调用实现。

如果未来真的采用 external tool：

必须是：

```text
explicit architecture decision
```

但 M6 默认目标是：

> native local Rust image pipeline.

---

# 112. No Shell

禁止：

```text
Command::new(...)
```

调用图像处理工具作为主要实现。

---

# 113. No Network

M6：

```text
100% local
```

不得：

```text
upload image
download remote codec
fetch metadata
```

---

# 114. Privacy

必须特别考虑：

```text
EXIF GPS
Camera Model
Creation Time
Location
Device identifiers
Software tags
```

这些信息：

```text
never leave machine
```

---

# 115. Logging

禁止日志写：

```text
full image metadata dump
GPS coordinates
file contents
private folder structure unnecessarily
```

优先：

```text
path redaction
metadata summary
error code
```

---

# 116. Security Threat Model

必须测试：

```text
Path Traversal
Symlink
Huge Dimensions
Huge Decoded Pixel Budget
Malformed Header
Malformed Chunk
Malformed Metadata
Huge Metadata
Integer Overflow
Memory Exhaustion
Recursive Metadata structures where applicable
Output Collision
TOCTOU
Partial Write
```

---

# 117. Metadata Bomb

Metadata 可以异常巨大。

必须设置：

```text
Max Metadata Bytes
Max Metadata Entries
Max String Length
```

达到上限：

```text
diagnostic
```

不得：

```text
无限解析
```

---

# 118. Huge Dimension Guard

例如：

```text
width = 1,000,000
height = 1,000,000
```

不得无条件：

```text
allocate RGBA buffer
```

必须：

```text
reject
```

或：

```text
bounded handling
```

---

# 119. Decode Memory Estimate

decode 前最好估算：

```text
width × height × channels × bytes
```

若超过预算：

```text
DecodeRejectedByResourceLimit
```

并告诉用户：

```text
Image dimensions exceed the configured safety limit.
```

---

# 120. Preview Memory Budget

缩略图也必须：

```text
bounded
```

例如：

```text
max preview dimension
```

不要：

```text
40000×30000
→
preview full decode
```

---

# 121. Multi-image Memory

禁止：

```text
100 images
→
all decoded simultaneously
```

必须：

```text
one-at-a-time
```

或：

```text
small bounded worker pool
```

---

# 122. Concurrency

多图处理：

必须：

```text
bounded concurrency
```

不能：

```text
one task per image
```

然后：

```text
10000 images
→
10000 memory-heavy decoders
```

---

# 123. Worker Sizing

不要默认：

```text
CPU count = decoder worker count
```

因为：

```text
memory per image
```

可能差异巨大。

实际 worker count 应综合：

```text
CPU
estimated decode memory
output workload
```

由测试确定。

---

# 124. Backpressure

必须：

```text
input enumeration
→
bounded queue
→
workers
→
output writer
```

不能：

```text
enumerate all
→
create all jobs
→
queue unlimited
```

---

# 125. Deterministic Result Ordering

无论多线程：

结果最终必须按：

```text
input deterministic order
```

或者：

```text
explicit result ordering
```

输出。

---

# 126. Input Enumeration

文件列表必须：

```text
sorted
```

或遵循：

```text
M1 deterministic filesystem listing
```

不要依赖：

```text
filesystem enumeration order
```

---

# 127. Batch Input Selection

允许：

```text
individual files
folder selection
drag multiple files
```

如果选择 folder：

必须：

```text
respect explicit recursion option
```

默认不要：

```text
无限递归
```

---

# 128. Unsupported Input

例如：

```text
document.pdf
video.mp4
README.txt
```

在 Image tool：

应：

```text
Unsupported input
```

并允许：

```text
Skip unsupported
```

或者：

```text
Show diagnostics
```

具体 UI 依当前框架。

---

# 129. Mixed Selection

用户一次拖入：

```text
a.jpg
b.png
c.pdf
d.csv
```

Image Tool：

应该：

```text
Detect
→
eligible images
→
unsupported entries
```

并显示：

```text
Images:
2

Unsupported:
2
```

不能：

```text
全部直接失败
```

也不能：

```text
忽略 PDF/CSV
```

然后不告诉用户。

---

# 130. Batch Filter

M6 可以使用简单：

```text
Image-only
```

过滤。

不要提前做：

```text
Generic file query language
```

---

# 131. Image Conversion Options

Options 必须结构化：

```text
ResizeOptions
CompressionOptions
ConversionOptions
MetadataOptions
```

避免：

```text
one giant options object with 100 nullable fields
```

---

# 132. ResizeOptions

至少：

```text
mode
width
height
scale
keep_aspect
prevent_upscale
resampling_filter
background_if_needed
```

只保留当前真实支持的字段。

---

# 133. CompressionOptions

例如：

```text
quality
lossless
compression_level
speed
```

根据格式能力。

禁止：

```text
quality
```

对所有 codec 强行使用。

---

# 134. MetadataOptions

至少：

```text
preserve_metadata
strip_all
strip_exif
strip_gps
preserve_icc
normalize_orientation
```

实际实现多少：

以 capability 为准。

---

# 135. ConvertOptions

至少：

```text
output_format
metadata_policy
alpha_policy
quality
compression
orientation_policy
```

---

# 136. Transform Plan

每一次图片操作必须可以形成：

```text
ImageOperationPlan
```

至少：

```text
Input
Output
Operation
Options
Expected Changes
Warnings
Resource Estimate
```

---

# 137. Plan 不代表成功

必须区分：

```text
Planned
Started
Completed
Failed
Skipped
Cancelled
```

---

# 138. Resource Estimate

Preview 或 plan 阶段可以估算：

```text
Decoded Memory
Output Estimate
Processing Cost
```

如果是 estimate：

必须：

```text
label = Estimated
```

---

# 139. Expected Changes

例如：

```text
Before:
4000×3000 JPEG
5.2 MB
EXIF GPS present

After:
1920×1440 WebP
Estimated 1.4 MB
GPS removed
Alpha: unsupported
```

---

# 140. Preview Safety

Preview 本身：

不能：

```text
write destination
```

也不能：

```text
modify source metadata
```

---

# 141. Preview Cache

可以使用：

```text
bounded in-memory preview cache
```

但不得：

```text
永久缓存原图
```

不得：

```text
写入用户目录
```

除非已有统一 cache infrastructure。

---

# 142. Cache Limits

至少：

```text
max entries
max decoded pixels
max bytes
TTL / session lifetime
```

达到限制：

```text
evict
```

---

# 143. Cache Identity

preview cache key 至少应包含：

```text
source identity
+
operation options
```

防止：

```text
same path
+
different operation
→
stale preview
```

---

# 144. Source Change Invalidation

文件在预览之后被外部修改：

必须使：

```text
preview stale
```

执行时重新：

```text
validate source
```

不能：

```text
沿用旧 preview
→
写到新 source
```

---

# 145. TOCTOU

所有 source mutation：

必须重新检查：

```text
expected identity
```

至少复用 M2。

---

# 146. Atomic Output

输出路径：

建议：

```text
temp
→
write
→
flush
→
commit
```

具体遵循：

```text
M2 Safe Write
```

---

# 147. Output Verification

完成后至少检查：

```text
exists
size > 0 where expected
decoder can reopen
```

必要时：

```text
format matches expected
dimensions match expected
metadata policy respected
```

---

# 148. Roundtrip Verification

例如：

```text
PNG
→
WebP
→
decode
```

必须验证：

```text
output opens
dimensions correct
alpha policy correct
```

不能只检查：

```text
file exists
```

---

# 149. Corrupt Output Detection

如果 encoder：

```text
returns success
```

但输出：

```text
cannot be decoded
```

必须：

```text
mark failure
```

不能：

```text
History = Success
```

---

