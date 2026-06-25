/// Terminal and PTY size requested by a native resize event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePtyResize {
    /// Terminal columns.
    pub cols: u16,
    /// Terminal rows.
    pub rows: u16,
    /// Pixel width of the PTY viewport.
    pub pixel_width: u16,
    /// Pixel height of the PTY viewport.
    pub pixel_height: u16,
}

/// Maps native window pixel sizes to terminal row/column counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeResizeGridMapper {
    cell_width_px: u16,
    line_height_px: u16,
    surface_padding_px: u16,
    cell_spacing_px: u16,
}

impl NativeResizeGridMapper {
    /// Create a mapper from non-empty rendered cell metrics.
    pub fn new(
        cell_width_px: u16,
        line_height_px: u16,
        surface_padding_px: u16,
        cell_spacing_px: u16,
    ) -> Option<Self> {
        if cell_width_px == 0 || line_height_px == 0 {
            return None;
        }
        Some(Self {
            cell_width_px,
            line_height_px,
            surface_padding_px,
            cell_spacing_px,
        })
    }

    /// Convert a native window size into a terminal and PTY resize request.
    pub fn resize_for_window(self, width_px: u32, height_px: u32) -> Option<NativePtyResize> {
        if width_px == 0 || height_px == 0 {
            return None;
        }
        let horizontal_padding = u32::from(self.surface_padding_px).saturating_mul(2);
        let vertical_padding = u32::from(self.surface_padding_px).saturating_mul(2);
        let cols = fitted_spaced_cells(
            width_px.saturating_sub(horizontal_padding),
            self.cell_width_px,
            self.cell_spacing_px,
        );
        let rows = fitted_spaced_cells(
            height_px.saturating_sub(vertical_padding),
            self.line_height_px,
            self.cell_spacing_px,
        );
        Some(NativePtyResize {
            cols,
            rows,
            pixel_width: clamp_u32_to_u16(width_px),
            pixel_height: clamp_u32_to_u16(height_px),
        })
    }
}

pub(crate) fn clamp_u32_to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

fn fitted_spaced_cells(available_px: u32, cell_px: u16, cell_spacing_px: u16) -> u16 {
    let cell_px = u32::from(cell_px);
    let spacing_px = u32::from(cell_spacing_px);
    let cells = available_px.saturating_add(spacing_px) / cell_px.saturating_add(spacing_px).max(1);
    clamp_u32_to_u16(cells.max(1))
}
