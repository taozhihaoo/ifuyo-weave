# ifuyo Weave 2.0 M2.10 Web 内容处理中心 — 实施总结

日期：2026-10-07
状态：**P0 PASS；P1 PARTIAL（QR/Barcode 未实现，如实）；Security PASS；Performance PARTIAL（有界性成立，基准表未做）；Regression PASS**。门禁全绿（42 套件 0 failed / fmt、clippy 0 / tsc、eslint 0 / vitest 38 / tauri build）。

---

## 1. 本阶段实现内容（P0 §102 逐项）

### URL
- **校验**（§7）：scheme 白名单 http/https；javascript:/file:/data:/blob:/ftp:/未知 scheme ⇒ `web.unsupportedScheme` 结构化拒绝，不静默转换；空/畸形 ⇒ `web.invalidUrl`。
- **规范化**（§6）：url crate（WhatWG 语义）——host 小写、空 path ⇒ "/"；Original/Normalized/Cleaned 三形态区分（Display 不用于请求）。
- **提取**（§10）：自由文本 URL 提取（保留原文位置；尾随标点剥离；不可解析跳过）。
- **Query Cleaning**（§8/§9）：`UrlCleanResult { removed/preserved/unknown/warnings }` 全可解释。
- **Functional/Tracking 区分**：高置信追踪清单（utm_*/fbclid/gclid/msclkid 等 24 项）⇒ Tracking 可删；**Functional 类别保留给未来用户显式白名单，本版不猜**（诚实声明）；token/signature/session 等 21 项 ⇒ AuthRelated **默认保留** + 警告。
- **Sensitive Parameter Masking**（§54）：`mask_sensitive`/`web_display_url`——AuthRelated 参数值 ⇒ `***`；日志/批量条目/Audit 用掩码形态。

### Network（§13-§21/§44/§45/§55/§73-§75）
- HTTP/HTTPS GET（ureq 2.12 + rustls + gzip；**唯一网络出口**，其它模块零联网 §17）。
- Timeout（默认 30s）+ connect 10s；**Redirect 上限 10**（ TooManyRedirects ⇒ `web.redirectError`）；**Size 上限 5MB**（解压后计量——压缩炸弹由读端上限拦截 §75）；并发上限（批量层，默认 4，≤8 §45）。
- **Cancellation**（§44）：Job CancellationToken 分块读取间检查——**drop reader 即断开连接**（真停，非 UI 假停）。
- **Status Handling**（§14）：非 2xx ⇒ `web.httpStatus` + 类别化 message（403="server refused access"，非"网页损坏"）。
- 错误分类（§58）：dnsFailure/tlsError(并入 networkError——ureq 2.12 无独立 TLS kind，如实)/redirectError/invalidUrl/networkError/cancelled/contentTooLarge/unsupportedContent/encodingError（best-effort 标注）。
- **不保存 Cookie/Authorization/登录态**（§73/§74）：每次请求全新 Agent；无凭据存储。
- Open Redirect（§55）：原始链接与解析结果分离保留（WebLink.raw_href + absolute_url），无自动深度跟踪。

### HTML
- **Parser**（§23/§78/§79）：html5ever（scraper crate）容错解析——非正则；ego-tree 迭代遍历无递归；极端 DOM（3000 层嵌套/10 万链接）测试不崩溃（链接截断 5000）。
- **Encoding**（§21）：HTTP header charset → BOM → meta charset（含引号容错、归一 encoding_rs 规范名）→ utf-8 默认（lossy 如实标注）；GB18030/GBK/UTF-16 验证；来源记录 HttpHeader/Bom/MetaTag/Utf8Default。
- **Plain Text**（§22/§76/§77）：文本节点按祖先链过滤 script/style/noscript/template；块级断行；空白归一；实体解码（html5ever）；无重复展开。
- **Link Extraction**（§11/§12）：a[href] 经 base `join` 解析；LinkType Internal/External/Relative/Mail/Tel/Anchor/Unknown（不可靠 ⇒ Unknown）；raw_href 保留原文（§55）。
- **Title/Metadata**（§28/§67）：title/description/author/published/modified/language/canonical/og:*——**缺失即缺席**，每字段带证据来源；canonical 仅记录不替换（§29）。
- **基础正文提取**（§26/§27）：语义容器（article/main/[role=main]）⇒ High；文本密度启发式（link-density 惩罚，导航/目录剔除）⇒ Medium；body 兜底 ⇒ Low；**UI 明示"启发式推断，非页面官方结构保证"**。

