use std::fs;

use gromaq::{
    ConfigFileReloader, CursorStyleSetting, DEFAULT_BACKGROUND_RGB8, GromaqConfig, GromaqError,
    ShellSettings, TerminalConfig, ThemePresetSetting,
};

#[test]
fn default_config_is_valid() {
    GromaqConfig::default().validate().unwrap();
}

#[test]
fn default_font_metrics_are_readable_for_native_terminal_windows() {
    let font = GromaqConfig::default().font;

    assert_eq!(font.size_px, 18.0);
    assert_eq!(font.renderer_font_size_px(), 18);
    assert_eq!(font.renderer_cell_width_px(), 10);
    assert_eq!(font.renderer_line_height_px(), 25);

    let width_ratio = f32::from(font.renderer_cell_width_px()) / font.size_px;
    let line_height_ratio = f32::from(font.renderer_line_height_px()) / font.size_px;

    assert!(
        (0.54..=0.58).contains(&width_ratio),
        "default terminal cell width ratio {width_ratio:.2} should stay readable without excessive letter spacing"
    );
    assert!(
        (1.35..=1.42).contains(&line_height_ratio),
        "default terminal line height ratio {line_height_ratio:.2} should keep dark-theme text legible"
    );
}

#[test]
fn default_theme_has_high_foreground_background_contrast() {
    let theme = GromaqConfig::default().theme;

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
fn invalid_dimensions_are_rejected() {
    let mut config = GromaqConfig::default();
    config.terminal.cols = 0;

    let error = config.validate().unwrap_err();
    assert!(error.to_string().contains("columns"));
}

#[test]
fn invalid_frame_target_is_rejected() {
    for target_fps in [0, 1_001] {
        let mut config = GromaqConfig::default();
        config.performance.target_fps = target_fps;

        let error = config.validate().unwrap_err();
        assert!(error.to_string().contains("target fps"));
    }
}

#[test]
fn invalid_font_sizes_are_rejected() {
    for size_px in [5.9, f32::NAN, f32::INFINITY, 513.0] {
        let mut config = GromaqConfig::default();
        config.font.size_px = size_px;

        let error = config.validate().unwrap_err();

        assert!(error.to_string().contains("font size"));
    }
}

#[test]
fn invalid_cell_widths_are_rejected() {
    for cell_width_px in [3.9, f32::NAN, f32::INFINITY, 513.0] {
        let mut config = GromaqConfig::default();
        config.font.cell_width_px = Some(cell_width_px);

        let error = config.validate().unwrap_err();

        assert!(error.to_string().contains("cell width"));
    }
}

#[test]
fn invalid_line_heights_are_rejected() {
    for line_height_px in [5.9, f32::NAN, f32::INFINITY, 1025.0] {
        let mut config = GromaqConfig::default();
        config.font.line_height_px = line_height_px;

        let error = config.validate().unwrap_err();

        assert!(error.to_string().contains("line height"));
    }

    let mut config = GromaqConfig::default();
    config.font.size_px = 18.0;
    config.font.line_height_px = 17.0;

    let error = config.validate().unwrap_err();

    assert!(error.to_string().contains("line height"));
}

#[test]
fn font_settings_round_renderer_font_size_for_cache_keys() {
    let mut config = GromaqConfig::default();
    config.font.size_px = 16.5;
    config.font.cell_width_px = Some(9.5);
    config.font.line_height_px = 20.5;

    config.validate().unwrap();

    assert_eq!(config.font.renderer_font_size_px(), 17);
    assert_eq!(config.font.renderer_cell_width_px(), 10);
    assert_eq!(config.font.renderer_line_height_px(), 21);
}

#[test]
fn partial_toml_config_uses_defaults_and_validates() {
    let config = GromaqConfig::from_toml_str(
        r#"
        [terminal]
        cols = 100

        [font]
        family = "JetBrains Mono"
        "#,
    )
    .unwrap();

    assert_eq!(config.terminal.cols, 100);
    assert_eq!(config.terminal.rows, GromaqConfig::default().terminal.rows);
    assert_eq!(
        config.terminal.scrollback_lines,
        GromaqConfig::default().terminal.scrollback_lines
    );
    assert_eq!(config.font.family, "JetBrains Mono");
    assert_eq!(config.shell, ShellSettings::default());
    assert_eq!(
        config.performance.target_fps,
        GromaqConfig::default().performance.target_fps
    );
    assert_eq!(config.theme, GromaqConfig::default().theme);
}

#[test]
fn toml_config_validation_rejects_invalid_values() {
    let error = GromaqConfig::from_toml_str(
        r#"
        [terminal]
        cols = 0
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidDimension {
            field: "columns",
            ..
        }
    ));
}

