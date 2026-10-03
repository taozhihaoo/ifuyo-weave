//! 原子文件写入（M4 §91/§92）：temp → flush → replace。
//!
//! 共享写基础设施（§10：文本工具不得自实现另一套 write）：任何上层
//! （src-tauri 服务）写文件都走这里；crash/磁盘满/权限失败不损坏原文件。

use std::io::Write;

use crate::fs::Filesystem;
use weave_core::prelude::WeaveError;

/// 原子写： sibling temp 文件 + fsync + rename 替换。
/// `make_temp_path` 由调用方给（约定 `.{filename}.weave-tmp-{suffix}`），
/// 失败时 temp 尽力清理。
pub fn atomic_write(
    fs: &dyn Filesystem,
    path: &std::path::Path,
    bytes: &[u8],
) -> Result<(), WeaveError> {
    let parent = path.parent().ok_or_else(|| {
        WeaveError::validation("atomic.noParent", "path has no parent directory")
    })?;
    let file_name = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "file".to_string());
    let temp = parent.join(format!(
        ".{file_name}.weave-tmp-{}",
        std::process::id()
    ));
    // 清理上次残留的同名 temp
    let _ = std::fs::remove_file(&temp);
    {
        let mut file = std::fs::File::create(&temp).map_err(|e| {
            WeaveError::io("atomic.createTemp", format!("cannot create temp file: {e}"))
        })?;
        file.write_all(bytes).map_err(|e| {
            let _ = std::fs::remove_file(&temp);
            WeaveError::io("atomic.write", format!("temp write failed: {e}"))
        })?;
        file.sync_all().map_err(|e| {
            let _ = std::fs::remove_file(&temp);
            WeaveError::io("atomic.flush", format!("temp flush failed: {e}"))
        })?;
    }
    fs.rename(&temp, path).map_err(|e| {
        let _ = std::fs::remove_file(&temp);
        WeaveError::io("atomic.replace", format!("atomic replace failed: {e}"))
    })?;
    Ok(())
}

#[cfg(test)]
mod atomic_tests;