### Workflow（§68/§69）
Analyze（URL 清理/抓取/解析）→ Preview（文本/Markdown/链接/元数据 Tab + 摘要）→ 用户确认 → Export（txt/md/html-Original）→ creation-undo 事务可撤销。**Fetch 不伪装成 Apply（§3）**；导出 output-first（目标存在 ⇒ `document.destinationExists` 拒绝）+ atomic_write + History 可撤销。

### Integration
- **M2.3 Clipboard**（§36/§51）：webCopyText 走 arboard + **suppress 指纹写前登记**（消自环+消 1s 竞态）；无第二剪贴板体系。
- **M2.0 Job**：fetch 与批量均经既有 JobTracker（register/poll/cancel）。
- **M2.0 Export/Apply**：atomic_write + record_creation_transaction（复用，未重建 §99）。
- **Audit**（§71）：tracing 元数据（输出名/字节数/format）；批量产物经 creation-undo 事务（History 可见可撤销）。
- **Report**（§72）：批量汇总（Total/Exported/Failed/Cancelled/Skipped）+ 逐条（掩码 URL/状态/文件名/错误码）。

### Reliability（§41-§46）
单项失败隔离（批量逐条 try，单 URL 崩溃不影响整体）；可取消（任务级+读取级）；应用关闭安全（无半成品——atomic_write）；内存有界（响应 5MB 截断、单文档缓存一槽、批量报告 5 槽、批量不缓存原文边取边写）。

## 2. 实际架构

```
src-tauri/src/web/          # 领域（纯函数 + ureq 边界）
  model.rs    # DTO（specta 安全：f64/u32/String）
  url.rs      # normalize/classify/clean/mask/extract/filename
  target.rs   # SSRF 分类 + GuardedResolver
  fetch.rs    # ureq 抓取（限制/取消/状态/编码）
  html.rs     # html5ever 解析/text/links/metadata/main-content
  markdown.rs # DOM→Markdown（Enter/Exit 帧栈）
src-tauri/src/web_service.rs # IPC 12 命令 + WebRuntime（配置/单文档槽/批量报告）
ui/src/features/web/WebPanel.tsx # 统一 Workspace（§59/§60）
```
远程 URL 与本地 HTML（web_analyze_html）走**同一 Parse/Extract/Transform 管线**（§63/§65）；Fetch 与 Parse 解耦（§65）。

## 3-6. URL/Fetch/HTML/Transform 模型

见 §1 逐项。亮点：
- **SSRF（§16 P0）**：SSRF 分类挂在自定义 `ureq::Resolver`——**解析结果逐地址分类后才交给连接层**，DNS rebinding TOCTOU 闭合（分类作用于真实连接目标）；请求前主机名预检（字面 IP/localhost 立即拦）；Localhost/Private(10/8、172.16/12、192.168/16、100.64/10 CGNAT)/LinkLocal(169.254/16、fe80::/10)/ULA(fc00::/7)/v4-mapped 全分类；`allow_private_targets` 默认 **false**，请求级 override 需 UI 显式勾选。
- **Markdown**（§24/§25）：H1-H6/P/Strong/Em/Del/Link/Image/OL/UL/Blockquote/Code/CodeBlock/HR/Table(GFM 表头分隔行)；`javascript:/data:` href 丢弃但文本保留；pending-space 状态机保证 `**bold**` 等行内标记闭合不吃空格。

## 7. UI

统一 Workspace：输入+Clean/Fetch+SSRF 放行勾选 → 清理卡（原始/清理后/已删/未知保留/警告）→ 摘要卡（状态/类型/编码/大小/时长/重定向/置信度+推断声明）→ 四 Tab（文本/Markdown/链接/元数据带证据）→ 复制三连+导出三格式 → 批量（文本粘贴自动提取→格式选择→汇总+逐条掩码表）→ 设置（§84 八项）。palette：Open Web / Fetch URL / Clean URL（§61 导航级）；i18n zh/en 同笔 key 对等。

## 8-9. Clipboard / Export Integration

见 §1 Integration。

## 10-11. Security Boundary / SSRF

见 §3-6 亮点。**已知残留（如实）**：分类基于 IP 拓扑——"公网 IP 但内网映射"的网络环境无法识别；DNS 层拦截依赖 GuardedResolver（ureq 用其返回地址建连），非浏览器级安全边界。

## 12. Resource Limits

timeout 30s / redirects 10 / response 5MB（解压后）/ batch ≤500 URLs / 并发 ≤8 / 链接 5000 / 本地分析 ≤20MB / 批量报告缓存 5 槽。全部设置驱动（§84）。

## 13. Cancellation

Job 级（批量项间）+ 读取级（分块间 drop reader 断连）。**验证**：fetch_cancelled 错误码 + 批量 cancelled 状态汇总。

## 14. Batch Processing

