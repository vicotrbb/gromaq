use gromaq::{GromaqConfig, GromaqError, ShellSettings};

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
