//! Color（M9 上 §75-§88/§104）：sRGB 基线（§77）；HEX/RGB/HSL/HSV/HWB
//! 解析（§84 strict：#12G45F 必须失败）；转换统一舍入策略（§86）；
//! round-trip 容差测试（§87）；对比度（§88 独立 capability）；确定性
//! （§104：hex 大写、alpha 0-1 归一）。

use crate::{UResult, UtilityError, UtilityErrorKind};

/// 颜色组件（§80：alpha 统一 0=透明 1=不透明）。
#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorRgb {
    pub r: f64,
    pub g: f64,
    pub b: f64,
    pub a: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorHsl {
    /// Hue 0-360（§81 边界：360 归一为 0）。
    pub h: f64,
    pub s: f64,
    pub l: f64,
    pub a: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorHsv {
    pub h: f64,
    pub s: f64,
    pub v: f64,
    pub a: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ColorHwb {
    pub h: f64,
    /// Whiteness 0-1。
    pub w: f64,
    /// Blackness 0-1（§83：W+B>100% 归一化语义）。
    pub b: f64,
    pub a: f64,
}

/// §78/§84 解析输入（strict）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ColorInput {
    Hex(String),
    Rgb { r: f64, g: f64, b: f64, a: f64 },
    Hsl { h: f64, s: f64, l: f64, a: f64 },
    Hsv { h: f64, s: f64, v: f64, a: f64 },
    Hwb { h: f64, w: f64, b: f64, a: f64 },
}

fn err(code: &'static str, message: impl Into<String>) -> UtilityError {
    UtilityError::new(UtilityErrorKind::InvalidColor, code, message)
}

fn clamp01(v: f64) -> f64 {
    v.clamp(0.0, 1.0)
}

fn clamp255(v: f64) -> f64 {
    v.clamp(0.0, 255.0)
}

/// §78 HEX：#RGB / #RGBA / #RRGGBB / #RRGGBBAA；§84 非 hex 字符必须失败。
pub fn parse_hex(input: &str) -> UResult<ColorRgb> {
    let trimmed = input.trim();
    let digits = trimmed.strip_prefix('#').ok_or_else(|| {
        err(
            "color.hexMissingHash",
            format!("'{input}' must start with '#'"),
        )
    })?;
    let valid = digits.chars().all(|c| c.is_ascii_hexdigit());
    if !valid {
        return Err(err(
            "color.invalidHex",
            format!("'{input}' contains non-hexadecimal characters (§84 strict)"),
        ));
    }
    // 3/4 位 = 单 nibble 双写；6/8 位 = 两位一字节
    let (r, g, b, a) = match digits.len() {
        3 => (
            expand_nibble(digits, 0),
            expand_nibble(digits, 1),
            expand_nibble(digits, 2),
            255.0,
        ),
        4 => (
            expand_nibble(digits, 0),
            expand_nibble(digits, 1),
            expand_nibble(digits, 2),
            expand_nibble(digits, 3) as f64,
        ),
        6 => (
            byte_at(digits, 0),
            byte_at(digits, 2),
            byte_at(digits, 4),
            255.0,
        ),
        8 => (
            byte_at(digits, 0),
            byte_at(digits, 2),
            byte_at(digits, 4),
            byte_at(digits, 6) as f64,
        ),
        other => {
            return Err(err(
                "color.invalidHexLength",
                format!("'{input}' length {other} is not 3/4/6/8 hex digits"),
            ));
        }
    };
    Ok(ColorRgb {
        r: r as f64,
        g: g as f64,
        b: b as f64,
        a: a / 255.0,
    })
}

/// 单 nibble 双写（3/4 位形态）：F ⇒ FF（F*17 = 255）。
fn expand_nibble(digits: &str, index: usize) -> u8 {
    let v = u8::from_str_radix(&digits[index..index + 1], 16).unwrap_or(0);
    v * 17
}

/// 两位 hex → 0-255。
fn byte_at(digits: &str, start: usize) -> u8 {
    u8::from_str_radix(&digits[start..start + 2], 16).unwrap_or(0)
}

fn hue_to_rgb_component(p: f64, q: f64, t: f64) -> f64 {
    let mut t = t;
    if t < 0.0 {
        t += 1.0;
    }
    if t > 1.0 {
        t -= 1.0;
    }
    if t < 1.0 / 6.0 {
        p + (q - p) * 6.0 * t
    } else if t < 0.5 {
        q
    } else if t < 2.0 / 3.0 {
        p + (q - p) * (2.0 / 3.0 - t) * 6.0
    } else {
        p
    }
}

/// RGB → HSL（§86 统一舍入：保留全精度，展示层负责舍入）。
pub fn rgb_to_hsl(c: &ColorRgb) -> ColorHsl {
    let r = clamp01(c.r / 255.0);
    let g = clamp01(c.g / 255.0);
    let b = clamp01(c.b / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let l = (max + min) / 2.0;
    let (h, s) = if (max - min).abs() < f64::EPSILON {
        (0.0, 0.0)
    } else {
        let d = max - min;
        let s = if l > 0.5 {
            d / (2.0 - max - min)
        } else {
            d / (max + min)
        };
        let h = match max {
            x if x == r => (g - b) / d + (if g < b { 6.0 } else { 0.0 }),
            x if x == g => (b - r) / d + 2.0,
            _ => (r - g) / d + 4.0,
        } * 60.0;
        (h % 360.0, s)
    };
    ColorHsl { h, s, l, a: c.a }
}

/// HSL → RGB。
pub fn hsl_to_rgb(c: &ColorHsl) -> ColorRgb {
    let h = ((c.h % 360.0) + 360.0) % 360.0 / 360.0;
    let s = clamp01(c.s);
    let l = clamp01(c.l);
    if s.abs() < f64::EPSILON {
        let v = l * 255.0;
        return ColorRgb {
            r: v,
            g: v,
            b: v,
            a: c.a,
        };
    }
    let q = if l < 0.5 {
        l * (1.0 + s)
    } else {
        l + s - l * s
    };
    let p = 2.0 * l - q;
    ColorRgb {
        r: clamp255(hue_to_rgb_component(p, q, h + 1.0 / 3.0) * 255.0),
        g: clamp255(hue_to_rgb_component(p, q, h) * 255.0),
        b: clamp255(hue_to_rgb_component(p, q, h - 1.0 / 3.0) * 255.0),
        a: c.a,
    }
}

/// RGB → HSV。
pub fn rgb_to_hsv(c: &ColorRgb) -> ColorHsv {
    let r = clamp01(c.r / 255.0);
    let g = clamp01(c.g / 255.0);
    let b = clamp01(c.b / 255.0);
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let d = max - min;
    let h = if d.abs() < f64::EPSILON {
        0.0
    } else {
        (match max {
            x if x == r => (g - b) / d + (if g < b { 6.0 } else { 0.0 }),
            x if x == g => (b - r) / d + 2.0,
            _ => (r - g) / d + 4.0,
        } * 60.0)
            % 360.0
    };
    let s = if max.abs() < f64::EPSILON {
        0.0
    } else {
        d / max
    };
    ColorHsv {
        h,
        s,
        v: max,
        a: c.a,
    }
}

/// HSV → RGB。
pub fn hsv_to_rgb(c: &ColorHsv) -> ColorRgb {
    let h = ((c.h % 360.0) + 360.0) % 360.0 / 60.0;
    let s = clamp01(c.s);
    let v = clamp01(c.v);
    let i = h.floor();
    let f = h - i;
    let p = v * (1.0 - s);
    let q = v * (1.0 - s * f);
    let t = v * (1.0 - s * (1.0 - f));
    let (r, g, b) = match i as i64 % 6 {
        0 => (v, t, p),
        1 => (q, v, p),
        2 => (p, v, t),
        3 => (p, q, v),
        4 => (t, p, v),
        _ => (v, p, q),
    };
    ColorRgb {
        r: clamp255(r * 255.0),
        g: clamp255(g * 255.0),
        b: clamp255(b * 255.0),
        a: c.a,
    }
}

/// HWB → RGB（§83：W+B ≥ 100% ⇒ 灰阶归一：各分量按 t = W/(W+B) 混合）。
pub fn hwb_to_rgb(c: &ColorHwb) -> ColorRgb {
    let hsl = ColorHsl {
        h: c.h,
        s: 1.0,
        l: 0.5,
        a: c.a,
    };
    let base = hsl_to_rgb(&hsl);
    let w = clamp01(c.w);
    let b = clamp01(c.b);
    if w + b >= 1.0 {
        let gray = w / (w + b) * 255.0;
        return ColorRgb {
            r: gray,
            g: gray,
            b: gray,
            a: c.a,
        };
    }
    let mix = |v: f64| (v * (1.0 - w - b) + w * 255.0).clamp(0.0, 255.0);
    ColorRgb {
        r: mix(base.r),
        g: mix(base.g),
        b: mix(base.b),
        a: c.a,
    }
}

/// §78/§85：解析 + 归一化输出（hex = 大写 RR/GG/BB[AA]，确定性 §104）。
pub fn to_hex(c: &ColorRgb, include_alpha: bool) -> String {
    let byte = |v: f64| format!("{:02X}", v.round().clamp(0.0, 255.0) as u8);
    if include_alpha || c.a < 1.0 {
        format!(
            "#{}{}{}{}",
            byte(c.r),
            byte(c.g),
            byte(c.b),
            byte(c.a * 255.0)
        )
    } else {
        format!("#{}{}{}", byte(c.r), byte(c.g), byte(c.b))
    }
}

/// §84 统一解析入口（strict）。
pub fn parse_color(input: ColorInput) -> UResult<ColorRgb> {
    match input {
        ColorInput::Hex(s) => parse_hex(&s),
        ColorInput::Rgb { r, g, b, a } => {
            for v in [r, g, b] {
                if !(0.0..=255.0).contains(&v) {
                    return Err(err("color.rgbOutOfRange", "RGB components must be 0-255"));
                }
            }
            Ok(ColorRgb {
                r,
                g,
                b,
                a: clamp01(a),
            })
        }
        ColorInput::Hsl { h, s, l, a } => {
            for v in [s, l] {
                if !(0.0..=1.0).contains(&v) {
                    return Err(err("color.hslOutOfRange", "HSL S/L must be 0-1"));
                }
            }
            Ok(hsl_to_rgb(&ColorHsl {
                h,
                s,
                l,
                a: clamp01(a),
            }))
        }
        ColorInput::Hsv { h, s, v, a } => {
            for v in [s, v] {
                if !(0.0..=1.0).contains(&v) {
                    return Err(err("color.hsvOutOfRange", "HSV S/V must be 0-1"));
                }
            }
            Ok(hsv_to_rgb(&ColorHsv {
                h,
                s,
                v,
                a: clamp01(a),
            }))
        }
        ColorInput::Hwb { h, w, b, a } => {
            for v in [w, b] {
                if !(0.0..=1.0).contains(&v) {
                    return Err(err("color.hwbOutOfRange", "HWB W/B must be 0-1"));
                }
            }
            Ok(hwb_to_rgb(&ColorHwb {
                h,
                w,
                b,
                a: clamp01(a),
            }))
        }
    }
}

/// §88 对比度（WCAG 相对亮度；独立 capability，不做设计套件）。
pub fn color_contrast(a: &ColorRgb, b: &ColorRgb) -> f64 {
    let lum = |c: &ColorRgb| -> f64 {
        let f = |v: f64| {
            let s = clamp01(v / 255.0);
            if s <= 0.03928 {
                s / 12.92
            } else {
                ((s + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * f(c.r) + 0.7152 * f(c.g) + 0.0722 * f(c.b)
    };
    let (l1, l2) = (lum(a), lum(b));
    let (hi, lo) = if l1 > l2 { (l1, l2) } else { (l2, l1) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() < 0.5 // §86/§87 tolerance（rounding policy）
    }

    #[test]
    fn hex_parsing_all_lengths() {
        // §78
        let c = parse_hex("#F00").expect("ok");
        assert_eq!((c.r, c.g, c.b), (255.0, 0.0, 0.0));
        let c = parse_hex("#FF0000").expect("ok");
        assert_eq!((c.r, c.g, c.b), (255.0, 0.0, 0.0));
        let c = parse_hex("#F00C").expect("ok");
        assert!((c.a - 204.0 / 255.0).abs() < 1e-9);
        let c = parse_hex("#FF000080").expect("ok");
        assert!((c.a - 128.0 / 255.0).abs() < 1e-9);
    }

    #[test]
    fn strict_parsing_rejects_invalid() {
        // §84：#12G45F 必须失败（不得剔除非法字符继续）
        assert!(parse_hex("#12G45F").is_err());
        assert!(parse_hex("FF0000").is_err(), "缺 # 也失败");
        assert!(
            parse_hex("#FF000").is_err(),
            "长度 5 非法（#FF00 是合法 RGBA 短式）"
        );
    }

    #[test]
    fn normalization_uppercase_deterministic() {
        // §85/§104：presentation normalization = 大写、确定性
        let c = parse_hex("#ff8800").expect("ok");
        assert_eq!(to_hex(&c, false), "#FF8800");
    }

    #[test]
    fn round_trips_within_tolerance() {
        // §87：HEX→RGB→HEX / RGB→HSL→RGB / RGB→HSV→RGB
        for hex in [
            "#FF8800", "#123456", "#00FF7F", "#808080", "#000000", "#FFFFFF",
        ] {
            let rgb = parse_hex(hex).expect("ok");
            let back = to_hex(&rgb, false);
            assert_eq!(back, hex, "HEX→RGB→HEX {hex}");
            let hsl = rgb_to_hsl(&rgb);
            let rgb2 = hsl_to_rgb(&hsl);
            assert!(close(rgb2.r, rgb.r) && close(rgb2.g, rgb.g) && close(rgb2.b, rgb.b));
            let hsv = rgb_to_hsv(&rgb);
            let rgb3 = hsv_to_rgb(&hsv);
            assert!(close(rgb3.r, rgb.r) && close(rgb3.g, rgb.g) && close(rgb3.b, rgb.b));
        }
    }

    #[test]
    fn hsl_hsv_named_colors() {
        // 语义正确性：红 = HSL(0,1,0.5) = HSV(0,1,1)
        let red = hsl_to_rgb(&ColorHsl {
            h: 0.0,
            s: 1.0,
            l: 0.5,
            a: 1.0,
        });
        assert!((red.r - 255.0).abs() < 0.5 && red.g.abs() < 0.5 && red.b.abs() < 0.5);
        // §82：HSV 与 HSL 不得混淆——HSL(0,1,1) = 白，HSV(0,1,1) = 红
        let white = hsl_to_rgb(&ColorHsl {
            h: 0.0,
            s: 1.0,
            l: 1.0,
            a: 1.0,
        });
        assert!(white.r > 254.0 && white.g > 254.0 && white.b > 254.0);
        let red2 = hsv_to_rgb(&ColorHsv {
            h: 0.0,
            s: 1.0,
            v: 1.0,
            a: 1.0,
        });
        assert!(red2.r > 254.0 && red2.g.abs() < 0.5);
    }

    #[test]
    fn hwb_gray_normalization() {
        // §83：W+B ≥ 100% ⇒ 灰阶归一
        let g = hwb_to_rgb(&ColorHwb {
            h: 0.0,
            w: 0.8,
            b: 0.8,
            a: 1.0,
        });
        assert!((g.r - g.g).abs() < 1e-9 && (g.g - g.b).abs() < 1e-9);
        assert!((g.r - 127.5).abs() < 0.5, "W=B ⇒ 50% 灰");
    }

    #[test]
    fn contrast_ratio_bounds() {
        let white = ColorRgb {
            r: 255.0,
            g: 255.0,
            b: 255.0,
            a: 1.0,
        };
        let black = ColorRgb {
            r: 0.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        };
        let ratio = color_contrast(&white, &black);
        assert!((ratio - 21.0).abs() < 0.1, "白/黑 = 21:1");
        let same = color_contrast(&white, &white);
        assert!((same - 1.0).abs() < 1e-9);
    }

    #[test]
    fn rgb_range_validation() {
        let e = parse_color(ColorInput::Rgb {
            r: 300.0,
            g: 0.0,
            b: 0.0,
            a: 1.0,
        })
        .expect_err("out of range");
        assert_eq!(e.kind, UtilityErrorKind::InvalidColor);
    }
}
