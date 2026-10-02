//! 协作式取消契约（M0 §18）。
//!
//! Weave 的取消是 cooperative cancellation：工具在安全点主动检查 token，
//! 而不是被外部 kill。这为 M7 Batch Engine 预留了明确的扩展位置。

use crate::error::WeaveError;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// 取消令牌：可在任意线程 clone / 触发，工具侧在安全点轮询。
#[derive(Clone, Default)]
pub struct CancellationToken {
    flag: Arc<AtomicBool>,
}

impl CancellationToken {
    pub fn new() -> Self {
        Self::default()
    }

    /// 触发取消。已取消后再调用是幂等的。
    pub fn cancel(&self) {
        self.flag.store(true, Ordering::SeqCst);
    }

    pub fn is_cancelled(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// 在安全点检查；已取消时返回结构化 Cancelled 错误。
    pub fn check(&self) -> Result<(), WeaveError> {
        if self.is_cancelled() {
            Err(WeaveError::cancelled(
                "operation.cancelled",
                "operation was cancelled by the user",
            ))
        } else {
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn token_starts_live_and_cancel_is_idempotent() {
        let token = CancellationToken::new();
        assert!(!token.is_cancelled());
        token.cancel();
        token.cancel();
        assert!(token.is_cancelled());
    }

    #[test]
    fn check_returns_cancelled_error_once_cancelled() {
        let token = CancellationToken::new();
        assert!(token.check().is_ok());
        token.cancel();
        let err = token.check().expect_err("must be cancelled");
        assert_eq!(err.code, "operation.cancelled");
    }

    #[test]
    fn cloned_token_shares_state() {
        let token = CancellationToken::new();
        let clone = token.clone();
        clone.cancel();
        assert!(token.is_cancelled());
    }
}
