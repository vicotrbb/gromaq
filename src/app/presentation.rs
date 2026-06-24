use super::{NativeGlyphFrameError, NativeTerminalApp};

impl NativeTerminalApp {
    pub(super) fn present_redraw_frame(
        &mut self,
    ) -> Result<super::NativeGlyphFramePresentation, NativeGlyphFrameError> {
        let Some(surface) = &mut self.surface else {
            self.runtime.render_terminal_frame(&mut self.renderer)?;
            return Ok(super::NativeGlyphFramePresentation::default());
        };
        super::render_and_present_terminal_glyph_frame_report(
            &mut self.runtime,
            &mut self.renderer,
            &mut self.glyph_cache,
            surface,
        )
    }
}
