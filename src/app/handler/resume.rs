use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;

use super::window_metadata::{scale_factor_milliscale, surface_present_mode_name};
use crate::app::{NativeAppAction, NativeTerminalApp};

impl NativeTerminalApp {
    pub(super) fn handle_resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.lifecycle.on_resumed() != NativeAppAction::CreateWindow {
            return;
        }
        match event_loop.create_window(self.lifecycle.config().window_attributes()) {
            Ok(window) => self.finish_window_startup(event_loop, Arc::new(window)),
            Err(error) => {
                self.startup_error = Some(error.to_string());
                event_loop.exit();
            }
        }
    }

    fn finish_window_startup(
        &mut self,
        event_loop: &ActiveEventLoop,
        window: Arc<winit::window::Window>,
    ) {
        let size = window.inner_size();
        self.window_id = Some(window.id());
        configure_window_screen_capture_policy(
            window.as_ref(),
            self.lifecycle.config().screen_capture_allowed,
        );
        window.set_ime_allowed(true);
        if let Err(error) =
            self.create_surface_for_window(Arc::clone(&window), size.width, size.height)
        {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
            return;
        }
        if let Err(error) = self.resize_runtime_to_window_pixels(size.width, size.height) {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
            return;
        }
        let monitor_refresh_millihertz = window
            .current_monitor()
            .and_then(|monitor| monitor.refresh_rate_millihertz());
        let surface_present_mode = self
            .surface
            .as_ref()
            .and_then(|surface| surface.present_mode())
            .map(surface_present_mode_name);
        let scale_milliscale = scale_factor_milliscale(window.scale_factor());
        self.window = Some(window);
        self.lifecycle.on_window_created_with_full_report(
            monitor_refresh_millihertz,
            surface_present_mode,
            Some(size.width),
            Some(size.height),
            Some(scale_milliscale),
        );
        if let Err(error) = self.runtime.start_shell(&self.pty_spawner) {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
            return;
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

fn configure_window_screen_capture_policy(window: &winit::window::Window, allowed: bool) {
    window.set_content_protected(!allowed);
}
