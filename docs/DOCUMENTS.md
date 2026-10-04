# DOCUMENTS — 文档工具（M8）

> 权威来源：`Spec/ifuyo Weave — M8 Documents 完整开发提示词（上/下）.md` +
> `DECISIONS.md` D59-D61。漂移以实际代码为准（weave-documents /
> weave-batch / src-tauri/document_service）。

## 能力矩阵（§190 —— 每项由真实测试证明，测试见 `crates/weave-documents/tests/fixture_tests.rs` 与单元测试）

| Format | Inspect | Metadata | Structural Stats | Merge | Split | Reorder | Rotate | Preview |
|---|---|---|---|---|---|---|---|---|
| PDF | ✅ 测试 | ✅ 测试（Info 字典） | ✅ 页数/尺寸/Rotate | ✅ 测试（顺序=内容标记） | ✅ 测试（2-4,8-10） | ✅ 测试（置换非排序） | ✅ 测试（/Rotate 元数据） | 结构化（无渲染 §51） |
| DOCX | ✅ 测试 | ✅ 测试（core props） | ✅ 段落/标题/表格/图片/超链接 | N/A | N/A | N/A | N/A | 结构化（§52） |
| XLSX | ✅ 测试 | ✅ 测试（core props） | ✅ sheets/可见性/dimension/populated/公式 | N/A | N/A | N/A | N/A | 结构化 |
| PPTX | ✅ 测试 | ✅ 测试（core props） | ✅ slides/hidden/文本/形状/图片 | N/A | N/A | N/A | N/A | 结构化 |
| TXT/Markdown | ✅ 基础（§0.1 M4 为主） | ❌ | ❌ | N/A | N/A | N/A | N/A | — |

语言约定（§191）：Supported / Limited / Unsupported——不用 "fully
supported"。

## 架构

```text
UI（DocumentsPanel——只做事实呈现/命令调用 §3/§200）
 ↓ IPC
document_service（校验/编排/History 记录 §180）
 ↓
weave-documents（detect/facts/page_range/pdf/office —— 域与解析适配 §181-§182）
 ↓                     ↘（批量）
lopdf / zip / quick-xml    weave-batch（StageSpec::DocumentInspect/PdfRotate §58）
```

## 安全语义

- 输出 = **output-first**（§66）：`{stem}.merged|rotated|extract.pdf`、
  `{stem}.part-NNN.pdf`；目标存在 ⇒ `document.destinationExists` 拒绝。
- 原子写出（§178-§179）：同目录 temp → 校验 → rename promote；失败清理。
- Merge TOCTOU（§141）：plan 携带输入快照，execute 重校验 ⇒
  `pdf.changedSincePlan`。
- 资源限额（§100）：`DocumentResourceLimits`（file 512MiB / entries
  65,536 / entry 256MiB / total 1GiB / xml 128MiB / pages 10,000 / sheets
  1,024 / slides 2,048）。
- History：产物 = BatchExecute 创建型事务（撤销 = stat 守卫删除，§77）。

## 恢复语义（§127-§130）

- 批量路径：M7 journal（paused/interrupted resume，M7 下机制）。
- 单文档操作：同步原子（temp+promote）——崩溃只会留下 `.weave-tmp-*`
  残片（可识别、可清理、非合法产物名）＝orphan 语义（§130）。

## 已知限制（§192 如实）

- PDF 渲染预览 NOT SUPPORTED（无光栅化器；预览=结构化事实）。
- Office→PDF NOT SUPPORTED（§92：不依赖本机 Office）。
- 加密 PDF/Office：结构化检测 + 拒绝（§84）；密码解锁不在本版。
- PDF forms/annotations/page-labels：不编辑；merge/split 的保留行为未
  逐项验证（§81/§82 已知限制）。
- 单文档操作为同步命令（批量取消经 M7 支持；单文档大文件取消待（下）
  后续）。词数 = whitespace 近似（§34 Estimated）。公式不重算（§42-§43）。

## 性能（实测见 docs/PERF.md M8 节）

批量 inspect：1000 docs ≈ 1.43s（~1.4 ms/doc）；PDF 单操作毫秒级
（synthetic 语料）。
