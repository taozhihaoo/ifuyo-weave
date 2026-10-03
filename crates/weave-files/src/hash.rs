//! 通用 Hash 服务（M1 §10）。
//!
//! - 只实现当前确认需要的 SHA-256（sha2 crate，成熟实现，不自研密码学原语）。
//! - 流式读取：内存占用 O(chunk)，禁止整读入内存（M1 §7.1）。
//! - 协作式取消：取消返回 `HashStatus::Cancelled` 结果而不是错误（M1 §10.4）。
//! - changed-during-read：哈希前后各 stat 一次，size/modified 变化 ⇒
//!   `HashStatus::Unstable`，digest 照常返回（best-effort，调用方必须看到状态）。
//!   策略选择与理由记录于 DECISIONS.md。

use std::io::Read;
use std::path::Path;
use std::time::Instant;

use sha2::{Digest, Sha256};
use weave_core::prelude::{
    CancellationToken, FileKind, HashAlgorithm, HashResult, HashStatus, OperationId, Progress,
    Recoverability, WeaveError, validate_absolute_path,
};

use crate::fs::Filesystem;

/// 流式读取块大小：256 KiB。
///
/// 足够大以摊薄 syscall 开销、足够小以保持内存常数级；集中在单一 pub 常量，
/// 未来可统一调整而无需散改（M1 §7.2，理由记入 DECISIONS.md）。
pub const HASH_CHUNK_SIZE: usize = 256 * 1024;

/// 计算文件 SHA-256（流式 + 协作取消 + 前后一致性检查）。
///
/// 只对 `RegularFile` 生效；目录/符号链接/其他类型返回结构化 Unsupported 错误
/// （调用方只对 RegularFile 提供显式 [Calculate SHA-256] 入口，M1 §34）。
pub fn hash_file(
    fs: &dyn Filesystem,
    path: &Path,
    cancel: &CancellationToken,
    report: &mut dyn FnMut(Progress),
) -> Result<HashResult, WeaveError> {
    let started = Instant::now();
    let validation = validate_absolute_path(&path.to_string_lossy())
        .map_err(|e| e.with_location("weave-files::hash"))?;
    let normalized = validation.normalized;

    let before = fs.stat(path).map_err(|e| stat_error(e, &normalized))?;
    if before.kind != FileKind::RegularFile {
        return Err(WeaveError::unsupported(
            "hash.notRegularFile",
            format!("'{normalized}' is not a regular file"),
        )
        .with_location("weave-files::hash"));
    }

    let mut reader = fs.open_read(path).map_err(|e| open_error(e, &normalized))?;

    let mut hasher = Sha256::new();
    let mut chunk = vec![0u8; HASH_CHUNK_SIZE];
    let mut processed: u64 = 0;
    let operation = OperationId::generate();

    loop {
        if cancel.is_cancelled() {
            // 取消 ≠ 失败（M1 §10.4）：结构化 Cancelled 结果，digest 为 None。
            return Ok(HashResult {
                algorithm: HashAlgorithm::Sha256,
                digest_hex: None,
                bytes_processed: processed,
                duration_ms: started.elapsed().as_millis() as u64,
                status: HashStatus::Cancelled,
            });
        }
        let read = reader
            .read(&mut chunk)
            .map_err(|e| read_error(e, &normalized))?;
        if read == 0 {
            break;
        }
        hasher.update(&chunk[..read]);
        processed += read as u64;
        report(Progress::running(
            operation.clone(),
            processed,
            Some(before.size),
        ));
    }

    // changed-during-hash：前后 stat 对比（size 与 modified）。
    let after = fs.stat(path).map_err(|e| stat_error(e, &normalized))?;
    let stable = after.size == before.size && after.modified == before.modified;
    let digest = hasher.finalize();
    let hex: String = digest.iter().map(|b| format!("{b:02x}")).collect();

    Ok(HashResult {
        algorithm: HashAlgorithm::Sha256,
        digest_hex: Some(hex),
        bytes_processed: processed,
        duration_ms: started.elapsed().as_millis() as u64,
        status: if stable {
            HashStatus::Completed
        } else {
            HashStatus::Unstable
        },
    })
}

