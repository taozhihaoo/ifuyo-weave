//! 文件 Hash/Checksum 工具域（M9 下 §5/§28/§131/§138/§140）。
//!
//! - 文件 Hash：**复用 M1 流式 hash_file**（SHA-256，JobTracker 取消/进度
//!   §5/§28 —— 不建第二文件哈希引擎）；
//! - 文件 Checksum：表驱动增量 CRC-32/ISO-HDLC / CRC-32C / Adler-32
//!   （§32-§36 同参数语义，常量内存 §138，增量=整体 golden 交叉验证）；
//! - 导出报告走 record_creation_transaction（M2 History，§144，IPC 层）。

use weave_core::prelude::CancellationToken;

use crate::checksum::ChecksumAlgorithm;
use crate::{UResult, UtilityError, UtilityErrorKind};

/// 文件校验和结果（§96 Result Table 行）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FileChecksumEntry {
    pub path: String,
    pub algorithm: String,
    /// 小写 hex 8 位（§34）。
    pub digest_hex: String,
    pub bytes_processed: f64,
    pub status: String,
}

const STREAM_CHUNK: usize = 256 * 1024;

/// 增量 CRC-32（reflected poly 表驱动；ISO-HDLC poly 0xEDB88320 /
/// CRC32C poly 0x82F63B78 —— 与 crc crate 同参数，golden 交叉验证 §36）。
pub struct Crc32Incremental {
    table: [u32; 256],
    value: u32,
}

impl Crc32Incremental {
    pub fn new(poly_reflected: u32) -> Self {
        let mut table = [0u32; 256];
        for (i, entry) in table.iter_mut().enumerate() {
            let mut c = i as u32;
            for _ in 0..8 {
                c = if c & 1 != 0 {
                    poly_reflected ^ (c >> 1)
                } else {
                    c >> 1
                };
            }
            *entry = c;
        }
        Self {
            table,
            value: 0xFFFF_FFFF,
        }
    }

    pub fn update(&mut self, data: &[u8]) {
        for &b in data {
            self.value = self.table[((self.value ^ b as u32) & 0xFF) as usize] ^ (self.value >> 8);
        }
    }

    pub fn finalize(self) -> u32 {
        self.value ^ 0xFFFF_FFFF
    }
}

/// Adler-32 增量（RFC 1950）。
pub struct Adler32Incremental {
    a: u32,
    b: u32,
}

impl Default for Adler32Incremental {
    fn default() -> Self {
        Self { a: 1, b: 0 }
    }
}

impl Adler32Incremental {
    pub fn update(&mut self, data: &[u8]) {
        const MOD: u32 = 65_521;
        for &byte in data {
            self.a = (self.a + byte as u32) % MOD;
            self.b = (self.b + self.a) % MOD;
        }
    }

    pub fn finalize(&self) -> u32 {
        (self.b << 16) | self.a
    }
}

/// 流式文件校验和（§138 常量内存；§140 取消；TOCTOU：完成后由调用方
/// stat 复核或接受单次读取语义——报告 status 如实）。
pub fn checksum_file(
    path: &std::path::Path,
    algorithm: ChecksumAlgorithm,
    cancel: &CancellationToken,
    on_progress: &mut dyn FnMut(f64),
) -> UResult<FileChecksumEntry> {
    use std::io::Read;
    // TOCTOU 面收敛：先确认存在且非常规文件直接拒绝（§167 snapshot 语义）
    let meta = std::fs::metadata(path).map_err(|e| {
        UtilityError::new(
            UtilityErrorKind::InvalidInput,
            "checksum.sourceMissing",
            format!("{}: {e}", path.display()),
        )
    })?;
    if !meta.is_file() {
        return Err(UtilityError::new(
            UtilityErrorKind::InvalidInput,
            "checksum.notRegularFile",
            format!("{} is not a regular file", path.display()),
        ));
    }
    let mut file = std::fs::File::open(path).map_err(|e| {
        UtilityError::new(
            UtilityErrorKind::InvalidInput,
            "checksum.openFailed",
            format!("{}: {e}", path.display()),
        )
    })?;

    let mut crc_iso = None;
    let mut crc_c = None;
    let mut adler = None;
    match algorithm {
        ChecksumAlgorithm::Crc32IsoHdlc => crc_iso = Some(Crc32Incremental::new(0xEDB8_8320)),
        ChecksumAlgorithm::Crc32c => crc_c = Some(Crc32Incremental::new(0x82F6_3B78)),
        ChecksumAlgorithm::Adler32 => adler = Some(Adler32Incremental::default()),
    }

    let mut buf = vec![0u8; STREAM_CHUNK];
    let mut processed: u64 = 0;
    loop {
        if cancel.is_cancelled() {
            return Err(UtilityError::new(
                UtilityErrorKind::InvalidInput,
                "checksum.cancelled",
                "cancelled by user",
            ));
        }
        let read = file.read(&mut buf).map_err(|e| {
            UtilityError::new(
                UtilityErrorKind::InvalidInput,
                "checksum.readFailed",
                format!("{}: {e}", path.display()),
            )
        })?;
        if read == 0 {
            break;
        }
        if let Some(c) = crc_iso.as_mut() {
            c.update(&buf[..read]);
        }
        if let Some(c) = crc_c.as_mut() {
            c.update(&buf[..read]);
        }
        if let Some(a) = adler.as_mut() {
            a.update(&buf[..read]);
        }
        processed += read as u64;
        on_progress(processed as f64);
    }

    let digest_hex = if let Some(c) = crc_iso {
        format!("{:08x}", c.finalize())
    } else if let Some(c) = crc_c {
        format!("{:08x}", c.finalize())
    } else {
        format!("{:08x}", adler.expect("algorithm set").finalize())
    };

    Ok(FileChecksumEntry {
        path: path.to_string_lossy().into_owned(),
        algorithm: format!("{algorithm:?}"),
        digest_hex,
        bytes_processed: processed as f64,
        status: "success".into(),
    })
}

