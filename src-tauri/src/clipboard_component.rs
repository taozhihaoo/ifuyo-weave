//! Clipboard Agent 组件（M2.6 §27）：包装 M2.3 捕获标志——生命周期归 Agent。
use crate::agent::{AgentComponent, ComponentHealth, ComponentState};
use std::sync::Mutex;
use std::sync::atomic::{AtomicBool, Ordering};

/// Clipboard 组件：经 M2.3 ClipboardRuntime 的标志位暂停/恢复捕获。
pub struct ClipboardComponent {
    pub paused_flag: std::sync::Arc<AtomicBool>,
    pub enabled_flag: std::sync::Arc<AtomicBool>,
    state: Mutex<ComponentState>,
}

impl ClipboardComponent {
    pub fn new() -> Self {
        Self {
            paused_flag: std::sync::Arc::new(AtomicBool::new(false)),
            enabled_flag: std::sync::Arc::new(AtomicBool::new(true)),
            state: Mutex::new(ComponentState::Stopped),
        }
    }

    /// 以既有标志位构造（M2.6 接线：Agent 直接共享 M2.3 的捕获标志）。
    pub fn with_flags(
        paused_flag: std::sync::Arc<AtomicBool>,
        enabled_flag: std::sync::Arc<AtomicBool>,
    ) -> Self {
        Self {
            paused_flag,
            enabled_flag,
            state: Mutex::new(ComponentState::Stopped),
        }
    }

    /// 绑定到 M2.3 ClipboardRuntime 的实际标志位（构造后调用一次）。
    pub fn bind_flags(
        &mut self,
        paused: std::sync::Arc<AtomicBool>,
        enabled: std::sync::Arc<AtomicBool>,
    ) {
        self.paused_flag = paused;
        self.enabled_flag = enabled;
    }

    /// M2.6 接线：由 Agent 启动时调用（start 语义内含 resume+enable）。
    pub fn clipboard_inner_bind(&mut self) {
        self.start();
    }
}

impl Default for ClipboardComponent {
    fn default() -> Self {
        Self::new()
    }
}

impl AgentComponent for ClipboardComponent {
    fn name(&self) -> &'static str {
        "clipboard"
    }

    fn start(&mut self) {
        self.enabled_flag.store(true, Ordering::SeqCst);
        self.paused_flag.store(false, Ordering::SeqCst);
        *self.state.lock().expect("state") = ComponentState::Running;
    }

    fn pause(&mut self) {
        self.paused_flag.store(true, Ordering::SeqCst);
        *self.state.lock().expect("state") = ComponentState::Paused;
    }

    fn resume(&mut self) {
        self.paused_flag.store(false, Ordering::SeqCst);
        *self.state.lock().expect("state") = ComponentState::Running;
    }

    fn stop(&mut self) {
        self.paused_flag.store(true, Ordering::SeqCst);
        *self.state.lock().expect("state") = ComponentState::Stopped;
    }

    fn health(&self) -> ComponentHealth {
        ComponentHealth {
            name: self.name().to_string(),
            state: self.state.lock().expect("state").as_str().to_string(),
            last_error: None,
            last_success: None,
        }
    }
}
