//! 路径安全契约（M0 §16，charter #17）。
//!
//! 统一原则：`User-selected paths → Normalize → Validate → Resolve → Operate`，
//! 而不是 `UI string → std::fs`。本模块是**纯字符串/语法层**契约：
//! 不做任何文件系统 I/O（那属于 Adapter 层），因此可以完整确定性测试。
//!
//! Windows 优先：盘符、保留设备名、非法字符、尾部点/空格、`..` 穿越、
//! UNC、`\\?\` 长路径前缀都在这里统一处理。
//!
//! M0 已知边界：大小写不敏感语义只提供规范化后简单折叠的等价判断；
//! 完整的 Windows case 契约与测试由 M1 File Core 建立（见 M1 提示词 #6.2）。

use crate::error::WeaveError;

/// Windows 保留设备名（按组件的 stem 匹配，大小写不敏感）。
const RESERVED_DEVICE_NAMES: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// 路径校验结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathValidation {
    /// 规范化后的路径：统一 `\` 分隔符、解析 `.` / `..`、折叠重复分隔符、盘符大写。
    pub normalized: String,
    /// 是否 UNC 路径（`\\server\share\...`）。
    pub is_unc: bool,
    /// 是否带 `\\?\` / `\\.\` 设备前缀。
    pub has_device_prefix: bool,
    /// 是否超过 legacy MAX_PATH（260）；超长时调用方应考虑 `\\?\` 前缀。
    pub exceeds_legacy_limit: bool,
    /// 规范化后是否是绝对路径（盘符 / UNC / 设备前缀 / 根路径）。
    pub is_absolute: bool,
}

fn path_error(code: &str, message: impl Into<String>) -> WeaveError {
    WeaveError::validation(code, message).with_location("weave-core::path")
}

fn is_separator(b: u8) -> bool {
    b == b'\\' || b == b'/'
}

/// 校验任意路径（绝对或相对），返回规范化结果。
pub fn validate_path(input: &str) -> Result<PathValidation, WeaveError> {
    if input.is_empty() {
        return Err(path_error("path.empty", "path must not be empty"));
    }
    if input.chars().any(char::is_control) {
        return Err(path_error(
            "path.invalidChar",
            "path must not contain control characters",
        ));
    }

    let has_device_prefix = input.starts_with("\\\\?\\") || input.starts_with("\\\\.\\");
    let is_unc = !has_device_prefix && input.starts_with("\\\\");
    let starts_with_sep =
        !has_device_prefix && !is_unc && (input.starts_with('\\') || input.starts_with('/'));

    // 设备前缀与 UNC 的前导 `\\` 单独处理，其余部分统一按分隔符切分。
    let body: &str = if has_device_prefix {
        &input[4..]
    } else if is_unc {
        &input[2..]
    } else {
        input
    };

    // 非法字符扫描针对 body：`\\?\` 前缀里的 '?' 是语法成分，不是用户内容。
    for ch in body.chars() {
        if matches!(ch, '<' | '>' | '|' | '?' | '*' | '"') {
            return Err(path_error(
                "path.invalidChar",
                format!("path must not contain '{ch}'"),
            ));
        }
    }

    // 盘符：`X:` 只允许出现在最前，X 为 ASCII 字母。
    let body_bytes = body.as_bytes();
    let drive_len =
        if body_bytes.len() >= 2 && body_bytes[0].is_ascii_alphabetic() && body_bytes[1] == b':' {
            2
        } else {
            0
        };
    if body[drive_len..].contains(':') {
        return Err(path_error(
            "path.invalidChar",
            "':' is only allowed as the drive separator",
        ));
    }
    // `C:\x` 有根；`C:x` 没有（Windows 语义：相对该盘当前目录）。
    let drive_with_root = drive_len > 0 && body_bytes.len() > 2 && is_separator(body_bytes[2]);
    let rooted = has_device_prefix || is_unc || starts_with_sep || drive_with_root;
    // UNC 的前两个组件（server, share）是根，`..` 不允许弹出。
    let protected_components = if is_unc { 2 } else { 0 };

    let components_part = &body[drive_len..];
    let mut components: Vec<String> = Vec::new();
    let mut leading_parents: Vec<String> = Vec::new();
    for raw_component in components_part.split(['\\', '/']) {
        if raw_component.is_empty() || raw_component == "." {
            continue;
        }
        if raw_component == ".." {
            if components.len() > protected_components {
                components.pop();
                continue;
            }
            if rooted {
                return Err(path_error(
                    "path.parentTraversal",
                    "'..' escapes past the path root",
                ));
            }
            // 相对路径允许保留前导 `..`（相对某个基准目录解析）。
            leading_parents.push("..".to_string());
            continue;
        }
        if raw_component.ends_with(' ') || raw_component.ends_with('.') {
            return Err(path_error(
                "path.trailingDotOrSpace",
                format!("path component '{raw_component}' ends with a dot or space"),
            )
            .with_suggestion(
                "Windows silently strips trailing dots and spaces, which changes the real target",
            ));
        }
        let stem = raw_component.split('.').next().unwrap_or(raw_component);
        if RESERVED_DEVICE_NAMES
            .iter()
            .any(|r| r.eq_ignore_ascii_case(stem))
        {
            return Err(path_error(
                "path.reservedName",
                format!("'{stem}' is a reserved Windows device name"),
            ));
        }
        components.push(raw_component.to_string());
    }

    // `\\server`（缺 share）是残缺 UNC，明确拒绝而不是拼出半截路径。
    if is_unc && components.len() < 2 {
        return Err(path_error(
            "path.malformed",
            "UNC path requires a \\\\server\\share root",
        ));
    }

    let is_absolute = rooted;

    let mut normalized = String::with_capacity(input.len() + 2);
    if has_device_prefix {
        normalized.push_str("\\\\?\\");
    } else if is_unc {
        normalized.push_str("\\\\");
    } else if starts_with_sep {
        normalized.push('\\');
    }
    if drive_len > 0 {
        normalized.push(body_bytes[0].to_ascii_uppercase() as char);
        normalized.push(':');
        if drive_with_root {
            normalized.push('\\');
        }
    }
    if !leading_parents.is_empty() {
        normalized.push_str(&leading_parents.join("\\"));
        if !components.is_empty() {
            normalized.push('\\');
        }
    }
    normalized.push_str(&components.join("\\"));

    if normalized.is_empty() {
        return Err(path_error(
            "path.empty",
            "path normalizes to an empty string",
        ));
    }

    Ok(PathValidation {
        exceeds_legacy_limit: is_absolute && normalized.len() > 260,
        normalized,
        is_unc,
        has_device_prefix,
        is_absolute,
    })
}

