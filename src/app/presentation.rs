use super::{NativeGlyphFrameError, NativeTerminalApp};

impl NativeTerminalApp {
    pub(super) fn present_redraw_frame(&mut self) -> Result<(), NativeGlyphFrameError> {
        let Some(surface) = &mut self.surface else {
            self.runtime.render_terminal_frame(&mut self.renderer)?;
            return Ok(());
        };
        super::render_and_present_terminal_glyph_frame(
            &mut self.runtime,
            &mut self.renderer,
            &mut self.glyph_cache,
            surface,
        )?;
        Ok(())
    }
}
