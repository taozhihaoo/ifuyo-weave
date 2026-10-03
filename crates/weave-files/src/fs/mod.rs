//! 文件系统抽象（M1 §7）。
//!
//! 目标不是包装 `std::fs` 的全部 API，而是抽象业务真正需要的最小边界：
//! exists / stat / read_dir / open_read（未来写操作按需扩展，M1 不预设计）。
//!
//! 设计约束：
//! - `stat` 返回自有的 [`FileStat`]（而非 `std::fs::Metadata`）——std 的
//!   Metadata 无法手工构造，会使测试替身不可能；自有结构使内存 fake 可行。
//! - 语义为 **lstat**（不跟随符号链接），配合 weave-files 的 symlink 策略。
//! - `read_dir` 按目录批量返回名称（目录内条目数有限，内存可控），
//!   调用方负责排序与递归（流式遍历语义由 scanner 保证）。

pub mod fault;
pub mod std_fs;

pub use fault::{FaultFilesystem, FsFault};
pub use std_fs::StdFilesystem;

use std::io::{self, Read, Seek};
use std::path::Path;

use crate::metadata::FileStat;

/// 可 Seek 的只读流：partial hash 的首/尾定位读需要（M3 §4）。
/// std::fs::File / io::Cursor 天然满足；测试替身用 Cursor 或自定义。
pub trait ReadSeek: Read + Seek + Send {}
impl<T: Read + Seek + Send> ReadSeek for T {}

/// 业务所需的文件系统边界。实现必须是线程安全且无内部可变状态
/// （故障注入等状态由包装器持有）。
pub trait Filesystem: Send + Sync {
    fn exists(&self, path: &Path) -> bool;

    /// lstat 语义：不跟随符号链接。
    fn stat(&self, path: &Path) -> io::Result<FileStat>;

    /// 列出目录的直接条目名（不含 `.` / `..`）。顺序不保证——调用方排序。
    fn read_dir(&self, path: &Path) -> io::Result<Vec<String>>;

    /// 打开只读流。Hash 等流式消费方按 chunk 读取，禁止整读入内存（M1 §7.1）。
    /// M2 扩展：返回 ReadSeek（partial hash 的首/尾定位读需要 Seek，M3 §4）。
    fn open_read(&self, path: &Path) -> io::Result<Box<dyn ReadSeek>>;

    /// 原生重命名/同卷移动（M2 §70：避免 copy+delete 模拟）。
    /// 目标已存在时的行为由平台决定——调用方必须先做碰撞校验（M2 §21）。
    fn rename(&self, from: &Path, to: &Path) -> io::Result<()>;

    /// 创建目录树（Organizer 目标目录；执行阶段按已确认 Plan 调用）。
    fn create_dir_all(&self, path: &Path) -> io::Result<()>;
}
