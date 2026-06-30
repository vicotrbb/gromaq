use std::fs;

use gromaq::{ConfigFileReloader, GromaqConfig, GromaqError};

use crate::support::test_config_path;

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
    config.font.fallback_families = vec!["Apple Color Emoji".to_owned()];

    let toml = config.to_toml_string().unwrap();
    let parsed = GromaqConfig::from_toml_str(&toml).unwrap();

    assert!(toml.contains("[terminal]"));
    assert!(toml.contains("[shell]"));
    assert!(toml.contains("[theme]"));
    assert!(toml.contains("fallback_families = [\"Apple Color Emoji\"]"));
    assert!(toml.contains("preset = \"gromaq-ghostty\""));
    assert_eq!(parsed, config);
}

#[test]
fn config_parses_tmux_workspace_presets() {
    let config = GromaqConfig::from_toml_str(
        r#"
        [tmux]
        enabled = true

        [tmux.workspaces.gromaq]
        session = "gromaq"
        root = "/Users/victorbona/Daedalus/gromaq"

        [[tmux.workspaces.gromaq.windows]]
        name = "code"
        panes = ["$SHELL"]

        [[tmux.workspaces.gromaq.windows]]
        name = "test"
        panes = ["cargo test --all", "cargo run -- --runtime-tool-workflow-smoke"]
        "#,
    )
    .unwrap();

    let workspace = config.tmux.workspaces.get("gromaq").unwrap();

    assert!(config.tmux.enabled);
    assert_eq!(workspace.session, "gromaq");
    assert_eq!(
        workspace.root.as_deref(),
        Some("/Users/victorbona/Daedalus/gromaq")
    );
    assert_eq!(workspace.windows.len(), 2);
    assert_eq!(workspace.windows[1].name, "test");
    assert_eq!(workspace.windows[1].panes.len(), 2);
}

#[test]
fn config_rejects_empty_tmux_workspace_pane_command() {
    let error = GromaqConfig::from_toml_str(
        r#"
        [tmux.workspaces.bad]
        session = "bad"

        [[tmux.workspaces.bad.windows]]
        name = "code"
        panes = [""]
        "#,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        GromaqError::InvalidTmuxWorkspace {
            workspace,
            field: "panes[0]",
        } if workspace == "bad"
    ));
}
