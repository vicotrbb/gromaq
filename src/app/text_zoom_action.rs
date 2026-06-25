use crate::app::native_input::{NativeResizeGridMapper, NativeTextZoomAction};
use crate::app::text_zoom::renderer_config_for_text_zoom;
use crate::app::{NativeAppError, NativeTerminalApp};
use crate::renderer::RendererConfig;

impl NativeTerminalApp {
    /// Apply a browser-style terminal text zoom action to the active renderer metrics.
    pub fn apply_text_zoom_action(
        &mut self,
        action: NativeTextZoomAction,
    ) -> Result<bool, NativeAppError> {
        let current = self.renderer.config().clone();
        let next = renderer_config_for_text_zoom(&current, action);
        if next == current {
            return Ok(false);
        }
        self.apply_renderer_config_to_current_viewport(next)?;
        Ok(true)
    }

    fn apply_renderer_config_to_current_viewport(
        &mut self,
        renderer_config: RendererConfig,
    ) -> Result<(), NativeAppError> {
        let resize_mapper = NativeResizeGridMapper::new(
            renderer_config.cell_width_px,
            renderer_config.line_height_px,
            renderer_config.surface_padding_px,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime("native renderer cell dimensions must be non-zero".to_owned())
        })?;
        let (width, height) = self
            .window
            .as_ref()
            .map(|window| {
                let size = window.inner_size();
                (size.width, size.height)
            })
            .unwrap_or_else(|| {
                (
                    self.lifecycle.config().width,
                    self.lifecycle.config().height,
                )
            });
        if let Some(resize) = resize_mapper.resize_for_window(width, height) {
            self.runtime.resize_terminal(resize)?;
        }
        self.resize_mapper = resize_mapper;
        self.renderer.reconfigure(renderer_config);
        self.runtime.invalidate_terminal_frame();
        Ok(())
    }
}
