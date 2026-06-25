use super::{NativeGlyphFrameError, NativeTerminalApp};

impl NativeTerminalApp {
    pub(super) fn present_redraw_frame(
        &mut self,
    ) -> Result<super::NativeGlyphFramePresentation, NativeGlyphFrameError> {
        let Some(surface) = &mut self.surface else {
            self.runtime.render_terminal_frame(&mut self.renderer)?;
            return Ok(super::NativeGlyphFramePresentation::default());
        };
        let snapshot_path = self.lifecycle.config().glyph_frame_snapshot_path.clone();
        super::render_and_present_terminal_glyph_frame_report_with_snapshot(
            &mut self.runtime,
            &mut self.renderer,
            &mut self.glyph_cache,
            surface,
            snapshot_path.as_deref(),
        )
    }
}
