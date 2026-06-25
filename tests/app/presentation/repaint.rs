use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, NativeWindowSurface,
    render_and_present_terminal_glyph_frame_report,
};
use gromaq::font::RasterizedGlyphCache;
use gromaq::pty::ShellCommand;
use gromaq::renderer::{RendererConfig, WgpuRenderer};

use crate::support::{
    MockPtySpawner, MockSurfaceBackend, supported_surface_capabilities, system_mono_font,
};

#[test]
fn native_surface_redraw_repaints_full_visible_grid_after_partial_output() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
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
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
    let mut glyph_cache = RasterizedGlyphCache::from_bytes(system_mono_font()).unwrap();
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());
    surface.configure_initial(1280, 800).unwrap();

    render_and_present_terminal_glyph_frame_report(
        &mut runtime,
        &mut renderer,
        &mut glyph_cache,
        &mut surface,
    )
    .unwrap();

    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"world\r\n".to_vec());
    runtime.pump_pty_output().unwrap();

    render_and_present_terminal_glyph_frame_report(
        &mut runtime,
        &mut renderer,
        &mut glyph_cache,
        &mut surface,
    )
    .unwrap();

    let plan = renderer
        .last_plan()
        .expect("surface redraw should leave a full visible render plan");
    let planned_text = plan
        .glyphs
        .iter()
        .map(|glyph| glyph.text.as_str())
        .collect::<String>();

    assert!(planned_text.contains("hello"));
    assert!(planned_text.contains("world"));
    assert_eq!(
        plan.clear_regions,
        vec![gromaq::dirty::DirtyRegion {
            row: 0,
            col: 0,
            rows: 4,
            cols: 20,
        }]
    );
    assert_eq!(surface.backend().presented_glyph_frames.borrow().len(), 2);
}

#[test]
fn native_surface_redraw_repaints_existing_output_without_new_pty_bytes() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
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
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
    let mut glyph_cache = RasterizedGlyphCache::from_bytes(system_mono_font()).unwrap();
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());
    surface.configure_initial(1280, 800).unwrap();

    render_and_present_terminal_glyph_frame_report(
        &mut runtime,
        &mut renderer,
        &mut glyph_cache,
        &mut surface,
    )
    .unwrap();

    let second_report = render_and_present_terminal_glyph_frame_report(
        &mut runtime,
        &mut renderer,
        &mut glyph_cache,
        &mut surface,
    )
    .unwrap();

    let plan = renderer
        .last_plan()
        .expect("clean redraw should still leave a full visible render plan");
    let planned_text = plan
        .glyphs
        .iter()
        .map(|glyph| glyph.text.as_str())
        .collect::<String>();

    assert!(planned_text.contains("hello"));
    assert!(second_report.rendered);
    assert!(second_report.glyph_frame_presented);
    assert_eq!(second_report.glyph_quads, 5);
    assert_eq!(surface.backend().presented_glyph_frames.borrow().len(), 2);
    assert!(surface.backend().presented_clear_colors.borrow().is_empty());
}

#[test]
fn native_surface_redraw_preserves_zsh_repainted_command_output() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 8,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/zsh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(
            b"\r\x1b[2K\x1b[1G> ls\x1b[K\r\n\
              Applications    Downloads\r\n\
              Documents       Projects\r\n\
              \r\x1b[J\r\n\
              \x1b[A~/Daedalus/gromaq ................................ rb 2.7.5 15:11\r\n\
              \x1b[2K\x1b[1G\x1b[38;5;76m>\x1b[39m \x1b[K\x1b[?2004h"
                .to_vec(),
        );
    runtime.pump_pty_output().unwrap();
    let mut renderer = WgpuRenderer::new(RendererConfig::default()).unwrap();
    let mut glyph_cache = RasterizedGlyphCache::from_bytes(system_mono_font()).unwrap();
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());
    surface.configure_initial(1280, 800).unwrap();

    let report = render_and_present_terminal_glyph_frame_report(
        &mut runtime,
        &mut renderer,
        &mut glyph_cache,
        &mut surface,
    )
    .unwrap();

    let plan = renderer
        .last_plan()
        .expect("zsh repaint surface redraw should keep a visible render plan");
    let planned_text = plan
        .glyphs
        .iter()
        .map(|glyph| glyph.text.as_str())
        .collect::<String>();

    assert!(
        planned_text.contains(">ls"),
        "render plan dropped command line after prompt repaint: {planned_text:?}"
    );
    assert!(
        planned_text.contains("Applications"),
        "render plan dropped first output row after prompt repaint: {planned_text:?}"
    );
    assert!(
        planned_text.contains("Documents"),
        "render plan dropped second output row after prompt repaint: {planned_text:?}"
    );
    assert!(
        planned_text.contains("~/Daedalus/gromaq"),
        "render plan dropped repainted prompt after command output: {planned_text:?}"
    );
    assert!(report.rendered);
    assert!(report.glyph_frame_presented);
    assert_eq!(surface.backend().presented_glyph_frames.borrow().len(), 1);
    assert!(surface.backend().presented_clear_colors.borrow().is_empty());
}
