//! 文件类型分类（M1 §37 / §8.4）。
//!
//! 分类是 **derived fact**，必须携带证据（evidence）：
//! - `Extension`：扩展名映射表命中
//! - `MagicBytes`：有界 magic-byte 嗅探命中（≤ [`super::inspect::SNIFF_LIMIT`] 字节）
//! - `Unknown`：未知是正常状态而不是错误（M1 §38）
//!
//! 禁止"foo.bin → Executable"式的凭空推断。M1 的映射表刻意保持小而诚实：
//! 只收录高置信度条目，不为覆盖率引入巨型 MIME 数据库（M1 §8.4 / §44）。

use weave_core::prelude::TextEncoding;

/// 文件类别（M1 §37 的有限、可扩展清单）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FileCategory {
    Text,
    Image,
    Audio,
    Video,
    Document,
    Archive,
    Executable,
    Code,
    Data,
    Unknown,
}

impl FileCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            FileCategory::Text => "text",
            FileCategory::Image => "image",
            FileCategory::Audio => "audio",
            FileCategory::Video => "video",
            FileCategory::Document => "document",
            FileCategory::Archive => "archive",
            FileCategory::Executable => "executable",
            FileCategory::Code => "code",
            FileCategory::Data => "data",
            FileCategory::Unknown => "unknown",
        }
    }
}

/// 分类证据：类别从哪里来，可追溯（M1 §37）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClassificationEvidence {
    Extension,
    MagicBytes,
    None,
}

/// 一次分类的结果。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Classification {
    pub category: FileCategory,
    pub evidence: ClassificationEvidence,
    /// 高置信度时的 MIME 标签；无法确定时 None。
    pub mime: Option<&'static str>,
}

const MIME_PNG: &str = "image/png";
const MIME_JPEG: &str = "image/jpeg";
const MIME_GIF: &str = "image/gif";
const MIME_PDF: &str = "application/pdf";
const MIME_ZIP: &str = "application/zip";

/// 扩展名 → (类别, mime)。键全部小写；调用方负责把扩展名小写化。
fn extension_table(ext: &str) -> Option<(FileCategory, Option<&'static str>)> {
    use FileCategory::*;
    Some(match ext {
        // 文本
        "txt" | "log" | "csv" | "tsv" | "md" | "ini" | "cfg" | "yaml" | "yml" | "toml" => {
            (Text, None)
        }
        // 代码
        "rs" | "py" | "js" | "mjs" | "cjs" | "ts" | "tsx" | "jsx" | "c" | "h" | "cpp" | "hpp"
        | "java" | "kt" | "go" | "rb" | "php" | "sh" | "bat" | "ps1" | "sql" | "css" | "html"
        | "json" | "xml" => (Code, None),
        // 图片
        "png" => (Image, Some(MIME_PNG)),
        "jpg" | "jpeg" => (Image, Some(MIME_JPEG)),
        "gif" => (Image, Some(MIME_GIF)),
        "bmp" | "webp" | "ico" | "tif" | "tiff" | "svg" => (Image, None),
        // 音视频
        "mp3" | "wav" | "flac" | "ogg" | "m4a" | "aac" => (Audio, None),
        "mp4" | "mkv" | "avi" | "mov" | "webm" | "wmv" => (Video, None),
        // 文档
        "pdf" => (Document, Some(MIME_PDF)),
        "doc" | "docx" | "xls" | "xlsx" | "ppt" | "pptx" | "odt" | "ods" | "odp" | "rtf" => {
            (Document, None)
        }
        // 归档
        "zip" => (Archive, Some(MIME_ZIP)),
        "7z" | "rar" | "gz" | "bz2" | "xz" | "tar" | "zst" | "cab" => (Archive, None),
        // 可执行（Windows 侧事实扩展名；不因 .bin 猜测）
        "exe" | "dll" | "sys" | "msi" | "com" => (Executable, None),
        // 数据
        "db" | "sqlite" | "sqlite3" | "parquet" | "arrow" | "npy" | "npz" => (Data, None),
        _ => return None,
    })
}

/// magic-byte 签名表（前缀匹配，全部高置信度）。
fn magic_table(bytes: &[u8]) -> Option<(FileCategory, Option<&'static str>)> {
    use FileCategory::*;
    Some(match bytes {
        [0x89, b'P', b'N', b'G', ..] => (Image, Some(MIME_PNG)),
        [0xFF, 0xD8, 0xFF, ..] => (Image, Some(MIME_JPEG)),
        [b'G', b'I', b'F', b'8', ..] => (Image, Some(MIME_GIF)),
        [b'%', b'P', b'D', b'F', ..] => (Document, Some(MIME_PDF)),
        [b'P', b'K', 0x03, 0x04, ..] => (Archive, Some(MIME_ZIP)),
        [0x7F, b'E', b'L', b'F', ..] => (Executable, None),
        [b'M', b'Z', ..] => (Executable, None),
        [b'R', b'I', b'F', b'F', ..] => (Audio, None), // RIFF 容器（WAV/AVI 细分留给未来）
        [b'z', b'h', b'u', b'?', ..] => (Archive, None),
        _ => return None,
    })
}

