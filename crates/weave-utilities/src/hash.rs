//! Hash（M9 上 §24-§31）：文本哈希（实际编码字节 §26-§27）+ 能力矩阵
//! （§24/§25：只声明真实实现并测试过的算法）。文件哈希复用 M1 流式
//! （§5/§28），不在本模块重复。
//!
//! 已知弱算法给 Warning 不给 Error（§31 legacy 兼容）。

use sha2::Digest;
// （RustCrypto digest：`use md-5/sha1/sha2` 类型经完整路径引用）

use crate::{UResult, UtilityDiagnostic, UtilityError, UtilityErrorKind, warn};

/// §25 能力矩阵条目（Availability = 真实实现 + golden 测试）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HashAlgorithmInfo {
    pub id: TextHashAlgorithm,
    pub name: &'static str,
    /// 已知碰撞抗性缺陷提示（§31）。
    pub security_note: Option<&'static str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum TextHashAlgorithm {
    Md5,
    Sha1,
    Sha224,
    Sha256,
    Sha384,
    Sha512,
    Blake3,
}

impl TextHashAlgorithm {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Md5 => "MD5",
            Self::Sha1 => "SHA-1",
            Self::Sha224 => "SHA-224",
            Self::Sha256 => "SHA-256",
            Self::Sha384 => "SHA-384",
            Self::Sha512 => "SHA-512",
            Self::Blake3 => "BLAKE3",
        }
    }

    pub fn parse(id: &str) -> Option<Self> {
        Some(match id.to_ascii_lowercase().as_str() {
            "md5" => Self::Md5,
            "sha1" | "sha-1" => Self::Sha1,
            "sha224" | "sha-224" => Self::Sha224,
            "sha256" | "sha-256" => Self::Sha256,
            "sha384" | "sha-384" => Self::Sha384,
            "sha512" | "sha-512" => Self::Sha512,
            "blake3" => Self::Blake3,
            _ => return None,
        })
    }
}

/// §24 能力矩阵（全部 = 真实实现 + §36/单元 golden 验证）。
pub fn hash_algorithm_matrix() -> Vec<HashAlgorithmInfo> {
    use TextHashAlgorithm::*;
    vec![
        HashAlgorithmInfo {
            id: Md5,
            name: "MD5",
            security_note: Some("not suitable for collision-resistant security purposes (§31)"),
        },
        HashAlgorithmInfo {
            id: Sha1,
            name: "SHA-1",
            security_note: Some("not suitable for collision-resistant security purposes (§31)"),
        },
        HashAlgorithmInfo {
            id: Sha224,
            name: "SHA-224",
            security_note: None,
        },
        HashAlgorithmInfo {
            id: Sha256,
            name: "SHA-256",
            security_note: None,
        },
        HashAlgorithmInfo {
            id: Sha384,
            name: "SHA-384",
            security_note: None,
        },
        HashAlgorithmInfo {
            id: Sha512,
            name: "SHA-512",
            security_note: None,
        },
        HashAlgorithmInfo {
            id: Blake3,
            name: "BLAKE3",
            security_note: None,
        },
    ]
}

/// §29 输出：小写十六进制（可选大写）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum HashCase {
    Lower,
    Upper,
}

/// §27 文本哈希结果（输入/编码/算法/输出全部显式）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TextHashResult {
    pub algorithm: TextHashAlgorithm,
    /// 实际参与哈希的字节数（§26：按编码字节，不是字符数）。
    pub bytes_processed: f64,
    /// 小写 hex（§29）；upper=true 时为大写。
    pub digest_hex: String,
    pub warnings: Vec<UtilityDiagnostic>,
}

fn digest_bytes(algorithm: TextHashAlgorithm, bytes: &[u8]) -> Vec<u8> {
    use TextHashAlgorithm as A;
    match algorithm {
        A::Md5 => md5::Md5::digest(bytes).to_vec(),
        A::Sha1 => sha1::Sha1::digest(bytes).to_vec(),
        A::Sha224 => sha2::Sha224::digest(bytes).to_vec(),
        A::Sha256 => sha2::Sha256::digest(bytes).to_vec(),
        A::Sha384 => sha2::Sha384::digest(bytes).to_vec(),
        A::Sha512 => sha2::Sha512::digest(bytes).to_vec(),
        A::Blake3 => blake3::hash(bytes).as_bytes().to_vec(),
    }
}

