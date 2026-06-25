use gromaq::{CursorStyleSetting, DEFAULT_BACKGROUND_RGB8, GromaqConfig, ThemePresetSetting};

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
        background_opacity = 0.82
        cursor_opacity = 0.73
        selection_opacity = 0.64
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
        cell_spacing_px = 2
        "##,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqDark);
    assert_eq!(config.theme.background_rgb8().unwrap(), [31, 32, 40]);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [232, 226, 214]);
    assert_eq!(config.theme.cursor_rgb8().unwrap(), [244, 192, 106]);
    assert_eq!(config.theme.selection_rgb8().unwrap(), [38, 54, 79]);
    assert_eq!(config.theme.background_opacity, 0.82);
    assert_eq!(config.theme.cursor_opacity, 0.73);
    assert_eq!(config.theme.selection_opacity, 0.64);
    assert_eq!(config.theme.cursor_style, CursorStyleSetting::Bar);
    assert!(!config.theme.cursor_blinking);
    assert_eq!(config.theme.dim_opacity, 0.72);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[0], [0, 0, 1]);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[15], [0, 0, 16]);
    assert_eq!(config.theme.surface_padding_px, 18);
    assert_eq!(config.theme.cell_spacing_px, 2);
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
    assert_eq!(config.theme.background_opacity, 1.0);
    assert_eq!(config.theme.cursor_opacity, 1.0);
    assert_eq!(config.theme.selection_opacity, 1.0);
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
    assert_eq!(config.theme.background_opacity, 1.0);
    assert_eq!(config.theme.cursor_opacity, 1.0);
    assert_eq!(config.theme.selection_opacity, 1.0);
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
    assert_eq!(config.theme.background_opacity, 1.0);
    assert_eq!(config.theme.cursor_opacity, 1.0);
    assert_eq!(config.theme.selection_opacity, 1.0);
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
        background_opacity = 0.91
        cursor_opacity = 0.8
        selection_opacity = 0.6
        cursor_blinking = false
        surface_padding_px = 20
        cell_spacing_px = 3
        "##,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqGraphite);
    assert_eq!(config.theme.background_rgb8().unwrap(), [16, 24, 32]);
    assert_eq!(config.theme.background_opacity, 0.91);
    assert_eq!(config.theme.cursor_opacity, 0.8);
    assert_eq!(config.theme.selection_opacity, 0.6);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [243, 246, 251]);
    assert!(!config.theme.cursor_blinking);
    assert_eq!(config.theme.surface_padding_px, 20);
    assert_eq!(config.theme.cell_spacing_px, 3);
}
