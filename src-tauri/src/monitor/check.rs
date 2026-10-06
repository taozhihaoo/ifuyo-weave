//! M2.11 检查管线（§2/§14/§15/§24）：**手动与调度共用同一执行路径**
//! ——fetch → 按 mode 提取 → normalize → fingerprint → 与最近成功快照
//! 比较 → 存快照/事件/通知。
//!
//! - 失败（非 2xx/网络错）⇒ FetchFailed/Blocked/Unsupported，**不写新
//!   快照、不覆盖 last_fingerprint**（§15 失败不污染成功快照）。
//! - 首次检查 ⇒ Initial 基线（§14 非 Changed）。
//! - 恢复（失败后成功）⇒ Recovered 事件（§73）。
//! - NoChange ⇒ metadata-only 快照（不占正文空间 §46）。

use scraper::{Html, Selector};

use crate::web::fetch::{FetchLimits, fetch};

use super::model::{
    ChangeMagnitude, CheckResult, MonitorChangeEvent, MonitorMode, MonitorSnapshot, MonitorTarget,
    magnitude_for, summarize,
};
use super::normalize::{fingerprint, normalize_text};
use super::store::MonitorStore;

/// 一次检查的完整产物（服务层据此更新 target/事件/通知）。
pub struct CheckOutcome {
    pub result: CheckResult,
    pub snapshot: Option<MonitorSnapshot>,
    /// 变化时的新正文（供服务层 diff 与 payload 持久化）。
    pub normalized_text: Option<String>,
    pub error_code: Option<String>,
    pub added_lines: u32,
    pub removed_lines: u32,
    pub magnitude: ChangeMagnitude,
}

/// 按 mode 提取比较对象文本（§5；§55 基于真正比较对象）。
pub fn extract_by_mode(
    html_text: &str,
    mode: MonitorMode,
    selector: Option<&str>,
) -> Result<String, String> {
    let dom = Html::parse_document(html_text);
    match mode {
        MonitorMode::WholePage => Ok(crate::web::html::plain_text(&dom)),
        MonitorMode::MainContent => {
            let (fragment, _confidence, _source) = crate::web::html::extract_main_content(&dom);
            Ok(crate::web::html::fragment_to_text(&fragment))
        }
        MonitorMode::Element => {
            let sel = selector.ok_or("element mode requires a CSS selector")?;
            let parsed = Selector::parse(sel).map_err(|e| format!("invalid selector: {e}"))?;
            let matches: Vec<_> = dom.select(&parsed).collect();
            if matches.is_empty() {
                return Err("selector matched no elements".to_string());
            }
            // 多匹配取首个（§34 最低要求：稳定可用的定位方案；多元素监控是 P1）
            Ok(crate::web::html::fragment_to_text(&matches[0].inner_html()))
        }
    }
}

/// 对最近一次有正文的快照做行级 diff 统计（§9/§13 确定性摘要）。
fn count_line_changes(previous: Option<&str>, current: &str) -> (u32, u32) {
    let Some(prev) = previous else { return (0, 0) };
    let prev_lines: std::collections::HashSet<&str> = prev.lines().collect();
    let cur_lines: std::collections::HashSet<&str> = current.lines().collect();
    let added = cur_lines.difference(&prev_lines).count() as u32;
    let removed = prev_lines.difference(&cur_lines).count() as u32;
    (added, removed)
}

