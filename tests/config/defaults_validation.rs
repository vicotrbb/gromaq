use gromaq::{GromaqConfig, TerminalConfig};

#[test]
fn default_config_is_valid() {
    GromaqConfig::default().validate().unwrap();
}

#[test]
fn default_font_metrics_are_readable_for_native_terminal_windows() {
    let font = GromaqConfig::default().font;

    assert_eq!(font.size_px, 37.0);
    assert_eq!(font.renderer_font_size_px(), 37);
    assert_eq!(font.renderer_cell_width_px(), 21);
    assert_eq!(font.renderer_line_height_px(), 51);

    let width_ratio = f32::from(font.renderer_cell_width_px()) / font.size_px;
    let line_height_ratio = f32::from(font.renderer_line_height_px()) / font.size_px;

    assert!(
        (0.56..=0.58).contains(&width_ratio),
        "default terminal cell width ratio {width_ratio:.2} should stay readable without excessive letter spacing"
    );
    assert!(
        (1.34..=1.40).contains(&line_height_ratio),
        "default terminal line height ratio {line_height_ratio:.2} should keep dark-theme text legible"
    );
}

#[test]
fn default_theme_metrics_keep_terminal_content_away_from_window_edges() {
    let theme = GromaqConfig::default().theme;

    assert_eq!(theme.surface_padding_px, 14);
    assert_eq!(theme.cell_spacing_px, 0);
    assert_eq!(theme.background, "#101216");
    assert_eq!(theme.foreground, "#eef4fb");
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
fn oversized_terminal_grid_is_rejected_before_allocation() {
    let terminal_error = TerminalConfig::new(u16::MAX, u16::MAX).unwrap_err();
    assert!(terminal_error.to_string().contains("terminal grid"));

    let mut config = GromaqConfig::default();
    config.terminal.cols = u16::MAX;
    config.terminal.rows = u16::MAX;

    let config_error = config.validate().unwrap_err();
    assert!(config_error.to_string().contains("terminal grid"));
}
