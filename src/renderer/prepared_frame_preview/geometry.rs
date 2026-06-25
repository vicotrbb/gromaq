#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct PixelRect {
    pub(super) x0: u32,
    pub(super) y0: u32,
    pub(super) x1: u32,
    pub(super) y1: u32,
}

pub(super) fn clipped_quad_rect(
    positions: impl Iterator<Item = [f32; 2]>,
    width: u32,
    height: u32,
) -> Option<PixelRect> {
    let mut min_x = f32::INFINITY;
    let mut min_y = f32::INFINITY;
    let mut max_x = f32::NEG_INFINITY;
    let mut max_y = f32::NEG_INFINITY;
    for [x, y] in positions {
        if !x.is_finite() || !y.is_finite() {
            return None;
        }
        min_x = min_x.min(x);
        min_y = min_y.min(y);
        max_x = max_x.max(x);
        max_y = max_y.max(y);
    }
    let x0 = min_x.floor().clamp(0.0, width as f32) as u32;
    let y0 = min_y.floor().clamp(0.0, height as f32) as u32;
    let x1 = max_x.ceil().clamp(0.0, width as f32) as u32;
    let y1 = max_y.ceil().clamp(0.0, height as f32) as u32;
    (x0 < x1 && y0 < y1).then_some(PixelRect { x0, y0, x1, y1 })
}
