use super::{NativeAppAction, NativeAppLifecycle};

impl NativeAppLifecycle {
    /// Handle a platform resume notification.
    pub fn on_resumed(&mut self) -> NativeAppAction {
        if self.has_window {
            NativeAppAction::None
        } else {
            NativeAppAction::CreateWindow
        }
    }

    /// Record that the native window was created.
    pub fn on_window_created(&mut self) {
        self.on_window_created_with_monitor_refresh(None);
    }

    /// Record that the native window was created on a monitor with a known refresh rate.
    pub fn on_window_created_with_monitor_refresh(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
    ) {
        self.on_window_created_with_surface_report(monitor_refresh_millihertz, None);
    }

    /// Record that the native window was created with known monitor/surface metadata.
    pub fn on_window_created_with_surface_report(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
        surface_present_mode: Option<&'static str>,
    ) {
        self.on_window_created_with_full_report(
            monitor_refresh_millihertz,
            surface_present_mode,
            None,
            None,
            None,
        );
    }

    /// Record that the native window was created with known monitor, surface, and window metadata.
    pub fn on_window_created_with_full_report(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
        surface_present_mode: Option<&'static str>,
        window_width_px: Option<u32>,
        window_height_px: Option<u32>,
        window_scale_milliscale: Option<u32>,
    ) {
        self.has_window = true;
        self.windows_created += 1;
        self.monitor_refresh_millihertz = monitor_refresh_millihertz;
        self.surface_present_mode = surface_present_mode;
        self.window_width_px = window_width_px;
        self.window_height_px = window_height_px;
        self.window_scale_milliscale = window_scale_milliscale;
    }

    /// Record that the native window requested application shutdown.
    pub fn on_close_requested(&mut self) -> NativeAppAction {
        self.close_requested = true;
        NativeAppAction::Exit
    }

    /// Record that the native window was destroyed.
    pub fn on_destroyed(&mut self) -> NativeAppAction {
        self.has_window = false;
        NativeAppAction::Exit
    }

    /// Whether the lifecycle currently owns a native window.
    pub fn has_window(&self) -> bool {
        self.has_window
    }

    /// Whether shutdown was requested.
    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    /// Count of native windows created by this lifecycle.
    pub fn windows_created(&self) -> u64 {
        self.windows_created
    }
}
