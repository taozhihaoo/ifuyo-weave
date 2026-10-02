//! 临时工作区：隔离的测试目录（M0 §23.2）。

use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static WORKSPACE_COUNTER: AtomicU64 = AtomicU64::new(0);

/// 一个自包含的临时目录。Drop 时尽力删除整个目录树。
///
/// 测试绝不使用 `C:\Users\<真实用户>\...` 之类的真实位置；
/// 一切产物都落在本结构内部。
pub struct TempWorkspace {
    root: PathBuf,
}

impl TempWorkspace {
    /// 在系统临时目录下创建 `weave-test-<label>-<unique>`。
    pub fn new(label: &str) -> io::Result<Self> {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_nanos() as u64)
            .unwrap_or(0);
        let seq = WORKSPACE_COUNTER.fetch_add(1, Ordering::Relaxed);
        let root = std::env::temp_dir().join(format!("weave-test-{label}-{nanos:x}-{seq:x}"));
        std::fs::create_dir_all(&root)?;
        Ok(Self { root })
    }

    pub fn path(&self) -> &Path {
        &self.root
    }

    /// 在工作区内创建相对目录（自动创建父目录）。
    pub fn dir(&self, relative: &str) -> io::Result<PathBuf> {
        let path = self.root.join(relative);
        std::fs::create_dir_all(&path)?;
        Ok(path)
    }

    /// 在工作区内写入 UTF-8 文件（自动创建父目录）。
    pub fn file(&self, relative: &str, contents: &str) -> io::Result<PathBuf> {
        self.bytes(relative, contents.as_bytes())
    }

    /// 在工作区内写入字节文件（自动创建父目录）。
    pub fn bytes(&self, relative: &str, contents: &[u8]) -> io::Result<PathBuf> {
        let path = self.root.join(relative);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(&path, contents)?;
        Ok(path)
    }
}

impl Drop for TempWorkspace {
    fn drop(&mut self) {
        // 清理失败仅表示临时目录残留（系统临时目录会被 OS 清理），
        // 不应让测试因清理问题产生误导性失败——这是有意的 best-effort。
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn creates_files_dirs_and_cleans_up_on_drop() {
        let path = {
            let ws = TempWorkspace::new("cleanup").expect("create workspace");
            ws.file("a/b/c.txt", "hello").expect("write file");
            ws.dir("d").expect("create dir");
            assert!(ws.path().join("a/b/c.txt").is_file());
            assert!(ws.path().join("d").is_dir());
            ws.path().to_path_buf()
        };
        assert!(!path.exists(), "workspace must be removed on drop");
    }

    #[test]
    fn workspaces_do_not_collide() {
        let a = TempWorkspace::new("unique").expect("create a");
        let b = TempWorkspace::new("unique").expect("create b");
        assert_ne!(a.path(), b.path());
    }
}