/// 校验绝对路径。工具要操作真实文件时必须用它（charter #17）。
pub fn validate_absolute_path(input: &str) -> Result<PathValidation, WeaveError> {
    let validation = validate_path(input)?;
    if !validation.is_absolute {
        return Err(path_error(
            "path.relative",
            "an absolute path is required for filesystem operations",
        )
        .with_suggestion("resolve the path against an explicit base directory first"));
    }
    Ok(validation)
}

/// Windows 大小写不敏感冲突检测：两个路径规范化后忽略大小写指向同一目标即冲突。
/// 这是 M2 碰撞检测（含 case-only rename 场景）的契约基础；完整语义由 M1/M2 持续完善。
pub fn paths_conflict(a: &str, b: &str) -> bool {
    windows_paths_equivalent(a, b)
}

/// Windows 语义下的等价判断：规范化后做大小写不敏感比较。
/// M0 使用简单折叠；完整大小写契约由 M1 建立。
pub fn windows_paths_equivalent(a: &str, b: &str) -> bool {
    match (validate_path(a), validate_path(b)) {
        (Ok(x), Ok(y)) => x.normalized.to_lowercase() == y.normalized.to_lowercase(),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn err_code(input: &str) -> String {
        validate_path(input).expect_err("must be rejected").code
    }

    #[test]
    fn normalizes_separators_and_dot_components() {
        let v = validate_path(r"C:\a\.\b\\c\").expect("valid");
        assert_eq!(v.normalized, r"C:\a\b\c");
        assert!(!v.is_unc);
        assert!(v.is_absolute);
    }

    #[test]
    fn uppercases_drive_letter() {
        let v = validate_path(r"d:/x/y").expect("valid");
        assert_eq!(v.normalized, r"D:\x\y");
        assert!(v.is_absolute);
    }

    #[test]
    fn drive_relative_paths_are_not_absolute() {
        // `C:x` 是"相对 C 盘当前目录"的路径，不是绝对路径。
        let v = validate_path(r"C:x\y").expect("valid");
        assert_eq!(v.normalized, r"C:x\y");
        assert!(!v.is_absolute);
        assert!(validate_absolute_path(r"C:x\y").is_err());
    }

    #[test]
    fn bare_drive_root_normalizes_with_separator() {
        let v = validate_path(r"C:\").expect("valid");
        assert_eq!(v.normalized, r"C:\");
    }

    #[test]
    fn resolves_parent_traversal_within_root() {
        let v = validate_path(r"C:\a\b\..\c").expect("valid");
        assert_eq!(v.normalized, r"C:\a\c");
    }

    #[test]
    fn rejects_parent_traversal_past_root() {
        assert_eq!(err_code(r"C:\..\"), "path.parentTraversal");
        assert_eq!(err_code(r"C:\.."), "path.parentTraversal");
        assert_eq!(err_code(r"\\server\share\..\..\"), "path.parentTraversal");
        assert_eq!(err_code(r"\\?\C:\a\..\.."), "path.parentTraversal");
    }

    #[test]
    fn unc_share_root_is_protected() {
        let v = validate_path(r"\\server\share\a\..\b").expect("valid");
        assert_eq!(v.normalized, r"\\server\share\b");
        assert_eq!(
            validate_path(r"\\server\..\x")
                .expect_err("above server root")
                .code,
            "path.parentTraversal"
        );
        assert_eq!(
            validate_path(r"\\server")
                .expect_err("unc without share")
                .code,
            "path.malformed"
        );
    }

    #[test]
    fn relative_paths_may_keep_leading_parent_hops() {
        let v = validate_path(r"..\data\file.txt").expect("valid");
        assert_eq!(v.normalized, r"..\data\file.txt");
        assert!(!v.is_absolute);

        let two = validate_path(r"..\..\x").expect("valid");
        assert_eq!(two.normalized, r"..\..\x");

        let collapse = validate_path(r"a\..\..\x").expect("valid");
        assert_eq!(collapse.normalized, r"..\x");
    }

    #[test]
    fn rejects_invalid_characters() {
        assert_eq!(err_code(r"C:\a<b"), "path.invalidChar");
        assert_eq!(err_code(r"C:\a|b"), "path.invalidChar");
        assert_eq!(err_code(r"C:\a*b"), "path.invalidChar");
        assert_eq!(err_code(r#"C:\a"b"#), "path.invalidChar");
        assert_eq!(err_code(r"C:\a:C\b"), "path.invalidChar");
    }

    #[test]
    fn rejects_reserved_device_names() {
        assert_eq!(err_code(r"C:\CON.txt"), "path.reservedName");
        assert_eq!(err_code(r"C:\dir\com1"), "path.reservedName");
        assert_eq!(err_code(r"C:\lpt9.log"), "path.reservedName");
    }

    #[test]
    fn rejects_trailing_dot_or_space_components() {
        assert_eq!(err_code(r"C:\file.txt "), "path.trailingDotOrSpace");
        assert_eq!(err_code(r"C:\file.txt."), "path.trailingDotOrSpace");
    }

    #[test]
    fn rejects_empty_and_control_characters() {
        assert_eq!(err_code(""), "path.empty");
        assert_eq!(err_code("C:\0x01\\"), "path.invalidChar");
    }

    #[test]
    fn handles_unc_and_device_prefix() {
        let unc = validate_path(r"\\server\share\a.txt").expect("valid");
        assert!(unc.is_unc);
        assert_eq!(unc.normalized, r"\\server\share\a.txt");

        let dev = validate_path(r"\\?\C:\very\long\path").expect("valid");
        assert!(dev.has_device_prefix);
        assert_eq!(dev.normalized, r"\\?\C:\very\long\path");
    }

    #[test]
    fn absolute_path_requirement_rejects_relative_input() {
        assert_eq!(
            validate_absolute_path(r"relative\file.txt")
                .expect_err("relative must fail")
                .code,
            "path.relative"
        );
        assert!(validate_absolute_path(r"C:\ok.txt").is_ok());
    }

    #[test]
    fn case_insensitive_equivalence_for_windows() {
        assert!(windows_paths_equivalent(
            r"C:\Data\File.txt",
            r"c:\data\file.TXT"
        ));
        assert!(!windows_paths_equivalent(r"C:\a.txt", r"C:\b.txt"));
    }

    #[test]
    fn case_insensitive_conflict_contract_for_m2_collision() {
        // case-only rename：同一目录内 a.jpg → A.jpg 属于同目标冲突，必须可检测。
        assert!(paths_conflict(r"C:\work\a.jpg", r"C:\WORK\A.JPG"));
        assert!(paths_conflict(r"C:\work\a.jpg", r"C:\work\A.jpg"));
        // 不同目标不冲突。
        assert!(!paths_conflict(r"C:\work\a.jpg", r"C:\work\b.jpg"));
        // 规范化差异（. 组件）不影响等价判断。
        assert!(paths_conflict(r"C:\work\.\a.jpg", r"C:\work\a.jpg"));
    }

    #[test]
    fn long_paths_are_flagged_not_rejected() {
        let deep = format!(r"C:\{}", "d\\".repeat(140));
        let v = validate_path(&deep).expect("valid");
        assert!(v.exceeds_legacy_limit);
    }
}
