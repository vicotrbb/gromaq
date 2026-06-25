const DEFAULT_FONT_SIZE_MIN_PX: u16 = 37;
const DEFAULT_CELL_WIDTH_MIN_PX: u16 = 21;
const DEFAULT_LINE_HEIGHT_MIN_PX: u16 = 51;
const DEFAULT_CELL_WIDTH_RATIO_MIN_X100: u64 = 54;
const DEFAULT_CELL_WIDTH_RATIO_MAX_X100: u64 = 62;
const DEFAULT_LINE_HEIGHT_RATIO_MIN_X100: u64 = 130;
const DEFAULT_LINE_HEIGHT_RATIO_MAX_X100: u64 = 145;

pub(super) fn has_readable_default_font_metrics(
    font_size_px: u16,
    cell_width_px: u16,
    line_height_px: u16,
) -> bool {
    if font_size_px < DEFAULT_FONT_SIZE_MIN_PX
        || cell_width_px < DEFAULT_CELL_WIDTH_MIN_PX
        || line_height_px < DEFAULT_LINE_HEIGHT_MIN_PX
    {
        return false;
    }
    let cell_width_ratio_x100 = ratio_x100(cell_width_px, font_size_px);
    let line_height_ratio_x100 = ratio_x100(line_height_px, font_size_px);
    (DEFAULT_CELL_WIDTH_RATIO_MIN_X100..=DEFAULT_CELL_WIDTH_RATIO_MAX_X100)
        .contains(&cell_width_ratio_x100)
        && (DEFAULT_LINE_HEIGHT_RATIO_MIN_X100..=DEFAULT_LINE_HEIGHT_RATIO_MAX_X100)
            .contains(&line_height_ratio_x100)
}

fn ratio_x100(numerator: u16, denominator: u16) -> u64 {
    if denominator == 0 {
        return 0;
    }
    ((f64::from(numerator) / f64::from(denominator)) * 100.0).round() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_metrics_gate_accepts_current_readable_defaults() {
        assert!(has_readable_default_font_metrics(37, 21, 51));
    }

    #[test]
    fn default_font_metrics_gate_rejects_tiny_defaults() {
        assert!(!has_readable_default_font_metrics(24, 13, 32));
    }

    #[test]
    fn default_font_metrics_gate_rejects_cramped_or_loose_geometry() {
        assert!(!has_readable_default_font_metrics(37, 16, 51));
        assert!(!has_readable_default_font_metrics(37, 21, 60));
    }
}
