//! M2.9 Wisp 保护会话（§11/§12/§19/§20/§42/§43）：Mask 不丢原文、
//! 恢复前指纹验证、TTL 过期、崩溃后不盲目恢复。
//!
//! - 同一时刻**至多一个**活动会话（§34 避免多会话 race）。
//! - 恢复守卫（§19）：当前剪贴板指纹 ≠ masked 指纹 ⇒ 用户已复制新内容，
//!   **绝不恢复旧内容**。
//! - 崩溃恢复（§20/§43）：启动时加载残留 session.json ⇒ 标记 Expired，
//!   不自动恢复；原文快照仅在 `retain_sensitive_payload` 开启时落盘
//!   （默认关闭——敏感内容仅当前会话内存 §25，不伪装保险库）。
//! - 无常驻定时器（§29 复用 Agent 生命周期纪律）：TTL 惰性判定。

use weave_core::prelude::WeaveError;

use crate::wisp::risk::TargetContext;

fn err(code: &'static str, message: impl Into<String>) -> WeaveError {
    WeaveError::io(code, message).with_location("wisp::session")
}

/// 会话状态（§12）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub enum SessionState {
    Prepared,
    Masked,
    Pasted,
    Restoring,
    Restored,
    Expired,
    Cancelled,
    Failed,
}

/// 保护会话（§12 ClipboardProtectionSession）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize, specta::Type)]
#[serde(rename_all = "camelCase")]
pub struct ProtectionSession {
    pub session_id: String,
    pub original_fingerprint: String,
    pub masked_fingerprint: String,
    pub created_at_ms: f64,
    /// TTL（§19；设置驱动）。
    pub expires_at_ms: f64,
    pub state: SessionState,
    /// 目标上下文快照（决策时记录；Unknown 如实）。
    pub target: TargetContext,
    pub original_level: crate::wisp::model::SensitivityLevel,
    /// 恢复被跳过的事实（§19：clipboard changed 时如实呈现）。
    pub last_restore_skip_reason: Option<String>,
}

/// 会话载荷：原文 + masked 文本（内存持有；持久化快照见 service 层）。
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionPayload {
    pub session_id: String,
    pub original_text: String,
    pub masked_text: String,
    pub state: SessionState,
    pub created_at_ms: f64,
    pub expires_at_ms: f64,
}

/// TTL 是否已过（惰性判定，无常驻定时器）。
pub fn is_expired(session: &ProtectionSession, now_ms: f64) -> bool {
    session.state == SessionState::Masked && now_ms >= session.expires_at_ms
}

/// 恢复前置检查（§19）：返回 Err ⇒ 不能恢复；Ok(masked_fingerprint) ⇒
/// 调用方写入原文前再对比剪贴板实际指纹。
pub fn precheck_restore(
    session: &ProtectionSession,
    now_ms: f64,
) -> Result<(), (SessionState, String)> {
    match session.state {
        SessionState::Masked => {}
        other => return Err((other, "session is not in Masked state".to_string())),
    }
    if now_ms >= session.expires_at_ms {
        return Err((SessionState::Expired, "session expired (TTL)".to_string()));
    }
    Ok(())
}

/// 恢复后状态迁移（指纹匹配 ⇒ Restored；不匹配 ⇒ Cancelled + 原因）。
pub fn apply_restore_outcome(
    session: &mut ProtectionSession,
    clipboard_matches_masked: bool,
) -> Result<(), String> {
    if clipboard_matches_masked {
        session.state = SessionState::Restored;
        Ok(())
    } else {
        session.state = SessionState::Cancelled;
        session.last_restore_skip_reason = Some("restore skipped because clipboard changed".into());
        Err("restore skipped because clipboard changed".to_string())
    }
}

/// 持久化快照路径（history_dir/wisp/session.json；单会话单文件）。
pub fn snapshot_path(dir: &std::path::Path) -> std::path::PathBuf {
    dir.join("session.json")
}

