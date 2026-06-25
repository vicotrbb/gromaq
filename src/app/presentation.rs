use super::{NativeGlyphFrameError, NativeTerminalApp};

impl NativeTerminalApp {
    pub(super) fn present_redraw_frame(
        &mut self,
    ) -> Result<super::NativeGlyphFramePresentation, NativeGlyphFrameError> {
        let frame_status_text = self.lifecycle.frame_status_text();
        let Some(surface) = &mut self.surface else {
            self.runtime.render_terminal_frame_with_status_overlay(
                &mut self.renderer,
                Some(&frame_status_text),
            )?;
            return Ok(super::NativeGlyphFramePresentation::default());
        };
        let snapshot_path = self.lifecycle.config().glyph_frame_snapshot_path.clone();
        super::surface::render_and_present_terminal_glyph_frame_report_with_snapshot_and_status_overlay(
            &mut self.runtime,
            &mut self.renderer,
            &mut self.glyph_cache,
            surface,
            snapshot_path.as_deref(),
            Some(&frame_status_text),
        )
    }
}
