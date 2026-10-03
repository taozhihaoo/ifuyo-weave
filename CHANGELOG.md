# Changelog

所有显著变更记录于此。格式遵循 Keep a Changelog；语义化版本。

## [Unreleased]

### Added
- M5 Data：CSV/TSV/JSON/JSONL 解析（csv crate + 观测 schema profiling +
  诚实采样标注）、CSV Studio（分页表格/过滤/排序）、Data Cleaner（有序
  规则计划：Trim/大小写/填充/去重/拆分/合并/日期与数值归一，Preview-first）、
  四向转换（嵌套展平/JSON cell/typed 可选/重复表头可见化）、Data
  Inspector（潜在类型/空值率/精确唯一值——超限 Unavailable）、导出走
  M4 安全管线（原子写+历史+撤销）。
- M4 Text：TextDocument/编码级联（GB18030/GBK/Latin-1，charter #27 补齐）/
  UTF-16 列 offset 契约；Formatter 七格式能力矩阵（JSON/XML/YAML/SQL/JS/
  CSS/Markdown，Unsupported 如实标注）；确定性 Compare（LCS 有界窗口 +
  诚实降级 + Moved 配对 + Unified）；Extractor 八类 + 用户正则（ReDoS
  安全引擎）；Transformer 九操作；安全写回（TOCTOU 快照 + 备份 + 原子
  替换 + 历史/撤销，§94 用户改动拒覆盖）；Text UI 四工具面板。
- M3 Duplicate Finder：精确重复检测（size → partial hash → 全量 SHA-256
  三级管线）、内容派生 GroupId、重复组选择模型（每组至少保留一份）、
  回收站执行（trash crate；绝不永久删除）、扫描快照 TOCTOU 防护
  （changedDuringScan / changedSinceScan）、事务 + 持久化历史 +
  回收站还原 Undo、Duplicates UI（组视图/选择摘要/回收确认）、
  性能基准（Scenario A–D，docs/PERF.md）。
- M2 Rename / Organizer：批量重命名（Prefix/Suffix/Replace/Regex/Counter/Date/
  Case/Extension/Template）、Preview→Confirm→Execute 闭环、四类碰撞检测、
  两阶段环安全、Organizer 规则整理（first-match-wins + root 边界）、
  事务 + 持久化历史 + LIFO Undo（swap 暂存）、Open File/Folder 入口。

### Added
- M1 File Core：weave-files（Filesystem 抽象 + 故障注入、流式 SHA-256、
  有界类型分类与编码检测、File Inspector、Directory Analyzer 增量统计）、
  IPC 命令（inspect/hash/analyze + 任务进度与协作取消）、Inspector/Analyzer UI、
  大小写不敏感路径冲突契约（M2 碰撞检测地基）、性能基准（docs/PERF.md）。

- M0 Foundation：Cargo workspace（weave-core 领域契约 + weave-testkit）、
  Tauri 2 桌面壳（类型化 IPC、brand 单一事实源、rolling 日志、versioned 配置）、
  React/TS 前端（design tokens、zh-CN/en i18n、命令注册表、Drop 探针冒烟）、
  CI（windows-latest，8 道质量门）、cargo-deny 与前端双 license 审计、文档基线。
