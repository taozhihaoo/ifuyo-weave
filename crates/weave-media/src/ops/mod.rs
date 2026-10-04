//! Resize / Convert / Compress / Metadata 操作（M6 上 §35–§62）。
//!
//! 全部纯逻辑（输入像素缓冲 + 选项 ⇒ 输出缓冲/编码字节）；文件写入在
//! 应用层（§81）。Resize 数学确定性（§40 rounding = round-half-up via
//! `f64.round()`），Fit 不拉伸（§37）、Fill 裁切不变形（§38）。

use crate::ImageFormat;
use crate::ImageLimits;
use crate::capabilities::capability;

/// Resize 模式（§36–§40）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FitMode {
    /// 等比适配目标框（§37：4000×3000 → 1920 框 ⇒ 1920×1440）。
    Fit,
    /// 等比填满后居中裁切（§38：不变形）。
    Fill,
    /// 精确尺寸（§39：比例会变，UI 必须提示）。
    Exact,
    /// 百分比缩放（§40）。
    Scale,
}

/// Alpha → 无 alpha 目标的合成策略（§14/§15：必须真实合成防 halo）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AlphaPolicy {
    White,
    Black,
}

impl AlphaPolicy {
    fn background_rgb(&self) -> [u8; 3] {
        match self {
            AlphaPolicy::White => [255, 255, 255],
            AlphaPolicy::Black => [0, 0, 0],
        }
    }
}

