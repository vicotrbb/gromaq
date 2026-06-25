use crate::app::{NativeAppLifecycle, NativeTerminalApp, NativeTerminalRuntime};
use crate::pty::PtySession;
use crate::renderer::WgpuRenderer;

impl NativeTerminalApp {
    /// Access lifecycle state.
    pub fn lifecycle(&self) -> &NativeAppLifecycle {
        &self.lifecycle
    }

    /// Access runtime state.
    pub fn runtime(&self) -> &NativeTerminalRuntime<PtySession> {
        &self.runtime
    }

    /// Access renderer state.
    pub fn renderer(&self) -> &WgpuRenderer {
        &self.renderer
    }

    /// Active configured font family or file path used by the native glyph cache.
    pub fn font_family(&self) -> &str {
        &self.font_family
    }

    /// Take a startup error captured from the event handler.
    pub fn take_startup_error(&mut self) -> Option<String> {
        self.startup_error.take()
    }
}
