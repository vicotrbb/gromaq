use std::collections::HashSet;

use super::super::{WELCOME_AVATAR_ANSI, layout::ansi_visible_width};
use super::support::{avatar_sgr_rgb_colors, contrast_ratio, is_braille_cell};

#[test]
fn default_welcome_avatar_is_trimmed_and_uses_supported_terminal_glyphs() {
    let lines: Vec<_> = WELCOME_AVATAR_ANSI.lines().collect();
    let widths: Vec<_> = lines.iter().map(|line| ansi_visible_width(line)).collect();
    let max_width = widths.iter().copied().max().unwrap_or(0);

    assert_eq!(lines.len(), 17);
    assert_eq!(max_width, 35);
    assert!(widths.iter().all(|width| *width == 35));
    assert!(WELCOME_AVATAR_ANSI.chars().any(is_braille_cell));

    // The avatar is baked for the default gromaq cell (18x44px). A near-square
    // source must render wider-than-tall in cell counts to avoid the vertical
    // stretching that left the old avatar 39% too narrow. The wider 17-row
    // avatar keeps extra subcell detail while staying inside the narrow
    // welcome layout.
    let ratio = max_width as f64 / lines.len() as f64;
    assert!(ratio >= 1.9, "avatar aspect ratio regressed: {ratio}");
}

#[test]
fn default_welcome_avatar_keeps_dense_color_detail() {
    let colors = avatar_sgr_rgb_colors(WELCOME_AVATAR_ANSI);
    let unique_colors: HashSet<_> = colors.iter().copied().collect();

    assert!(
        colors.len() >= 260,
        "avatar foreground detail too sparse: {} colored cells",
        colors.len()
    );
    assert!(
        unique_colors.len() >= 250,
        "avatar color detail too low: {} unique foreground colors",
        unique_colors.len()
    );
}

#[test]
fn default_welcome_avatar_uses_subcell_glyph_detail() {
    let braille_cells = WELCOME_AVATAR_ANSI
        .chars()
        .filter(|ch| is_braille_cell(*ch))
        .count();

    assert!(
        braille_cells >= 300,
        "welcome avatar is still too blocky: {braille_cells} braille cells"
    );
}

#[test]
fn default_welcome_avatar_keeps_readable_contrast_on_dark_background() {
    // The avatar is baked for the default gromaq-ghostty background (#101216).
    // Guard the luminance floor added to boostTerminalColor so the avatar cannot
    // silently regress to the muddy low-contrast output (min 1.13:1, 83% below
    // 3:1) that left it nearly invisible on the dark background.
    const BACKGROUND: [u8; 3] = [0x10, 0x12, 0x16];
    const MIN_CONTRAST: f64 = 3.0;

    let ratios: Vec<f64> = avatar_sgr_rgb_colors(WELCOME_AVATAR_ANSI)
        .into_iter()
        .map(|color| contrast_ratio(color, BACKGROUND))
        .collect();

    assert!(!ratios.is_empty(), "avatar must define truecolor cells");
    let mean = ratios.iter().sum::<f64>() / ratios.len() as f64;
    let readable = ratios
        .iter()
        .filter(|&&ratio| ratio >= MIN_CONTRAST)
        .count();
    assert!(
        mean >= MIN_CONTRAST,
        "avatar mean contrast {mean:.2} below {MIN_CONTRAST}:1"
    );
    assert!(
        readable * 100 >= ratios.len() * 85,
        "only {readable}/{} avatar cells reach {MIN_CONTRAST}:1",
        ratios.len()
    );
}
