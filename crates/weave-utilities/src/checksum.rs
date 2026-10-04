//! Checksum（M9 上 §32-§36）：与 Hash 明确区分（integrity/error-detection
//! oriented，§32）。CRC 参数全部引用标准定义（§35）并过标准测试向量
//! （§36："123456789"）。
//!
//! - CRC-32/ISO-HDLC（即常说的 "CRC32"）：poly 0x04C11DB7（reflected
//!   0xEDB88320），init 0xFFFFFFFF，refin/refout true，xorout 0xFFFFFFFF。
//! - CRC-32C（Castagnoli，iSCSI）：poly 0x1EDC6F41（reflected 0x82F63B78），
//!   init/xorout 0xFFFFFFFF，reflected。
//! - Adler-32（RFC 1950）：mod 65521 双和。

use crc::{CRC_32_ISCSI, CRC_32_ISO_HDLC, Crc};

use crate::UResult;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ChecksumAlgorithm {
    /// CRC-32/ISO-HDLC（§35：非"随便的 CRC32"）。
    Crc32IsoHdlc,
    /// CRC-32C（Castagnoli）。
    Crc32c,
    /// Adler-32（RFC 1950）。
    Adler32,
}

impl ChecksumAlgorithm {
    pub fn parse(id: &str) -> Option<Self> {
        Some(match id.to_ascii_lowercase().as_str() {
            "crc32" | "crc-32" | "crc32isohdlc" => Self::Crc32IsoHdlc,
            "crc32c" | "crc-32c" => Self::Crc32c,
            "adler32" | "adler-32" => Self::Adler32,
            _ => return None,
        })
    }

    /// §34 输出格式：全部小写 hex 8 位（确定性 §98）。
    fn format(self, value: u32) -> String {
        format!("{value:08x}")
    }
}

pub fn crc32_iso_hdlc(data: &[u8]) -> u32 {
    Crc::<u32>::new(&CRC_32_ISO_HDLC).checksum(data)
}

pub fn crc32c(data: &[u8]) -> u32 {
    Crc::<u32>::new(&CRC_32_ISCSI).checksum(data)
}

/// Adler-32（RFC 1950 §8）：A=1+Σbytes mod 65521；B=ΣA_i mod 65521。
pub fn adler32(data: &[u8]) -> u32 {
    const MOD: u32 = 65_521;
    let mut a: u32 = 1;
    let mut b: u32 = 0;
    for &byte in data {
        a = (a + byte as u32) % MOD;
        b = (b + a) % MOD;
    }
    (b << 16) | a
}

/// 统一入口：字节串 → 8 位小写 hex。
pub fn checksum_bytes(algorithm: ChecksumAlgorithm, data: &[u8]) -> UResult<String> {
    let value = match algorithm {
        ChecksumAlgorithm::Crc32IsoHdlc => crc32_iso_hdlc(data),
        ChecksumAlgorithm::Crc32c => crc32c(data),
        ChecksumAlgorithm::Adler32 => adler32(data),
    };
    Ok(algorithm.format(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    // §36：标准测试向量 "123456789"
    #[test]
    fn golden_standard_vectors() {
        assert_eq!(
            crc32_iso_hdlc(b"123456789"),
            0xCBF4_3926,
            "CRC-32/ISO-HDLC check value"
        );
        assert_eq!(crc32c(b"123456789"), 0xE306_9283, "CRC-32C check value");
        // Adler-32：权威值经 zlib 交叉验证
        assert_eq!(adler32(b"123456789"), 0x091E_01DE);
        // Adler-32 RFC 1950 已知值："Wikipedia" = 0x11E60398
        assert_eq!(adler32(b"Wikipedia"), 0x11E6_0398);
    }

    #[test]
    fn output_format_is_8_lowercase_hex() {
        let r = checksum_bytes(ChecksumAlgorithm::Crc32IsoHdlc, b"123456789").expect("ok");
        assert_eq!(r, "cbf43926");
        let r = checksum_bytes(ChecksumAlgorithm::Adler32, b"Wikipedia").expect("ok");
        assert_eq!(r, "11e60398");
    }

    #[test]
    fn empty_input_deterministic() {
        assert_eq!(crc32_iso_hdlc(b""), 0x0000_0000);
        assert_eq!(adler32(b""), 1);
    }
}