/// 导出报告组装（§146/§147）：JSON（serde 确定性字段序）与 CSV 行
/// （§147：逗号/引号/换行转义；写入经 IPC 层 M2 安全写，不在域内写盘）。
pub fn export_report(entries: &[FileChecksumEntry], format: ExportFormat) -> String {
    match format {
        ExportFormat::Json => serde_json::to_string_pretty(entries).unwrap_or_else(|_| "[]".into()),
        ExportFormat::Csv => {
            let mut out = String::from("path,algorithm,digest,status\n");
            for e in entries {
                out.push_str(&csv_escape(&e.path));
                out.push(',');
                out.push_str(&csv_escape(&e.algorithm));
                out.push(',');
                out.push_str(&csv_escape(&e.digest_hex));
                out.push(',');
                out.push_str(&csv_escape(&e.status));
                out.push('\n');
            }
            out
        }
        ExportFormat::Txt => {
            let mut out = String::new();
            for e in entries {
                out.push_str(&format!("{}  {}  {}\n", e.digest_hex, e.algorithm, e.path));
            }
            out
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ExportFormat {
    Json,
    Csv,
    Txt,
}

/// RFC 4180 CSV 字段转义（含逗号/引号/换行）。
fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_owned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::checksum::{adler32, crc32_iso_hdlc, crc32c};

    // §36：增量与整体一致（crc crate golden 交叉验证）
    #[test]
    fn incremental_matches_one_shot() {
        let data = b"123456789";
        let mut iso = Crc32Incremental::new(0xEDB8_8320);
        iso.update(data);
        assert_eq!(iso.finalize(), crc32_iso_hdlc(data), "ISO-HDLC");

        let mut c = Crc32Incremental::new(0x82F6_3B78);
        c.update(data);
        assert_eq!(c.finalize(), crc32c(data), "CRC32C");

        let mut a = Adler32Incremental::default();
        a.update(data);
        assert_eq!(a.finalize(), adler32(data), "Adler-32");
    }

    #[test]
    fn chunked_updates_equal_single_update() {
        let data: Vec<u8> = (0..=255u8).cycle().take(100_000).collect();
        let mut whole = Crc32Incremental::new(0xEDB8_8320);
        whole.update(&data);
        let mut chunked = Crc32Incremental::new(0xEDB8_8320);
        for chunk in data.chunks(7_777) {
            chunked.update(chunk);
        }
        assert_eq!(chunked.finalize(), whole.finalize());

        let mut whole_a = Adler32Incremental::default();
        whole_a.update(&data);
        let mut chunked_a = Adler32Incremental::default();
        for chunk in data.chunks(7_777) {
            chunked_a.update(chunk);
        }
        assert_eq!(chunked_a.finalize(), whole_a.finalize());
    }

    #[test]
    fn file_checksum_real_file() {
        // §165 真实文件系统
        let dir = tempfile::tempdir().expect("ws");
        let p = dir.path().join("data.bin");
        std::fs::write(&p, b"123456789").expect("write");
        let entry = checksum_file(
            &p,
            ChecksumAlgorithm::Crc32IsoHdlc,
            &weave_core::prelude::CancellationToken::new(),
            &mut |_| {},
        )
        .expect("ok");
        assert_eq!(entry.digest_hex, "cbf43926");
        assert_eq!(entry.bytes_processed, 9.0);
    }

    #[test]
    fn export_csv_escapes_and_json_deterministic() {
        // §147：逗号/引号转义；§146：JSON 稳定
        let entries = vec![FileChecksumEntry {
            path: r#"C:\a,b""c.pdf"#.into(),
            algorithm: "Crc32IsoHdlc".into(),
            digest_hex: "cbf43926".into(),
            bytes_processed: 9.0,
            status: "success".into(),
        }];
        let csv = export_report(&entries, ExportFormat::Csv);
        assert_eq!(
            csv,
            "path,algorithm,digest,status\n\"C:\\a,b\"\"\"\"c.pdf\",Crc32IsoHdlc,cbf43926,success\n"
        );
        let json = export_report(&entries, ExportFormat::Json);
        assert!(json.contains("\"path\""));
        assert!(json.contains("cbf43926"));
        let txt = export_report(&entries, ExportFormat::Txt);
        assert!(txt.starts_with("cbf43926"));
    }
}
