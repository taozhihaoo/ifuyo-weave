//! weave-text — Text formatting, comparison, transformation and extraction (M4).
//!
//! 职责（§11）：TextDocument / Encoding / LineEnding / Format Detection /
//! Formatter / Comparator / Extractor / Transformer / Text Diagnostics。
//! 不负责：Tauri、React、filesystem 实现、路径校验、历史持久化、回收站。
//!
//! Offset 契约（§18，高风险边界）：域内 byte offset；IPC 额外携带 line +
//! UTF-16 column；UI 不得把 byte offset 当 JS string index。

pub mod detection;
pub mod diagnostics;
pub mod encoding;
pub mod extract;
pub mod model;
pub mod offset;
pub mod transform;

pub use detection::{detect_format, detect_format_by_extension};
pub use diagnostics::{Severity, TextDiagnostic};
pub use encoding::{Bom, DecodeError, DecodedText, decode, decode_as, encode};
pub use model::{
    LineEnding, LineEndingCounts, SourceKind, TextDocument, TextFormat, WritePolicy,
    count_line_endings, detect_line_ending,
};
pub use offset::{LineIndex, TextRange};

pub use transform::{
    BlankPolicy, CaseForm, KeepPolicy, PadMode, TransformKind, TransformResult, apply_transform,
};

pub use extract::{ExtractKind, ExtractMatch, ExtractOptions, extract_matches, extract_regex};
