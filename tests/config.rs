use std::fs;

use gromaq::{ConfigFileReloader, GromaqConfig, GromaqError, ShellSettings, TerminalConfig};

#[test]
fn default_config_is_valid() {
    GromaqConfig::default().validate().unwrap();
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
fn font_settings_round_renderer_font_size_for_cache_keys() {
    let mut config = GromaqConfig::default();
    config.font.size_px = 16.5;

    config.validate().unwrap();

    assert_eq!(config.font.renderer_font_size_px(), 17);
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
        background = "#1f2028"
        foreground = "#e8e2d6"
        cursor = "#f4c06a"
        "##,
    )
    .unwrap();

    assert_eq!(config.theme.background_rgb8().unwrap(), [31, 32, 40]);
    assert_eq!(config.theme.foreground_rgb8().unwrap(), [232, 226, 214]);
    assert_eq!(config.theme.cursor_rgb8().unwrap(), [244, 192, 106]);
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
