//! [`Filesystem`] 的 std 实现：生产环境的唯一实现。

use std::io;
use std::path::Path;

use super::Filesystem;
use crate::metadata::FileStat;

#[derive(Debug, Default, Clone, Copy)]
pub struct StdFilesystem;

impl Filesystem for StdFilesystem {
    fn exists(&self, path: &Path) -> bool {
        // 存在性判断不跟随链接：悬空链接也是"存在"的事实。
        path.symlink_metadata().is_ok()
    }

    fn stat(&self, path: &Path) -> io::Result<FileStat> {
        std::fs::symlink_metadata(path).map(|m| FileStat::from_std(&m))
    }

    fn read_dir(&self, path: &Path) -> io::Result<Vec<String>> {
        let mut names = Vec::new();
        for entry in std::fs::read_dir(path)? {
            let entry = entry?;
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
        Ok(names)
    }

    fn open_read(&self, path: &Path) -> io::Result<Box<dyn io::Read + Send>> {
        // 显式只读 + 不创建；share mode 使用 std 默认（Windows 上允许共享读）。
        Ok(Box::new(std::fs::File::open(path)?))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use weave_testkit::TempWorkspace;

    #[test]
    fn std_fs_round_trip_over_the_four_operations() {
        let ws = TempWorkspace::new("stdfs").expect("workspace");
        let file = ws.file("a.txt", "hello").expect("write");
        let dir = ws.dir("sub").expect("mkdir");
        let fs = StdFilesystem;

        assert!(fs.exists(&file));
        assert!(fs.exists(&dir));
        assert!(!fs.exists(&ws.path().join("missing.txt")));

        let stat = fs.stat(&file).expect("stat");
        assert_eq!(stat.size, 5);

        let mut names = fs.read_dir(ws.path()).expect("read_dir");
        names.sort();
        assert_eq!(names, vec!["a.txt".to_string(), "sub".to_string()]);

        use std::io::Read;
        let mut handle = fs.open_read(&file).expect("open");
        let mut content = String::new();
        handle.read_to_string(&mut content).expect("read");
        assert_eq!(content, "hello");
    }

    #[test]
    fn missing_path_stat_is_structured_io_error() {
        let ws = TempWorkspace::new("stdfs-missing").expect("workspace");
        let fs = StdFilesystem;
        let err = fs.stat(&ws.path().join("nope")).expect_err("must fail");
        assert_eq!(err.kind(), std::io::ErrorKind::NotFound);
    }
}
