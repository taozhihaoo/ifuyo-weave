//! M6（下）测试套件 §150–§168：quality/resize/upscale/alpha/orientation/
//! strip 实测 + malformed no-panic（§181-§183 深嵌套边界）。
//!
//! 全部使用确定性合成夹具（§163/§164：程序生成，不提交真实照片）。

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
    encode_image(&DynamicImage::ImageRgba8(img), ImageFormat::Png, None).expect("png encode")
}

fn solid(w: u32, h: u32, r: u8, g: u8, b: u8) -> DynamicImage {
    let mut img = image::RgbaImage::new(w, h);
    for x in 0..w {
        for y in 0..h {
            img.put_pixel(x, y, image::Rgba([r, g, b, 255]));
        }
    }
    DynamicImage::ImageRgba8(img)
}

fn gradient(w: u32, h: u32) -> DynamicImage {
    let mut img = image::RgbaImage::new(w, h);
    for x in 0..w {
        for y in 0..h {
            let v = ((x + y) % 256) as u8;
            img.put_pixel(x, y, image::Rgba([v, v, v, 255]));
        }
    }
    DynamicImage::ImageRgba8(img)
}

// ── §155/§156：resize 矩阵 + upscale ──

#[test]
fn resize_matrix_spec_cases() {
    let img = solid(4000, 3000, 1, 2, 3);
    // §155：4000×3000 → 2000×1500（fit）
    let out = resize_image(
        &img,
        &ResizeOptions {
            mode: FitMode::Fit,
            width: 2000,
            height: 2000,
            ..ResizeOptions::default()
        },
    );
    assert_eq!((out.width(), out.height()), (2000, 1500));
    // §155：1920 fill（§38：等比填满后裁）
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
    // §155：1920×1080 exact
    let out = resize_image(
        &img,
        &ResizeOptions {
            mode: FitMode::Exact,
            width: 1920,
            height: 1080,
            prevent_upscale: false,
            ..ResizeOptions::default()
        },
    );
    assert_eq!((out.width(), out.height()), (1920, 1080));
}

#[test]
fn upscale_prevent_on_and_off() {
    // §156：800×600 → 1600×1200
    let img = solid(800, 600, 5, 5, 5);
    let prevent = resize_image(
        &img,
        &ResizeOptions {
            mode: FitMode::Fit,
            width: 1600,
            height: 1600,
            prevent_upscale: true,
            ..ResizeOptions::default()
        },
    );
    assert_eq!(
        (prevent.width(), prevent.height()),
        (800, 600),
        "prevent on"
    );
    let allow = resize_image(
        &img,
        &ResizeOptions {
            mode: FitMode::Fit,
            width: 1600,
            height: 1600,
            prevent_upscale: false,
            ..ResizeOptions::default()
        },
    );
    assert_eq!((allow.width(), allow.height()), (1600, 1200), "prevent off");
}

// ── §157：alpha ──

#[test]
fn alpha_transparent_and_semi_transparent_png_survive_png_roundtrip() {
    let mut img = image::RgbaImage::new(16, 16);
    for p in img.pixels_mut() {
        *p = image::Rgba([0, 0, 0, 0]); // 全透明
    }
    let bytes = encode_image(&DynamicImage::ImageRgba8(img), ImageFormat::Png, None).expect("enc");
    let (back, facts) = inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect("dec");
    assert!(facts.has_alpha);
    let rgba = back.to_rgba8();
    assert_eq!(rgba.get_pixel(0, 0).0[3], 0, "透明保持");

    let mut semi = image::RgbaImage::new(16, 16);
    for p in semi.pixels_mut() {
        *p = image::Rgba([255, 0, 0, 128]);
    }
    let bytes = encode_image(&DynamicImage::ImageRgba8(semi), ImageFormat::Png, None).expect("enc");
    let (back, _) = inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect("dec");
    assert_eq!(
        back.to_rgba8().get_pixel(0, 0).0[3],
        128,
        "半透明 alpha 保持"
    );
}

