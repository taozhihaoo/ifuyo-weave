# DATA_FORMATS

M5 Data 支持的数据格式、解析语义与已知边界（下 §192 Tool-Level Documentation）。

## CSV / TSV

| 项 | 语义 |
| --- | --- |
| 分隔符 | CSV `,`；TSV `\t`；打开时可自动检测（首行候选计数，Heuristic，可手选覆盖） |
| 引号 | 默认 `"`（parser 级转义 `""`）；custom quote 可配置 |
| Header | 显式 yes/no 或启发式自动检测（confidence + reason 如实入诊断） |
| 重复表头 | 显示名 `name_2`/`name_3`；内部身份恒为稳定 `col_N` |
| 空 header | 显示名 fallback `Column N` |
| Ragged 行 | 短行补空 + 诊断；多余单元格保留（导出原样带回），绝不截断/移位 |
| 编码 | M4 解码级联（BOM/UTF-8/GB18030/Latin-1 兜底）；NUL ⇒ binary 拒绝 |
| 单元格 | 恒为原始文本；类型只是 schema/profile 层的 potential 观测 |

限制：输入 ≤ 32 MiB（DataLimits）；单元格 ≤ 8 MiB；列数 ≤ 4096；行数
≤ 500k（会话驻留）。超出 ⇒ 结构化错误（`text.tooLarge` / `data.tooManyRows`
/ `data.cellTooLarge` / `data.tooManyColumns`）。

已知边界：Latin-1 兜底可能让无 NUL 的二进制"成功解码"——控制字符密度
守卫兜底第二道；`flexible(true)` 下 ragged 归 M5 语义，出现未知错位即 Stop。

## JSON

- **Root 语义**：记录集操作（profiling/转换）要求 root = 对象数组；
  单对象/原始值 ⇒ `data.jsonRootObject` / `data.jsonRootNotRecords`
  结构化拒绝（不静默包装）。
- **嵌套**：对象按点路径展平（`$.profile.age`）；数组整体为 JSON cell
  （不展开 `[*]`）。
- **null**：JSON→CSV 时 null → 空串（可配置导出，v1 默认）。
- **数值精度**：Preserve Strings 模式零转换；Typed 模式前导零绝不推断、
  超 i64/u64 整数保留文本（§7/§57）。
- 递归限深 128（serde_json 内建）；深嵌套 ⇒ 结构化错误非栈溢出。

## JSONL

- 一行一 JSON 值；BufRead 逐行流式（非 read-all）。
- 空行跳过并计数（`skipped_empty`）。
- 畸形行：FailFast（Stop）/ CollectErrors（继续 + valid/invalid 计数 +
  行号诊断）双模式（§62）。

## 导出（Export）

- **范围**：`all` = 底表全行；`view` = 当前过滤+排序视图（§176/§179——
  UI 必须展示 N of M）。
- **写入**：目标已存在 ⇒ `data.destinationExists` 拒绝；走
  `weave-files::atomic_write`（temp+flush+rename）+ 事务/历史/Undo。
- **行尾**：LF（默认）/ CRLF / CR 显式选择。
