use std::fs;
use std::path::PathBuf;
use std::time::Duration;

use gromaq::app::{
    NativeAppConfig, NativeTerminalApp, NativeTerminalRuntimeConfig, NativeTextZoomAction,
};
use gromaq::config::{TmuxWorkspaceSettings, TmuxWorkspaceWindowSettings};
use gromaq::pty::ShellCommand;
use gromaq::renderer::RendererConfig;
use gromaq::{ConfigFileReloader, CursorShape, CursorStyleSetting, GromaqConfig};
use winit::dpi::Size;

use crate::support::{system_mono_font_path, test_app_config_path};

#[path = "config/reload.rs"]
mod reload;

fn expected_grid_for_window(
    width_px: u32,
    height_px: u32,
    renderer_config: &RendererConfig,
) -> (u16, u16) {
    let width = width_px.saturating_sub(u32::from(renderer_config.surface_padding_px) * 2);
    let height = height_px.saturating_sub(u32::from(renderer_config.surface_padding_px) * 2);
    let spacing = u32::from(renderer_config.cell_spacing_px);
    let cols =
        width.saturating_add(spacing) / (u32::from(renderer_config.cell_width_px) + spacing).max(1);
    let rows = height.saturating_add(spacing)
        / (u32::from(renderer_config.line_height_px) + spacing).max(1);
    (
        u16::try_from(cols.max(1)).unwrap(),
        u16::try_from(rows.max(1)).unwrap(),
    )
}

fn linear_clear_color(red: u8, green: u8, blue: u8) -> [f64; 4] {
    [
        f64::from(srgb8_to_linear_f32(red)),
        f64::from(srgb8_to_linear_f32(green)),
        f64::from(srgb8_to_linear_f32(blue)),
        1.0,
    ]
}

fn srgb8_to_linear_f32(value: u8) -> f32 {
    let srgb = f32::from(value) / 255.0;
    if srgb <= 0.04045 {
        srgb / 12.92
    } else {
        ((srgb + 0.055) / 1.055).powf(2.4)
    }
}

#[test]
fn native_app_config_builds_terminal_window_attributes() {
    let config = NativeAppConfig::default();

    let attributes = config.window_attributes();

    assert_eq!(attributes.title, "Gromaq");
    assert!(attributes.visible);
    assert!(attributes.resizable);
    assert!(attributes.window_icon.is_some());
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
fn native_app_config_carries_enabled_tmux_workspace_presets() {
    let mut user_config = GromaqConfig::default();
    user_config.tmux.enabled = true;
    user_config.tmux.workspaces.insert(
        "gromaq".to_owned(),
        TmuxWorkspaceSettings {
            session: "gromaq".to_owned(),
            root: Some("/repo".to_owned()),
            windows: vec![TmuxWorkspaceWindowSettings {
                name: "code".to_owned(),
                panes: vec!["nvim".to_owned()],
            }],
        },
    );

    let app_config = NativeAppConfig::from_gromaq_config(&user_config).unwrap();

    assert_eq!(app_config.tmux_workspaces.len(), 1);
    assert_eq!(app_config.tmux_workspaces[0].key, "gromaq");
    assert_eq!(app_config.tmux_workspaces[0].settings.session, "gromaq");
}

#[test]
fn native_app_config_enables_tmux_ui_by_default() {
    let user_config = GromaqConfig::default();

    let app_config = NativeAppConfig::from_gromaq_config(&user_config).unwrap();

    assert!(app_config.tmux_ui_enabled);
}

#[test]
fn native_app_config_can_disable_tmux_ui() {
    let mut user_config = GromaqConfig::default();
    user_config.tmux.enabled = false;

    let app_config = NativeAppConfig::from_gromaq_config(&user_config).unwrap();

    assert!(!app_config.tmux_ui_enabled);
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
    assert_eq!(
        (grid.cols, grid.rows),
        expected_grid_for_window(1280, 800, &RendererConfig::default())
    );
    assert_eq!(app.runtime().config().scrollback_lines, 64);
    assert_eq!(app.runtime().config().pixel_width, 1280);
    assert_eq!(app.runtime().config().pixel_height, 800);
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
fn native_app_text_zoom_reconfigures_renderer_metrics_and_grid() {
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig::default(),
    )
    .unwrap();
    let default_grid = (
        app.runtime().terminal().dump_grid().cols,
        app.runtime().terminal().dump_grid().rows,
    );

    assert!(
        app.apply_text_zoom_action(NativeTextZoomAction::Increase)
            .unwrap()
    );

    assert_eq!(app.renderer().config().font_size_px, 37);
    assert_eq!(app.renderer().config().cell_width_px, 21);
    assert_eq!(app.renderer().config().line_height_px, 51);
    assert!(
        app.runtime().terminal().dump_grid().cols < default_grid.0,
        "zooming in should reduce visible columns"
    );
    assert!(
        app.runtime().terminal().dump_grid().rows < default_grid.1,
        "zooming in should reduce visible rows"
    );

    assert!(
        app.apply_text_zoom_action(NativeTextZoomAction::Reset)
            .unwrap()
    );

    assert_eq!(app.renderer().config().font_size_px, 32);
    assert_eq!(app.renderer().config().cell_width_px, 18);
    assert_eq!(app.renderer().config().line_height_px, 44);
    assert_eq!(
        (
            app.runtime().terminal().dump_grid().cols,
            app.runtime().terminal().dump_grid().rows,
        ),
        default_grid
    );
}

#[test]
fn native_app_can_start_with_configured_font_file_path() {
    let font_path = system_mono_font_path();

    let app = NativeTerminalApp::new_with_runtime_renderer_and_font_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig::default(),
        font_path.to_string_lossy(),
    )
    .unwrap();

    assert_eq!(app.font_family(), font_path.to_string_lossy());
}

#[test]
fn native_app_can_start_with_configured_font_fallback_paths() {
    let font_path = system_mono_font_path();
    let fallback = font_path.to_string_lossy().into_owned();

    let app = NativeTerminalApp::new_with_runtime_renderer_font_and_fallback_config(
        NativeAppConfig::default(),
        NativeTerminalRuntimeConfig::default(),
        RendererConfig::default(),
        font_path.to_string_lossy(),
        vec![fallback.clone()],
    )
    .unwrap();

    assert_eq!(app.font_family(), font_path.to_string_lossy());
    assert_eq!(app.font_fallback_families(), &[fallback]);
}
