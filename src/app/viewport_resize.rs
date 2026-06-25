use crate::app::{NativeAppError, NativeTerminalApp};

impl NativeTerminalApp {
    pub(crate) fn resize_runtime_to_window_pixels(
        &mut self,
        width: u32,
        height: u32,
    ) -> Result<(), NativeAppError> {
        if let Some(resize) = self.resize_mapper.resize_for_window(width, height) {
            self.runtime.resize_terminal(resize)?;
            self.runtime.invalidate_terminal_frame();
        }
        Ok(())
    }
}
