# IMAGE_FORMATS

M6 Image 支持的格式、编解码能力与边界（下 §265）。能力由真实编解码测试
固化（§11），非"理论上支持"。

## Capability Matrix（实测，D49）

| 格式 | Decode | Encode | Alpha | Animation | EXIF 读 | 有损编码 |
| --- | --- | --- | --- | --- | --- | --- |
| PNG  | ✅ | ✅ | ✅（读写） | ❌（APNG v1 不支持） | ✅（eXIf chunk） | ❌（无损） |
| JPEG | ✅ | ✅ | ❌（无 alpha） | ❌ | ✅（APP1 EXIF） | ✅（quality 0–100） |
| WebP | ✅ | ✅ | ✅（无损 WebP 保留） | ❌（animated WebP v1 不支持） | ✅（EXIF chunk） | ❌（**纯 Rust 生态无有损编码**，§109 spike） |
| GIF  | ✅（含多帧计数） | ✅（单帧） | ✅（二值透明） | ❌（v1 Static Only） | ❌ | ❌ |
| BMP  | ✅ | ✅ | ✅（32-bit） | ❌ | ❌ | ❌ |
| TIFF | ✅（首页） | ✅ | ✅ | ❌（多页 v1 不支持） | ✅ | ❌ |

## 解码安全

- Header 守卫先于解码：dimension ≤ 16384、pixels ≤ 80M、decoded RGBA
  ≤ 256 MiB（超 ⇒ `DecodeRejectedByResourceLimit`）。
- 畸形/截断输入 ⇒ 结构化错误，绝不 panic（fuzz-boundary 测试覆盖）。
- 动画：GIF/WebP 帧数为事实；转换到静态格式显式拒绝保留动画（§59）。

## 元数据（Metadata）

- **读取**：kamadak-exif（JPEG APP1 / TIFF / PNG eXIf / WebP chunk）；
  只报告真实解析成功的字段（§98 Fact only）。
- **Strip 语义**（§61）：strip = 重编码且不写入 EXIF/ICC/XMP——
  "Remove removable metadata supported by the selected format and
  encoder"。不宣称绝对删除一切。
- **Orientation**（§25/§27）：strip/转换时物理归一像素（宽高互换），
  输出不再携带 orientation tag——显示方向不破坏。

## Resize 语义（§35–§44）

- Fit 等比适配 / Fill 等比填满+居中裁 / Exact 强制 / Scale 百分比
  （round-half-up，确定性）。
- 防放大默认 ON（§41）。
- 过滤器：Lanczos3（默认）/ CatmullRom / Triangle / Nearest。

## 已知边界

- WebP 有损编码 NOT SUPPORTED（纯 Rust 生态，§109 spike）
- 动画 GIF/WebP：Static Only
- TIFF 多页：仅首页
- 16-bit PNG：解码后按 8-bit 处理（位深事实如实报告）
- JPEG→JPEG：重编码（pixel data may be re-encoded，§55 警示）
