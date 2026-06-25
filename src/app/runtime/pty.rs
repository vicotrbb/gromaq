use tracing::trace;

use crate::app::NativeAppError;
use crate::app::lifecycle::{NativeAppAction, NativeAppLifecycle};
use crate::app::native_input::NativePtyResize;
use crate::app::perf::add_usize_counter;
use crate::app::pty_bridge::{NativePtySessionIo, NativeTerminalRuntimeConfig};
use crate::clipboard::HostClipboard;

use super::NativeTerminalRuntime;

impl<S> NativeTerminalRuntime<S>
where
    S: NativePtySessionIo,
{
    /// Drain available PTY output and feed it into the terminal parser.
    pub fn pump_pty_output(&mut self) -> Result<usize, NativeAppError> {
        let Some(session) = self.shell_session.as_mut() else {
            return Ok(0);
        };
        let output = session
            .drain_output()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        if output.is_empty() {
            return Ok(0);
        }
        self.perf.pty_output_batches += 1;
        add_usize_counter(&mut self.perf.pty_output_bytes, output.len());
        self.terminal
            .write_bytes(&output)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let response = self.terminal.take_pending_response_bytes();
        if !response.is_empty() {
            session
                .write_input(&response)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
            self.perf.pty_response_writes += 1;
            add_usize_counter(&mut self.perf.pty_response_bytes, response.len());
        }
        trace!(
            output_bytes = output.len(),
            response_bytes = response.len(),
            output_batches = self.perf.pty_output_batches,
            total_output_bytes = self.perf.pty_output_bytes,
            "pumped native PTY output"
        );
        Ok(output.len())
    }

    /// Resize terminal state and notify the retained PTY session.
    pub fn resize_terminal(&mut self, size: NativePtyResize) -> Result<(), NativeAppError> {
        self.terminal
            .resize_with_pixel_size(size.cols, size.rows, size.pixel_width, size.pixel_height)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.config.terminal_cols = size.cols;
        self.config.terminal_rows = size.rows;
        self.config.pixel_width = size.pixel_width;
        self.config.pixel_height = size.pixel_height;
        if let Some(session) = self.shell_session.as_mut() {
            session
                .resize(size)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        }
        self.perf.resize_events += 1;
        Ok(())
    }

    /// Reconfigure terminal dimensions, pixel size, and scrollback retention without restarting the PTY.
    pub fn reconfigure_terminal(
        &mut self,
        config: NativeTerminalRuntimeConfig,
    ) -> Result<(), NativeAppError> {
        self.terminal
            .reconfigure(config.terminal_config()?)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let resize = NativePtyResize {
            cols: config.terminal_cols,
            rows: config.terminal_rows,
            pixel_width: config.pixel_width,
            pixel_height: config.pixel_height,
        };
        if let Some(session) = self.shell_session.as_mut() {
            session
                .resize(resize)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        }
        self.config = config;
        self.perf.resize_events += 1;
        Ok(())
    }

    /// Drain PTY output at the event-loop idle boundary and request redraw only when output changed.
    pub fn pump_output_and_schedule_redraw(
        &mut self,
        lifecycle: &mut NativeAppLifecycle,
    ) -> Result<NativeAppAction, NativeAppError> {
        let pumped_bytes = self.pump_pty_output()?;
        if pumped_bytes > 0 {
            Ok(lifecycle.on_terminal_output_ready())
        } else {
            Ok(lifecycle.on_about_to_wait())
        }
    }

    /// Drain PTY output, sync terminal clipboard state, and request redraw only when output changed.
    pub fn pump_output_sync_clipboard_and_schedule_redraw<C>(
        &mut self,
        lifecycle: &mut NativeAppLifecycle,
        clipboard: &mut C,
    ) -> Result<NativeAppAction, NativeAppError>
    where
        C: HostClipboard,
    {
        let pumped_bytes = self.pump_pty_output()?;
        if pumped_bytes > 0 {
            self.sync_terminal_clipboard(clipboard);
            Ok(lifecycle.on_terminal_output_ready())
        } else {
            Ok(lifecycle.on_about_to_wait())
        }
    }
}
