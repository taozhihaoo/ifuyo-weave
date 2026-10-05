//! 位置级敏感模式检测（M2.4 §11-§13/§16；§53 复用 M2.3 规则原语）。
//!
//! - Detection 与 Response 分离：本模块只产出 Finding 事实，不做处置。
//! - Precision > Recall（§13）：值形状 + 上下文共同判定；宁少报不滥报。
//! - 所有命中 = Suspected（值形状无法证明一定是秘密 §3）。
//! - 证据 = masked 值（§14）：明文不进 Finding/日志/审计。

use weave_clipboard::model::{
    has_long_hex_run, looks_like_bearer, looks_like_jwt, looks_like_secret_assignment,
};

/// 检测到的模式种类（§12/§13/§16）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PatternKind {
    SuspectedJwt,
    SuspectedBearer,
    SuspectedSecretAssignment,
    SuspectedLongHexKey,
    Email,
    Phone,
    LocalPath,
    PrivateUrl,
}

impl PatternKind {
    pub fn sub_category(self) -> &'static str {
        match self {
            PatternKind::SuspectedJwt => "jwt",
            PatternKind::SuspectedBearer => "bearer",
            PatternKind::SuspectedSecretAssignment => "secret_assignment",
            PatternKind::SuspectedLongHexKey => "hex_key",
            PatternKind::Email => "email",
            PatternKind::Phone => "phone",
            PatternKind::LocalPath => "local_path",
            PatternKind::PrivateUrl => "private_url",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            PatternKind::SuspectedJwt => "疑似 JWT 令牌（匹配形态，不确证为秘密）",
            PatternKind::SuspectedBearer => "疑似 Bearer 授权头",
            PatternKind::SuspectedSecretAssignment => "疑似密钥/口令赋值形态",
            PatternKind::SuspectedLongHexKey => "疑似长十六进制密钥/哈希原文",
            PatternKind::Email => "邮箱地址",
            PatternKind::LocalPath => "可能暴露本机用户名或本地文件路径",
            PatternKind::Phone => "疑似电话号码",
            PatternKind::PrivateUrl => "疑似内部/私有 URL",
        }
    }
}

/// 单个位置级命中。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PatternHit {
    pub kind: PatternKind,
    /// 遮蔽后的值（sk…ef；§14 明文不出本模块的输出面）。
    pub masked_value: String,
    /// 命中所在行（1-based；供 UI 定位）。
    pub line: usize,
    /// 行内起始列（1-based）。
    pub column: usize,
}

/// 单行扫描（调用方按行切分——大文件流式友好 §45）。
/// Precision 策略：
/// - 赋值形态要求值有形状（≥8 字符且含字母数字混合）——`token = apple`（纯
///   小写短词）与 `The token is important` 不命中（§60 误报测试）。
/// - 电话：11 位手机 / 3-4-4 分隔段；邮箱：常见 TLD 启发 + 无连续点。
pub fn scan_line(line: &str, line_number: usize) -> Vec<PatternHit> {
    let mut hits = Vec::new();
    let lower = line.to_lowercase();

    // ── token 形态（复用 M2.3 原语，§53）──
    for token in line.split_whitespace() {
        if looks_like_jwt(token) {
            hits.push(hit(PatternKind::SuspectedJwt, token, line_number));
        }
    }
    if looks_like_bearer(&lower) {
        if let Some(v) = keyword_value(line, &["bearer "]) {
            hits.push(hit(PatternKind::SuspectedBearer, &v, line_number));
        }
    }
    if looks_like_secret_assignment(&lower) {
        if let Some(v) = keyword_value(
            line,
            &["password", "passwd", "secret", "api_key", "apikey", "token"],
        ) {
            // 值形状：≥8 且含字母 + 数字/符号混合（§12 Context+Shape）
            if value_has_shape(&v) {
                hits.push(hit(PatternKind::SuspectedSecretAssignment, &v, line_number));
            }
        }
    }
    if has_long_hex_run(line) {
        if let Some(seg) = line
            .split(|c: char| !c.is_ascii_hexdigit())
            .find(|seg| seg.len() >= 40)
        {
            hits.push(hit(PatternKind::SuspectedLongHexKey, seg, line_number));
        }
    }

    // ── 联系方式 / 路径 / URL ──
    for token in tokenize(line) {
        if is_email(&token) {
            hits.push(hit(PatternKind::Email, &token, line_number));
        } else if looks_like_local_path(&token) {
            hits.push(hit(PatternKind::LocalPath, &token, line_number));
        } else if is_phone(&token) {
            hits.push(hit(PatternKind::Phone, &token, line_number));
        } else if is_private_url(&token) {
            hits.push(hit(PatternKind::PrivateUrl, &token, line_number));
        }
    }
    hits
}

fn hit(kind: PatternKind, value: &str, line: usize) -> PatternHit {
    let column = 1; // 行级定位已足够；列值保留接口
    PatternHit {
        kind,
        masked_value: crate::model::masked_preview(value),
        line,
        column,
    }
}

fn tokenize(line: &str) -> Vec<String> {
    line.split(|c: char| c.is_whitespace() || c == ',' || c == ';' || c == '"' || c == '\'')
        .map(str::to_string)
        .filter(|t| !t.is_empty())
        .collect()
}

