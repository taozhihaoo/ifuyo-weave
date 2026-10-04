//! 解码（M6 上 §8 Decode Policy）：header 尺寸守卫 → 守卫内解码 →
//! 事实提取。EXIF 读取（§22）只报告真实解析成功的字段（§98 Fact only）。

use image::DynamicImage;

use crate::ImageFormat;
use crate::capabilities::capability;
use crate::detect::ExtensionMismatch;
use crate::limits::ImageLimits;

/// 图像事实（§4 ImageDocument / §33 Inspector 的域内来源；§98 Fact only）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ImageFacts {
    pub format: Option<ImageFormat>,
    pub width: u32,
    pub height: u32,
    pub has_alpha: bool,
    pub color_model: String,
    pub bit_depth: u16,
    /// 帧数 >1 ⇒ 动画（§59：v1 Static Only，转换显式 NOT SUPPORTED）。
    pub frame_count: u32,
    pub extension_mismatch: Option<ExtensionMismatch>,
    /// EXIF 摘要（§22 只报真实解析成功字段；§23 GPS 隐私显式标注）。
    pub metadata: MetadataFacts,
}

/// EXIF / 元数据摘要（§20 Metadata 模型的读取侧）。
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct MetadataFacts {
    pub exif_present: bool,
    pub camera_make: Option<String>,
    pub camera_model: Option<String>,
    pub datetime: Option<String>,
    pub orientation: Option<u16>,
    /// §23 GPS Privacy：存在即显式标注（值不入日志）。
    pub gps_present: bool,
    pub icc_present: bool,
}

/// 格式 → image crate 解码格式枚举。
fn image_format_to_codec(format: ImageFormat) -> image::ImageFormat {
    match format {
        ImageFormat::Png => image::ImageFormat::Png,
        ImageFormat::Jpeg => image::ImageFormat::Jpeg,
        ImageFormat::WebP => image::ImageFormat::WebP,
        ImageFormat::Gif => image::ImageFormat::Gif,
        ImageFormat::Bmp => image::ImageFormat::Bmp,
        ImageFormat::Tiff => image::ImageFormat::Tiff,
    }
}

/// 读取 EXIF 摘要（kamadak-exif；§22 只报真实解析成功字段；§117 巨大
/// metadata 由读取器自身限制 + max_metadata_bytes 守卫）。
fn read_exif(bytes: &[u8], format: crate::ImageFormat) -> MetadataFacts {
    let mut facts = MetadataFacts::default();
    let reader = match format {
        crate::ImageFormat::Jpeg => exif::Reader::new()
            .read_from_container(&mut std::io::Cursor::new(bytes))
            .ok(),
        crate::ImageFormat::Tiff => exif::Reader::new()
            .read_from_container(&mut std::io::Cursor::new(bytes))
            .ok(),
        _ => None,
    };
    let Some(exif) = reader else {
        return facts;
    };
    facts.exif_present = exif.fields().count() > 0;
    let get = |tag: &exif::Tag| -> Option<String> {
        exif.get_field(*tag, exif::In::PRIMARY)
            .map(|f| f.display_value().to_string())
    };
    facts.camera_make = get(&exif::Tag::Make);
    facts.camera_model = get(&exif::Tag::Model);
    facts.datetime = get(&exif::Tag::DateTimeOriginal).or_else(|| get(&exif::Tag::DateTime));
    if let Some(field) = exif.get_field(exif::Tag::Orientation, exif::In::PRIMARY)
        && let Some(v) = field.value.get_uint(0)
    {
        facts.orientation = Some(v as u16);
    }
    // §23 GPS Privacy：GPSVersionID / GPSLatitude 任一存在即标注 presence
    // §23 GPS presence：GPSInfoIFDPointer 存在即 GPS 子目录存在（Fact）
    facts.gps_present = exif
        .get_field(exif::Tag::GPSInfoIFDPointer, exif::In::PRIMARY)
        .is_some();
    facts
}

/// 打开并解码（§8 Decode Policy：header 尺寸守卫先于全量解码）。
/// mismatch 事实随结果返回（§10）。动画帧数 >1 记录为事实（§59 转换侧拒绝）。
pub fn inspect_bytes(
    bytes: &[u8],
    path_extension: Option<&str>,
    limits: &ImageLimits,
) -> Result<(DynamicImage, ImageFacts), String> {
    let (Some(format), mismatch) = crate::detect_format(bytes, path_extension) else {
        return Err("unsupported or unrecognized image format".to_string());
    };

    // §8/§119：header 尺寸守卫先于像素缓冲分配
    let reader = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("io reader failed: {e}"))?;
    let (width, height) = reader
        .into_dimensions()
        .map_err(|e| format!("failed to read image header: {e}"))?;
    limits
        .check_dimensions(width, height)
        .map_err(|e| format!("DecodeRejectedByResourceLimit: {e}"))?;

    let codec = image_format_to_codec(format);
    let decoder = image::ImageReader::new(std::io::Cursor::new(bytes))
        .with_guessed_format()
        .map_err(|e| format!("io reader failed: {e}"))?
        .decode()
        .map_err(|e| format!("image decode failed: {e}"))?;

    // §59：帧数事实（GIF 帧 iterator；WebP has_animation；其余单帧）
    let frame_count = match format {
        crate::ImageFormat::Gif => {
            use image::AnimationDecoder;
            image::codecs::gif::GifDecoder::new(std::io::Cursor::new(bytes))
                .map(|d| d.into_frames().count() as u64)
                .unwrap_or(1)
        }
        crate::ImageFormat::WebP => {
            image::codecs::webp::WebPDecoder::new(std::io::Cursor::new(bytes))
                .map(|d| u64::from(u8::from(d.has_animation())) + 1)
                .unwrap_or(1)
        }
        _ => 1,
    };
    let _ = codec;

    let has_alpha = decoder.color().has_alpha();
    let facts = ImageFacts {
        format: Some(format),
        width: decoder.width(),
        height: decoder.height(),
        has_alpha,
        color_model: format!("{:?}", decoder.color()).to_string(),
        bit_depth: decoder.color().bits_per_pixel(),
        frame_count: frame_count as u32,
        extension_mismatch: mismatch,
        metadata: read_exif(bytes, format),
    };
    let _ = capability(format);
    Ok((decoder, facts))
}