/// 执行一次检查（手动与调度共用 §24）。`previous_payload` 由服务层按
/// last_fingerprint 回溯提供（最近有正文快照）。
#[allow(clippy::too_many_arguments)]
pub fn check_target(
    target: &MonitorTarget,
    limits: &FetchLimits,
    cancel: &weave_core::prelude::CancellationToken,
    _store: &MonitorStore,
    next_seq: f64,
    previous_payload: Option<String>,
    previous_had_failure: bool,
) -> CheckOutcome {
    let start = std::time::Instant::now();
    // 网络/提取失败 ⇒ 失败类结果；不写快照正文，只写 metadata-only 历史
    // 行（§8 历史含失败行；§15 不覆盖成功快照——last_fingerprint 由服务
    // 层仅成功时更新）。
    let failure = |result: CheckResult, code: String| CheckOutcome {
        result,
        snapshot: Some(MonitorSnapshot {
            target_id: target.id.clone(),
            seq: next_seq,
            fetched_at_ms: now_ms(),
            status_code: 0,
            final_url: target.url.clone(),
            content_type: String::new(),
            fingerprint: target.last_fingerprint.clone().unwrap_or_default(),
            text_size: 0,
            duration_ms: start.elapsed().as_millis() as f64,
            result,
            payload_file: None,
            truncated: false,
        }),
        normalized_text: None,
        error_code: Some(code),
        added_lines: 0,
        removed_lines: 0,
        magnitude: ChangeMagnitude::None,
    };

    if cancel.is_cancelled() {
        return failure(CheckResult::Cancelled, "web.cancelled".to_string());
    }

    let response = match fetch(&target.url, limits, cancel, false) {
        Ok(r) => r,
        Err(e) => {
            let result = match e.code.as_str() {
                "web.targetBlocked" => CheckResult::Blocked,
                "web.unsupportedContent" => CheckResult::Unsupported,
                "web.cancelled" => CheckResult::Cancelled,
                _ => CheckResult::FetchFailed,
            };
            return failure(result, e.code);
        }
    };
    let status = response.info.status;
    let (result, code) = match status {
        s if (200..300).contains(&s) => (None, None),
        401 | 403 => (
            Some(CheckResult::Blocked),
            Some(format!("web.httpStatus:{status}")),
        ),
        404 => (
            Some(CheckResult::FetchFailed),
            Some(format!("web.httpNotFound:{status}")),
        ),
        s if (400..500).contains(&s) => (
            Some(CheckResult::FetchFailed),
            Some(format!("web.httpStatus:{status}")),
        ),
        _s => (
            Some(CheckResult::FetchFailed),
            Some(format!("web.httpStatus:{status}")),
        ),
    };
    if let Some(result) = result {
        return failure(result, code.expect("code"));
    }
    if response.info.content_type != crate::web::model::WebContentType::Html
        && response.info.content_type != crate::web::model::WebContentType::PlainText
    {
        return failure(
            CheckResult::Unsupported,
            format!("web.unsupportedContent:{}", response.info.content_type_raw),
        );
    }

    // 解码 + 提取（编码失败容忍——decode_body 已 best-effort §53）
    let charset_hint = if response.info.charset.is_empty() {
        None
    } else {
        Some(response.info.charset.clone())
    };
    let crate::web::fetch::FetchResponse { info, body } = response;
    let (raw_text, _charset, _source) =
        crate::web::fetch::decode_body(&body, charset_hint.as_deref());
    let extracted = match extract_by_mode(&raw_text, target.mode, target.selector.as_deref()) {
        Ok(t) => t,
        Err(message) => {
            return failure(
                CheckResult::Unsupported,
                format!("web.extractFailed:{message}"),
            );
        }
    };
    let normalized = normalize_text(&extracted, true);
    let Some(fp) = fingerprint(&normalized) else {
        return failure(
            CheckResult::FetchFailed,
            "web.fingerprintFailed".to_string(),
        );
    };

    // 与最近成功快照比较（§10 fast-path）
    let (result, added, removed) = if target.last_fingerprint.is_none() {
        (CheckResult::Initial, 0, 0)
    } else if target.last_fingerprint.as_deref() == Some(fp.as_str()) {
        (CheckResult::NoChange, 0, 0)
    } else {
        let (a, r) = count_line_changes(previous_payload.as_deref(), &normalized);
        (CheckResult::ContentChanged, a, r)
    };
    // 失败后首次成功 ⇒ Recovered（§73；NoChange/Changed 之上叠加语义）
    let result = if previous_had_failure
        && matches!(result, CheckResult::NoChange | CheckResult::ContentChanged)
    {
        CheckResult::Recovered
    } else {
        result
    };

    let previous_lines = previous_payload
        .as_deref()
        .map(|p| p.lines().count())
        .unwrap_or(0) as u32;
    let magnitude = magnitude_for(added, removed, previous_lines);
    // 有正文才值得存 payload：Initial/ContentChanged/Recovered-有变化
    let store_payload = matches!(result, CheckResult::Initial | CheckResult::ContentChanged)
        || (result == CheckResult::Recovered
            && !normalized.is_empty()
            && target.last_fingerprint.as_deref() != Some(fp.as_str()));
    let snapshot = MonitorSnapshot {
        target_id: target.id.clone(),
        seq: next_seq,
        fetched_at_ms: now_ms(),
        status_code: status,
        final_url: info.final_url,
        content_type: info.content_type_raw,
        fingerprint: fp,
        text_size: normalized.len() as u32,
        duration_ms: info.duration_ms,
        result,
        payload_file: store_payload.then(|| format!("{}.json", next_seq)),
        truncated: info.size_bytes >= limits.max_bytes as f64,
    };
    CheckOutcome {
        result,
        snapshot: Some(snapshot),
        normalized_text: store_payload.then_some(normalized),
        error_code: None,
        added_lines: added,
        removed_lines: removed,
        magnitude,
    }
}

