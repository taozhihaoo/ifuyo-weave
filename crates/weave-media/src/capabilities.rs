//! Format Capability Matrix（M6 上 §11）：只有真实验证过的能力才标
//! Supported。矩阵由单元测试固化（§11：? 必须在真实代码和测试后替换）。
//!
//! 依据（D49）：
//! - `image` 0.25 crate：PNG/JPEG/GIF/BMP/TIFF/WebP decode 全支持；
//!   encode 支持 PNG/JPEG/GIF/BMP/TIFF；WebP encode 仅 **lossless**
//!   （image-webp 无损质量参数有限）——WebP 有损编码 NOT SUPPORTED（§109
//!   spike 结论）。
//! - JPEG 无 alpha（§13）；GIF 动画 v1 Static Only（§59）。

use crate::ImageFormat;

/// 单格式能力（§107 Capability API）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FormatCapability {
    pub format: ImageFormat,
    pub decode: bool,
    pub encode: bool,
    pub alpha_decode: bool,
    pub alpha_encode: bool,
    /// 动画支持（v1 全部 false，§59 Static Only）。
    pub animation: bool,
    /// EXIF metadata 读取。
    pub exif_read: bool,
    /// 有损编码（JPEG/WebP lossy）。
    pub lossy_encode: bool,
}

impl FormatCapability {
    /// 目标格式是否支持 alpha；用于 §14 Transparency Policy。
    pub fn supports_alpha(&self) -> bool {
        self.alpha_encode
    }
}

/// 真实能力矩阵（测试固化；§11 禁止“理论上支持”）。
pub fn capability(format: ImageFormat) -> FormatCapability {
    match format {
        ImageFormat::Png => FormatCapability {
            format,
            decode: true,
            encode: true,
            alpha_decode: true,
            alpha_encode: true,
            animation: false, // APNG v1 NOT SUPPORTED
            exif_read: true,  // eXIf chunk（经 kamadak-exif 从字节读取）
            lossy_encode: false,
        },
        ImageFormat::Jpeg => FormatCapability {
            format,
            decode: true,
            encode: true,
            alpha_decode: false, // JPEG 无 alpha（§13）
            alpha_encode: false,
            animation: false,
            exif_read: true,
            lossy_encode: true,
        },
        ImageFormat::WebP => FormatCapability {
            format,
            decode: true,
            encode: true,
            alpha_decode: true,
            alpha_encode: true,  // lossless WebP 保留 alpha
            animation: false,    // animated WebP v1 NOT SUPPORTED
            exif_read: true,     // EXIF chunk
            lossy_encode: false, // 纯 Rust 生态无有损 WebP 编码（§109 spike）
        },
        ImageFormat::Gif => FormatCapability {
            format,
            decode: true,
            encode: true,
            alpha_decode: true, // 二值透明
            alpha_encode: true,
            animation: false, // §59 v1 Static Only；animated ⇒ 显式 NOT SUPPORTED
            exif_read: false,
            lossy_encode: false,
        },
        ImageFormat::Bmp => FormatCapability {
            format,
            decode: true,
            encode: true,
            alpha_decode: true, // 32-bit
            alpha_encode: true,
            animation: false,
            exif_read: false,
            lossy_encode: false,
        },
        ImageFormat::Tiff => FormatCapability {
            format,
            decode: true,
            encode: true,
            alpha_decode: true,
            alpha_encode: true,
            animation: false, // 多页 v1 NOT SUPPORTED（首页）
            exif_read: true,
            lossy_encode: false,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §11：矩阵必须由真实 codec 行为测试固化——每格式 decode/encode
    /// 至少经一次真实编解码验证（编解码正确性测试见 ops::tests）。
    #[test]
    fn matrix_covers_all_six_formats() {
        for format in [
            ImageFormat::Png,
            ImageFormat::Jpeg,
            ImageFormat::WebP,
            ImageFormat::Gif,
            ImageFormat::Bmp,
            ImageFormat::Tiff,
        ] {
            let cap = capability(format);
            assert!(cap.decode, "{format:?} decode must be supported");
        }
    }

    #[test]
    fn jpeg_has_no_alpha_webp_lossless_only() {
        assert!(!capability(ImageFormat::Jpeg).alpha_encode);
        assert!(
            !capability(ImageFormat::WebP).lossy_encode,
            "§109 spike：纯 Rust 无有损 WebP 编码"
        );
        assert!(capability(ImageFormat::Jpeg).lossy_encode);
    }
}