/// 分类：扩展名优先（声明事实），缺失/未知时有界嗅探（检测事实），仍未知则 Unknown。
pub fn classify(extension: Option<&str>, sniffed: Option<&[u8]>) -> Classification {
    if let Some(ext) = extension {
        let ext = ext.to_ascii_lowercase();
        if let Some((category, mime)) = extension_table(&ext) {
            return Classification {
                category,
                evidence: ClassificationEvidence::Extension,
                mime,
            };
        }
    }
    if let Some((category, mime)) = sniffed.and_then(magic_table) {
        return Classification {
            category,
            evidence: ClassificationEvidence::MagicBytes,
            mime,
        };
    }
    Classification {
        category: FileCategory::Unknown,
        evidence: ClassificationEvidence::None,
        mime: None,
    }
}

/// 类别在扫描场景下的轻量替代：仅按扩展名（扫描不做 sniff，见 DECISIONS D20）。
pub fn classify_by_extension(extension: Option<&str>) -> Classification {
    classify(extension, None)
}

/// UTF 系编码对文本类别的佐证：BOM 命中即视为 Text（M1 §9 的事实性辅助）。
pub fn category_with_encoding_hint(base: Classification, encoding: TextEncoding) -> Classification {
    if base.category == FileCategory::Unknown
        && matches!(
            encoding,
            TextEncoding::Utf8Bom | TextEncoding::Utf16Le | TextEncoding::Utf16Be
        )
    {
        return Classification {
            category: FileCategory::Text,
            evidence: ClassificationEvidence::MagicBytes,
            mime: None,
        };
    }
    base
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extension_evidence_maps_known_types() {
        let c = classify(Some("JPG"), None);
        assert_eq!(c.category, FileCategory::Image);
        assert_eq!(c.evidence, ClassificationEvidence::Extension);
        assert_eq!(c.mime, Some(MIME_JPEG));

        let c = classify(Some("exe"), None);
        assert_eq!(c.category, FileCategory::Executable);
    }

    #[test]
    fn magic_bytes_override_missing_or_unknown_extension() {
        // 无扩展名但内容是 PNG。
        let png = [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        let c = classify(None, Some(&png));
        assert_eq!(c.category, FileCategory::Image);
        assert_eq!(c.evidence, ClassificationEvidence::MagicBytes);
        assert_eq!(c.mime, Some(MIME_PNG));

        // .bin 不在表中 → 不凭空猜 Executable（M1 §37 反例）。
        let mz = [b'M', b'Z', 0x90, 0x00];
        let c = classify(Some("bin"), Some(&mz));
        assert_eq!(c.category, FileCategory::Executable);
        assert_eq!(c.evidence, ClassificationEvidence::MagicBytes);
        let c = classify(Some("bin"), Some(&[0x00, 0x01, 0x02]));
        assert_eq!(c.category, FileCategory::Unknown);
        assert_eq!(c.evidence, ClassificationEvidence::None);
    }

    #[test]
    fn corrupt_extension_vs_real_content_reports_magic_evidence() {
        // foo.jpg 内容实际是 ZIP：扩展名命中 Image 在先——本函数记录的是
        // "声明 vs 检测"分离的证据链；调用方（inspector）会同时展示两者。
        let zip = [b'P', b'K', 0x03, 0x04, 0x00];
        let by_ext = classify(Some("jpg"), None);
        assert_eq!(by_ext.category, FileCategory::Image);
        let by_magic = classify(None, Some(&zip));
        assert_eq!(by_magic.category, FileCategory::Archive);
    }

    #[test]
    fn unknown_is_a_normal_state() {
        let c = classify(Some("weird-ext-xyz"), Some(&[1, 2, 3]));
        assert_eq!(c.category, FileCategory::Unknown);
        assert_eq!(c.evidence, ClassificationEvidence::None);
        assert_eq!(FileCategory::Unknown.as_str(), "unknown");
    }

    #[test]
    fn bom_encoding_hint_promotes_unknown_to_text() {
        let base = Classification {
            category: FileCategory::Unknown,
            evidence: ClassificationEvidence::None,
            mime: None,
        };
        let hinted = category_with_encoding_hint(base, TextEncoding::Utf16Le);
        assert_eq!(hinted.category, FileCategory::Text);
        assert_eq!(hinted.evidence, ClassificationEvidence::MagicBytes);
    }

    #[test]
    fn scan_helper_uses_extension_only() {
        let c = classify_by_extension(Some("CSV"));
        assert_eq!(c.category, FileCategory::Text);
        assert_eq!(c.evidence, ClassificationEvidence::Extension);
    }
}
