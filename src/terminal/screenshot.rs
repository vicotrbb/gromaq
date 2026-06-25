use super::Terminal;
use super::snapshot::cell_screenshot_color;
use super::types::Screenshot;

impl Terminal {
    /// Return a deterministic one-pixel-per-cell RGBA screenshot of the visible grid.
    pub fn screenshot(&self) -> Screenshot {
        let width = u32::from(self.config.cols);
        let height = u32::from(self.config.rows);
        let mut rgba =
            Vec::with_capacity(usize::from(self.config.cols) * usize::from(self.config.rows) * 4);
        let grid = self.dump_grid();
        let cursor = self.dump_cursor();
        for row in 0..self.config.rows {
            for col in 0..self.config.cols {
                let color = if cursor.visible && cursor.row == row && cursor.col == col {
                    [64, 160, 255, 255]
                } else {
                    cell_screenshot_color(grid.cell(row, col))
                };
                rgba.extend_from_slice(&color);
            }
        }
        Screenshot {
            width,
            height,
            rgba,
        }
    }
}