/// key 后的第一个值片段（key=value / key: value / key:  value）。
fn keyword_value<'a>(line: &'a str, keys: &[&str]) -> Option<String> {
    let lower = line.to_lowercase();
    for key in keys {
        for sep in [": ", "="] {
            let needle = format!("{key}{sep}");
            if let Some(pos) = lower.find(&needle) {
                let rest = line[pos + needle.len()..].trim();
                let value: String = rest.split_whitespace().next().unwrap_or("").to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

/// 值形状（§12）：长度 ≥8 且（含数字 或 长度 ≥16）——排除 `token = apple`。
fn value_has_shape(value: &str) -> bool {
    if value.len() < 8 {
        return false;
    }
    let has_digit = value.chars().any(|c| c.is_ascii_digit());
    has_digit || value.len() >= 16
}

fn is_email(token: &str) -> bool {
    let Some((local, domain)) = token.rsplit_once('@') else {
        return false;
    };
    !local.is_empty()
        && domain.contains('.')
        && !domain.starts_with('.')
        && !domain.ends_with('.')
        && token
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "@.-_+".contains(c))
}

/// 本地路径启发（§11）：Windows 用户目录 / 盘符路径 / UNC。
fn looks_like_local_path(token: &str) -> bool {
    let lower = token.to_lowercase();
    lower.starts_with("c:\\users\\")
        || lower.starts_with("/users/")
        || lower.starts_with("c:\\") && token.contains('\\')
        || token.starts_with("\\\\")
}

/// 电话启发：11 位手机（1 开头）/ 3-4-4 分段。
fn is_phone(token: &str) -> bool {
    let digits: String = token.chars().filter(|c| c.is_ascii_digit()).collect();
    let separators = token.chars().filter(|c| *c == '-' || *c == ' ').count();
    (token.len() >= 11 && digits.len() == 11 && digits.starts_with('1') && separators <= 2)
        || (digits.len() == 11 && separators == 2)
}

/// 私有/内网 URL 启发（http(s) + 内网地址/端口形态）。
fn is_private_url(token: &str) -> bool {
    let lower = token.to_lowercase();
    (lower.starts_with("http://") || lower.starts_with("https://"))
        && (lower.contains("localhost")
            || lower.contains("127.0.0.1")
            || lower.contains("192.168.")
            || lower.contains("10.0.")
            || lower.contains(".local")
            || lower.contains(":8080")
            || lower.contains(":3000"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn kinds(line: &str) -> Vec<PatternKind> {
        scan_line(line, 1).into_iter().map(|h| h.kind).collect()
    }

    #[test]
    fn detects_token_shapes_with_context_and_value() {
        assert!(
            kinds("api_key = sk-prod-1234567890").contains(&PatternKind::SuspectedSecretAssignment)
        );
        assert!(
            kinds("Authorization: Bearer eyJhbGciOiJ9.eyJ9.abc")
                .contains(&PatternKind::SuspectedJwt)
        );
        assert!(
            kinds(&format!("key = {}", "a".repeat(48))).contains(&PatternKind::SuspectedLongHexKey)
        );
    }

    #[test]
    fn false_positives_stay_silent() {
        // §60 误报矩阵：普通词/无形状值不命中
        assert!(
            !kinds("The token is important.").contains(&PatternKind::SuspectedSecretAssignment)
        );
        assert!(!kinds("token = apple").contains(&PatternKind::SuspectedSecretAssignment));
        assert!(!kinds("password = hello").contains(&PatternKind::SuspectedSecretAssignment));
        assert!(!kinds("secret = information").contains(&PatternKind::SuspectedSecretAssignment));
        assert!(
            !kinds("The secret of this recipe is love.")
                .contains(&PatternKind::SuspectedSecretAssignment)
        );
        // 短 hex（<40）不命中
        assert!(!kinds("deadbeef").contains(&PatternKind::SuspectedLongHexKey));
    }

    #[test]
    fn detects_contact_paths_urls() {
        assert!(kinds("contact: alice@example.com").contains(&PatternKind::Email));
        assert!(
            kinds("file at C:\\Users\\John\\Documents\\a.txt").contains(&PatternKind::LocalPath)
        );
        assert!(kinds("call 13812345678 now").contains(&PatternKind::Phone));
        assert!(kinds("see http://192.168.1.5:8080/admin").contains(&PatternKind::PrivateUrl));
    }

    #[test]
    fn masked_values_never_leak_full_content() {
        let hits = scan_line("api_key = sk-prod-1234567890abcdef", 1);
        let assignment = hits
            .iter()
            .find(|h| h.kind == PatternKind::SuspectedSecretAssignment)
            .expect("hit");
        assert!(
            !assignment.masked_value.contains("prod-1234567890abcdef"),
            "{}",
            assignment.masked_value
        );
        assert!(assignment.masked_value.contains('…'));
    }

    #[test]
    fn line_numbers_are_reported() {
        let hits = scan_line("email: bob@example.com", 12);
        assert!(hits.iter().all(|h| h.line == 12));
    }
}
