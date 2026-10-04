//! Pixel / metadata 资源限额（M6 上 §5/§7/§117–§120）。
//!
//! §7 Decompression Bomb：磁盘 2 MB 的图可能解码为 12000×12000 RGBA。
//! 必须以 decoded pixel budget 控制，而非文件大小。

/// 图像资源限额。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ImageLimits {
    /// 单边最大尺寸（§118 Huge Dimension Guard）。
    pub max_dimension: u32,
    /// 解码像素总数上限（width × height，§7/§119）。
    pub max_pixels: u64,
    /// 解码后 RGBA 字节预算（width × height × 4，§119）。
    pub max_decoded_bytes: u64,
    /// Metadata 块字节上限（§117 Metadata Bomb）。
    pub max_metadata_bytes: u64,
    /// UI 预览最大边长（§120：预览不做全尺寸解码后的大缓冲暴露）。
    pub max_preview_dimension: u32,
}

impl Default for ImageLimits {
    fn default() -> Self {
        // §5：实际值经本仓库环境测试校准（非拍脑袋）——单张解码预算
        // 256 MiB RGBA ≈ 8000×8000×4，覆盖常见相机/截图，拒绝解压炸弹。
        Self {
            max_dimension: 16_384,
            max_pixels: 80_000_000,
            max_decoded_bytes: 256 * 1024 * 1024,
            max_metadata_bytes: 4 * 1024 * 1024,
            max_preview_dimension: 2048,
        }
    }
}

impl ImageLimits {
    /// §6/§119：checked 算术估算解码 RGBA 字节数；超预算 ⇒ Err。
    pub fn check_dimensions(&self, width: u32, height: u32) -> Result<(), String> {
        if width == 0 || height == 0 {
            return Err("image has zero dimension".to_string());
        }
        if width > self.max_dimension || height > self.max_dimension {
            return Err(format!(
                "image dimension {width}×{height} exceeds max dimension {}",
                self.max_dimension
            ));
        }
        let pixels = u64::from(width) * u64::from(height);
        if pixels > self.max_pixels {
            return Err(format!(
                "image has {pixels} pixels, exceeding budget {}",
                self.max_pixels
            ));
        }
        let decoded = pixels.saturating_mul(4);
        if decoded > self.max_decoded_bytes {
            return Err(format!(
                "decoded size would be {decoded} bytes, exceeding budget {}",
                self.max_decoded_bytes
            ));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dimension_guard_rejects_bombs_and_accepts_normal() {
        let limits = ImageLimits::default();
        // §7：2 MB 文件可声明 12000×12000（≈576 MiB RGBA）⇒ 拒绝
        assert!(limits.check_dimensions(12_000, 12_000).is_err());
        // 常见相机 4000×3000（48 MB RGBA）⇒ 通过
        assert!(limits.check_dimensions(4000, 3000).is_ok());
        // §118：超大维度直接拒绝
        assert!(limits.check_dimensions(1_000_000, 1_000_000).is_err());
        // 零维度
        assert!(limits.check_dimensions(0, 100).is_err());
    }

    #[test]
    fn checked_arithmetic_no_overflow() {
        // §6：u32::MAX 边界不得 overflow panic
        let limits = ImageLimits::default();
        assert!(limits.check_dimensions(u32::MAX, u32::MAX).is_err());
    }
}
