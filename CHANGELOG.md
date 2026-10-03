# Changelog

所有显著变更记录于此。格式遵循 Keep a Changelog；语义化版本。

## [Unreleased]

### Added
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
