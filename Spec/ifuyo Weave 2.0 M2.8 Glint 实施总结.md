# ifuyo Weave 2.0 M2.8 Glint 屏幕捕获中心 — 实施总结

日期：2026-10-06
状态：**P0 完成（如实 Partial 项见 §14）**；门禁全绿。

---

## 1. 实际完成范围

- **捕获核心**（`src-tauri/src/glint/capture.rs`）：
  - 显示器枚举（`screenshots` crate，物理像素，坐标可负 §33）
  - 全屏捕获（单显示器 / 多显示器按 index）
  - 区域捕获 = 全屏捕获后裁剪（`crop_rgba` 纯函数，边界校验）
  - 前台窗口捕获（`x-win` 安全封装；`ForegroundWindow` 结构含屏幕矩形+所属显示器；最小化/零尺寸/未知显示器 ⇒ 结构化错误 `glint.windowMinimized` / `glint.windowBoundsFailed` / `glint.monitorNotFound`，**不伪造截图** §7）
  - 唯一输出命名 `Glint_<epoch-ms>.png`，冲突追加 `-N`，**不覆盖**（§8，单测覆盖）
- **标注**（`glint/annotation.rs`）：非破坏 Annotation Layer——`render(base, w, h, &[Annotation])` 纯函数返回新缓冲，原图永不修改（§12）。7 种 P0 形状全部实现：Arrow / Rectangle / Ellipse / Freehand / Text / Blur（3×3 盒模糊×3 迭代）/ Pixelate（块平均）。Text 经 `ab_glyph` 读取系统字体（msyh.ttc → simsun.ttc → arial.ttf，全部缺失 ⇒ `glint.fontUnavailable`，**不伪造**）；支持中文。
- **OCR**（`glint/ocr.rs`）：Windows.Media.Ocr（WinRT，`windows` crate 安全封装，workspace `unsafe_code=forbid` 不受影响）。语言 hint → 用户档案语言回退；大图超 `MaxImageDimension` 先缩放；行级文本保留，**行级 confidence WinRT 不提供 ⇒ None（不伪造 §16）**；引擎不可用 ⇒ 结构化 `glint.ocrUnavailable`。
- **存储**（`glint/storage.rs`）：`history_dir/glint-captures`；`list_recent`（mtime 降序、仅管 `Glint_` 前缀、**超出 max_recent=100 自动删最旧** §26）；`delete_capture` 路径穿越防护（`/`、`\`、`..` 拒绝 + `starts_with` 校验）。
- **IPC 服务**（`glint_service.rs`，13 条命令）：`glint_list_monitors` / `glint_list_windows` / `glint_list_recent` / `glint_capture_screen` / `glint_capture_foreground_window` / `glint_crop_capture`（从已捕获图裁剪，**不重拍，时刻一致**）/ `glint_save_annotated`（渲染+导出 PNG/JPEG，写新文件）/ `glint_preview`(fit 1600) / `glint_thumbnail`(fit 320) / `glint_delete_capture` / `glint_copy_capture_to_clipboard` / `glint_ocr_capture` / `glint_ocr_region`。
- **UI**（`ui/src/features/glint/GlintPanel.tsx`）：显示器选择+全屏/前台窗口捕获按钮；预览（元数据：物理尺寸/文件大小/时间/来源）；区域模式（图上拖拽选区、实时尺寸、**Enter 确认 / Esc 重置** §6）；标注模式（7 工具+撤销+保存，选区/自由手实时可视化）；OCR 结果面板+复制文本；最近捕获（懒加载缩略图前 24 张 §36、打开/删除）；导航页 + palette 命令（`Open Glint` / `Capture Full Screen` / `Capture Window` §46）；i18n zh/en 同笔、key 集合一致（既有测试守门）。

## 2. 未完成范围（如实）

- **Agent/Hook 触发**（§21）：palette 命令已接；全局热键→捕获未接（M2.6 无热键基建，未造第二套）。
- **Recipe 节点**（§39）：仅接口就绪（Glint 能力即服务命令；CaptureService/OcrService 边界真实存在）；无 Recipe Action 类型新增——按 Spec 本阶段不要求。
- **录屏**（§52）：接口边界未单列（P1 且现有技术栈成本高，按 Spec 允许不引入视频依赖）。
- **Settings 页 Glint 段**（§47）：默认值合理硬编码（PNG、保存目录 glint-captures、max_recent=100）；设置面板项未加。
- **Privacy Preflight 集成**（§28）：未接（Glint 自身有 Blur/Pixelate；M2.4 扫描仅对用户显式导出路径——未接入，如实 Partial）。

## 3. 代码变更

- 新增：`src-tauri/src/glint/{capture,ocr,storage,annotation,mod}.rs`、`src-tauri/src/glint_service.rs`、`ui/src/features/glint/GlintPanel.tsx`
- 修改：`Cargo.toml`（workspace：`windows`/`screenshots`/`x-win`/`ab_glyph`）、`src-tauri/Cargo.toml`（同 + WinRT features）、`src-tauri/src/lib.rs`（模块+13 命令注册）、`ui/src/app/App.tsx`（导航/palette/捕获请求）、`ui/src/stores/appStore.ts`（`glintCaptureRequest`）、`ui/src/i18n/{en,zh-CN}.json`（33 键）、`ui/src/generated/bindings.ts`

## 4. 架构复用情况（§44 无第二套）

| 能力 | 复用 | 不重复建 |
|---|---|---|
| Image 编码/缩放 | `weave-media::ops::{encode_image,resize_image}` | 无第二套 Image Pipeline（§18）|
| Clipboard 写入 | M2.3 `arboard` 路径 + `suppress` 指纹 | 无第二套 ClipboardWriter（§10）|
| 存储/命名 | `glint::storage` 沿用 history_dir 约定 | 无第二套 Storage |
| 错误/路径安全 | `WeaveError` + 统一 IpcError 翻译 | 无第二套 Sanitizer（§37）|
| 通知 | 错误经既有 error 通道显示 | 无连续 toast（§48 安静）|

## 5. Capture 实现方式

`screen-rs` 家族 `screenshots 0.8`（内部封装 unsafe；**我们的代码保持 safe**——workspace `unsafe_code="forbid"` 约束下不能手写 Win32 DXGI/GDI）。窗口枚举/前台窗口用 `x-win 2`（同样安全包装）。区域=「先捕获后裁剪」物理像素模型。

## 6. DPI / Multi-monitor 处理

- 全链路**物理像素**（screenshots 返回物理尺寸；UI 拖拽经 `naturalWidth/clientWidth` 比例映射回物理坐标）——逻辑/物理不混用（§5）。
- 显示器坐标系含负值（左侧显示器，§33）；窗口中心点归属显示器判定；窗口跨显示器超出部分裁掉（如实）。
- Window capture = 屏幕位图裁剪窗口矩形（DWM 阴影未去除——如实限制）。

## 7. Clipboard 集成

`glint_copy_capture_to_clipboard`：单次解码（§34 不重复 Bitmap Copy；arboard 需要所有权 ⇒ 仅像素缓冲克隆一次）→ `set_image` → **抑制指纹 = 规范 PNG（RGBA→PNG）SHA-256**，与 M2.3 监听端 `hash_hex(encode_image(Png))` 同构 ⇒ 自循环消除（§10/§15）。OCR 文本复制走前端 `navigator.clipboard.writeText` → M2.3 作为外部文本正常入历史（§17 期望行为）。

## 8. Annotation

非破坏：`Original + AnnotationDocument = Rendered`；保存标注 ⇒ **新文件**（原图保留）；UI 撤销=移除最后对象；渲染器纯 Rust 零平台依赖、7 测试（区域外像素不变、块一致性、越界 clamp、字体缺失结构化）。

## 9. OCR

WinRT `Media.Ocr`；字节→`InMemoryRandomAccessStream`→`BitmapDecoder`→`SoftwareBitmap`→`RecognizeAsync`（`.get()` 阻塞于后台 IPC 线程，不阻塞 UI 主线程 §20）。**OCR 文本只经返回值**，日志无内容（§27）。无引擎环境（CI）返回 `glint.ocrUnavailable`——测试双向接受。

## 10. Storage

有界：`list_recent` 每次列目录时清理超限旧文件（max 100）。命名 `Glint_<epoch-ms>` 保证唯一+可排序（同毫秒冲突 `-N`）。删除仅限 Glint 目录内文件。

## 11. Performance

编码/缩放/OCR 全在 Rust IPC 线程（不阻塞 UI 主线程）；预览 fit-1600、缩略图 fit-320（懒加载、上限 24 张在列表）；单次解码纪律。**未做**：专门基准（Spec §34 的耗时表——P2）。

## 12. Lifecycle

无后台常驻线程/句柄；OCR 的 WinRT async 用 `.get()` 同步等待（作用域内完成，无泄漏路径）；App Exit 无需 Glint 清理（§32：文件输出是 Create New File，无部分写入——`fs::write` 原子性依赖 OS）。

## 13. Tests

- Rust：glint 10 测试（crop 边界、唯一命名、7 种标注渲染/非破坏/越界/字体缺失结构化、OCR 不可用结构化）
- 全工作区 **42 套件 0 failed**；`fmt`/`clippy` **0**；tsc 0；eslint 绿；**vitest 38 绿**（含 i18n key 对等、design-token 无裸色值守门）；`tauri build` 通过
- 集成真实捕获/窗口枚举需要桌面环境——单测覆盖纯函数部分，系统调用部分以结构化错误路径测试

## 14. 已知限制（P0/P1/P2）

- **P2**：捕获包含 Weave 自身窗口（未做 hide-then-capture；窗口捕获未去 DWM 阴影）；OCR 行级 confidence 为 None（引擎不提供）；Audit 走 tracing 元数据日志（weave-history Audit 是文件事务派生，不适用 Create-New-File 场景）；Settings 未加 Glint 段；热键未接。
- **P1**：无。已知限制均为如实标注的能力边界，非正确性/稳定性缺陷。
- Spec §6「重调边缘/移动选区」、§24「Show in Explorer」：P2 未做（Enter/Esc+拖拽已覆盖主流程）。

## 15. P0/P1/P2 问题清单

无 P0/P1。代码审查（§57 FACT/HYPOTHESIS）：
- FACT：上述全部已实现且门禁全绿。
- HYPOTHESIS：`x-win` 前台窗口在多显示器混合 DPI 下矩形可能含逻辑像素（未在 125%/150% 实机矩阵验证——单机 100% 开发环境）。
- INFERENCE：区域裁剪模型下即使窗口矩形偏差也只影响窗口捕获，不影响全屏/区域路径。

## 16. 后续 M2.9 接口影响

Glint 能力以服务命令存在（`glint_capture_*` / `glint_ocr_*`），M2.9 Wisp（Paste Guard）与 M2.10+ 可直接调用；无需改动 Glint 公共面。`GlintCaptureSaved.source` 已带窗口标题/进程等元数据，供后续 History 呈现。
