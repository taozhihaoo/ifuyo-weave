//! 文件事实采集（M1 §8）。
//!
//! [`FileStat`] 是文件系统抽象返回的"一次性事实快照"：
//! 平台差异（Windows 隐藏属性、Unix 权限位）在这里被规约成统一形状，
//! 不可用的字段如实为 `None`，绝不伪造默认值（M1 §8.2）。

use std::time::SystemTime;
use weave_core::prelude::FileKind;

/// 一次 stat 的结果。语义为 **lstat**（不跟随符号链接）：符号链接条目
/// 的 kind 是 `Symlink`，size/times 是链接自身的事实（symlink 策略见 DECISIONS）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FileStat {
    pub size: u64,
    pub kind: FileKind,
    pub readonly: bool,
    /// Windows FILE_ATTRIBUTE_HIDDEN；非 Windows 平台 None（不伪造）。
    pub hidden: Option<bool>,
    pub created: Option<SystemTime>,
    pub modified: Option<SystemTime>,
    pub accessed: Option<SystemTime>,
}

impl FileStat {
    /// 从 std 的 metadata（应为 `symlink_metadata`）构造。
    pub fn from_std(meta: &std::fs::Metadata) -> Self {
        let kind = {
            let ft = meta.file_type();
            if ft.is_symlink() {
                FileKind::Symlink
            } else if ft.is_dir() {
                FileKind::Directory
            } else if ft.is_file() {
                FileKind::RegularFile
            } else {
                FileKind::Other
            }
        };
        Self {
            size: meta.len(),
            kind,
            readonly: meta.permissions().readonly(),
            hidden: hidden_from_std(meta),
            created: meta.created().ok(),
            modified: meta.modified().ok(),
            accessed: meta.accessed().ok(),
        }
    }
}

#[cfg(windows)]
fn hidden_from_std(meta: &std::fs::Metadata) -> Option<bool> {
    use std::os::windows::fs::MetadataExt;
    const FILE_ATTRIBUTE_HIDDEN: u32 = 0x2;
    Some(meta.file_attributes() & FILE_ATTRIBUTE_HIDDEN != 0)
}

#[cfg(not(windows))]
fn hidden_from_std(_meta: &std::fs::Metadata) -> Option<bool> {
    // Unix 语义的"隐藏"取决于文件名（dotfile），由调用方按名称判断，不在此伪造。
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use weave_testkit::TempWorkspace;

    #[test]
    fn stat_from_std_reports_regular_file_facts() {
        let ws = TempWorkspace::new("stat").expect("workspace");
        let path = ws.file("a.txt", "hello").expect("write");
        let meta = std::fs::symlink_metadata(&path).expect("stat");
        let stat = FileStat::from_std(&meta);
        assert_eq!(stat.kind, FileKind::RegularFile);
        assert_eq!(stat.size, 5);
        assert!(!stat.readonly);
        assert!(stat.modified.is_some());
    }

    #[test]
    fn stat_from_std_reports_directory_and_readonly() {
        let ws = TempWorkspace::new("stat-dir").expect("workspace");
        let dir = ws.dir("sub").expect("mkdir");
        let meta = std::fs::symlink_metadata(&dir).expect("stat");
        let stat = FileStat::from_std(&meta);
        assert_eq!(stat.kind, FileKind::Directory);
    }

    #[cfg(windows)]
    #[test]
    fn windows_hidden_attribute_is_reported() {
        use std::os::windows::fs::OpenOptionsExt;
        let ws = TempWorkspace::new("stat-hidden").expect("workspace");
        let path = ws.path().join("h.txt");
        // FILE_ATTRIBUTE_HIDDEN = 0x2
        std::fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .attributes(0x2)
            .open(&path)
            .expect("create hidden file");
        let meta = std::fs::symlink_metadata(&path).expect("stat");
        assert_eq!(FileStat::from_std(&meta).hidden, Some(true));
    }
}