/// Resize 选项（§132）。
#[derive(Debug, Clone, PartialEq)]
pub struct ResizeOptions {
    pub mode: FitMode,
    pub width: u32,
    pub height: u32,
    /// §40 Scale %（mode = Scale 时用；1–1000）。
    pub scale_percent: u32,
    /// §41 防放大（默认 ON）。
    pub prevent_upscale: bool,
    /// 重采样过滤器（§42：默认高质量 Lanczos3）。
    pub filter: ResizeFilter,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResizeFilter {
    Nearest,
    Triangle,
    CatmullRom,
    Lanczos3,
}

impl ResizeFilter {
    fn to_image(self) -> image::imageops::FilterType {
        match self {
            ResizeFilter::Nearest => image::imageops::FilterType::Nearest,
            ResizeFilter::Triangle => image::imageops::FilterType::Triangle,
            ResizeFilter::CatmullRom => image::imageops::FilterType::CatmullRom,
            ResizeFilter::Lanczos3 => image::imageops::FilterType::Lanczos3,
        }
    }
}

impl Default for ResizeOptions {
    fn default() -> Self {
        Self {
            mode: FitMode::Fit,
            width: 1920,
            height: 1920,
            scale_percent: 100,
            prevent_upscale: true,
            filter: ResizeFilter::Lanczos3,
        }
    }
}

/// §40 rounding：四舍五入（round half up，至少 1px）。
fn scaled_dimension(value: u32, percent: u32) -> u32 {
    ((u64::from(value) * u64::from(percent) + 50) / 100).max(1) as u32
}

/// 计算目标尺寸（§35–§41；纯函数，确定性）。
pub fn resize_dimensions(src_w: u32, src_h: u32, options: &ResizeOptions) -> (u32, u32) {
    let (mut w, mut h) = match options.mode {
        FitMode::Scale => (
            scaled_dimension(src_w, options.scale_percent),
            scaled_dimension(src_h, options.scale_percent),
        ),
        FitMode::Exact => (options.width.max(1), options.height.max(1)),
        FitMode::Fit => {
            let (tw, th) = (options.width.max(1), options.height.max(1));
            let src_ratio = f64::from(src_w) / f64::from(src_h);
            let box_ratio = f64::from(tw) / f64::from(th);
            if src_ratio > box_ratio {
                (tw, ((f64::from(tw) / src_ratio).round() as u32).max(1))
            } else {
                (((f64::from(th) * src_ratio).round() as u32).max(1), th)
            }
        }
        FitMode::Fill => {
            let (tw, th) = (options.width.max(1), options.height.max(1));
            let src_ratio = f64::from(src_w) / f64::from(src_h);
            let box_ratio = f64::from(tw) / f64::from(th);
            if src_ratio > box_ratio {
                (((f64::from(th) * src_ratio).round() as u32).max(1), th)
            } else {
                (tw, ((f64::from(tw) / src_ratio).round() as u32).max(1))
            }
        }
    };
    if options.prevent_upscale && (w > src_w || h > src_h) {
        w = src_w;
        h = src_h;
    }
    (w.max(1), h.max(1))
}

/// 执行 resize（§42 重采样 + §43 alpha 通道随 RGBA 保留）。
pub fn resize_image(image: &DynamicImage, options: &ResizeOptions) -> DynamicImage {
    let (w, h) = resize_dimensions(image.width(), image.height(), options);
    if (w, h) == (image.width(), image.height()) {
        return image.clone();
    }
    let filter = options.filter.to_image();
    image.resize_exact(w, h, filter)
}

/// §14/§15：RGBA + 背景 → RGB 合成（半透明边缘防 halo——先展开 alpha
/// 后丢弃通道，不做简单 drop）。
pub fn composite_on_background(image: &DynamicImage, policy: AlphaPolicy) -> DynamicImage {
    let rgba = image.to_rgba8();
    let bg = policy.background_rgb();
    let mut out = image::RgbaImage::new(rgba.width(), rgba.height());
    for (x, y, px) in rgba.enumerate_pixels() {
        let [r, g, b, a] = px.0;
        let alpha = u16::from(a);
        let inv = 255 - alpha;
        let composite =
            |c: u8, bgc: u8| -> u8 { ((u16::from(c) * alpha + u16::from(bgc) * inv) / 255) as u8 };
        out.put_pixel(
            x,
            y,
            image::Rgba([
                composite(r, bg[0]),
                composite(g, bg[1]),
                composite(b, bg[2]),
                255,
            ]),
        );
    }
    DynamicImage::ImageRgba8(out)
}

/// 编码（§45–§51）：quality 语义按格式分档（JPEG 0–100 有损；WebP 仅
/// 无损——§109 spike；PNG/GIF/BMP/TIFF 无损无 quality）。
pub fn encode_image(
    image: &DynamicImage,
    format: ImageFormat,
    quality: Option<u8>,
) -> Result<Vec<u8>, String> {
    let cap = capability(format);
    let mut out = Vec::new();
    let mut cursor = std::io::Cursor::new(&mut out);
    match format {
        ImageFormat::Jpeg => {
            // §14/§15：JPEG 无 alpha ⇒ 先合成背景（White 默认）
            let flattened = if image.color().has_alpha() {
                composite_on_background(image, AlphaPolicy::White)
            } else {
                image.clone()
            };
            let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(
                &mut cursor,
                quality.unwrap_or(85).clamp(1, 100),
            );
            encoder
                .encode_image(&flattened)
                .map_err(|e| format!("jpeg encode failed: {e}"))?;
        }
        ImageFormat::WebP => {
            // §109 spike：image 0.25 WebP 编码 = lossless（无损）
            image::DynamicImage::write_to(image, &mut cursor, image::ImageFormat::WebP)
                .map_err(|e| format!("webp encode failed: {e}"))?;
        }
        _ => {
            let codec = match format {
                ImageFormat::Png => image::ImageFormat::Png,
                ImageFormat::Gif => image::ImageFormat::Gif,
                ImageFormat::Bmp => image::ImageFormat::Bmp,
                ImageFormat::Tiff => image::ImageFormat::Tiff,
                ImageFormat::Jpeg | ImageFormat::WebP => unreachable!("handled above"),
            };
            image::DynamicImage::write_to(image, &mut cursor, codec)
                .map_err(|e| format!("{:?} encode failed: {e}", format))?;
        }
    }
    let _ = cap;
    Ok(out)
}

use image::DynamicImage;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ImageLimits;
    use crate::inspect::inspect_bytes;