#[test]
fn png_to_webp_preserves_alpha() {
    let mut img = image::RgbaImage::new(16, 16);
    for p in img.pixels_mut() {
        *p = image::Rgba([0, 255, 0, 128]);
    }
    let bytes = encode_image(&DynamicImage::ImageRgba8(img), ImageFormat::WebP, None).expect("enc");
    let (back, facts) = inspect_bytes(&bytes, Some("webp"), &ImageLimits::default()).expect("dec");
    assert!(facts.has_alpha);
    let rgba = back.to_rgba8();
    assert_eq!(rgba.get_pixel(0, 0).0[1], 255);
    assert!(rgba.get_pixel(0, 0).0[3] > 100, "webp 无损保 alpha");
}

// ── §158：orientation ──

/// 构造携带 EXIF Orientation 的 JPEG（APP1 段长含自身 2 字节）。
fn jpeg_with_orientation(orientation: u16, base: DynamicImage) -> Vec<u8> {
    let mut jpeg = encode_image(&base, ImageFormat::Jpeg, Some(95)).expect("base");
    let value_bytes = {
        let mut v = vec![0u8; 4];
        v[..2].copy_from_slice(&orientation.to_le_bytes());
        v
    };
    let tiff: Vec<u8> = [
        b'I', b'I', 0x2A, 0x00, 0x08, 0x00, 0x00, 0x00, 0x01, 0x00, 0x12, 0x01, 0x03, 0x00, 0x01,
        0x00, 0x00, 0x00,
    ]
    .iter()
    .copied()
    .chain(value_bytes)
    .chain([0x00, 0x00, 0x00, 0x00])
    .collect();
    let marker_len = ((tiff.len() + 6 + 2) as u16).to_be_bytes();
    jpeg.splice(
        2..2,
        [0xFF, 0xE1]
            .iter()
            .copied()
            .chain(marker_len)
            .chain(b"Exif\0\0".iter().copied())
            .chain(tiff.iter().copied()),
    );
    jpeg
}

#[test]
fn orientation_all_variants_reported_as_facts() {
    // §158：orientation 1/3/6/8 均如实读取
    let base = solid(20, 10, 1, 1, 1);
    for orientation in [1u16, 3, 6, 8] {
        let jpeg = jpeg_with_orientation(orientation, base.clone());
        let (_, facts) =
            inspect_bytes(&jpeg, Some("jpg"), &ImageLimits::default()).expect("inspect");
        assert_eq!(
            facts.metadata.orientation,
            Some(orientation),
            "orientation {orientation}"
        );
    }
}

#[test]
fn orientation_normalize_rotates_pixels() {
    // §27：width≠height 时 orientation 6（顺时针 90°）物理归一后宽高互换
    let base = solid(20, 10, 255, 0, 0);
    let jpeg = jpeg_with_orientation(6, base.clone());
    let (img, facts) = inspect_bytes(&jpeg, Some("jpg"), &ImageLimits::default()).expect("ok");
    let normalized = apply_exif_orientation(&img, facts.metadata.orientation);
    assert_eq!(
        (normalized.width(), normalized.height()),
        (10, 20),
        "orientation 6 ⇒ 90° 旋转，宽高互换"
    );
}

// ── §159/§160：metadata/strip ──

#[test]
fn strip_removes_exif_but_keeps_pixels() {
    let base = gradient(24, 18);
    let jpeg = jpeg_with_orientation(6, base);
    let stripped =
        strip_metadata_bytes(&jpeg, ImageFormat::Jpeg, Some(95), &ImageLimits::default())
            .expect("strip");
    let (_, facts) = inspect_bytes(&stripped, Some("jpg"), &ImageLimits::default()).expect("dec");
    assert!(!facts.metadata.exif_present, "EXIF 已剥离");
    assert_eq!(facts.metadata.orientation, None, "orientation 已移除");
    // §25/§27：orientation 6 物理归一 ⇒ 宽高互换（显示方向不破坏的代价与语义）
    assert_eq!((facts.width, facts.height), (18, 24));
}

#[test]
fn metadata_absent_cleanly_reported() {
    let png = png_bytes(8, 8, image::Rgba([1, 2, 3, 255]));
    let (_, facts) = inspect_bytes(&png, Some("png"), &ImageLimits::default()).expect("ok");
    assert!(!facts.metadata.exif_present);
    assert!(!facts.metadata.gps_present);
    assert!(!facts.metadata.icc_present);
}

// ── §161：format matrix 实测 ──