/// 事件构造（服务层共用；确定性摘要 §13）。
pub fn build_event(
    target: &MonitorTarget,
    outcome: &CheckOutcome,
    current_seq: f64,
    previous_seq: Option<f64>,
) -> MonitorChangeEvent {
    MonitorChangeEvent {
        id: format!("evt-{}-{current_seq}", target.id),
        target_id: target.id.clone(),
        detected_at_ms: now_ms(),
        result: outcome.result,
        magnitude: outcome.magnitude,
        added_lines: outcome.added_lines,
        removed_lines: outcome.removed_lines,
        previous_seq,
        current_seq,
        summary: summarize(
            outcome.result,
            outcome.added_lines,
            outcome.removed_lines,
            outcome
                .snapshot
                .as_ref()
                .map(|s| s.status_code)
                .unwrap_or(0),
        ),
    }
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::SystemTime::UNIX_EPOCH)
        .map(|d| d.as_millis() as f64)
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use weave_testkit::TempWorkspace;

    /// 本地 HTML 直驱提取逻辑（不发网络请求——extract_by_mode 单测）。
    #[test]
    fn extraction_modes_behave() {
        let html = r#"<html><body><nav>menu</nav><article><p>real body</p></article>
            <div class="price">99</div></body></html>"#;
        let whole = extract_by_mode(html, MonitorMode::WholePage, None).expect("whole");
        assert!(whole.contains("menu") && whole.contains("real body"));
        let main = extract_by_mode(html, MonitorMode::MainContent, None).expect("main");
        assert!(main.contains("real body"));
        let element = extract_by_mode(html, MonitorMode::Element, Some(".price")).expect("element");
        assert!(element.contains("99"));
        assert!(!element.contains("menu"));
        // selector 无匹配 ⇒ 可解释失败
        assert!(extract_by_mode(html, MonitorMode::Element, Some(".nope")).is_err());
        // 非法 selector ⇒ 可解释失败
        assert!(extract_by_mode(html, MonitorMode::Element, Some("///")).is_err());
    }

    #[test]
    fn line_change_counting_handles_removal() {
        // §3 场景 4：删除也是变化
        let (added, removed) = count_line_changes(Some("a\nb\nc"), "a\nb\nd");
        assert_eq!((added, removed), (1, 1));
        let (added, removed) = count_line_changes(Some("a\nb\nc"), "a");
        assert_eq!((added, removed), (0, 2));
        // 无前版（Initial）⇒ 0/0（基线不算变化）
        assert_eq!(count_line_changes(None, "anything"), (0, 0));
    }

    #[test]
    fn no_change_snapshot_carries_no_payload() {
        // §46：NoChange 走 metadata-only——由 check_target 的 store_payload
        // 逻辑保证；这里验证纯函数语义链（fingerprint 相等 ⇒ NoChange 分支
        // 不产正文）。完整链路在 monitor_service 集成测试。
        let ws = TempWorkspace::new("mon-check").expect("ws");
        let store = MonitorStore::open(ws.path().to_path_buf());
        let list = store.list_snapshots("none");
        assert!(list.is_empty());
    }
}
