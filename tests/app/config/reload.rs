use super::*;

#[test]
fn native_app_applies_reloadable_gromaq_render_config_without_restarting_runtime() {
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig {
            clear_color: [0.1, 0.2, 0.3, 1.0],
            ..RendererConfig::default()
        },
    )
    .unwrap();
    let mut config = GromaqConfig::default();
    config.performance.target_fps = 120;
    config.performance.dirty_region_rendering = false;
    config.font.size_px = 18.0;
    config.font.line_height_px = 22.0;
    config.theme.background = "#1f2028".to_owned();
    config.theme.foreground = "#e8e2d6".to_owned();
    config.theme.cursor = "#f4c06a".to_owned();
    config.theme.surface_padding_px = 18;
    config.theme.dim_opacity = 0.42;

    app.apply_reloadable_gromaq_config(&config).unwrap();

    assert_eq!(app.lifecycle().config().target_fps, 120);
    assert_eq!(app.renderer().config().target_fps, 120);
    assert!(!app.renderer().config().dirty_regions);
    assert_eq!(app.renderer().config().font_size_px, 18);
    assert_eq!(app.renderer().config().cell_width_px, 10);
    assert_eq!(app.renderer().config().line_height_px, 22);
    assert_eq!(
        app.renderer().config().clear_color,
        linear_clear_color(31, 32, 40)
    );
    assert_eq!(
        app.renderer().config().default_foreground_rgb8,
        [232, 226, 214]
    );
    assert_eq!(
        app.renderer().config().cursor_color_rgba8,
        [244, 192, 106, 255]
    );
    assert_eq!(app.renderer().config().surface_padding_px, 18);
    assert_eq!(app.renderer().config().dim_opacity, 0.42);
    assert_eq!(
        (
            app.runtime().terminal().dump_grid().cols,
            app.runtime().terminal().dump_grid().rows,
        ),
        expected_grid_for_window(1280, 800, app.renderer().config())
    );
    assert!(!app.runtime().has_shell_session());
}

#[test]
fn native_app_applies_reloadable_font_file_path_without_restarting_runtime() {
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig::default(),
    )
    .unwrap();
    let font_path = system_mono_font_path();
    let mut config = GromaqConfig::default();
    config.font.family = font_path.to_string_lossy().into_owned();

    app.apply_reloadable_gromaq_config(&config).unwrap();

    assert_eq!(app.font_family(), font_path.to_string_lossy());
    assert!(!app.runtime().has_shell_session());
}

#[test]
fn native_app_applies_reloadable_terminal_config_without_restarting_runtime() {
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig {
            terminal_cols: 20,
            terminal_rows: 4,
            scrollback_lines: 100,
            pixel_width: 0,
            pixel_height: 0,
            cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
            cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
            shell: ShellCommand {
                program: "/bin/sh".into(),
                args: Vec::new(),
                cwd: None,
            },
        },
        RendererConfig::default(),
    )
    .unwrap();
    let mut config = GromaqConfig::default();
    config.terminal.cols = 12;
    config.terminal.rows = 3;
    config.terminal.scrollback_lines = 16;
    config.shell.program = Some("/bin/zsh".to_owned());
    config.theme.cursor_style = CursorStyleSetting::Underline;
    config.theme.cursor_blinking = false;

    app.apply_reloadable_gromaq_config(&config).unwrap();

    assert_eq!(
        (
            app.runtime().terminal().dump_grid().cols,
            app.runtime().terminal().dump_grid().rows,
        ),
        expected_grid_for_window(1280, 800, &RendererConfig::default())
    );
    assert_eq!(
        (
            app.runtime().config().terminal_cols,
            app.runtime().config().terminal_rows,
        ),
        expected_grid_for_window(1280, 800, &RendererConfig::default())
    );
    assert_eq!(app.runtime().config().scrollback_lines, 16);
    assert_eq!(
        app.runtime().config().shell.program,
        PathBuf::from("/bin/zsh")
    );
    assert_eq!(
        app.runtime().terminal().dump_cursor().shape,
        CursorShape::Underline
    );
    assert!(!app.runtime().terminal().dump_cursor().blinking);
    assert_eq!(app.runtime().dump_runtime_perf_metrics().resize_events, 1);
    assert!(!app.runtime().has_shell_session());
}

#[test]
fn native_app_applies_reloadable_shell_config_before_runtime_starts() {
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig {
            shell: ShellCommand {
                program: "/bin/sh".into(),
                args: Vec::new(),
                cwd: None,
            },
            ..NativeTerminalRuntimeConfig::default()
        },
        RendererConfig::default(),
    )
    .unwrap();
    let mut config = GromaqConfig::default();
    config.shell.program = Some("/bin/zsh".to_owned());
    config.shell.args = vec!["-l".to_owned()];
    config.shell.cwd = Some("/tmp".to_owned());

    app.apply_reloadable_gromaq_config(&config).unwrap();

    assert_eq!(
        app.runtime().config().shell.program,
        PathBuf::from("/bin/zsh")
    );
    assert_eq!(app.runtime().config().shell.args, vec![PathBuf::from("-l")]);
    assert_eq!(
        app.runtime().config().shell.cwd,
        Some(PathBuf::from("/tmp"))
    );
    assert_eq!(app.runtime().dump_runtime_perf_metrics().resize_events, 0);
    assert!(!app.runtime().has_shell_session());
}

#[test]
fn native_app_polls_config_file_and_applies_reloadable_render_settings() {
    let path = test_app_config_path("reload-render-config.toml");
    fs::write(&path, "[performance]\ntarget_fps = 144\n").unwrap();
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig::default(),
    )
    .unwrap();
    app.set_config_reloader(ConfigFileReloader::from_file(path.clone()).unwrap());

    assert!(!app.reload_config_if_changed().unwrap());

    fs::write(
        &path,
        r#"
        [terminal]
        cols = 24
        rows = 6
        scrollback_lines = 64

        [performance]
        target_fps = 120
        dirty_region_rendering = false

        [font]
        size_px = 18.0

        [shell]
        program = "/bin/zsh"
        args = ["-l"]
        cwd = "/tmp"
        "#,
    )
    .unwrap();

    assert!(app.reload_config_if_changed().unwrap());
    assert_eq!(app.lifecycle().config().target_fps, 120);
    assert_eq!(
        (
            app.runtime().terminal().dump_grid().cols,
            app.runtime().terminal().dump_grid().rows,
        ),
        expected_grid_for_window(1280, 800, app.renderer().config())
    );
    assert_eq!(app.runtime().config().scrollback_lines, 64);
    assert_eq!(
        app.runtime().config().shell.program,
        PathBuf::from("/bin/zsh")
    );
    assert_eq!(app.runtime().config().shell.args, vec![PathBuf::from("-l")]);
    assert_eq!(
        app.runtime().config().shell.cwd,
        Some(PathBuf::from("/tmp"))
    );
    assert_eq!(app.renderer().config().target_fps, 120);
    assert!(!app.renderer().config().dirty_regions);
    assert_eq!(app.renderer().config().font_size_px, 18);
    assert!(!app.runtime().has_shell_session());
    let _ = fs::remove_file(path);
}
