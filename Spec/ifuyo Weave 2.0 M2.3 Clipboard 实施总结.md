# ifuyo Weave 2.0 — M2.3 Clipboard 实施总结

日期：2026-10-06。提交链：C1 1139865 → lint 8b6356a → C1b（thumbnail 字段）→
C2/C3 e8dd076 → C4 34c607c → C5（perf + 本文档 + PROGRESS）。

## 1. 实现范围

可靠记录剪贴板历史（Text/Image/File），搜索/预览/重新复制/收藏/删除/清空，
保留策略与隐私边界（暂停/容量上限/敏感标记），Agent 式后台监听生命周期。
**不越界**：无 Paste Guard（M2.9）、无云同步、无密码保险箱、无 AI、无 Windows
Service（§88 全部未触碰）。

## 2. 复用的基础设施

- M2.0：WeaveError/AppConfig（serde default 向下兼容）/CancellationToken 语义；
  历史目录约定（history_dir）；原子写模式（temp+rename）。
- M2.0 `weave-files::hash`：新增 `hash_bytes`（与 hash_file 同源 sha2 实现，
  剪贴板指纹复用——不重写 CalculateHash §86）。
- M2.2 范式：Job 化缓存领取模式第三次复用（本次为直读运行时，无需 job——
  监听是常驻线程而非一次性任务）。
- `weave-media::ops`：缩略图 resize/encode（320px PNG，§9 失败降级）。
- 存储：JSON 索引 + payload 文件（§31 Index/Payload 解耦）——与
  HistoryStore 同构（versioned schema/原子写/损坏隔离），未引入数据库。

## 3-4. 架构与数据模型

```text
平台层（src-tauri clipboard_service）        领域层（weave-clipboard crate）
arboard（text/image 读写）  ─┐
clipboard-win（CF_HDROP 文件）─→ CaptureInput ──▶ CapturePipeline（指纹/去重/
1s 轮询 + 图像 5s 节流探测       │                 保留/敏感启发）→ Store
暂停/启用 AtomicBool             │                 ├─ entries.json（v1 原子写）
抑制指纹（§15）──────────────▶ suppress          └─ payloads/<hash>.bin(+thumb)
```

ClipboardEntry：id=(kind:hash)、created/updated、preview/search_text（§81 三
分离，search_text 有界 4096）、payload_file/thumbnail_file、size、source
（Unknown——不伪造来源 §16）、file_paths、is_favorite、sensitivity、status
（§50 七态）。RetentionPolicy：500 条/30 天/200MB 默认。

## 5. Windows Clipboard 实现

workspace `unsafe_code = forbid` ⇒ 不手写 Win32 listener/AddClipboardFormat
Listener；按 §14 采用 **fallback 轮询**（1s 文本 + 5s 节流图像探测 + 文件列表
每 tick），全程可暂停/随进程退出。依赖：arboard（text/image 安全读写）+
clipboard-win（CF_HDROP 文件列表读写）。读取失败 ⇒ 跳过本 tick 继续监听（§48）。

## 6-7. Storage / Deduplication

索引损坏 ⇒ 隔离 `.corrupt-<ts>` + 空降级（应用存活 §30）；payload 缺失 ⇒
条目 Missing 事实（不丢弃）；孤儿/崩溃 temp ⇒ GC 清扫。去重（§13/§74/§75）：
(kind, sha256) 键、同内容更新时间+移顶+保留收藏、不同 kind 不互去重——
A,A×19,B,A ⇒ 2 条（测试固化）。

## 8. Search

大小写不敏感多词（preview 命中加权排序）；只搜元数据/preview/search_text，
绝不加载 payload（§44）。50k 条搜索 28ms（实测）。

## 9. Restore

按 kind 写回（text/html→arboard set_text；image→PNG 解码→RGBA→set_image；
file→CF_HDROP 写回）；payload 缺失 ⇒ `clipboard.cannotRestore` 结构化错误
（§26 不静默）；写回前登记抑制指纹（§15，与去重双保险——Demo 6）；重复制
提升条目+更新时间。

## 10. Privacy

- 日志/审计/报告只进 id/kind/size（§52/§53；代码注释与错误构造处均有守卫）。
- 暂停监听（●/○ 状态用户可见 §17）；Max Entries/Retention/Storage 三重上限。
- 敏感启发（JWT/password=/Bearer/40+hex）只标 **PotentiallySensitive**，
  UI 弱化徽章，绝不说 Safe（§18/§19）；误报边界有测试。
- 文件历史删除 ≠ 文件删除（§65 有测试：payload 删、原文件完好）。

## 11. Agent Integration

监听线程在 app setup 启动（设置 enabled 驱动）；pause/resume/设置变更经
`apply_config` 立即生效（§42 无需重启）；进程退出线程即终止（§76 不装
Service）。全局热键留给 M2.6（§40 只做内部命令）。

## 12. UI

ClipboardPanel（§70 验收全项）：●/○ 状态、搜索、6 筛选、分页列表（load
more）、行内重复制/收藏/删除、详情（hash/文件状态 available-missing-
inaccessible/缩略图）、cannotRestore 文案、清空历史；导航 + palette 入口；
i18n zh/en 同笔。

## 13. Tests

weave-clipboard 23 单元（去重序列/收藏保留/保留三级/敏感边界/preview 规范
化/多词搜索/store 往返/损坏隔离/缺 payload/删除≠原文件/GC 孤儿/存储用量/
file 条目）+ 1 ignored perf；全工作区 40 套件 0 failed；tsc/eslint/vitest 38 绿。

## 14. Performance（release 实测）

1k：save 7ms / load 32ms / search 0.6ms；10k：71ms / 285ms / 5.7ms；
**50k：365ms / 1.43s / 28ms**——线性，无 O(n²)、无全量 payload 加载。

## 15-17. Known Limitations / P0-P2 / 未完成项

- P0：§91 全项有证据（P0-1…P0-13；无回归=门禁全绿）。
- P1（§92，按证据实现与否）：HTML 捕获未做（模型支持 kind，平台层不产出
  ——arboard 无 HTML 读取，诚实降级）；来源应用未做（§16 不伪造）；排除
  应用未做（§43 接口已留：sensitivity/source 字段在模型中）。
- P2：轮询方案 CPU（1s 文本读取 + 5s 图像节流——事件驱动需 unsafe，待
  M2.6 评估安全监听通道）；增量搜索索引；缩略图 LRU 缓存。

## 18. 与 M2.6 / M2.9 的边界

M2.3 只提供 Clipboard Capability（Capture/Store/Search/Restore/Retention/
Privacy 字段）——未做 Watcher framework/Hotkey/Notification（M2.6）、未做
任何 paste 拦截/遮蔽/风险评分（M2.9 Wisp），sensitivity 字段是其接入点。