    fn png_bytes(w: u32, h: u32, rgba: image::Rgba<u8>) -> Vec<u8> {
        let mut img = image::RgbaImage::new(w, h);
        for x in 0..w {
            for y in 0..h {
                img.put_pixel(x, y, rgba);
            }
        }
        let dyn_img = DynamicImage::ImageRgba8(img);
        encode_image(&dyn_img, ImageFormat::Png, None).expect("png encode")
    }

    #[test]
    fn fit_mode_preserves_aspect_ratio() {
        // §37：4000×3000 → Fit 1920×1920 ⇒ 1920×1440（不得 1920×1920）
        let (w, h) = resize_dimensions(
            4000,
            3000,
            &ResizeOptions {
                mode: FitMode::Fit,
                width: 1920,
                height: 1920,
                ..ResizeOptions::default()
            },
        );
        assert_eq!((w, h), (1920, 1440));
    }

    #[test]
    fn fill_mode_crops_not_distorts() {
        // §38：4000×3000 → Fill 1920×1920 ⇒ 中间产物 2560×1920（再裁切）
        let (w, h) = resize_dimensions(
            4000,
            3000,
            &ResizeOptions {
                mode: FitMode::Fill,
                width: 1920,
                height: 1920,
                ..ResizeOptions::default()
            },
        );
        assert_eq!((w, h), (2560, 1920));
    }

    #[test]
    fn exact_mode_changes_ratio_and_scale_rounds() {
        // §39/§40：Exact 强制目标；Scale 50% round half up
        let (w, h) = resize_dimensions(
            4000,
            3000,
            &ResizeOptions {
                mode: FitMode::Exact,
                width: 1920,
                height: 1080,
                ..ResizeOptions::default()
            },
        );
        assert_eq!((w, h), (1920, 1080));
        let (w, h) = resize_dimensions(
            4000,
            3000,
            &ResizeOptions {
                mode: FitMode::Scale,
                scale_percent: 50,
                ..ResizeOptions::default()
            },
        );
        assert_eq!((w, h), (2000, 1500));
    }

    #[test]
    fn prevent_upscale_keeps_small_images_small() {
        // §41：fit to 4000px 不得把 800px 图放大
        let (w, h) = resize_dimensions(
            800,
            600,
            &ResizeOptions {
                mode: FitMode::Fit,
                width: 4000,
                height: 4000,
                prevent_upscale: true,
                ..ResizeOptions::default()
            },
        );
        assert_eq!((w, h), (800, 600));
    }

    #[test]
    fn decode_rejects_dimension_bombs() {
        // §7/§119：header 声明超大尺寸 ⇒ DecodeRejectedByResourceLimit
        let limits = ImageLimits::default();
        let err = limits.check_dimensions(30_000, 30_000).expect_err("bomb");
        assert!(err.contains("budget") || err.contains("dimension"));
    }

    #[test]
    fn decode_guard_via_inspect_on_crafted_header() {
        // 构造大尺寸 PNG header（IHDR 声明 20000×20000，数据截断）
        // ⇒ header 守卫在解码前拒绝
        let mut bytes = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
        bytes.extend_from_slice(&20_000u32.to_be_bytes());
        bytes.extend_from_slice(&20_000u32.to_be_bytes());
        let err =
            inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect_err("bomb rejected");
        assert!(err.contains("ResourceLimit") || err.contains("header"));
    }

    #[test]
    fn png_decode_inspect_facts() {
        let bytes = png_bytes(64, 32, image::Rgba([255, 0, 0, 255]));
        let (img, facts) = inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect("ok");
        assert_eq!((img.width(), img.height()), (64, 32));
        assert_eq!(facts.format, Some(ImageFormat::Png));
        assert!(facts.has_alpha);
    }

