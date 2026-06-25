use crate::error::Result;

use super::params::default_tab_stops;
use super::{Terminal, TerminalConfig, reflow};

impl Terminal {
    /// Resize the visible grid while preserving top-left content.
    pub fn resize(&mut self, cols: u16, rows: u16) -> Result<()> {
        self.resize_with_pixel_size(
            cols,
            rows,
            self.config.pixel_width,
            self.config.pixel_height,
        )
    }

    /// Resize the visible grid and update native pixel dimensions.
    pub fn resize_with_pixel_size(
        &mut self,
        cols: u16,
        rows: u16,
        pixel_width: u16,
        pixel_height: u16,
    ) -> Result<()> {
        let config = TerminalConfig {
            cols,
            rows,
            pixel_width,
            pixel_height,
            scrollback_limit: self.config.scrollback_limit,
            cursor_shape: self.config.cursor_shape,
            cursor_blinking: self.config.cursor_blinking,
        }
        .validate()?;
        self.reconfigure(config)
    }

    /// Reconfigure terminal dimensions, pixel size, and scrollback retention.
    pub fn reconfigure(&mut self, config: TerminalConfig) -> Result<()> {
        let config = config.validate()?;
        self.flush_dirty_run();
        self.scrollback.set_limit(config.scrollback_limit);
        self.scrollback.reflow(self.config.cols, config.cols);
        let (grid, hard_breaks) = self.reflow_visible_grid(config.cols, config.rows);
        self.grid = grid;
        self.hard_breaks = hard_breaks;
        if let Some(saved) = &mut self.saved_primary {
            let (grid, hard_breaks) =
                reflow::reflow_grid(&saved.grid, &saved.hard_breaks, config.cols, config.rows);
            saved.grid = grid;
            saved.hard_breaks = hard_breaks;
            saved.tab_stops = default_tab_stops(config.cols);
            saved.scroll_top = 0;
            saved.scroll_bottom = config.rows - 1;
            saved.cursor.row = saved.cursor.row.min(config.rows - 1);
            saved.cursor.col = saved.cursor.col.min(config.cols - 1);
            saved.wrap_pending = false;
        }
        self.tab_stops = default_tab_stops(config.cols);
        self.scroll_top = 0;
        self.scroll_bottom = config.rows - 1;
        self.cursor.row = self.cursor.row.min(config.rows - 1);
        self.cursor.col = self.cursor.col.min(config.cols - 1);
        if self.config.cursor_shape != config.cursor_shape
            || self.config.cursor_blinking != config.cursor_blinking
        {
            self.cursor.shape = config.cursor_shape;
            self.cursor.blinking = config.cursor_blinking;
        }
        self.wrap_pending = false;
        self.scrollback_view_offset = 0;
        self.selection = None;
        self.dirty.mark_viewport(config.rows, config.cols);
        self.config = config;
        self.perf.resizes += 1;
        Ok(())
    }
}