提取（自由文本）→ 校验（≤500）→ 有界并发 workers（config.max_concurrent）→ 逐条 fetch→convert→write（冲突 `-N`、dest-exists ⇒ Skipped、单条失败隔离）→ 进度（done/total）→ 汇总报告 + creation-undo 一次登记全部产物。

## 15. QR / Barcode

**未实现**（§103 P1 建议项；spec §32-§35 允许后置）。接口预留：web_clean_url → (QR 生成位) → 复制/导出均可独立执行。

## 16. Tests

- **web 域 33 测试**：URL（规范化/scheme 拒绝/参数分类三态/掩码/文本提取/文件名）+ SSRF（字面 IP 全分类/localhost/.local/Resolver 拒绝/precheck）+ fetch（Content-Type 分类/状态 message/编码优先级 BOM·GBK·meta/本地目标拒绝无需真实网络）+ HTML（script 剔除/实体/链接 base join + 七类/元数据证据+缺席/正文三档置信/敌意 DOM 3000 层+10 万链接）+ Markdown（14 元素/javascript 丢弃/表格 GFM/敌意嵌套）。
- 门禁：**42 套件 0 failed**（--no-fail-fast 验证）；fmt/clippy **0**；tsc/eslint **0**；**vitest 38 绿**；tauri build 通过。
- **附带修复**：weave-compare `nested_directories` 预存量 flake（两侧相同内容 + mtime 同毫秒 ⇒ Metadata 档偶发 Same）——测试内容改相异，意图不变。

## 17. Performance Results

结构性有界（§12 全清单）+ 敌意输入测试（3000 层 DOM、10 万链接、5MB 截断）不崩溃不无限增长。**未做**：1KB-100MB HTML 耗时基准表、10/100/500/1000 URL 批量基准（§80/§81 P2——如实）。

## 18. Known Limitations（如实）

- **未实现**：QR 生成/识别、Barcode 识别、ETag/Last-Modified/304（接口已留——info 无此字段，M2.11 前补）、资源统计（§30 P1）、HTML 清理规则（Sanitized HTML 导出——html 导出为 **Original 语义**并在 UI 声明）、Recipe 自动化节点（§50 只暴露能力）、Recent URLs 历史（§83 可选）、URL 转二维码链路。
- **LIMITATION**：批量 UI 无虚拟化（数据有界 500；渲染前 50 条+汇总）；正文提取为启发式（UI 明示）；preview 为纯文本/Markdown 文本域——**不渲染不可信 HTML、不执行页面脚本**（§40，WebView 从未加载抓取内容）。
- **NOT SUPPORTED**：浏览器/爬虫/网站镜像/登录自动化/CAPTCHA/付费墙绕过/JS 执行（§104 全清单未越界）；Cookie/Authorization 持久化。

## 19. P0/P1/P2

- **P0 全部 PASS**（§102 逐项见 §1）。
- P1 PARTIAL：Markdown ✓、可复用 Fetch API ✓（web::fetch pub + build_document pub）；QR/Barcode/资源统计/ETag 未实现（如实）。
- P2：性能基准表、批量 UI 虚拟化、Recent URLs。

## 20. Out of Scope（未越界确认）

§104 清单逐项未实现：无浏览器/Tab/书签/History/Cookie Manager/登录/OAuth/CAPTCHA/爬虫/镜像/全站下载/SEO/搜索/代理池/Cloud Fetch/Headless/JS 平台/Page Monitor/AI 功能。**WebView 未加载任何抓取内容**（M2.10 未引入 Chromium 行为）。

## 21. 对 M2.11 的接口准备

`web::fetch::fetch()`（限制可配）+ `WebFetchInfo { final_url, status, size, duration }` + 内容管线（WebDocument）+ `decode_body`——M2.11 Website Monitor 只需在其上叠加调度（M2.6 Schedule）与变化检测（需先补 ETag/Last-Modified/ContentHash 接口——已在 §18 列为 M2.11 前置）。

## 最终结论（§109）

```
M2.10 Web
Status:
- P0: PASS
- P1: PARTIAL（QR/Barcode/资源统计/ETag 未实现，如实）
- Security: PASS（SSRF Resolver 级拦截+预检；scheme 白名单；掩码；无凭据存储；不执行抓取内容）
- Performance: PARTIAL（结构性有界+敌意输入验证；基准表未做）
- Regression: PASS（42 套件 0 failed；附带修复 1 个预存量 compare 测试 flake）
```

- **真正完成**：URL 核心/抓取管线/HTML 解析提取/Markdown/批量/导出/Clipboard 集成/UI。
- **只是基础设施**：ETag 接口位（未实现 304 协商）。
- **已知限制**：见 §18。
- **留给 M2.11**：ETag/Last-Modified/ContentHash、调度监控、变化检测历史。
