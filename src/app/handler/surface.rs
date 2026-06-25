//! Surface lifecycle helpers for native app event handling.

use std::sync::Arc;

use winit::event_loop::ActiveEventLoop;
use winit::window::Window;

use super::super::{NativeAppError, NativeTerminalApp};
use crate::native_gpu::{GpuBootstrap, GpuBootstrapConfig};

impl NativeTerminalApp {
    pub(super) fn handle_window_resized(
        &mut self,
        event_loop: &ActiveEventLoop,
        width: u32,
        height: u32,
    ) {
        if let Some(surface) = &mut self.surface
            && let Err(error) = surface.resize(width, height)
        {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
            return;
        }
        if let Err(error) = self.resize_runtime_to_window_pixels(width, height) {
            self.startup_error = Some(error.to_string());
            event_loop.exit();
        }
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    pub(super) fn create_surface_for_window(
        &mut self,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Result<(), NativeAppError> {
        let context = GpuBootstrap::new(GpuBootstrapConfig::native_default())
            .initialize_native()
            .map_err(NativeAppError::from)?;
        let gpu_surface = context
            .create_window_surface(window)
            .map_err(NativeAppError::from)?;
        let surface =
            super::super::NativeWindowSurface::from_gpu_surface(gpu_surface, width, height)
                .map_err(NativeAppError::from)?;
        self.gpu_context = Some(context);
        self.surface = Some(surface);
        Ok(())
    }
}
