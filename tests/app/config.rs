use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use gromaq::app::{NativeAppConfig, NativeTerminalApp, NativeTerminalRuntimeConfig};
use gromaq::pty::ShellCommand;
use gromaq::renderer::RendererConfig;
use gromaq::{ConfigFileReloader, CursorShape, CursorStyleSetting, GromaqConfig};
use winit::dpi::Size;

use crate::support::test_app_config_path;

#[test]
fn native_app_config_builds_terminal_window_attributes() {
    let config = NativeAppConfig::default();

    let attributes = config.window_attributes();

    assert_eq!(attributes.title, "Gromaq");
    assert!(attributes.visible);
    assert!(attributes.resizable);
    assert_eq!(
        attributes.inner_size,
        Some(Size::Logical(winit::dpi::LogicalSize::new(1280.0, 800.0)))
    );
    assert_eq!(
        config.target_frame_interval(),
        Duration::from_nanos(6_944_444)
    );
}

#[test]
fn native_app_config_uses_validated_gromaq_performance_target() {
    let mut user_config = GromaqConfig::default();
    user_config.performance.target_fps = 120;

    let app_config = NativeAppConfig::from_gromaq_config(&user_config).unwrap();

    assert_eq!(app_config.target_fps, 120);
    assert_eq!(
        app_config.target_frame_interval(),
        Duration::from_nanos(8_333_333)
    );
}

#[test]
fn native_app_config_rejects_invalid_gromaq_performance_target() {
    let mut user_config = GromaqConfig::default();
    user_config.performance.target_fps = 0;

    let error = NativeAppConfig::from_gromaq_config(&user_config).unwrap_err();

    assert!(error.to_string().contains("target fps"));
}

#[test]
fn native_runtime_config_uses_validated_gromaq_terminal_settings() {
    let mut user_config = GromaqConfig::default();
    user_config.terminal.cols = 100;
    user_config.terminal.rows = 28;
    user_config.terminal.scrollback_lines = 2048;
    user_config.theme.cursor_style = CursorStyleSetting::Underline;
    user_config.theme.cursor_blinking = false;
    let shell = ShellCommand {
        program: "/bin/zsh".into(),
        args: vec!["-l".into()],
        cwd: Some("/tmp".into()),
    };

    let runtime_config =
        NativeTerminalRuntimeConfig::from_gromaq_config(&user_config, shell.clone()).unwrap();

    assert_eq!(runtime_config.terminal_cols, 100);
    assert_eq!(runtime_config.terminal_rows, 28);
    assert_eq!(runtime_config.scrollback_lines, 2048);
    assert_eq!(runtime_config.cursor_shape, CursorShape::Underline);
    assert!(!runtime_config.cursor_blinking);
    assert_eq!(runtime_config.shell, shell);
}

#[test]
fn native_app_can_start_with_explicit_runtime_config() {
    let runtime_config = NativeTerminalRuntimeConfig {
        terminal_cols: 40,
        terminal_rows: 10,
        scrollback_lines: 64,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: CursorShape::Bar,
        cursor_blinking: false,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    };

    let app =
        NativeTerminalApp::new_with_runtime_config(NativeAppConfig::default(), runtime_config)
            .unwrap();

    let grid = app.runtime().terminal().dump_grid();
    assert_eq!(grid.cols, 40);
    assert_eq!(grid.rows, 10);
    assert_eq!(
        app.runtime().terminal().dump_cursor().shape,
        CursorShape::Bar
    );
    assert!(!app.runtime().terminal().dump_cursor().blinking);
}

#[test]
fn native_app_can_start_with_explicit_renderer_config() {
    let renderer_config = RendererConfig {
        font_size_px: 18,
        dirty_regions: false,
        ..RendererConfig::default()
    };

    let app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        renderer_config.clone(),
    )
    .unwrap();

    assert_eq!(app.renderer().config(), &renderer_config);
}

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

    app.apply_reloadable_gromaq_config(&config).unwrap();

    assert_eq!(app.lifecycle().config().target_fps, 120);
    assert_eq!(app.renderer().config().target_fps, 120);
    assert!(!app.renderer().config().dirty_regions);
    assert_eq!(app.renderer().config().font_size_px, 18);
    assert_eq!(app.renderer().config().line_height_px, 22);
    assert_eq!(
        app.renderer().config().clear_color,
        [
            f64::from(31) / 255.0,
            f64::from(32) / 255.0,
            f64::from(40) / 255.0,
            1.0
        ]
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

    assert_eq!(app.runtime().terminal().dump_grid().cols, 12);
    assert_eq!(app.runtime().terminal().dump_grid().rows, 3);
    assert_eq!(app.runtime().config().terminal_cols, 12);
    assert_eq!(app.runtime().config().terminal_rows, 3);
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
    assert_eq!(app.runtime().terminal().dump_grid().cols, 24);
    assert_eq!(app.runtime().terminal().dump_grid().rows, 6);
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
