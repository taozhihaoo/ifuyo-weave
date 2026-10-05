//! 分层验证执行（M2.0 §7）：weave-core 的策略模型在文件系统上的应用。
//!
//! - `capture_fingerprint`：Plan 构建时捕获期望快照（按策略可含 Hash）。
//! - `verify_fingerprint`：Apply 前重新验证；任何字段失配 ⇒ 结构化
//!   Conflict（`{base}.sourceChanged`），绝不智能纠正（M2 §38 同源纪律）。
//!
//! 本模块不决定"谁用哪档策略"——场景自己选；默认档是 SizeMtime，
//! FullHash 的成本必须由调用场景显式承担（M2.0 §25）。

use std::path::Path;

use weave_core::prelude::{
    CancellationToken, FileFingerprint, HashAlgorithm, VerificationPolicy, WeaveError,
};

use crate::fs::Filesystem;

/// 按策略捕获文件状态快照。stat 失败按 base_code 精确分类（M2.0 §14）。
pub fn capture_fingerprint(
    fs: &dyn Filesystem,
    path: &Path,
    policy: VerificationPolicy,
) -> Result<FileFingerprint, WeaveError> {
    let stat = fs.stat(path).map_err(|e| {
        WeaveError::from_io(&e, "verify", format!("cannot stat {}", path.display()))
            .with_location("weave-files::verify")
    })?;
    let digest_hex = match policy {
        VerificationPolicy::FullHash(algorithm) => {
            let result = hash_with(fs, path, algorithm)?;
            Some(result)
        }
        VerificationPolicy::SizeMtime | VerificationPolicy::Exists => None,
    };
    Ok(FileFingerprint {
        path: path.to_string_lossy().into_owned(),
        size: Some(stat.size),
        modified: stat.modified,
        digest_hex,
    })
}

/// 按策略重新验证：失配 ⇒ `{base}.sourceChanged`（Conflict）。
/// expected 中为 None 的字段跳过（快照时不可得 ⇒ 验证时不猜测）。
pub fn verify_fingerprint(
    fs: &dyn Filesystem,
    expected: &FileFingerprint,
    policy: VerificationPolicy,
    base_code: &str,
) -> Result<(), WeaveError> {
    let path = Path::new(&expected.path);
    let stat = fs.stat(path).map_err(|e| {
        WeaveError::from_io(
            &e,
            base_code,
            format!("revalidation failed for {}", expected.path),
        )
        .with_location("weave-files::verify")
    })?;
    if matches!(
        policy,
        VerificationPolicy::SizeMtime | VerificationPolicy::FullHash(_)
    ) {
        if let Some(exp) = expected.size
            && stat.size != exp
        {
            return Err(WeaveError::conflict(
                &format!("{base_code}.sourceChanged"),
                format!("source changed since plan (size {} != {})", stat.size, exp),
            )
            .with_location("weave-files::verify"));
        }
        if let (Some(exp), Some(act)) = (expected.modified, stat.modified)
            && exp != act
        {
            return Err(WeaveError::conflict(
                &format!("{base_code}.sourceChanged"),
                format!("source was modified after the plan was built (mtime {act:?} != {exp:?})"),
            )
            .with_location("weave-files::verify"));
        }
    }
    if let VerificationPolicy::FullHash(algorithm) = policy {
        let actual = hash_with(fs, path, algorithm)?;
        if let Some(exp) = &expected.digest_hex
            && &actual != exp
        {
            return Err(WeaveError::conflict(
                &format!("{base_code}.hashMismatch"),
                format!("content hash changed since plan ({actual} != {exp})",),
            )
            .with_location("weave-files::verify"));
        }
    }
    Ok(())
}

fn hash_with(
    fs: &dyn Filesystem,
    path: &Path,
    algorithm: HashAlgorithm,
) -> Result<String, WeaveError> {
    debug_assert_eq!(algorithm, HashAlgorithm::Sha256, "only Sha256 exists today");
    let result = crate::hash::hash_file(fs, path, &CancellationToken::new(), &mut |_| {})?;
    match result.digest_hex {
        Some(hex) => Ok(hex),
        None => Err(WeaveError::io(
            "verify.hashUnavailable",
            format!("hash unavailable for {}", path.display()),
        )
        .with_location("weave-files::verify")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::{FaultFilesystem, FsFault, StdFilesystem};
    use weave_testkit::TempWorkspace;

    #[test]
    fn capture_and_verify_roundtrip_passes_on_unchanged_file() {
        let ws = TempWorkspace::new("verify-ok").expect("ws");
        let file = ws.file("a.txt", "stable content").expect("write");
        let fs = StdFilesystem;

        let fp = capture_fingerprint(&fs, &file, VerificationPolicy::SizeMtime).expect("capture");
        assert_eq!(fp.size, Some("stable content".len() as u64));
        assert_eq!(fp.digest_hex, None);
        verify_fingerprint(&fs, &fp, VerificationPolicy::SizeMtime, "rename").expect("verify");
    }

    #[test]
    fn full_hash_policy_detects_content_change() {
        let ws = TempWorkspace::new("verify-hash").expect("ws");
        let file = ws.file("a.txt", "original").expect("write");
        let fs = StdFilesystem;

        let fp = capture_fingerprint(
            &fs,
            &file,
            VerificationPolicy::FullHash(HashAlgorithm::Sha256),
        )
        .expect("capture");
        assert!(fp.digest_hex.is_some());
        std::fs::write(&file, "tampered").expect("tamper");
        let err = verify_fingerprint(
            &fs,
            &fp,
            VerificationPolicy::FullHash(HashAlgorithm::Sha256),
            "move",
        )
        .expect_err("content changed");
        // size+mtime 先于 hash 失配（同文件覆盖写通常也变了 size）
        assert!(
            err.code == "move.sourceChanged" || err.code == "move.hashMismatch",
            "{err}"
        );
    }

    #[test]
    fn size_mtime_policy_is_cheap_and_still_detects_change() {
        let ws = TempWorkspace::new("verify-stat").expect("ws");
        let file = ws.file("a.txt", "v1").expect("write");
        let fs = StdFilesystem;

        let fp = capture_fingerprint(&fs, &file, VerificationPolicy::SizeMtime).expect("capture");
        std::fs::write(&file, "v2 with different size").expect("rewrite");
        let err = verify_fingerprint(&fs, &fp, VerificationPolicy::SizeMtime, "rename")
            .expect_err("changed");
        assert_eq!(err.code, "rename.sourceChanged");
    }

    #[test]
    fn verification_stat_failure_is_classified_not_generic() {
        let ws = TempWorkspace::new("verify-stat-fail").expect("ws");
        let file = ws.file("a.txt", "x").expect("write");
        let fs = FaultFilesystem::wrapping(Box::new(StdFilesystem));
        fs.arm(FsFault::StatVanished);

        let fp = capture_fingerprint(&StdFilesystem, &file, VerificationPolicy::Exists)
            .expect("capture with std fs");
        let err = verify_fingerprint(&fs, &fp, VerificationPolicy::Exists, "rename")
            .expect_err("vanished");
        assert_eq!(err.code, "rename.sourceNotFound");
        assert_eq!(fs.trips(FsFault::StatVanished), 1);
    }
}
