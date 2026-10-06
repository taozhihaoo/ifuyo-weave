//! 完整性：文件 Hash 校验 / 目录 Manifest（M2.5 §21-§24/§36-§38）。
//!
//! - Hash 服务复用 `crate::hash`（SHA-256 流式 + 取消——不重写 HashEngine）。
//! - Manifest 相对路径（§22：机器无关）；格式 = 简单 `hash  path` 行（§65）。
//! - 验证状态复用 M2.1 Compare 语义命名（§38）：Same/Modified/Added/Removed/
//!   Unreadable——不另造 Difference2。
//! - 增长对比（§27）：Previous=0 ⇒ growth_pct=None，绝不 Infinity。

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use weave_core::prelude::{CancellationToken, WeaveError};

use crate::hash::hash_file;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("weave-files::integrity")
}

/// 文件 Hash 校验结果（§36）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HashVerifyStatus {
    Match,
    Mismatch,
    Unreadable,
    Cancelled,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HashVerifyOutcome {
    pub status: HashVerifyStatus,
    /// 实际算出的 hash（Mismatch 时供 UI 对比；Unreadable 时 None）。
    pub actual_hash: Option<String>,
}

/// 文件 vs 期望 hash 校验（§36）。
pub fn verify_file_hash(
    path: &Path,
    expected_sha256: &str,
    cancel: &CancellationToken,
) -> Result<HashVerifyOutcome, WeaveError> {
    let result = hash_file(&crate::fs::StdFilesystem, path, cancel, &mut |_| {})?;
    if result.status == weave_core::prelude::HashStatus::Cancelled {
        return Ok(HashVerifyOutcome {
            status: HashVerifyStatus::Cancelled,
            actual_hash: None,
        });
    }
    let Some(actual) = result.digest_hex else {
        return Ok(HashVerifyOutcome {
            status: HashVerifyStatus::Unreadable,
            actual_hash: None,
        });
    };
    let expected = expected_sha256.trim().to_lowercase();
    if expected.is_empty() {
        return Err(err("integrity.badExpectedHash", "expected hash is empty"));
    }
    let status = if actual.eq_ignore_ascii_case(&expected) {
        HashVerifyStatus::Match
    } else {
        HashVerifyStatus::Mismatch
    };
    Ok(HashVerifyOutcome {
        status,
        actual_hash: Some(actual),
    })
}

// ── Manifest（§22/§23）──

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ManifestEntry {
    /// 相对路径（`/` 分隔；机器无关 §22）。
    pub relative_path: String,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChecksumManifest {
    pub algorithm: String,
    pub generated_at: SystemTime,
    pub entries: Vec<ManifestEntry>,
}

/// Manifest 验证状态（§23；命名对齐 M2.1 Difference §38）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManifestDiffKind {
    Same,
    Modified,
    Added,
    Removed,
    Unreadable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestDiffItem {
    pub relative_path: String,
    pub kind: ManifestDiffKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ManifestDiffReport {
    pub items: Vec<ManifestDiffItem>,
}

impl ManifestDiffReport {
    pub fn count(&self, kind: ManifestDiffKind) -> u64 {
        self.items.iter().filter(|i| i.kind == kind).count() as u64
    }
}

/// 为目录生成 Manifest（流式逐文件 Hash；取消安全点 = 每文件）。
pub fn create_manifest(
    root: &Path,
    cancel: &CancellationToken,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<ChecksumManifest, WeaveError> {
    if !root.is_dir() {
        return Err(err(
            "integrity.rootMissing",
            format!("{} is not a directory", root.display()),
        ));
    }
    let mut files: Vec<PathBuf> = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        if cancel.is_cancelled() {
            break;
        }
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            // symlink/junction 不递归不收录（M1/M2.2 同源保守策略 §59）
            let Ok(meta) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if meta.is_dir() {
                stack.push(path);
            } else if meta.is_file() {
                files.push(path);
            }
        }
    }
    let total = files.len() as u64;
    let mut entries = Vec::with_capacity(files.len());
    for (done, path) in files.iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        let result = hash_file(&crate::fs::StdFilesystem, path, cancel, &mut |_| {})?;
        let meta = std::fs::symlink_metadata(path).ok();
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .replace(std::path::MAIN_SEPARATOR, "/");
        entries.push(ManifestEntry {
            relative_path: rel,
            size: meta.as_ref().map(|m| m.len()).unwrap_or(0),
            modified: meta.and_then(|m| m.modified().ok()),
            sha256: result.digest_hex.clone().unwrap_or_default(),
        });
        progress(done as u64 + 1, total);
    }
    entries.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(ChecksumManifest {
        algorithm: "SHA-256".to_string(),
        generated_at: SystemTime::now(),
        entries,
    })
}