#[test]
fn shell_toml_config_accepts_program_args_and_cwd() {
    let config = GromaqConfig::from_toml_str(
        r#"
        [shell]
        program = "/bin/zsh"
        args = ["-l", "-i"]
        cwd = "/tmp"
        "#,
    )
    .unwrap();

    assert_eq!(config.shell.program.as_deref(), Some("/bin/zsh"));
    assert_eq!(config.shell.args, ["-l", "-i"]);
    assert_eq!(config.shell.cwd.as_deref(), Some("/tmp"));
}

#[test]
fn invalid_shell_settings_are_rejected() {
    let invalid_cases = [
        (
            r#"
            [shell]
            program = "   "
            "#,
            "shell program",
        ),
        (
            r#"
            [shell]
            args = [""]
            "#,
            "shell argument",
        ),
        (
            r#"
            [shell]
            cwd = "   "
            "#,
            "shell working directory",
        ),
    ];

    for (toml, expected_message) in invalid_cases {
        let error = GromaqConfig::from_toml_str(toml).unwrap_err();
        assert!(
            error.to_string().contains(expected_message),
            "{error} did not contain {expected_message}"
        );
    }
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
    assert_eq!(config.theme.ansi_rgb8().unwrap()[0], [0, 0, 1]);
    assert_eq!(config.theme.ansi_rgb8().unwrap()[15], [0, 0, 16]);
    assert_eq!(config.theme.surface_padding_px, 18);
}

