//! 故障注入文件系统（M1 §23 / §50）。
//!
//! 在 [`Filesystem`] 边界上注入确定性故障，让 walker / hash / inspector 的
//! 失败路径无需真实 OS 权限即可测试（Windows 上无法轻易构造不可读目录）。
//! 操作级粒度（stat / read_dir / open / read）远比"笼统 permission denied"
//! 有用：扫描器对"读目录失败"与"stat 失败"的聚合行为不同。

use std::collections::{BTreeMap, BTreeSet};
use std::io;
use std::path::Path;
use std::sync::Mutex;

use super::Filesystem;
use crate::metadata::FileStat;

/// 可注入的文件系统故障点。
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum FsFault {
    /// stat 返回 PermissionDenied。
    StatDenied,
    /// stat 返回 NotFound（文件在扫描中途消失）。
    StatVanished,
    /// read_dir 返回 PermissionDenied（目录不可进入）。
    ReadDirDenied,
    /// open_read 返回锁定/共享冲突。
    OpenLocked,
    /// 读取中途返回 I/O 错误（部分读取后失败）。
    ReadFailure,
    /// rename 返回 PermissionDenied。
    RenameDenied,
}

/// 包装任意 [`Filesystem`]，在布防的故障点注入失败。
/// 每次命中计数，供测试断言"故障真的走到了"。
pub struct FaultFilesystem {
    inner: Box<dyn Filesystem>,
    armed: Mutex<BTreeSet<FsFault>>,
    trips: Mutex<BTreeMap<FsFault, u64>>,
}

impl FaultFilesystem {
    pub fn wrapping(inner: Box<dyn Filesystem>) -> Self {
        Self {
            inner,
            armed: Mutex::new(BTreeSet::new()),
            trips: Mutex::new(BTreeMap::new()),
        }
    }

    pub fn arm(&self, fault: FsFault) -> &Self {
        self.armed.lock().expect("fault lock").insert(fault);
        self
    }

    pub fn trips(&self, fault: FsFault) -> u64 {
        self.trips
            .lock()
            .expect("fault lock")
            .get(&fault)
            .copied()
            .unwrap_or(0)
    }

    fn trip(&self, fault: FsFault, error: impl FnOnce() -> io::Error) -> Option<io::Error> {
        let armed = self.armed.lock().expect("fault lock").contains(&fault);
        if armed {
            self.trips
                .lock()
                .expect("fault lock")
                .entry(fault)
                .and_modify(|n| *n += 1)
                .or_insert(1);
            Some(error())
        } else {
            None
        }
    }
}

impl Filesystem for FaultFilesystem {
    fn exists(&self, path: &Path) -> bool {
        self.inner.exists(path)
    }

    fn stat(&self, path: &Path) -> io::Result<FileStat> {
        if let Some(err) = self.trip(FsFault::StatDenied, || {
            io::Error::from(io::ErrorKind::PermissionDenied)
        }) {
            return Err(err);
        }
        if let Some(err) = self.trip(FsFault::StatVanished, || {
            io::Error::from(io::ErrorKind::NotFound)
        }) {
            return Err(err);
        }
        self.inner.stat(path)
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<String>> {
        if let Some(err) = self.trip(FsFault::ReadDirDenied, || {
            io::Error::from(io::ErrorKind::PermissionDenied)
        }) {
            return Err(err);
        }
        self.inner.read_dir(path)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn io::Read + Send>> {
        if let Some(err) = self.trip(FsFault::OpenLocked, || {
            io::Error::from_raw_os_error(windows_sharing_violation_code())
        }) {
            return Err(err);
        }
        let reader = self.inner.open_read(path)?;
        if self
            .armed
            .lock()
            .expect("fault lock")
            .contains(&FsFault::ReadFailure)
        {
            // 故障是确定性的：命中计数在构造时预记（FailAfterReader 无回 borrowed self）。
            self.trips
                .lock()
                .expect("fault lock")
                .entry(FsFault::ReadFailure)
                .and_modify(|n| *n += 1)
                .or_insert(1);
            return Ok(Box::new(FailAfterReader {
                inner: reader,
                bytes_before_failure: 3,
            }));
        }
        Ok(reader)
    }

    fn rename(&self, from: &Path, to: &Path) -> io::Result<()> {
        if let Some(err) = self.trip(FsFault::RenameDenied, || {
            io::Error::from(io::ErrorKind::PermissionDenied)
        }) {
            return Err(err);
        }
        self.inner.rename(from, to)
    }

    fn create_dir_all(&self, path: &Path) -> io::Result<()> {
        self.inner.create_dir_all(path)
    }
}

fn windows_sharing_violation_code() -> i32 {
    // 32 (0x20): ERROR_SHARING_VIOLATION —— 与真实 Windows 文件锁定一致。
    #[cfg(windows)]
    {
        32
    }
    #[cfg(not(windows))]
    {
        5 // EBUSY-ish; platform label is irrelevant to the tests that use it
    }
}

/// 读取 N 字节后注入 I/O 错误——覆盖"partial read 后失败"路径。
struct FailAfterReader {
    inner: Box<dyn io::Read + Send>,
    bytes_before_failure: u64,
}

impl io::Read for FailAfterReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        if self.bytes_before_failure == 0 {
            return Err(io::Error::other("injected read failure"));
        }
        let limit = buf.len().min(self.bytes_before_failure as usize);
        let n = self.inner.read(&mut buf[..limit])?;
        self.bytes_before_failure -= n as u64;
        if n == 0 {
            // EOF 先于计数到来（文件比预期短）：同样以故障收场，不静默成功。
            return Err(io::Error::other("injected read failure"));
        }
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fs::StdFilesystem;
    use weave_testkit::TempWorkspace;

    #[test]
    fn armed_faults_fail_and_count() {
        let ws = TempWorkspace::new("fault-fs").expect("workspace");
        let file = ws.file("a.txt", "hello").expect("write");
        let fs = FaultFilesystem::wrapping(Box::new(StdFilesystem));
        fs.arm(FsFault::StatDenied);

        let err = fs.stat(&file).expect_err("injected");
        assert_eq!(err.kind(), io::ErrorKind::PermissionDenied);
        assert_eq!(fs.trips(FsFault::StatDenied), 1);
    }

    #[test]
    fn read_failure_injects_after_partial_read() {
        let ws = TempWorkspace::new("fault-read").expect("workspace");
        let file = ws.file("a.txt", "hello world").expect("write");
        let fs = FaultFilesystem::wrapping(Box::new(StdFilesystem));
        fs.arm(FsFault::ReadFailure);

        let mut reader = fs.open_read(&file).expect("open");
        let mut chunk = [0u8; 16];
        let first = reader.read(&mut chunk).expect("first read is partial ok");
        assert_eq!(first, 3);
        let err = reader.read(&mut chunk).expect_err("then injected failure");
        assert_ne!(err.kind(), io::ErrorKind::WouldBlock);
        assert_eq!(fs.trips(FsFault::ReadFailure), 1);
    }
}
