use gromaq::{DEFAULT_DIM_OPACITY, GromaqConfig, ThemePresetSetting, ThemeSettings};

use crate::support::{assert_contrast_at_least, contrast_ratio};

use super::support::blend_rgb8;

#[test]
fn default_theme_has_high_foreground_background_contrast() {
    let theme = GromaqConfig::default().theme;

    assert_eq!(theme.dim_opacity, DEFAULT_DIM_OPACITY);

    let contrast = contrast_ratio(
        theme.foreground_rgb8().unwrap(),
        theme.background_rgb8().unwrap(),
    );

    assert!(
        contrast >= 12.0,
        "default theme contrast ratio {contrast:.2} should stay highly legible"
    );
}

#[test]
fn default_theme_has_readable_selection_contrast() {
    let theme = GromaqConfig::default().theme;

    let contrast = contrast_ratio(
        theme.foreground_rgb8().unwrap(),
        theme.selection_rgb8().unwrap(),
    );

    assert!(
        contrast >= 8.0,
        "default selection contrast ratio {contrast:.2} should stay readable"
    );
}

#[test]
fn built_in_theme_presets_keep_core_terminal_colors_legible() {
    for preset in [
        ThemePresetSetting::GromaqDark,
        ThemePresetSetting::GromaqGraphite,
        ThemePresetSetting::GromaqGhostty,
    ] {
        let theme = ThemeSettings::from_preset(preset);
        let background = theme.background_rgb8().unwrap();
        let foreground = theme.foreground_rgb8().unwrap();
        let cursor = theme.cursor_rgb8().unwrap();
        let selection = theme.selection_rgb8().unwrap();
        let ansi = theme.ansi_rgb8().unwrap();

        assert_contrast_at_least("foreground/background", foreground, background, 12.0);
        assert_contrast_at_least("cursor/background", cursor, background, 7.0);
        assert_contrast_at_least("foreground/selection", foreground, selection, 7.0);

        for color in ansi.iter().take(8).skip(1) {
            assert_contrast_at_least("ansi/background", *color, background, 6.0);
        }
        for color in ansi.iter().skip(9) {
            assert_contrast_at_least("bright ansi/background", *color, background, 7.0);
        }
    }
}

#[test]
fn built_in_theme_presets_keep_dim_text_readable() {
    for preset in [
        ThemePresetSetting::GromaqDark,
        ThemePresetSetting::GromaqGraphite,
        ThemePresetSetting::GromaqGhostty,
    ] {
        let theme = ThemeSettings::from_preset(preset);
        let background = theme.background_rgb8().unwrap();
        let foreground = theme.foreground_rgb8().unwrap();
        let dim_foreground = blend_rgb8(foreground, background, theme.dim_opacity);

        assert_contrast_at_least("dim foreground/background", dim_foreground, background, 7.0);
    }
}
