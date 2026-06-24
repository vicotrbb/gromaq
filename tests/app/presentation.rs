use gromaq::app::{
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, NativeWindowSurface,
    render_and_present_terminal_glyph_frame_report,
};
use gromaq::font::RasterizedGlyphCache;
use gromaq::pty::ShellCommand;
use gromaq::renderer::{RendererConfig, WgpuRenderer};

use crate::support::{
    MockPtySession, MockPtySpawner, MockSurfaceBackend, supported_surface_capabilities,
    system_mono_font,
};

#[test]
fn native_redraw_presents_dirty_runtime_frame_as_glyph_frame() {
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

    let report = render_and_present_terminal_glyph_frame_report(
        &mut runtime,
        &mut renderer,
        &mut glyph_cache,
        &mut surface,
    )
    .unwrap();

    assert!(surface.backend().presented_clear_colors.borrow().is_empty());
    let presented_frames = surface.backend().presented_glyph_frames.borrow();
    assert_eq!(presented_frames.len(), 1);
    assert_eq!(presented_frames[0].quads, 5);
    assert!(presented_frames[0].width > 0);
    assert!(presented_frames[0].height > 0);
    assert!(presented_frames[0].atlas_pixels > 0);
    assert!(report.rendered);
    assert!(report.glyph_frame_presented);
    assert!(!report.clear_presented);
    assert_eq!(report.width, presented_frames[0].width);
    assert_eq!(report.height, presented_frames[0].height);
    assert_eq!(report.glyph_quads, presented_frames[0].quads);
    assert_eq!(report.atlas_bytes / 4, presented_frames[0].atlas_pixels);
    assert_eq!(report.background_quads, 0);
    assert_eq!(report.decoration_quads, 0);
    assert!(report.atlas_occupied_slots > 0);
}

#[test]
fn native_redraw_presents_blank_runtime_cursor_frame_without_clear_only_fallback() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
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

    assert!(surface.backend().presented_clear_colors.borrow().is_empty());
    assert_eq!(surface.backend().presented_glyph_frames.borrow().len(), 1);
    assert!(report.rendered);
    assert!(report.glyph_frame_presented);
    assert!(!report.clear_presented);
    assert_eq!(report.glyph_quads, 0);
    assert_eq!(report.cursor_quads, 1);
    assert_eq!(report.atlas_occupied_slots, 0);
    assert!(report.atlas_bytes > 0);
}

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
