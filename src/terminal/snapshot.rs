use crate::cell::{CellSnapshot, Color, Style};

pub(super) fn cell_screenshot_color(cell: &CellSnapshot) -> [u8; 4] {
    if cell.is_wide_trailing {
        return [255, 255, 255, 255];
    }
    if cell.text.is_empty() {
        return color_to_rgba(cell.style.background, [0, 0, 0, 255]);
    }
    color_to_rgba(cell.style.foreground, [255, 255, 255, 255])
}

fn color_to_rgba(color: Color, default: [u8; 4]) -> [u8; 4] {
    match color {
        Color::Default => default,
        Color::Ansi(index) => ansi_color_to_rgba(index),
        Color::Indexed(index) => indexed_color_to_rgba(index),
        Color::Rgb(red, green, blue) => [red, green, blue, 255],
    }
}

fn ansi_color_to_rgba(index: u8) -> [u8; 4] {
    const ANSI: [[u8; 4]; 16] = [
        [0, 0, 0, 255],
        [205, 49, 49, 255],
        [13, 188, 121, 255],
        [229, 229, 16, 255],
        [36, 114, 200, 255],
        [188, 63, 188, 255],
        [17, 168, 205, 255],
        [229, 229, 229, 255],
        [102, 102, 102, 255],
        [241, 76, 76, 255],
        [35, 209, 139, 255],
        [245, 245, 67, 255],
        [59, 142, 234, 255],
        [214, 112, 214, 255],
        [41, 184, 219, 255],
        [255, 255, 255, 255],
    ];
    ANSI[usize::from(index.min(15))]
}

fn indexed_color_to_rgba(index: u8) -> [u8; 4] {
    if index < 16 {
        return ansi_color_to_rgba(index);
    }
    if index < 232 {
        let offset = index - 16;
        let red = color_cube_component(offset / 36);
        let green = color_cube_component((offset / 6) % 6);
        let blue = color_cube_component(offset % 6);
        return [red, green, blue, 255];
    }
    let gray = 8 + (index - 232) * 10;
    [gray, gray, gray, 255]
}

fn color_cube_component(value: u8) -> u8 {
    if value == 0 { 0 } else { 55 + value * 40 }
}

pub(super) fn push_snapshot_row(
    target: &mut Vec<CellSnapshot>,
    row: Option<&[CellSnapshot]>,
    cols: u16,
) {
    let blank = CellSnapshot {
        text: String::new(),
        style: Style::default(),
        hyperlink_id: 0,
        is_wide_leading: false,
        is_wide_trailing: false,
    };
    for col in 0..usize::from(cols) {
        target.push(
            row.and_then(|cells| cells.get(col))
                .cloned()
                .unwrap_or_else(|| blank.clone()),
        );
    }
}