/// 文本哈希：先按 encoding 编码为实际字节（§26/§27），再摘要。
pub fn hash_text(text: &str, algorithm: TextHashAlgorithm, upper: bool) -> UResult<TextHashResult> {
    // 编码层复用 M4：UTF-8 为默认显式路径（charter #27）
    let bytes = text.as_bytes(); // &str 即 UTF-8（charter #27：输入统一 UTF-8）
    let digest = digest_bytes(algorithm, bytes);
    let hex = hex_encode(&digest);
    let mut warnings = Vec::new();
    let matrix = hash_algorithm_matrix();
    if let Some(info) = matrix.iter().find(|i| i.id == algorithm)
        && let Some(note) = info.security_note
    {
        warnings.push(warn("hash.weakAlgorithm", note));
    }
    Ok(TextHashResult {
        algorithm,
        bytes_processed: bytes.len() as f64,
        digest_hex: if upper { hex.to_uppercase() } else { hex },
        warnings,
    })
}

pub(crate) fn hex_encode(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push_str(&format!("{b:02x}"));
    }
    s
}

/// §24/§27 输入校验示例路径（非法算法名 = 结构化错误，不静默回退）。
pub fn require_algorithm(id: &str) -> UResult<TextHashAlgorithm> {
    TextHashAlgorithm::parse(id).ok_or_else(|| {
        UtilityError::new(
            UtilityErrorKind::UnsupportedAlgorithm,
            "hash.unknownAlgorithm",
            format!("unknown hash algorithm '{id}' (see capability matrix §24)"),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    // RustCrypto/BLAKE3 官方测试向量（"abc"）
    #[test]
    fn golden_known_vectors() {
        let v = |a, s| hash_text(s, a, false).expect("ok").digest_hex;
        assert_eq!(
            v(TextHashAlgorithm::Md5, "abc"),
            "900150983cd24fb0d6963f7d28e17f72"
        );
        assert_eq!(
            v(TextHashAlgorithm::Sha1, "abc"),
            "a9993e364706816aba3e25717850c26c9cd0d89d"
        );
        assert_eq!(
            v(TextHashAlgorithm::Sha256, "abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            v(TextHashAlgorithm::Sha512, "abc"),
            "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a\
             2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f"
        );
        assert_eq!(
            v(TextHashAlgorithm::Blake3, "abc"),
            "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85"
        );
    }

    #[test]
    fn bytes_not_chars_and_uppercase() {
        // §26：按编码字节而非字符数——"你好" = 6 UTF-8 字节
        let r = hash_text("你好", TextHashAlgorithm::Sha256, true).expect("ok");
        assert_eq!(r.bytes_processed, 6.0);
        assert!(
            r.digest_hex.chars().all(|c| !c.is_ascii_lowercase()),
            "无小写 a-f"
        );
        // 大小写仅展示差异
        let lower = hash_text("你好", TextHashAlgorithm::Sha256, false).expect("ok");
        assert_eq!(r.digest_hex.to_lowercase(), lower.digest_hex);
    }

    #[test]
    fn weak_algorithms_warn_not_error() {
        // §31：MD5 legacy 兼容 = Warning
        let r = hash_text("x", TextHashAlgorithm::Md5, false).expect("ok");
        assert!(r.warnings.iter().any(|w| w.code == "hash.weakAlgorithm"));
        let r = hash_text("x", TextHashAlgorithm::Sha256, false).expect("ok");
        assert!(r.warnings.is_empty());
    }

    #[test]
    fn unknown_algorithm_is_structured_error() {
        let e = require_algorithm("sha3-999").expect_err("unknown");
        assert_eq!(e.kind, UtilityErrorKind::UnsupportedAlgorithm);
    }
}