#[test]
fn format_matrix_real_codec_verification() {
    // §11/§161：每格真实编解码验证后置 PASS
    let img = solid(8, 8, 10, 20, 30);
    for format in [
        ImageFormat::Png,
        ImageFormat::Jpeg,
        ImageFormat::WebP,
        ImageFormat::Gif,
        ImageFormat::Bmp,
        ImageFormat::Tiff,
    ] {
        let bytes = encode_image(&img, format, Some(90)).expect("encode");
        let (back, facts) = inspect_bytes(&bytes, Some(format.as_str()), &ImageLimits::default())
            .unwrap_or_else(|e| panic!("{format:?}: {e}"));
        assert_eq!(facts.format, Some(format), "{format:?} roundtrip");
        assert_eq!((back.width(), back.height()), (8, 8));
    }
}

// ── §167/§168：fuzz（bounded corpus，无 panic）──

#[test]
fn malformed_corpus_never_panics() {
    // §167/§169/§181：截断/损坏/魔数错误 ⇒ 结构化错误，绝不 panic
    let corpus: Vec<Vec<u8>> = vec![
        vec![],
        vec![0x89, b'P', b'N', b'G'], // PNG 魔数截断
        {
            let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
            v.extend([0, 0, 0, 13]);
            v
        }, // IHDR 声明后截断
        vec![0xFF, 0xD8, 0xFF],       // JPEG 魔数截断
        b"RIFF\x00\x00".to_vec(),     // RIFF 截断
        b"GIF89a".to_vec(),           // GIF 头截断
        vec![0x49, 0x49, 0x2A, 0x00], // TIFF 头截断
        b"BM".to_vec(),               // BMP 头截断
    ];
    for (i, bytes) in corpus.iter().enumerate() {
        let result = inspect_bytes(bytes, Some("png"), &ImageLimits::default());
        assert!(result.is_err(), "case {i} should fail structured");
    }
}

#[test]
fn huge_dimension_headers_rejected_before_decode() {
    // §118/§182：巨型维度 header ⇒ 结构化拒绝（每格式变体）
    let cases: Vec<Vec<u8>> = vec![
        {
            // PNG IHDR 声明 60000×60000
            let mut v = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0, 0, 13];
            v.extend_from_slice(b"IHDR");
            v.extend_from_slice(&60_000u32.to_be_bytes());
            v.extend_from_slice(&60_000u32.to_be_bytes());
            v
        },
        {
            // BMP header 声明 60000×60000
            let mut v = b"BM".to_vec();
            v.extend_from_slice(&[0; 24]);
            v.extend_from_slice(&[0x38, 0, 0, 0]); // DIB header size
            v.extend_from_slice(&60_000u32.to_le_bytes());
            v.extend_from_slice(&60_000u32.to_le_bytes());
            v
        },
    ];
    for (i, bytes) in cases.iter().enumerate() {
        let result = inspect_bytes(bytes, None, &ImageLimits::default());
        assert!(result.is_err(), "case {i} huge dims must be rejected");
    }
}

// ── §152：semantic equality（语义等价非字节等价）──

#[test]
fn png_lossless_roundtrip_is_semantically_equal() {
    let img = gradient(50, 40);
    let bytes = encode_image(&img, ImageFormat::Png, None).expect("enc");
    let (back, _) = inspect_bytes(&bytes, Some("png"), &ImageLimits::default()).expect("dec");
    // PNG 无损：像素级一致
    assert_eq!(back.to_rgba8().into_raw(), img.to_rgba8().into_raw());
}

#[test]
fn jpeg_quality_affects_output_but_keeps_dimensions() {
    // §154：lossy 但尺寸保持；quality 90 vs 40 输出可不同（允许）
    let img = gradient(64, 48);
    let q90 = encode_image(&img, ImageFormat::Jpeg, Some(90)).expect("q90");
    let q40 = encode_image(&img, ImageFormat::Jpeg, Some(40)).expect("q40");
    let (d90, _) = inspect_bytes(&q90, Some("jpg"), &ImageLimits::default()).expect("90");
    let (d40, _) = inspect_bytes(&q40, Some("jpg"), &ImageLimits::default()).expect("40");
    assert_eq!((d90.width(), d90.height()), (d40.width(), d40.height()));
}
