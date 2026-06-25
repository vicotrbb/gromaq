use super::*;

#[test]
fn native_terminal_app_new_rejects_zero_window_reference_size() {
    let config = NativeAppConfig {
        width: 0,
        ..NativeAppConfig::default()
    };
    let error = match NativeTerminalApp::new(config) {
        Ok(_) => panic!("zero-width native app config should be rejected"),
        Err(error) => error,
    };

    assert_eq!(
        error.to_string(),
        "native runtime failed: native window dimensions must be non-zero"
    );
}

#[test]
fn native_terminal_app_new_loads_default_glyph_cache() {
    let app = NativeTerminalApp::new(NativeAppConfig::default()).unwrap();

    assert!(app.glyph_cache.is_empty());
}

#[test]
fn native_terminal_app_new_writes_default_welcome_screen() {
    let app = NativeTerminalApp::new(NativeAppConfig::default()).unwrap();
    let visible = visible_runtime_text(&app);

    assert!(visible.contains("Gromaq"));
    assert!(visible.contains("native Rust GPU terminal"));
    assert!(visible.contains("System"));
    assert!(visible.contains("Terminal"));
    assert!(visible.contains("Renderer"));
    assert!(visible.contains("Theme"));
}

#[test]
fn native_terminal_app_new_uses_explicit_startup_text_over_default_welcome() {
    let app = NativeTerminalApp::new(NativeAppConfig {
        startup_text: Some("custom startup\r\n".to_owned()),
        ..NativeAppConfig::default()
    })
    .unwrap();
    let visible = visible_runtime_text(&app);

    assert!(visible.contains("custom startup"));
    assert!(!visible.contains("native Rust GPU terminal"));
}

#[test]
fn native_terminal_app_new_can_disable_default_welcome_screen() {
    let app = NativeTerminalApp::new(NativeAppConfig {
        welcome_screen: false,
        ..NativeAppConfig::default()
    })
    .unwrap();
    let visible = visible_runtime_text(&app);

    assert!(!visible.contains("Gromaq"));
    assert!(!visible.contains("native Rust GPU terminal"));
}

#[test]
fn native_terminal_app_can_sync_runtime_to_actual_window_pixels() {
    let mut app = NativeTerminalApp::new(NativeAppConfig::default()).unwrap();
    let expected_resize = app.resize_mapper.resize_for_window(2560, 1600).unwrap();

    app.resize_runtime_to_window_pixels(2560, 1600).unwrap();

    assert_eq!(app.runtime.config().pixel_width, 2560);
    assert_eq!(app.runtime.config().pixel_height, 1600);
    assert_eq!(
        app.runtime.terminal().dump_grid().cols,
        expected_resize.cols
    );
    assert_eq!(
        app.runtime.terminal().dump_grid().rows,
        expected_resize.rows
    );
}

fn visible_runtime_text(app: &NativeTerminalApp) -> String {
    let grid = app.runtime().terminal().dump_grid();
    (0..grid.rows)
        .map(|row| grid.line_text(row))
        .collect::<Vec<_>>()
        .join("\n")
}
