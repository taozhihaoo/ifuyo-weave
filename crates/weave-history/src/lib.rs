//! weave-history — 操作历史（M2 §46–§58）。
//!
//! 职责：OperationTransaction / HistoryEntry 的模型、版本化本地持久化
//! （原子写 + 损坏隔离 + 安全降级）、查询。不依赖 React / Tauri。
//!
//! 存储 shape（DECISIONS D29）：
//!
//! ```text
//! <dir>/entries.json                 ← HistoryEntry 列表（最新在前，容量有界）
//! <dir>/transactions/<op_id>.json    ← 每个操作一份完整事务（供 Undo）
//! ```
//!
//! History 描述**实际发生的**变更，不是意图（M2 §79 Transaction Accuracy）。
//! 与应用日志严格分离（M2 §58）。

pub mod model;
pub mod store;

pub use model::{
    HISTORY_SCHEMA_VERSION, HistoryEntry, OperationStatus, OperationTransaction, Reversibility,
    TransactionItem, TransactionItemStatus,
};
pub use store::{HistoryStore, LoadOutcome, MAX_ENTRIES};

#[cfg(test)]
mod store_tests;
