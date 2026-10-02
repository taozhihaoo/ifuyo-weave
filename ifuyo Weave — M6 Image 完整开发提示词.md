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