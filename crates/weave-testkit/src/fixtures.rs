//! Fixture 构建器（M1 §25 / §5.3）。
//!
//! 全部 fixture 在测试运行时**程序化生成**（charter 补丁条款：不提交进 git），
//! 内容确定性（固定字节/固定布局），测试不依赖用户真实文件。

use std::io;

use crate::TempWorkspace;

/// [`standard_tree`] 产出的统计，供扫描结果断言。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TreeStats {
    pub files: u64,
    pub directories: u64,
    pub total_size: u64,
}

/// 构建覆盖 M1 §25 的标准 fixture 树：
///
/// ```text
/// root/
/// ├── documents/   a.txt(10B) b.md(20B)
/// ├── images/      picture.bin  ← PNG 头伪装扩展名（声明 vs 检测）
/// ├── archives/    old.zip(40B)
/// ├── empty/                      ← 空目录
/// ├── unicode/     中文 文件.txt(9B)
/// ├── nested/      e/f/g.txt(7B)  ← 嵌套（root=0 → g 所在 f 深度 2）
/// ├── .hidden(1B)  noext(2B)  UPPER.TXT(3B)
/// ```
pub fn standard_tree(ws: &TempWorkspace) -> io::Result<TreeStats> {
    ws.file("documents/a.txt", "0123456789")?;
    ws.file("documents/b.md", "01234567890123456789")?;
    ws.bytes(
        "images/picture.bin",
        &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0],
    )?;
    ws.file(
        "archives/old.zip",
        "0123456789012345678901234567890123456789",
    )?;
    ws.dir("empty")?;
    ws.file("unicode/中文 文件.txt", "你好呀")?;
    ws.file("nested/e/f/g.txt", "0123456")?;
    ws.file(".hidden", "x")?;
    ws.file("noext", "xy")?;
    ws.file("UPPER.TXT", "xyz")?;

    Ok(TreeStats {
        files: 9,
        directories: 8, // documents/images/archives/empty/unicode/nested/nested-e/nested-e-f
        total_size: 10 + 20 + 10 + 40 + 9 + 7 + 1 + 2 + 3,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_tree_builds_deterministically() {
        let ws = TempWorkspace::new("fixture-tree").expect("ws");
        let stats = standard_tree(&ws).expect("build");
        assert_eq!(stats.files, 9);
        // documents, images, archives, empty, unicode, nested, nested/e, nested/e/f
        assert_eq!(stats.directories, 8);
        assert_eq!(stats.total_size, 102);
    }
}
