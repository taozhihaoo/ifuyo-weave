//! 批量重命名（M2 §12–§25）。
//!
//! 管线（M2 §19，顺序固定并记录于此）：
//!
//! ```text
//! 输入路径 → Path Safety 校验 → stat 快照 → Base/Ext 拆分
//!   → 依用户列表顺序逐条应用规则（前条输出 = 后条输入）
//!   → 目标全路径组装 → 目标路径安全校验 → 碰撞检测
//!   → Plan（Preview 与 Execute 共用）
//! ```
//!
//! 规则语义（DECISIONS D28）：
//! - Prefix/Suffix：在 base 前后拼接
//! - Replace / RegexReplace：仅替换 base（扩展名不动，防 `.jpg.jpg`，§17）
//! - Case：base 大小写折叠（lower/upper/title；Unicode 感知）
//! - Extension：显式设置扩展名（去点规范化）
//! - Counter：**整体替换 base** 为格式化序号（Start/Step/Width；§14 的 001/002 语义）
//! - Date：**整体替换 base** 为格式化日期（Modified 优先；§15）
//! - Template：最后一步，用 `{name}/{ext}/{counter}/{date}/{original}`
//!   重组最终名；未知占位符 ⇒ Plan Error（§18 不得静默输出原文）
//!
//! 排序（M2 §20）：输入按完整路径 case-insensitive 升序（与 M1 扫描一致），
//! 序号分配基于该顺序，与 OS 枚举顺序无关。

pub mod plan;
pub mod rule;

pub use plan::build_rename_plan;
pub use rule::{CaseForm, DateField, DateFormat, RenameRule, TemplateError};

#[cfg(test)]
mod tests;