    #[test]
    fn alpha_composite_onto_white_for_jpeg() {
        // §14/§15：半透明红色 → 白底合成 = 浅红（非黑、非透明丢弃）
        let semi_red = image::Rgba([255, 0, 0, 128]);
        let bytes = png_bytes(8, 8, semi_red);
        let (img, _) = inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect("ok");
        let flattened = composite_on_background(&img, AlphaPolicy::White);
        let rgba_buf = flattened.to_rgba8();
        let px = rgba_buf.get_pixel(0, 0);
        assert_eq!(px[3], 255, "输出无 alpha");
        // 50% 红合成白底 ≈ 255,128,128（防 halo：按 alpha 加权而非 drop）
        assert!(px[0] >= 250 && px[1].abs_diff(128) <= 2 && px[2].abs_diff(128) <= 2);
    }

    #[test]
    fn jpeg_encode_strips_alpha_and_produces_jpeg() {
        let bytes = png_bytes(16, 16, image::Rgba([0, 200, 0, 255]));
        let (img, _) = inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect("ok");
        let jpeg = encode_image(&img, ImageFormat::Jpeg, Some(85)).expect("encode");
        assert_eq!(&jpeg[0..3], &[0xFF, 0xD8, 0xFF]);
        // roundtrip：能重新解码
        let (_, facts) =
            inspect_bytes(&jpeg, Some("jpg"), &ImageLimits::default()).expect("re-decode");
        assert_eq!(facts.format, Some(ImageFormat::Jpeg));
    }

    #[test]
    fn webp_lossless_roundtrip() {
        let bytes = png_bytes(16, 16, image::Rgba([0, 100, 200, 255]));
        let (img, _) = inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect("ok");
        let webp = encode_image(&img, ImageFormat::WebP, None).expect("encode");
        let (back, facts) =
            inspect_bytes(&webp, Some("webp"), &ImageLimits::default()).expect("decode");
        assert_eq!(facts.format, Some(ImageFormat::WebP));
        assert_eq!((back.width(), back.height()), (16, 16));
    }
}

/// §26/§27：EXIF Orientation → 像素物理归一（重编码前应用），
/// 导出后 orientation metadata 按策略移除/写 1。
pub fn apply_exif_orientation(image: &DynamicImage, orientation: Option<u16>) -> DynamicImage {
    match orientation {
        Some(2) => image.flipv(),
        Some(3) => image.rotate180(),
        Some(4) => image.fliph(),
        Some(5) => image.rotate270().fliph(),
        Some(6) => image.rotate90(),
        Some(7) => image.rotate90().fliph(),
        Some(8) => image.rotate270(),
        _ => image.clone(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MetadataStripMode {
    /// 全部可移除元数据剥离（重编码即剥离 EXIF/ICC/XMP）。
    #[default]
    StripAll,
    Keep,
}

/// §61：strip = 重编码且不写入 EXIF/ICC chunk——"Remove removable metadata
/// supported by the selected format and encoder"（不宣称绝对删除一切）。
pub fn strip_metadata_bytes(
    bytes: &[u8],
    target: ImageFormat,
    quality: Option<u8>,
    limits: &ImageLimits,
) -> Result<Vec<u8>, String> {
    let (img, facts) = crate::inspect::inspect_bytes(bytes, None, limits)?;
    // §25：strip 后显示方向不得破坏——物理归一像素，输出不再携带 orientation
    let normalized = apply_exif_orientation(&img, facts.metadata.orientation);
    encode_image(&normalized, target, quality)
}

/// EXIF GPS presence（§23，公开给应用层检查后决定 strip）。
pub fn exif_gps_present(bytes: &[u8], format: ImageFormat) -> bool {
    let mut cursor = std::io::Cursor::new(bytes);
    let exif = match format {
        ImageFormat::Jpeg | ImageFormat::Tiff => {
            exif::Reader::new().read_from_container(&mut cursor).ok()
        }
        _ => None,
    };
    let Some(exif) = exif else { return false };
    exif.get_field(exif::Tag::GPSInfoIFDPointer, exif::In::PRIMARY)
        .is_some()
}

#[cfg(test)]
mod ops_tests;