#[test]
fn theme_toml_config_accepts_named_default_preset() {
    let config = GromaqConfig::from_toml_str(
        r#"
        [theme]
        preset = "gromaq-dark"
        "#,
    )
    .unwrap();

    assert_eq!(config.theme.preset, ThemePresetSetting::GromaqDark);
    assert_eq!(
        config.theme.background_rgb8().unwrap(),
        DEFAULT_BACKGROUND_RGB8
    );
    assert_eq!(config.theme, GromaqConfig::default().theme);
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

fn contrast_ratio(foreground: [u8; 3], background: [u8; 3]) -> f64 {
    let foreground = relative_luminance(foreground);
    let background = relative_luminance(background);
    let lighter = foreground.max(background);
    let darker = foreground.min(background);
    (lighter + 0.05) / (darker + 0.05)
}

fn relative_luminance([red, green, blue]: [u8; 3]) -> f64 {
    let [red, green, blue] = [
        srgb_component(red),
        srgb_component(green),
        srgb_component(blue),
    ];
    (0.2126 * red) + (0.7152 * green) + (0.0722 * blue)
}

fn srgb_component(component: u8) -> f64 {
    let value = f64::from(component) / 255.0;
    if value <= 0.03928 {
        value / 12.92
    } else {
        ((value + 0.055) / 1.055).powf(2.4)
    }
}

#[test]
fn malformed_toml_config_reports_parse_error() {
    let error = GromaqConfig::from_toml_str("[terminal").unwrap_err();

    assert!(matches!(error, GromaqError::ConfigParse { .. }));
}

#[test]
fn config_can_be_loaded_from_toml_file() {
    let path = test_config_path("gromaq-config-load.toml");
    fs::write(
        &path,
        r#"
        [terminal]
        rows = 48

        [performance]
        target_fps = 120
        "#,
    )
    .unwrap();

    let config = GromaqConfig::from_toml_file(&path).unwrap();

    assert_eq!(config.terminal.rows, 48);
    assert_eq!(config.performance.target_fps, 120);
    let _ = fs::remove_file(path);
}

#[test]
fn missing_config_file_reports_read_error() {
    let path = test_config_path("missing-gromaq-config.toml");
    let _ = fs::remove_file(&path);

    let error = GromaqConfig::from_toml_file(&path).unwrap_err();

    assert!(matches!(error, GromaqError::ConfigRead { .. }));
}

#[test]
fn config_file_reloader_reports_unchanged_valid_file() {
    let path = test_config_path("gromaq-config-reload-unchanged.toml");
    fs::write(
        &path,
        r#"
        [terminal]
        cols = 100
        "#,
    )
    .unwrap();
    let mut reloader = ConfigFileReloader::from_file(path.clone()).unwrap();

    let reload = reloader.reload_if_changed().unwrap();

    assert!(!reload.changed);
    assert_eq!(reload.config.terminal.cols, 100);
    assert_eq!(reloader.current().terminal.cols, 100);
    assert_eq!(reloader.path(), path.as_path());
    let _ = fs::remove_file(path);
}

#[test]
fn config_file_reloader_applies_changed_valid_file() {
    let path = test_config_path("gromaq-config-reload-changed.toml");
    fs::write(
        &path,
        r#"
        [terminal]
        cols = 100
        "#,
    )
    .unwrap();
    let mut reloader = ConfigFileReloader::from_file(path.clone()).unwrap();
    fs::write(
        &path,
        r#"
        [terminal]
        cols = 132
        rows = 40
        "#,
    )
    .unwrap();

    let reload = reloader.reload_if_changed().unwrap();

    assert!(reload.changed);
    assert_eq!(reload.config.terminal.cols, 132);
    assert_eq!(reload.config.terminal.rows, 40);
    assert_eq!(reloader.current().terminal.cols, 132);
    assert_eq!(reloader.current().terminal.rows, 40);
    let _ = fs::remove_file(path);
}

#[test]
fn config_file_reloader_preserves_previous_config_when_changed_file_is_invalid() {
    let path = test_config_path("gromaq-config-reload-invalid.toml");
    fs::write(
        &path,
        r#"
        [terminal]
        cols = 100
        "#,
    )
    .unwrap();
    let mut reloader = ConfigFileReloader::from_file(path.clone()).unwrap();
    fs::write(
        &path,
        r#"
        [terminal]
        cols = 0
        "#,
    )
    .unwrap();

    let error = reloader.reload_if_changed().unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidDimension {
            field: "columns",
            ..
        }
    ));
    assert_eq!(reloader.current().terminal.cols, 100);
    let _ = fs::remove_file(path);
}

#[test]
fn config_serializes_to_valid_pretty_toml() {
    let mut config = GromaqConfig::default();
    config.terminal.cols = 96;
    config.shell.program = Some("/bin/zsh".to_owned());
    config.shell.args = vec!["-l".to_owned()];
    config.shell.cwd = Some("/tmp".to_owned());
    config.font.family = "Gromaq Mono".to_owned();

    let toml = config.to_toml_string().unwrap();
    let parsed = GromaqConfig::from_toml_str(&toml).unwrap();

    assert!(toml.contains("[terminal]"));
    assert!(toml.contains("[shell]"));
    assert!(toml.contains("[theme]"));
    assert!(toml.contains("preset = \"gromaq-dark\""));
    assert_eq!(parsed, config);
}

#[test]
fn oversized_terminal_grid_is_rejected_before_allocation() {
    let terminal_error = TerminalConfig::new(u16::MAX, u16::MAX).unwrap_err();
    assert!(terminal_error.to_string().contains("terminal grid"));

    let mut config = GromaqConfig::default();
    config.terminal.cols = u16::MAX;
    config.terminal.rows = u16::MAX;

    let config_error = config.validate().unwrap_err();
    assert!(config_error.to_string().contains("terminal grid"));
}

fn test_config_path(name: &str) -> std::path::PathBuf {
    let directory = std::env::current_dir()
        .unwrap()
        .join("target")
        .join("gromaq-config-tests");
    fs::create_dir_all(&directory).unwrap();
    directory.join(format!("{}-{name}", std::process::id()))
}
