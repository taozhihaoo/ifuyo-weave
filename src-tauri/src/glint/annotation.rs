//! M2.8 Glint 标注（§11-§14）：非破坏 Annotation Layer。
//!
//! - `Original Capture + AnnotationDocument = Rendered Output`（§12）：
//!   渲染是纯函数，原图字节永不修改；保存标注图 ⇒ 写新文件。
//! - 形状绘制/模糊/像素化纯 Rust 实现（零新增依赖）；Text 经 ab_glyph
//!   读取系统字体（微软雅黑 → 宋体 → Arial；全部缺失 ⇒ 结构化
//!   `glint.fontUnavailable`，不伪造）。
//! - Blur/Pixelate（§14）：渲染结果才是输出；不宣称"绝对脱敏"。

use std::path::PathBuf;
use std::sync::OnceLock;

use ab_glyph::{Font as _, ScaleFont as _};

use weave_core::prelude::WeaveError;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("glint::annotation")
}

/// 标注样式（§12：只实现真正需要的字段）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct AnnotationStyle {
    pub stroke_width: f64,
    /// RGBA。
    pub color: [u8; 4],
    pub font_size: f64,
    pub filled: bool,
}

impl Default for AnnotationStyle {
    fn default() -> Self {
        Self {
            stroke_width: 2.0,
            color: [255, 64, 64, 255],
            font_size: 24.0,
            filled: false,
        }
    }
}

