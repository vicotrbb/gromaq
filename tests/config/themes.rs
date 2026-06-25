use gromaq::{
    CursorStyleSetting, DEFAULT_BACKGROUND_RGB8, DEFAULT_DIM_OPACITY, GromaqConfig, GromaqError,
    ThemePresetSetting, ThemeSettings, format_theme_preset,
};

use crate::support::{assert_contrast_at_least, contrast_ratio};

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

#[test]
fn theme_preset_formatter_returns_documented_toml_names() {
    assert_eq!(
        format_theme_preset(ThemePresetSetting::GromaqDark),
        "gromaq-dark"
    );
    assert_eq!(
        format_theme_preset(ThemePresetSetting::GromaqGraphite),
        "gromaq-graphite"
    );
    assert_eq!(
        format_theme_preset(ThemePresetSetting::GromaqGhostty),
        "gromaq-ghostty"
    );
}

#[test]
fn theme_toml_config_accepts_hex_rgb_colors() {
    let config = GromaqConfig::from_toml_str(
        r##"
        [theme]
        preset = "gromaq-dark"
        background = "#1f2028"
        foreground = "#e8e2d6"
        cursor = "#f4c06a"
        selection = "#26364f"
        cursor_style = "bar"
        cursor_blinking = false
        dim_opacity = 0.72
        ansi = [
            "#000001", "#000002", "#000003", "#000004",
            "#000005", "#000006", "#000007", "#000008",
            "#000009", "#00000a", "#00000b", "#00000c",
            "#00000d", "#00000e", "#00000f", "#000010",
        ]
        surface_padding_px = 18
        "##,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqDark);
    assert_eq!(config.theme.background_rgb8().unwrap(), [31, 32, 40]);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [232, 226, 214]);
    assert_eq!(config.theme.cursor_rgb8().unwrap(), [244, 192, 106]);
    assert_eq!(config.theme.selection_rgb8().unwrap(), [38, 54, 79]);
    assert_eq!(config.theme.cursor_style, CursorStyleSetting::Bar);
    assert!(!config.theme.cursor_blinking);
    assert_eq!(config.theme.dim_opacity, 0.72);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[0], [0, 0, 1]);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[15], [0, 0, 16]);
    assert_eq!(config.theme.surface_padding_px, 18);
}

#[test]
fn theme_toml_config_accepts_named_default_preset() {
    let config = GromaqConfig::from_toml_str(
        r#"
        [theme]
        preset = "gromaq-ghostty"
        "#,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqGhostty);
    assert_eq!(
        config.theme.background_rgb8().unwrap(),
        DEFAULT_BACKGROUND_RGB8
    );
    assert_eq!(config.theme, GromaqConfig::default().theme);
}

#[test]
fn theme_toml_config_applies_named_dark_preset() {
    let config = GromaqConfig::from_toml_str(
        r#"
        [theme]
        preset = "gromaq-dark"
        "#,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqDark);
    assert_eq!(config.theme.background_rgb8().unwrap(), [23, 27, 36]);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [237, 243, 251]);
    assert_eq!(config.theme.cursor_rgb8().unwrap(), [246, 193, 119]);
    assert_eq!(config.theme.selection_rgb8().unwrap(), [51, 68, 95]);
    assert_eq!(config.theme.dim_opacity, 0.66);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[0], [42, 47, 58]);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[15], [247, 251, 255]);
}

#[test]
fn theme_toml_config_applies_named_graphite_preset() {
    let config = GromaqConfig::from_toml_str(
        r#"
        [theme]
        preset = "gromaq-graphite"
        "#,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqGraphite);
    assert_eq!(config.theme.background_rgb8().unwrap(), [11, 15, 20]);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [243, 246, 251]);
    assert_eq!(config.theme.cursor_rgb8().unwrap(), [255, 209, 102]);
    assert_eq!(config.theme.selection_rgb8().unwrap(), [38, 68, 95]);
    assert_eq!(config.theme.dim_opacity, 0.7);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[0], [31, 38, 48]);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[15], [255, 255, 255]);
}

#[test]
fn theme_toml_config_applies_named_ghostty_preset() {
    let config = GromaqConfig::from_toml_str(
        r##"
        [theme]
        preset = "gromaq-ghostty"
        "##,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqGhostty);
    assert_eq!(config.theme.background_rgb8().unwrap(), [16, 18, 22]);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [238, 244, 251]);
    assert_eq!(config.theme.cursor_rgb8().unwrap(), [246, 193, 119]);
    assert_eq!(config.theme.selection_rgb8().unwrap(), [47, 59, 82]);
    assert_eq!(config.theme.dim_opacity, 0.68);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[0], [36, 41, 51]);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[15], [247, 251, 255]);
}

#[test]
fn theme_toml_config_preserves_explicit_overrides_on_named_preset() {
    let config = GromaqConfig::from_toml_str(
        r##"
        [theme]
        preset = "gromaq-graphite"
        background = "#101820"
        cursor_blinking = false
        surface_padding_px = 20
        "##,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqGraphite);
    assert_eq!(config.theme.background_rgb8().unwrap(), [16, 24, 32]);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [243, 246, 251]);
    assert!(!config.theme.cursor_blinking);
    assert_eq!(config.theme.surface_padding_px, 20);
}

#[test]
fn invalid_theme_colors_are_rejected() {
    let invalid_cases = [
        (
            r##"
            [theme]
            background = "1f2028"
            "##,
            "background",
        ),
        (
            r##"
            [theme]
            foreground = "#zzzzzz"
            "##,
            "foreground",
        ),
        (
            r##"
            [theme]
            cursor = "#12345"
            "##,
            "cursor",
        ),
        (
            r##"
            [theme]
            selection = "#12345"
            "##,
            "selection",
        ),
    ];

    for (toml, field) in invalid_cases {
        let error = GromaqConfig::from_toml_str(toml).unwrap_err();
        assert!(matches!(
            error,
            GromaqError::InvalidThemeColor {
                field: actual_field,
                ..
            } if actual_field == field
        ));
    }
}

#[test]
fn invalid_theme_surface_padding_is_rejected() {
    let error = GromaqConfig::from_toml_str(
        r#"
        [theme]
        surface_padding_px = 513
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidThemePadding {
            maximum: 512,
            actual: 513,
        }
    ));
}

#[test]
fn invalid_theme_dim_opacity_is_rejected() {
    for dim_opacity in [0.09, f32::NAN, f32::INFINITY, 1.01] {
        let mut config = GromaqConfig::default();
        config.theme.dim_opacity = dim_opacity;

        let error = config.validate().unwrap_err();

        assert!(
            error.to_string().contains("dim opacity"),
            "{error} did not mention dim opacity"
        );
    }
}

#[test]
fn invalid_theme_ansi_palette_length_is_rejected() {
    let error = GromaqConfig::from_toml_str(
        r##"
        [theme]
        ansi = ["#000000", "#111111"]
        "##,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidThemeAnsiPaletteLength {
            expected: 16,
            actual: 2,
        }
    ));
}

fn blend_rgb8(foreground: [u8; 3], background: [u8; 3], opacity: f32) -> [u8; 3] {
    [
        blend_channel(foreground[0], background[0], opacity),
        blend_channel(foreground[1], background[1], opacity),
        blend_channel(foreground[2], background[2], opacity),
    ]
}

fn blend_channel(foreground: u8, background: u8, opacity: f32) -> u8 {
    ((f32::from(foreground) * opacity) + (f32::from(background) * (1.0 - opacity)))
        .round()
        .clamp(0.0, 255.0) as u8
}