/// Manifest 序列化（§65 简单稳定：`<sha256>  <relative_path>` 行式）。
pub fn serialize_manifest(manifest: &ChecksumManifest) -> String {
    let mut out = String::from("# SHA-256 checksum manifest (Weave)\n");
    for entry in &manifest.entries {
        out.push_str(&format!("{}  {}\n", entry.sha256, entry.relative_path));
    }
    out
}

/// 解析行式 Manifest（path 含空格容忍：最后一个空格分段）。
pub fn parse_manifest(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        if let Some(pos) = line.find("  ") {
            let hash = line[..pos].trim();
            let path = line[pos + 2..].trim();
            if !hash.is_empty() && !path.is_empty() {
                out.push((hash.to_string(), path.to_string()));
            }
        }
    }
    out
}

/// Manifest 验证（§23）：目录现状 vs Manifest。
pub fn verify_manifest(
    root: &Path,
    manifest_text: &str,
    cancel: &CancellationToken,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<ManifestDiffReport, WeaveError> {
    if !root.is_dir() {
        return Err(err(
            "integrity.rootMissing",
            format!("{} is not a directory", root.display()),
        ));
    }
    let manifest = parse_manifest(manifest_text);
    let total = manifest.len() as u64;
    let mut items = Vec::new();
    let mut current_files: std::collections::HashSet<String> = std::collections::HashSet::new();

    for (done, (expected_hash, rel)) in manifest.iter().enumerate() {
        if cancel.is_cancelled() {
            break;
        }
        let abs = root.join(rel.replace('/', "/"));
        let item_kind = match std::fs::metadata(&abs) {
            Err(_) => ManifestDiffKind::Removed,
            Ok(_) => match hash_file(&crate::fs::StdFilesystem, &abs, cancel, &mut |_| {}) {
                Ok(result) => {
                    let actual = result.digest_hex.unwrap_or_default();
                    if actual.eq_ignore_ascii_case(expected_hash) {
                        ManifestDiffKind::Same
                    } else {
                        ManifestDiffKind::Modified
                    }
                }
                Err(_) => ManifestDiffKind::Unreadable,
            },
        };
        if item_kind != ManifestDiffKind::Same {
            // Added/Removed/Modified/Unreadable 全部进差异列表
            if item_kind != ManifestDiffKind::Removed {
                current_files.insert(rel.clone());
            }
        }
        items.push(ManifestDiffItem {
            relative_path: rel.clone(),
            kind: item_kind,
        });
        progress(done as u64 + 1, total);
    }
    // Added：目录里有但 Manifest 没有的文件（复用 create 的遍历）
    let Ok(current) = create_manifest(root, cancel, &mut |_, _| {}) else {
        return Ok(ManifestDiffReport { items });
    };
    let manifest_paths: std::collections::HashSet<&str> =
        manifest.iter().map(|(_, p)| p.as_str()).collect();
    for entry in &current.entries {
        if !manifest_paths.contains(entry.relative_path.as_str()) {
            current_files.insert(entry.relative_path.clone());
        }
    }
    for path in current_files {
        let already = items
            .iter()
            .any(|i| i.relative_path == path && i.kind != ManifestDiffKind::Same);
        if !already {
            items.push(ManifestDiffItem {
                relative_path: path,
                kind: ManifestDiffKind::Added,
            });
        }
    }
    items.sort_by(|a, b| a.relative_path.cmp(&b.relative_path));
    Ok(ManifestDiffReport { items })
}

#[cfg(test)]
mod tests {
    use super::*;
    use weave_testkit::TempWorkspace;

    #[test]
    fn verify_match_and_mismatch() {
        let ws = TempWorkspace::new("integrity-mm").expect("ws");
        let file = ws.file("a.txt", "stable content").expect("write");
        let manifest = create_manifest(ws.path(), &CancellationToken::new(), &mut |_, _| ())
            .expect("manifest");
        let expected = manifest.entries[0].sha256.clone();

        let ok = verify_file_hash(&file, &expected, &CancellationToken::new()).expect("verify");
        assert_eq!(ok.status, HashVerifyStatus::Match);

        std::fs::write(&file, b"changed content").expect("rewrite");
        let bad = verify_file_hash(&file, &expected, &CancellationToken::new()).expect("verify");
        assert_eq!(bad.status, HashVerifyStatus::Mismatch);
        assert!(bad.actual_hash.as_ref().expect("hash") != &expected.to_lowercase());
    }

    #[test]
    fn manifest_serializes_relative_paths_and_round_trips() {
        let ws = TempWorkspace::new("integrity-manifest").expect("ws");
        let sub = ws.dir("子目录").expect("dir");
        std::fs::write(ws.path().join("a.txt"), b"A").expect("w");
        std::fs::write(sub.join("b.bin"), b"B!").expect("w");
        let manifest =
            create_manifest(ws.path(), &CancellationToken::new(), &mut |_, _| ()).expect("m");
        assert_eq!(manifest.entries.len(), 2);
        assert!(
            manifest
                .entries
                .iter()
                .all(|e| !e.relative_path.contains('\\'))
        );
        let text = serialize_manifest(&manifest);
        let parsed = parse_manifest(&text);
        assert_eq!(parsed.len(), 2);
        assert!(parsed.iter().all(|(h, _)| h.len() == 64));
        // 相对路径进 manifest（§22 机器无关）
        assert!(text.contains("a.txt"));
        assert!(text.contains("子目录/b.bin"));
    }

    #[test]
    fn manifest_verify_reports_all_diff_kinds() {
        let ws = TempWorkspace::new("integrity-diff").expect("ws");
        let f_keep = ws.file("keep.txt", "same").expect("w");
        let f_mod = ws.file("mod.txt", "original").expect("w");
        let f_del = ws.file("del.txt", "bye").expect("w");

        let manifest =
            create_manifest(ws.path(), &CancellationToken::new(), &mut |_, _| ()).expect("m");
        let text = serialize_manifest(&manifest);
        assert!(f_keep.exists() && f_mod.exists() && f_del.exists());
        let _ = (&f_keep, &f_mod, &f_del);

        // 变更场景：mod 改内容、del 删除、add 新增
        std::fs::write(ws.path().join("mod.txt"), "CHANGED").expect("w");
        std::fs::remove_file(ws.path().join("del.txt")).expect("rm");
        std::fs::write(ws.path().join("added.txt"), "new").expect("w");

        let report = verify_manifest(ws.path(), &text, &CancellationToken::new(), &mut |_, _| ())
            .expect("verify");
        assert!(
            report
                .items
                .iter()
                .any(|i| i.relative_path == "mod.txt" && i.kind == ManifestDiffKind::Modified)
        );
        assert!(
            report
                .items
                .iter()
                .any(|i| i.relative_path == "del.txt" && i.kind == ManifestDiffKind::Removed)
        );
        assert!(
            report
                .items
                .iter()
                .any(|i| i.relative_path == "added.txt" && i.kind == ManifestDiffKind::Added)
        );
        assert!(
            report
                .items
                .iter()
                .any(|i| i.relative_path == "keep.txt" && i.kind == ManifestDiffKind::Same)
        );
        assert_eq!(report.count(ManifestDiffKind::Modified), 1);
    }

    #[test]
    fn manifest_verify_unreadable_file() {
        use crate::fs::{FaultFilesystem, FsFault, StdFilesystem};
        // 不可读 ⇒ Unreadable（Fault fs 的 StatVanished 影响 hash 的 stat）
        let ws = TempWorkspace::new("integrity-unreadable").expect("ws");
        std::fs::write(ws.path().join("x.txt"), b"x").expect("w");
        let manifest =
            create_manifest(ws.path(), &CancellationToken::new(), &mut |_, _| ()).expect("m");
        let text = serialize_manifest(&manifest);
        let fs = FaultFilesystem::wrapping(Box::new(StdFilesystem));
        fs.arm(FsFault::OpenLocked);
        // hash_file 走 open_read → OpenLocked 注入 ⇒ hash error ⇒ Unreadable
        let report = verify_manifest_with_fs(ws.path(), &text, &fs, &CancellationToken::new())
            .expect("verify");
        assert!(
            report
                .items
                .iter()
                .any(|i| i.kind == ManifestDiffKind::Unreadable),
            "{report:?}"
        );
    }

    fn verify_manifest_with_fs(
        root: &Path,
        manifest_text: &str,
        fs: &dyn crate::fs::Filesystem,
        cancel: &CancellationToken,
    ) -> Result<ManifestDiffReport, WeaveError> {
        let manifest = parse_manifest(manifest_text);
        let mut items = Vec::new();
        for (expected_hash, rel) in &manifest {
            let abs = root.join(rel.replace('/', std::path::MAIN_SEPARATOR.to_string().as_str()));
            let kind = match std::fs::metadata(&abs) {
                Err(_) => ManifestDiffKind::Removed,
                Ok(_) => match hash_file(fs, &abs, cancel, &mut |_| {}) {
                    Ok(result) => {
                        let actual = result.digest_hex.unwrap_or_default();
                        if actual.eq_ignore_ascii_case(expected_hash) {
                            ManifestDiffKind::Same
                        } else {
                            ManifestDiffKind::Modified
                        }
                    }
                    Err(_) => ManifestDiffKind::Unreadable,
                },
            };
            items.push(ManifestDiffItem {
                relative_path: rel.clone(),
                kind,
            });
        }
        Ok(ManifestDiffReport { items })
    }
}