fn stat_error(e: std::io::Error, path: &str) -> WeaveError {
    let message = format!("cannot stat '{path}': {e}");
    let base = match e.kind() {
        std::io::ErrorKind::NotFound => WeaveError::io("path.notFound", message),
        std::io::ErrorKind::PermissionDenied => {
            WeaveError::permission("path.permissionDenied", message)
        }
        _ => WeaveError::io("hash.statFailed", message),
    };
    base.with_location("weave-files::hash")
}

fn open_error(e: std::io::Error, path: &str) -> WeaveError {
    let base = match e.kind() {
        std::io::ErrorKind::PermissionDenied => {
            WeaveError::permission("hash.openDenied", format!("cannot open '{path}': {e}"))
        }
        _ => WeaveError::io("hash.openFailed", format!("cannot open '{path}': {e}")),
    };
    base.with_location("weave-files::hash")
        .with_recoverability(Recoverability::UserActionRequired)
}

fn read_error(e: std::io::Error, path: &str) -> WeaveError {
    WeaveError::io("hash.readFailed", format!("read failed on '{path}': {e}"))
        .with_location("weave-files::hash")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::StdFilesystem;
    use std::io;
    use std::path::Path;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::time::Duration;
    use weave_core::prelude::{CancellationToken, HashStatus, Progress};
    use weave_testkit::TempWorkspace;

    fn noop_progress() -> impl FnMut(Progress) {
        |_| {}
    }

    #[test]
    fn sha256_matches_known_golden_values() {
        let ws = TempWorkspace::new("hash-golden").expect("ws");
        let fs = StdFilesystem;
        let cancel = CancellationToken::new();

        let hello = ws.file("hello.txt", "hello").expect("write");
        let result = hash_file(&fs, &hello, &cancel, &mut noop_progress()).expect("hash");
        assert_eq!(result.status, HashStatus::Completed);
        assert_eq!(
            result.digest_hex.as_deref(),
            Some("2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824")
        );
        assert_eq!(result.bytes_processed, 5);

        let empty = ws.file("empty.bin", "").expect("write");
        let result = hash_file(&fs, &empty, &cancel, &mut noop_progress()).expect("hash");
        assert_eq!(
            result.digest_hex.as_deref(),
            Some("e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855")
        );
        assert_eq!(result.bytes_processed, 0);
    }

    #[test]
    fn streaming_matches_single_pass_across_many_chunks() {
        // 内容大于一个 chunk，覆盖多块读取路径；期望值用单次 digest 独立计算。
        let ws = TempWorkspace::new("hash-stream").expect("ws");
        let mut rng = weave_testkit::DetRandom::new(7);
        let mut content = vec![0u8; HASH_CHUNK_SIZE * 2 + 777];
        rng.fill_bytes(&mut content);
        let path = ws.bytes("big.bin", &content).expect("write");

        let result = hash_file(
            &StdFilesystem,
            &path,
            &CancellationToken::new(),
            &mut noop_progress(),
        )
        .expect("hash");
        let expected = Sha256::digest(&content);
        let expected_hex: String = expected.iter().map(|b| format!("{b:02x}")).collect();
        assert_eq!(result.digest_hex.as_deref(), Some(expected_hex.as_str()));
        assert_eq!(result.bytes_processed, content.len() as u64);
    }

    #[test]
    fn same_content_same_hash_regardless_of_path_or_name() {
        let ws = TempWorkspace::new("hash-det").expect("ws");
        let cancel = CancellationToken::new();
        let a = ws.file("dir1/same.txt", "identical").expect("write");
        let b = ws.file("dir2/other-name.bin", "identical").expect("write");
        let h1 = hash_file(&StdFilesystem, &a, &cancel, &mut noop_progress()).expect("h1");
        let h2 = hash_file(&StdFilesystem, &b, &cancel, &mut noop_progress()).expect("h2");
        assert_eq!(h1.digest_hex, h2.digest_hex);
    }

    #[test]
    fn cancellation_returns_cancelled_result_with_no_digest() {
        let ws = TempWorkspace::new("hash-cancel").expect("ws");
        let content = vec![0u8; HASH_CHUNK_SIZE * 2];
        let path = ws.bytes("big.bin", &content).expect("write");
        let cancel = CancellationToken::new();
        let first_progress = AtomicUsize::new(0);
        let mut report = |_p: Progress| {
            if first_progress.fetch_add(1, Ordering::SeqCst) == 0 {
                cancel.cancel();
            }
        };
        let result = hash_file(&StdFilesystem, &path, &cancel, &mut report).expect("cancelled ok");
        assert_eq!(result.status, HashStatus::Cancelled);
        assert!(result.digest_hex.is_none());
        assert!(result.bytes_processed < content.len() as u64);
    }

    #[test]
    fn non_regular_file_is_structured_unsupported() {
        let ws = TempWorkspace::new("hash-kind").expect("ws");
        let dir = ws.dir("sub").expect("mkdir");
        let err = hash_file(
            &StdFilesystem,
            &dir,
            &CancellationToken::new(),
            &mut noop_progress(),
        )
        .expect_err("directory must not hash");
        assert_eq!(err.code, "hash.notRegularFile");
    }

    #[test]
    fn missing_file_maps_to_structured_not_found() {
        let ws = TempWorkspace::new("hash-missing").expect("ws");
        let missing = ws.path().join("nope.txt");
        let err = hash_file(
            &StdFilesystem,
            &missing,
            &CancellationToken::new(),
            &mut noop_progress(),
        )
        .expect_err("missing");
        assert_eq!(err.code, "path.notFound");
    }

    #[test]
    fn changed_during_hash_is_reported_unstable_not_silent() {
        // 测试替身：第二次 stat 返回不同的 modified 时间，模拟哈希期间文件被改。
        struct ShiftStatFs {
            calls: AtomicUsize,
        }
        impl Filesystem for ShiftStatFs {
            fn exists(&self, _p: &Path) -> bool {
                true
            }
            fn stat(&self, p: &Path) -> io::Result<crate::metadata::FileStat> {
                let mut stat = StdFilesystem.stat(p)?;
                if self.calls.fetch_add(1, Ordering::SeqCst) >= 1 {
                    stat.modified = stat.modified.map(|t| t + Duration::from_secs(1));
                }
                Ok(stat)
            }
            fn read_dir(&self, _p: &Path) -> io::Result<Vec<String>> {
                Ok(vec![])
            }
            fn open_read(&self, p: &Path) -> io::Result<Box<dyn io::Read + Send>> {
                StdFilesystem.open_read(p)
            }
            fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
                StdFilesystem.rename(from, to)
            }
            fn create_dir_all(&self, path: &Path) -> io::Result<()> {
                StdFilesystem.create_dir_all(path)
            }
        }

        let ws = TempWorkspace::new("hash-unstable").expect("ws");
        let path = ws.file("changing.txt", "content").expect("write");
        let fs = ShiftStatFs {
            calls: AtomicUsize::new(0),
        };
        let result =
            hash_file(&fs, &path, &CancellationToken::new(), &mut noop_progress()).expect("hash");
        assert_eq!(result.status, HashStatus::Unstable);
        assert!(
            result.digest_hex.is_some(),
            "best-effort digest still returned"
        );
    }

    #[test]
    fn relative_path_is_rejected_before_any_fs_access() {
        let err = hash_file(
            &StdFilesystem,
            Path::new("relative.txt"),
            &CancellationToken::new(),
            &mut noop_progress(),
        )
        .expect_err("relative input");
        assert_eq!(err.code, "path.relative");
    }
}
