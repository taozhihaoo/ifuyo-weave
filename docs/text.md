# Text Tools（M4）

四个文本工具共用一套基座（TextDocument / 编码解码 / Offset / Diagnostics /
TextLimits），全部本地执行、预览无副作用、写回走事务。

## 共同行为

- **输入来源**：粘贴/输入、按路径加载、文本页拖入文件（§102）。
- **编码**：BOM 严格 → UTF-8 严格 → GB18030 严格 → Latin-1 无损兜底
  （零 U+FFFD；Latin-1 兜底在 UI 如实标注）。支持 GBK/UTF-16 手动覆盖。
  二进制守卫：NUL 命中或控制字符密集 ⇒ `text.binaryDetected` 拒绝。
- **换行**：Preserve 默认；行级操作按行携带各自结尾；CRLF/LF 差异不参与
  Compare。
- **Offset 契约**：域内 byte；UI/IPC 额外携带 line + UTF-16 列。
- **限额（TextLimits，§104/§166）**：document 2 MiB / format 1 MiB /
  compare 2 MiB / extract 2 MiB / transform 2 MiB / 提取结果 10,000 条
  （超出截断并如实标注）。超出 ⇒ `text.tooLarge` 结构化拒绝（不冻结）。
- **写回**：Plan（TextTransform）→ 快照 TOCTOU 校验（§90）→ 备份 →
  原子替换 → 事务/历史 → 撤销（用户后续编辑 ⇒ 拒绝覆盖，§94）。
  Save As：目标已存在 ⇒ `text.destinationExists`。

## Text Formatter

- **Input**：文本 + 格式（Auto 由服务端保守检测；Unknown ⇒ 要求手选）。
- **能力矩阵**：

| 格式 | Validate | Format | Minify | Sort | Normalize |
| --- | --- | --- | --- | --- | --- |
| JSON | ✅ 真解析（行/列） | ✅ 保键序 | ✅ | ✅ 递归键（数组保序） | ✅ ≡ Sort+Format |
| XML | ✅ well-formed + 根元素 | ✅ 事件流重排 | ✅ 去纯空白文本 | ❌ 子元素顺序语义 | ✅ ≡ Format |
| YAML | ✅ 真解析（marker） | ❌ 重序列化丢注释 | ❌ | ✅ 递归键 + ⚠️ 丢注释/锚点展开 | ❌ |
| SQL | ✅ 词法级 + 括号平衡（非完整语法） | ✅ 主句布局 | ✅ 去注释/空白 | ❌ | ❌ |
| JavaScript | ❌ 需真 parser | ❌ | ✅ 安全子集 | ❌ | ❌ |
| CSS | ❌ | ✅ 规则块布局 | ✅ | ❌ 层叠语义 | ❌ |
| Markdown | ❌ 无严格语法 | ❌ 用 Normalize | ❌ | — | ✅ 标题空行/空行折叠/围栏不可侵犯 |

- **JSON 语义**：Format 不改键序/任何值；Sort 仅对象键递归，数组绝不排序；
  Minify 不改语义；重复键保留全部出现（parser 真实语义，最后出现生效于
  读取方——如实保留事实）。
- **YAML Sort 警告**：输出为重序列化结果，注释丢失、锚点/别名展开。
- **JS Minify 限制**：regex 字面量含 `//` 可能被误判为行注释。
- **SQL 方言**：保守 ANSI 通用子集（词法级，非完整语法校验）。
- **确定性**：同输入同选项必同输出；幂等性有性质测试（§127/§128）。

## Text Compare

- **Input**：文本 A / B（可分别加载文件）；解码后按文本比较（§196）。
- **选项**：空白三档（None/Trailing/All）+ 忽略大小写（Unicode 折叠）。
- **模型**：Equal/Added/Removed/Changed/Moved（Moved = 引擎同键配对事实，
  非语义断言）；Side-by-side 统计 + Unified 输出。
- **确定性** + **限额**：4 MiB 单侧 / LCS 窗口 2000²——超出降级为单块替换
  并标注 degraded；只读，绝无 auto-merge（§161）。

## Text Extractor

- **Input**：文本 + 类型 + （regex 时）模式；唯一值开关。
- **类型**：URL（尾标点裁剪/括号平衡）、Email（practical）、FilePath
  （盘符/UNC/unix 绝对/显式相对；裸 word/word 不算）、Number（含科学计数/
  百分比；不含 hex/currency 符号本身）、IPv4（0-255 校验）、IPv6（std 解析
  器）、JSON（平衡扫描 + 语法终验，最外层）、Markdown 链接（跳过代码区）、
  Regex（Rust regex 线性引擎；零长匹配跳过；`$1` 替换语法的同源契约）。
- **输出**：value/raw/byte range/line/UTF-16 列；出现次数与唯一值双视图；
  10k 条上限 + 截断标注。

## Text Transformer

- **操作**：TrimLines/TrimDocument、Deduplicate（First/Last × 空行参与/
  保留）、SortLines（升降/大小写/空行首尾保位）、AddPrefix/AddSuffix、
  Case（Upper/Lower/Title/Sentence——简单模式语义见 D40）、NumberLines
  （start/step/separator/补零）、Find/Replace（literal/regex、首个/全部、
  匹配计数展示）。
- **全部预览优先**；写回与 Formatter 同一套安全管线。

## Known Limitations（汇总）

- YAML 重序列化丢注释/锚点展开；JS Format/Validate Unsupported（无 parser）；
  JS Minify 的 regex `//` 误判可能；SQL 校验为词法级；超窗口 Compare 为
  降级结果；Open 对话框 D27 挂起（路径输入/拖入可用）。
