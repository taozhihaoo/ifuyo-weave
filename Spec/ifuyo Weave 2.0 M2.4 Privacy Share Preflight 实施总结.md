# ifuyo Weave 2.0 — M2.4 Privacy / Share Preflight 实施总结

日期：2026-10-06。提交链：P1 a36a518 → fixes d0be933/703bce7 → P2 aaf0c3f →
P3 11d1648 (+lint) → P4 5f8f0a7 (+lint) → P5 6c65858 → P6（perf + 本文档）。

## 1. 实施范围

分享前隐私预检：Scan → Findings（Confirmed/Suspected/Unknown 三级证据）→
Review → 选择 → Export Safe Copy（原文件零接触）→ Re-scan 对比 → Audit。
**Analysis-first**；Candidate/Finding ≠ 敏感证明；未发现 ≠ 安全（§0/§77）。

## 2. 架构复用（§1 审计结论 → 落实）

- **M2.0**：WeaveError/verification（stale 重验 §42）/atomic_write（共享写原语
  §16）/HistoryStore（事务+undo，复用创建型语义 = undo 删 Safe Copy）/历史目录。
- **M2.1**：fingerprint-stable 对比思想（§27/§28）。
- **M2.2**：文件遍历纪律 + Job 化缓存领取模式（第四次复用）。
- **M2.3**：敏感模式原语（`looks_like_jwt/secret_assignment/bearer/long_hex`
  从 heuristic 提取为 pub 导出，heuristic 重构为组合调用——行为不变，23 测试
  照绿；M2.4 位置级检测在其上构建，**无双套规则漂移** §63）。
- **M6**：`inspect_bytes`（EXIF/GPS/ICC 事实）+ resize 语义。
- 无第二套 Plan/Apply/Recovery/Audit（P0-16 ✓，§76 Q1/Q2 答 YES 复用/NO）。

## 3. 新增模块

- `weave-privacy` crate：model（FindingFacts/PrivacyFinding/Session/comparison）、
  patterns（位置级检测）、scanners（image/office/pdf/text/zip）、cleanup（字节
  变换 + verify_safe_copy）。
- `src-tauri/privacy_service.rs`：扫描 Job + 结果缓存 + 导出 + 对比 IPC。

## 4. 支持格式与 Finding（§74 如实清单）

| 格式 | 扫描（Confirmed 级） | 清理 |
|---|---|---|
| JPEG/PNG/WebP | GPS(High)/EXIF 设备时间(Medium)/ICC(Info) | 元数据剥离（重编码，方向已应用；JPEG92 有损/ICC 丢失**明示**） |
| DOCX/XLSX/PPTX | core 作者/修改者/标题等 + app 公司 + 隐藏工作表(High)/修订(High)/批注/隐藏幻灯片/备注 | 仅 core/app 字段置空（ZIP 重写 part 不动）；隐藏内容 = scan-only（§9 发现 ≠ 可删） |
| PDF (lopdf) | Info 字典 Author/Creator/Producer/Title/Subject/Keywords | scan-only（§15 不粗暴重建） |
| TXT/MD/LOG/CSV/JSON/XML | JWT/Bearer/secret 赋值（值形状+上下文）/40+hex/邮箱/电话/本地路径/内网 URL（全部 Suspected） | 命中值 → [REDACTED]（高风险，仅显式选择 §24） |
| ZIP | 文件名 + 内部文本有界抽查（深度 1/1000 条/1MB/20 文件 §40） | 无 |
| 其他格式 | **Not fully checked + reason 明示**（§38 不静默） | 无 |

## 5. 安全边界（§14/§47/§50）

Finding 只携带 masked 证据（sk…ef/••••）；fingerprint 用 evidence_identity
hash；日志/审计/历史只有路径+计数+状态（隐私服务 zero 内容字段）。

## 6. 原文件保护（§21/§59 P0-15）

Export = stale 重验（size+mtime vs 扫描快照，服务端 stat fallback 已注释）→
变换 → verify_safe_copy（空/解码/ZIP 检查 §36）→ atomic_write（temp+rename）
→ 输出已存在 ⇒ 拒绝 → 事务落盘（undo = 删除 Safe Copy）。原文件全程只读。

## 7. 测试结果

weave-privacy 23 单元（模型/模式/误报矩阵 §60/五格式扫描/边界上限/单文件失败
隔离 §44/cleanup 各格式/原始 part 不动/结构化错误）+ 1 ignored perf；
weave-clipboard 23（共享原语重构后照绿）；全工作区 42 套件 0 failed；
tsc/eslint/vitest 38 绿；fmt/clippy 0；tauri build 通过。

## 8. 性能（release 实测）

位置级模式检测 10k 行 = **23ms**（线性）。扫描 Job 有取消（→PartiallyCompleted）
与 10k 文件遍历上限。

## 9. Known Limitations / P1 / P2

- P1：Office 隐藏内容/修订/批注的**删除**（当前 scan-only，§9 明确不做自动删）；
  PDF 元数据清理（scan-only）；更多 secret 模式；WebP 编码质量语义。
- P2：导出 stale 校验的快照由 UI 传 size/mtime（当前 UI 未传 ⇒ 服务端现读
  stat 作基线——窗口极小且输出永不覆盖，代码注释已明示）；嵌套 archive 不
  递归（§40 设计选择）；多词搜索/高级筛选。
- Out of Scope 未越界（§71）：杀毒/DLP/密码破解/云端/OCR/AI 全部未实现。

## 10. 为 M2.5 提供的能力

Finding/Evidence 模型、模式原语、Session/对比范式、Job+缓存领取、事务式导出
——Archive/Integrity 中心的哈希清单/完整性报告可直接复用同一批骨架。
