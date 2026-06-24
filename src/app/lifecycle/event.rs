use std::sync::Arc;

use winit::event_loop::EventLoopProxy;

/// User events sent into the native app event loop from background workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAppEvent {
    /// The PTY background reader observed output and the app should pump it promptly.
    PtyOutputReady,
}

/// Clonable sender for native app user events.
#[derive(Clone)]
pub struct NativeAppEventProxy {
    sender: Arc<dyn Fn(NativeAppEvent) + Send + Sync>,
}

impl NativeAppEventProxy {
    /// Build a proxy from a custom sender.
    pub fn from_sender<F>(sender: F) -> Self
    where
        F: Fn(NativeAppEvent) + Send + Sync + 'static,
    {
        Self {
            sender: Arc::new(sender),
        }
    }

    /// Send one user event into the native app loop.
    pub fn send(&self, event: NativeAppEvent) {
        (self.sender)(event);
    }
}

impl std::fmt::Debug for NativeAppEventProxy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeAppEventProxy")
            .finish_non_exhaustive()
    }
}

impl From<EventLoopProxy<NativeAppEvent>> for NativeAppEventProxy {
    fn from(proxy: EventLoopProxy<NativeAppEvent>) -> Self {
        Self::from_sender(move |event| {
            let _ = proxy.send_event(event);
        })
    }
}
