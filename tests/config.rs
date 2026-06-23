use gromaq::{GromaqConfig, TerminalConfig};

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
    let mut config = GromaqConfig::default();
    config.performance.target_fps = 0;

    let error = config.validate().unwrap_err();
    assert!(error.to_string().contains("target fps"));
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
