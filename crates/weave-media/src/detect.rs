//! 格式检测（M6 上 §9–§10）：magic bytes 优先，扩展名仅作 hint。
//! 检测结果必须标记 Detected / Unknown，不猜测。

/// 支持的图像格式（§0.2 核心六格式）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageFormat {
    Png,
    Jpeg,
    WebP,
    Gif,
    Bmp,
    Tiff,
}

impl ImageFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ImageFormat::Png => "png",
            ImageFormat::Jpeg => "jpeg",
            ImageFormat::WebP => "webp",
            ImageFormat::Gif => "gif",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Tiff => "tiff",
        }
    }

    pub fn default_extension(self) -> &'static str {
        self.as_str()
    }

    /// Magic bytes 检测（§9：Content Signature 优先于扩展名）。
    pub fn from_magic(bytes: &[u8]) -> Option<Self> {
        if bytes.starts_with(&[0x89, b'P', b'N', b'G']) {
            Some(ImageFormat::Png)
        } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
            Some(ImageFormat::Jpeg)
        } else if bytes.len() >= 12 && &bytes[0..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
            Some(ImageFormat::WebP)
        } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
            Some(ImageFormat::Gif)
        } else if bytes.starts_with(b"BM") {
            Some(ImageFormat::Bmp)
        } else if (bytes.starts_with(&[0x49, 0x49, 0x2A, 0x00]))
            || (bytes.starts_with(&[0x4D, 0x4D, 0x00, 0x2A]))
        {
            Some(ImageFormat::Tiff)
        } else {
            None
        }
    }

    pub fn from_extension(ext: &str) -> Option<Self> {
        match ext.to_ascii_lowercase().as_str() {
            "png" => Some(ImageFormat::Png),
            "jpg" | "jpeg" => Some(ImageFormat::Jpeg),
            "webp" => Some(ImageFormat::WebP),
            "gif" => Some(ImageFormat::Gif),
            "bmp" => Some(ImageFormat::Bmp),
            "tif" | "tiff" => Some(ImageFormat::Tiff),
            _ => None,
        }
    }
}

/// 扩展名与内容不一致的事实（§10：报告 Mismatch，不强制当扩展名格式）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionMismatch {
    pub extension: String,
    pub detected: ImageFormat,
}

/// Magic bytes 检测 + mismatch 事实收集。
pub fn detect_format(
    bytes: &[u8],
    path_extension: Option<&str>,
) -> (Option<ImageFormat>, Option<ExtensionMismatch>) {
    let detected = ImageFormat::from_magic(bytes);
    let mismatch = match (detected, path_extension) {
        (Some(fmt), Some(ext)) => {
            let ext_fmt = ImageFormat::from_extension(ext);
            if ext_fmt.is_some_and(|e| e != fmt) {
                Some(ExtensionMismatch {
                    extension: ext.to_string(),
                    detected: fmt,
                })
            } else {
                None
            }
        }
        _ => None,
    };
    (detected, mismatch)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn magic_bytes_detect_all_six_formats() {
        assert_eq!(
            ImageFormat::from_magic(&[0x89, b'P', b'N', b'G', 0x0D, 0x0A]),
            Some(ImageFormat::Png)
        );
        assert_eq!(
            ImageFormat::from_magic(&[0xFF, 0xD8, 0xFF, 0xE0]),
            Some(ImageFormat::Jpeg)
        );
        assert_eq!(
            ImageFormat::from_magic(b"RIFF\x00\x00\x00\x00WEBPVP8 "),
            Some(ImageFormat::WebP)
        );
        assert_eq!(
            ImageFormat::from_magic(b"GIF89a...."),
            Some(ImageFormat::Gif)
        );
        assert_eq!(
            ImageFormat::from_magic(b"BM\x00\x00"),
            Some(ImageFormat::Bmp)
        );
        assert_eq!(
            ImageFormat::from_magic(&[0x49, 0x49, 0x2A, 0x00, 0x08]),
            Some(ImageFormat::Tiff)
        );
        assert_eq!(ImageFormat::from_magic(b"not an image"), None);
    }

    #[test]
    fn extension_mismatch_reported_not_forced() {
        // §10：photo.jpg 内容是 PNG ⇒ 报告 Mismatch，不强制当 JPEG
        let (fmt, mismatch) = detect_format(&[0x89, b'P', b'N', b'G'], Some("jpg"));
        assert_eq!(fmt, Some(ImageFormat::Png));
        assert_eq!(
            mismatch,
            Some(ExtensionMismatch {
                extension: "jpg".into(),
                detected: ImageFormat::Png,
            })
        );
        // 一致时无 mismatch
        let (_, mismatch) = detect_format(&[0xFF, 0xD8, 0xFF], Some("jpg"));
        assert!(mismatch.is_none());
    }
}