/// 保存会话快照（崩溃恢复用；原子写失败不 panic——保护能力降级不升级 §47）。
pub fn save_snapshot(dir: &std::path::Path, payload: &SessionPayload) -> Result<(), WeaveError> {
    std::fs::create_dir_all(dir).map_err(|e| err("wisp.sessionDirFailed", format!("{e}")))?;
    let tmp = dir.join("session.json.tmp");
    let json = serde_json::to_string(payload)
        .map_err(|e| err("wisp.sessionSerializeFailed", format!("{e}")))?;
    std::fs::write(&tmp, json).map_err(|e| err("wisp.sessionWriteFailed", format!("{e}")))?;
    std::fs::rename(&tmp, snapshot_path(dir))
        .map_err(|e| err("wisp.sessionWriteFailed", format!("{e}")))?;
    Ok(())
}

/// 清除会话快照（恢复/取消/过期后调用——不留原文残留 §54）。
pub fn clear_snapshot(dir: &std::path::Path) {
    let _ = std::fs::remove_file(snapshot_path(dir));
}

/// 加载残留快照（启动时；绝不自动恢复 §20——只把事实交还给 UI）。
pub fn load_snapshot(dir: &std::path::Path) -> Option<SessionPayload> {
    let bytes = std::fs::read(snapshot_path(dir)).ok()?;
    serde_json::from_slice(&bytes).ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::wisp::model::SensitivityLevel;

    fn session(state: SessionState, expires_in_ms: f64, now_ms: f64) -> ProtectionSession {
        ProtectionSession {
            session_id: "s1".into(),
            original_fingerprint: "orig".into(),
            masked_fingerprint: "masked".into(),
            created_at_ms: now_ms,
            expires_at_ms: now_ms + expires_in_ms,
            state,
            target: TargetContext::unknown(),
            original_level: SensitivityLevel::HighlySensitive,
            last_restore_skip_reason: None,
        }
    }

    #[test]
    fn restore_requires_masked_state() {
        let now = 1_000.0;
        let s = session(SessionState::Masked, 5_000.0, now);
        assert!(precheck_restore(&s, now + 1.0).is_ok());
        // 非 Masked 状态拒绝
        let prepared = session(SessionState::Prepared, 5_000.0, now);
        let (state, _) = precheck_restore(&prepared, now).unwrap_err();
        assert_eq!(state, SessionState::Prepared);
    }

    #[test]
    fn expired_session_cannot_restore() {
        let now = 1_000.0;
        let s = session(SessionState::Masked, 5_000.0, now);
        let (state, reason) = precheck_restore(&s, now + 6_000.0).unwrap_err();
        assert_eq!(state, SessionState::Expired);
        assert!(reason.contains("TTL"));
    }

    #[test]
    fn restore_outcome_fingerprint_guard() {
        // §19：剪贴板仍是被 mask 的内容 ⇒ 正常恢复
        let mut s = session(SessionState::Masked, 5_000.0, 0.0);
        assert!(apply_restore_outcome(&mut s, true).is_ok());
        assert_eq!(s.state, SessionState::Restored);
        // 用户复制了新内容 ⇒ 取消且绝不恢复
        let mut s2 = session(SessionState::Masked, 5_000.0, 0.0);
        assert!(apply_restore_outcome(&mut s2, false).is_err());
        assert_eq!(s2.state, SessionState::Cancelled);
        assert_eq!(
            s2.last_restore_skip_reason.as_deref(),
            Some("restore skipped because clipboard changed")
        );
    }

    #[test]
    fn snapshot_roundtrip_and_clear() {
        let ws = weave_testkit::TempWorkspace::new("wisp-session").expect("ws");
        let payload = SessionPayload {
            session_id: "s1".into(),
            original_text: "ghp_secret".into(),
            masked_text: "ghp_••••••••".into(),
            state: SessionState::Masked,
            created_at_ms: 1.0,
            expires_at_ms: 2.0,
        };
        save_snapshot(ws.path(), &payload).expect("save");
        let loaded = load_snapshot(ws.path()).expect("load");
        assert_eq!(loaded, payload);
        clear_snapshot(ws.path());
        assert!(load_snapshot(ws.path()).is_none());
    }

    #[test]
    fn corrupted_snapshot_is_none_not_panic() {
        let ws = weave_testkit::TempWorkspace::new("wisp-corrupt").expect("ws");
        std::fs::write(snapshot_path(ws.path()), b"{broken").expect("write");
        assert!(load_snapshot(ws.path()).is_none());
    }
}
