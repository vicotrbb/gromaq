use gromaq::GromaqConfig;

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
