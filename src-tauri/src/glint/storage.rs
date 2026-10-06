//! Glint 存储与最近捕获（M2.5 §26：有界、不覆盖、保留策略）。
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("glint::storage")
}

pub const DEFAULT_MAX_RECENT: usize = 100;

/// 输出目录（history_dir/glint-captures）。
pub fn captures_dir(app: &tauri::AppHandle) -> Result<PathBuf, WeaveError> {
    let dir = crate::rename_service::resolve_history_dir(app)?;
    let dir = dir.join("glint-captures");
    std::fs::create_dir_all(&dir).map_err(|e| err("glint.dirFailed", format!("{e}")))?;
    Ok(dir)
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct CaptureRecord {
    pub file_name: String,
    pub path: String,
    pub size: u64,
    pub modified_ms: Option<f64>,
    /// 物理像素（PNG 头解析；失败 ⇒ 0,0）
    pub width: u32,
    pub height: u32,
}

/// 列出最近捕获（mtime 降序；超限旧文件删除——§26 有界）。
/// 只管理 Glint 目录内 `Glint_*` 前缀文件——绝不动用户其他文件。
pub fn list_recent(dir: &Path, max_recent: usize) -> Vec<CaptureRecord> {
    let mut records: Vec<CaptureRecord> = Vec::new();
    let Ok(read) = std::fs::read_dir(dir) else {
        return records;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().map(|n| n.to_string_lossy().into_owned()) else {
            continue;
        };
        if !name.starts_with("Glint_") {
            continue;
        }
        let Ok(meta) = entry.metadata() else { continue };
        if !meta.is_file() {
            continue;
        }
        let modified = meta.modified().ok();
        let (width, height) = png_dimensions(&path).unwrap_or((0, 0));
        records.push(CaptureRecord {
            file_name: name,
            path: path.to_string_lossy().into_owned(),
            size: meta.len(),
            modified_ms: modified
                .and_then(|t| t.duration_since(SystemTime::UNIX_EPOCH).ok())
                .map(|d| d.as_millis() as f64),
            width,
            height,
        });
    }
    records.sort_by(|a, b| b.modified_ms.partial_cmp(&a.modified_ms).unwrap_or(std::cmp::Ordering::Equal));
    // §26 有界：超出 max_recent 的最旧文件删除（均为 Glint 自产输出）
    if records.len() > max_recent {
        for old in records.iter().skip(max_recent) {
            let _ = std::fs::remove_file(&old.path);
        }
        records.truncate(max_recent);
    }
    records
}

/// 删除捕获（§24：仅限 Glint 目录内文件——路径穿越防护复用统一检查 §37）。
pub fn delete_capture(dir: &Path, file_name: &str) -> Result<bool, WeaveError> {
    if file_name.contains(['/', '\\']) || file_name.contains("..") {
        return Err(err("glint.pathTraversal", "illegal file name"));
    }
    let path = dir.join(file_name);
    if !path.starts_with(dir) || !path.is_file() {
        return Ok(false);
    }
    std::fs::remove_file(path)
        .map(|()| true)
        .map_err(|e| err("glint.deleteFailed", format!("{e}")))
}

fn png_dimensions(path: &Path) -> Option<(u32, u32)> {
    let mut f = std::fs::File::open(path).ok()?;
    let mut header = [0u8; 24];
    f.read_exact(&mut header).ok()?;
    // PNG: 8 sig + 4 len + "IHDR" + 4w + 4h
    if &header[12..16] != b"IHDR" {
        return None;
    }
    let w = u32::from_be_bytes([header[16], header[17], header[18], header[19]]);
    let h = u32::from_be_bytes([header[20], header[21], header[22], header[23]]);
    Some((w, h))
}