/// 标注对象（坐标均为图像像素空间；越界部分裁掉，不 panic）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AnnotationShape {
    Arrow {
        x1: f64,
        y1: f64,
        x2: f64,
        y2: f64,
    },
    Rectangle {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    Ellipse {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    Freehand {
        points: Vec<[f64; 2]>,
    },
    Text {
        x: f64,
        y: f64,
        text: String,
    },
    Blur {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    },
    Pixelate {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
        block: f64,
    },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct Annotation {
    pub id: String,
    pub shape: AnnotationShape,
    pub style: AnnotationStyle,
}

/// 渲染标注到 RGBA 副本（原图不受影响——非破坏 §12）。
pub fn render(
    base_rgba: &[u8],
    width: u32,
    height: u32,
    annotations: &[Annotation],
) -> Result<Vec<u8>, WeaveError> {
    let expected = width as usize * height as usize * 4;
    if base_rgba.len() != expected {
        return Err(err(
            "glint.invalidRegion",
            format!("rgba size {} != {width}x{height}x4", base_rgba.len()),
        ));
    }
    let mut out = base_rgba.to_vec();
    for ann in annotations {
        render_one(&mut out, width, height, ann)?;
    }
    Ok(out)
}

fn render_one(buf: &mut [u8], width: u32, height: u32, ann: &Annotation) -> Result<(), WeaveError> {
    let style = &ann.style;
    match &ann.shape {
        AnnotationShape::Arrow { x1, y1, x2, y2 } => {
            draw_line(buf, width, height, *x1, *y1, *x2, *y2, style);
            // 箭头头部：两翼线段
            let (dx, dy) = (x2 - x1, y2 - y1);
            let len = (dx * dx + dy * dy).sqrt();
            if len > 1.0 {
                let head = (len * 0.2).clamp(6.0, 32.0);
                let (ux, uy) = (dx / len, dy / len);
                let spread = 0.5f64; // ~30°
                for sign in [-1.0, 1.0] {
                    let bx = -ux * head * spread.cos() - uy * sign * head * spread.sin();
                    let by = -uy * head * spread.cos() + ux * sign * head * spread.sin();
                    draw_line(buf, width, height, *x2, *y2, x2 + bx, y2 + by, style);
                }
            }
        }
        AnnotationShape::Rectangle { x, y, w, h } => {
            let (x, y, w, h) = norm_rect(*x, *y, *w, *h);
            if style.filled {
                for py in y as u32..(y + h).ceil() as u32 {
                    for px in x as u32..(x + w).ceil() as u32 {
                        blend_px(buf, width, height, px as i64, py as i64, &style.color);
                    }
                }
            } else {
                draw_line(buf, width, height, x, y, x + w, y, style);
                draw_line(buf, width, height, x + w, y, x + w, y + h, style);
                draw_line(buf, width, height, x + w, y + h, x, y + h, style);
                draw_line(buf, width, height, x, y + h, x, y, style);
            }
        }
        AnnotationShape::Ellipse { x, y, w, h } => {
            let (x, y, w, h) = norm_rect(*x, *y, *w, *h);
            let (cx, cy) = (x + w / 2.0, y + h / 2.0);
            let (rx, ry) = (w / 2.0, h / 2.0);
            if rx <= 0.0 || ry <= 0.0 {
                return Ok(());
            }
            let steps = ((rx + ry) * 4.0).max(32.0) as usize;
            if style.filled {
                for py in y as u32..(y + h).ceil() as u32 {
                    for px in x as u32..(x + w).ceil() as u32 {
                        let fx = px as f64 + 0.5 - cx;
                        let fy = py as f64 + 0.5 - cy;
                        if (fx * fx) / (rx * rx) + (fy * fy) / (ry * ry) <= 1.0 {
                            blend_px(buf, width, height, px as i64, py as i64, &style.color);
                        }
                    }
                }
            } else {
                for i in 0..steps {
                    let t0 = (i as f64) / steps as f64 * std::f64::consts::TAU;
                    let t1 = ((i + 1) as f64) / steps as f64 * std::f64::consts::TAU;
                    draw_line(
                        buf,
                        width,
                        height,
                        cx + rx * t0.cos(),
                        cy + ry * t0.sin(),
                        cx + rx * t1.cos(),
                        cy + ry * t1.sin(),
                        style,
                    );
                }
            }
        }
        AnnotationShape::Freehand { points } => {
            for pair in points.windows(2) {
                draw_line(
                    buf, width, height, pair[0][0], pair[0][1], pair[1][0], pair[1][1], style,
                );
            }
        }
        AnnotationShape::Text { x, y, text } => {
            draw_text(buf, width, height, *x, *y, text, style)?;
        }
        AnnotationShape::Blur { x, y, w, h } => {
            let (x, y, w, h) = norm_rect(*x, *y, *w, *h);
            box_blur_region(buf, width, height, x, y, w, h, 3);
        }
        AnnotationShape::Pixelate { x, y, w, h, block } => {
            let (x, y, w, h) = norm_rect(*x, *y, *w, *h);
            let block = (*block).max(2.0) as u32;
            pixelate_region(
                buf, width, height, x as u32, y as u32, w as u32, h as u32, block,
            );
        }
    }
    Ok(())
}

fn norm_rect(x: f64, y: f64, w: f64, h: f64) -> (f64, f64, f64, f64) {
    let (x, w) = if w < 0.0 { (x + w, -w) } else { (x, w) };
    let (y, h) = if h < 0.0 { (y + h, -h) } else { (y, h) };
    (x, y, w, h)
}

/// 有宽度线段（沿线刷圆）；越界点被 blend_px 裁掉。
#[allow(clippy::too_many_arguments)]
fn draw_line(
    buf: &mut [u8],
    width: u32,
    height: u32,
    x1: f64,
    y1: f64,
    x2: f64,
    y2: f64,
    style: &AnnotationStyle,
) {
    let dist = ((x2 - x1).powi(2) + (y2 - y1).powi(2)).sqrt();
    let steps = (dist.ceil() as usize * 2).max(1);
    let r = (style.stroke_width / 2.0).max(0.5);
    for i in 0..=steps {
        let t = i as f64 / steps as f64;
        let (cx, cy) = (x1 + (x2 - x1) * t, y1 + (y2 - y1) * t);
        for py in (cy - r).floor() as i64..=(cy + r).ceil() as i64 {
            for px in (cx - r).floor() as i64..=(cx + r).ceil() as i64 {
                let dx = px as f64 + 0.5 - cx;
                let dy = py as f64 + 0.5 - cy;
                if dx * dx + dy * dy <= r * r {
                    blend_px(buf, width, height, px, py, &style.color);
                }
            }
        }
    }
}

/// alpha 混合单像素（越界安全）。
fn blend_px(buf: &mut [u8], width: u32, height: u32, x: i64, y: i64, color: &[u8; 4]) {
    if x < 0 || y < 0 || x >= width as i64 || y >= height as i64 {
        return;
    }
    let idx = ((y as u32 * width + x as u32) * 4) as usize;
    let a = color[3] as u16;
    if a == 255 {
        buf[idx..idx + 4].copy_from_slice(color);
    } else {
        for c in 0..3 {
            let base = buf[idx + c] as u16;
            buf[idx + c] = ((color[c] as u16 * a + base * (255 - a)) / 255) as u8;
        }
    }
}

/// 区域盒模糊（迭代 3x3；只在区域内读写，边缘 clamp）。
#[allow(clippy::too_many_arguments)]
fn box_blur_region(
    buf: &mut [u8],
    width: u32,
    height: u32,
    x: f64,
    y: f64,
    w: f64,
    h: f64,
    passes: u32,
) {
    let x0 = x.max(0.0) as u32;
    let y0 = y.max(0.0) as u32;
    let x1 = ((x + w) as u32).min(width);
    let y1 = ((y + h) as u32).min(height);
    if x1 <= x0 || y1 <= y0 {
        return;
    }
    for _ in 0..passes {
        let snapshot = buf.to_vec();
        for py in y0..y1 {
            for px in x0..x1 {
                let (mut sr, mut sg, mut sb, mut sa) = (0u32, 0u32, 0u32, 0u32);
                let mut n = 0u32;
                for oy in -1i32..=1 {
                    for ox in -1i32..=1 {
                        let sx = (px as i32 + ox).clamp(0, width as i32 - 1) as u32;
                        let sy = (py as i32 + oy).clamp(0, height as i32 - 1) as u32;
                        let idx = ((sy * width + sx) * 4) as usize;
                        sr += snapshot[idx] as u32;
                        sg += snapshot[idx + 1] as u32;
                        sb += snapshot[idx + 2] as u32;
                        sa += snapshot[idx + 3] as u32;
                        n += 1;
                    }
                }
                let idx = ((py * width + px) * 4) as usize;
                buf[idx] = (sr / n) as u8;
                buf[idx + 1] = (sg / n) as u8;
                buf[idx + 2] = (sb / n) as u8;
                buf[idx + 3] = (sa / n) as u8;
            }
        }
    }
}

/// 区域块像素化（块内取平均色）。
#[allow(clippy::too_many_arguments)]
fn pixelate_region(
    buf: &mut [u8],
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    block: u32,
) {
    let x1 = (x + w).min(width);
    let y1 = (y + h).min(height);
    let mut by = y;
    while by < y1 {
        let mut bx = x;
        while bx < x1 {
            let (mut sr, mut sg, mut sb, mut sa) = (0u64, 0u64, 0u64, 0u64);
            let mut n = 0u64;
            for py in by..(by + block).min(y1) {
                for px in bx..(bx + block).min(x1) {
                    let idx = ((py * width + px) * 4) as usize;
                    sr += buf[idx] as u64;
                    sg += buf[idx + 1] as u64;
                    sb += buf[idx + 2] as u64;
                    sa += buf[idx + 3] as u64;
                    n += 1;
                }
            }
            let n = n.max(1); // 块非空 ⇒ n>=1；max 仅满足除法静态安全
            {
                let avg = [
                    (sr / n) as u8,
                    (sg / n) as u8,
                    (sb / n) as u8,
                    (sa / n) as u8,
                ];
                for py in by..(by + block).min(y1) {
                    for px in bx..(bx + block).min(x1) {
                        let idx = ((py * width + px) * 4) as usize;
                        buf[idx..idx + 4].copy_from_slice(&avg);
                    }
                }
            }
            bx += block;
        }
        by += block;
    }
}

// ── Text：系统字体（微软雅黑 → 宋体 → Arial）＋ ab_glyph 光栅化 ──

static FONT_CACHE: OnceLock<Option<Vec<u8>>> = OnceLock::new();

fn font_candidates() -> Vec<PathBuf> {
    let windir = std::env::var("WINDIR").unwrap_or_else(|_| "C:\\Windows".to_string());
    let fonts = PathBuf::from(windir).join("Fonts");
    ["msyh.ttc", "simsun.ttc", "arial.ttf"]
        .iter()
        .map(|f| fonts.join(f))
        .collect()
}

fn load_font() -> Option<&'static Vec<u8>> {
    FONT_CACHE
        .get_or_init(|| {
            for cand in font_candidates() {
                if let Ok(bytes) = std::fs::read(&cand)
                    && ab_glyph::FontRef::try_from_slice(&bytes).is_ok()
                {
                    return Some(bytes);
                }
            }
            None
        })
        .as_ref()
}

fn draw_text(
    buf: &mut [u8],
    width: u32,
    height: u32,
    x: f64,
    y: f64,
    text: &str,
    style: &AnnotationStyle,
) -> Result<(), WeaveError> {
    let Some(bytes) = load_font() else {
        return Err(err("glint.fontUnavailable", "no system font found"));
    };
    // FontRef 借用缓存；绘制期间缓存不会被替换（OnceLock）
    let font: ab_glyph::FontRef<'_> = ab_glyph::FontRef::try_from_slice(bytes)
        .map_err(|e| err("glint.fontUnavailable", format!("{e}")))?;
    let px = style.font_size.max(4.0) as f32;
    let scaled = font.as_scaled(ab_glyph::PxScale::from(px));
    let mut pen_x = x as f32;
    let pen_y = y as f32 + px; // y 视为文本顶部
    for ch in text.chars() {
        let mut glyph = scaled.scaled_glyph(ch);
        glyph.position = ab_glyph::point(pen_x, pen_y);
        let glyph_id = glyph.id;
        if let Some(outline) = font.outline_glyph(glyph) {
            let bb = outline.px_bounds();
            outline.draw(|gx, gy, v| {
                let px_i = bb.min.x as i64 + gx as i64;
                let py_i = bb.min.y as i64 + gy as i64;
                let mut color = style.color;
                color[3] = (color[3] as f32 * v).round().clamp(0.0, 255.0) as u8;
                blend_px(buf, width, height, px_i, py_i, &color);
            });
        }
        pen_x += scaled.h_advance(glyph_id);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base(w: u32, h: u32) -> Vec<u8> {
        // 灰度渐变，保证 blur/pixelate 有变化可测
        let mut v = Vec::with_capacity(w as usize * h as usize * 4);
        for y in 0..h {
            for x in 0..w {
                // 非线性渐变（线性梯度会被盒模糊原样保留，无法测出变化）
                let g = (((x * x + y * y) * 255) / ((w * w + h * h).max(1))) as u8;
                v.extend_from_slice(&[g, g, g, 255]);
            }
        }
        v
    }

    fn style() -> AnnotationStyle {
        AnnotationStyle {
            stroke_width: 2.0,
            color: [255, 0, 0, 255],
            font_size: 24.0,
            filled: false,
        }
    }

    #[test]
    fn empty_annotations_equal_base() {
        let b = base(16, 16);
        let out = render(&b, 16, 16, &[]).expect("render");
        assert_eq!(out, b);
    }

    #[test]
    fn rectangle_draws_only_in_region() {
        let b = base(32, 32);
        let out = render(
            &b,
            32,
            32,
            &[Annotation {
                id: "r1".into(),
                shape: AnnotationShape::Rectangle {
                    x: 8.0,
                    y: 8.0,
                    w: 8.0,
                    h: 8.0,
                },
                style: style(),
            }],
        )
        .expect("render");
        // 外部像素不变（非破坏）
        assert_eq!(&out[..8 * 4], &b[..8 * 4]);
        // 边框上像素变红
        let idx = ((8 * 32 + 8) * 4) as usize;
        assert_eq!(&out[idx..idx + 3], &[255, 0, 0]);
    }

    #[test]
    fn out_of_bounds_annotation_clamps_without_panic() {
        let b = base(16, 16);
        let out = render(
            &b,
            16,
            16,
            &[Annotation {
                id: "r2".into(),
                shape: AnnotationShape::Rectangle {
                    x: 12.0,
                    y: 12.0,
                    w: 20.0,
                    h: 20.0,
                },
                style: style(),
            }],
        );
        assert!(out.is_ok());
    }

    #[test]
    fn blur_reduces_local_variance_only_in_region() {
        let b = base(32, 32);
        let out = render(
            &b,
            32,
            32,
            &[Annotation {
                id: "b1".into(),
                shape: AnnotationShape::Blur {
                    x: 8.0,
                    y: 8.0,
                    w: 8.0,
                    h: 8.0,
                },
                style: style(),
            }],
        )
        .expect("render");
        // 区域内：像素变化（模糊抹平渐变）
        let idx = ((10 * 32 + 10) * 4) as usize;
        assert_ne!(out[idx], b[idx], "blur should alter region center");
        // 区域外：不变
        let out_idx = ((2 * 32 + 2) * 4) as usize;
        assert_eq!(out[out_idx], b[out_idx]);
    }

    #[test]
    fn pixelate_makes_uniform_blocks() {
        let b = base(32, 32);
        let out = render(
            &b,
            32,
            32,
            &[Annotation {
                id: "p1".into(),
                shape: AnnotationShape::Pixelate {
                    x: 0.0,
                    y: 0.0,
                    w: 16.0,
                    h: 16.0,
                    block: 8.0,
                },
                style: style(),
            }],
        )
        .expect("render");
        // 第一块 (0..8) 内所有像素相同
        let first = &out[0..4];
        for (x, y) in [(3u32, 3u32), (7u32, 0u32), (0u32, 7u32)] {
            let idx = ((y * 32 + x) * 4) as usize;
            assert_eq!(&out[idx..idx + 4], first, "block must be uniform");
        }
        // 区域外不变
        let out_idx = ((20 * 32 + 20) * 4) as usize;
        assert_eq!(out[out_idx], b[out_idx]);
    }

    #[test]
    fn text_renders_or_reports_font_unavailable() {
        let b = base(64, 64);
        let result = render(
            &b,
            64,
            64,
            &[Annotation {
                id: "t1".into(),
                shape: AnnotationShape::Text {
                    x: 4.0,
                    y: 4.0,
                    text: "Hi 截图".into(),
                },
                style: style(),
            }],
        );
        match result {
            Ok(out) => {
                // 至少一个像素变化
                assert!(out.iter().enumerate().any(|(i, v)| *v != b[i]));
            }
            Err(e) => {
                assert!(e.code.starts_with("glint.fontUnavailable"), "{e}");
            }
        }
    }

    #[test]
    fn size_mismatch_is_structured_error() {
        let b = vec![0u8; 10];
        let e = render(&b, 16, 16, &[]).expect_err("mismatch");
        assert!(e.code.starts_with("glint.invalidRegion"));
    }
}
