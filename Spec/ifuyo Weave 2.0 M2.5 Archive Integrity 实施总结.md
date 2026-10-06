# ifuyo Weave 2.0 — M2.5 Archive / Integrity 实施总结

日期：2026-10-06。提交链：A1 (weave-files::archive create/inspect/extract +
slip/duplicate/bomb 防护) → A2 (weave-files::integrity manifest/verify) →
A3 (archive_service IPC) → A4 (ArchivePanel) → A5 (perf + 本文档)。

## 1. 实施摘要

- **Done**：ZIP 创建（临时件→结构验证→rename 落位；取消删临时件 §17）、Inspect
  （条目/大小/CRC/重复/unsafe/bomb 检测）、安全解压（Zip Slip 逐分量解析 §9、
  Windows 保留名 §60、重复保守 Skip §61、炸弹阈值 Block §34、覆盖策略集中
  §29）、SHA-256 文件校验（Match/Mismatch/Unreadable/Cancelled）、目录
  Manifest 生成/校验（相对路径 §22、行式格式 §65、Same/Modified/Added/
  Removed/Unreadable 复用 M2.1 语义 §38）。
- **Partial**：Level 2 内容验证（Hash 对比）目前以 zip crate 读取时 CRC 自动
  校验 + hash_file 可组合实现，未做独立"解压流 hash vs 原文件 hash"一键命令。
- **Not Done**：SHA-1/MD5（§4 建议——明确不包装为安全保证）、ZIP 内预览、
  嵌套 ZIP 递归（§40 设计排除）、密码保护 ZIP（§71 Out of Scope）。

## 2. 复用能力

- M2.0：Job/取消/进度（JobTracker 第 5 次复用）、HistoryStore（Audit 闭环）、
  atomic_write 纪律（temp+rename 模式延伸到 zip 临时件）、错误模型。
- M2.1：Difference 语义命名（§38 Manifest diff）。
- 既有 hash 服务（`crate::hash::hash_file` 流式+取消——未重写 HashEngine）。
- zip 8.6 / lopdf / sysinfo 均为已有依赖族；本阶段新增 zip 到 weave-files。

## 3. 安全结果（§80 Security）

- **Zip Slip**：`is_safe_entry_name` 逐分量解析（拒绝 Prefix/RootDir/ParentDir
  + 深度复核），解压 `resolve_entry_target` 二次校验——8 名测试矩阵全 Block
  （`../`、`..\`、绝对、盘符、UNC、混合分隔符）。
- **Windows 保留名**：CON/NUL/COM1…+尾点/尾空格 ⇒ Blocked。
- **重复条目**：zip crate 双端（write finish + read open）拒绝；外部重复包
  Inspect 报 malformed 且错误携带条目名；规范化重复（./ 前缀）检出 + Skip。
- **Zip Bomb**：总量 > 上限 ⇒ bomb_suspect ⇒ 服务端 Block（§34）。
- **取消**：条目间安全点；create 取消 ⇒ 删临时件（测试守护）。

## 4. 完整性结果

- create_zip 内置 Level 0 结构验证（重开+条目数匹配）后 rename。
- Level 1 CRC：zip crate 读取时自动校验（CRC 错 ⇒ Read error ⇒ failed 条目）。
- verify_file_hash + Manifest：Same/Modified/Added/Removed/Unreadable 全覆盖
  （测试：keep/mod/del/add 四态一测）。

## 5. 性能（release 实测）

ZIP Normal deflate：1k 文件 create=97ms / extract=280ms；10k 文件
create=781ms / extract=3.7s——线性，流式无整读。M2.0 验证 perf@10k
SizeMtime 133ms。

## 6. 测试

weave-files 176+2 ignored（archive 9：slip 矩阵/保留名/round-trip/取消/
output-exists/slip-block/重复/bomb/CRC；integrity 4：match-mismatch/manifest
往返/diff 四态/unreadable）；全工作区 42 套件 0 failed；vitest 38；fmt/clippy 0。

## 7. Known Limitations

- Level 2 一键内容验证命令未做（CRC+verify_file_hash 已可组合）。
- SHA-1/MD5 未加（当前产品无该需求；sha2 单算法）。
- ZIP 内文本/图片预览未做（P1 §79）。
- 嵌套 ZIP 递归解压不默认（§40 设计选择）。

## 8. P0 / P1

- P0：§78 十九项逐条有证据（安全/取消/部分状态/内存/临时件/回归全绿）。
- P1：§79 列表未动（P0 稳定优先）。

## 9. 最终判断

**M2.5 COMPLETE**（P0 全项证据齐备；部分能力按 §74 如实标注 Partial/Not Done）。
