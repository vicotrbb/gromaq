use std::path::PathBuf;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};
use std::time::Duration;

use gromaq::app::{
    NativeAppAction, NativeAppConfig, NativeAppEvent, NativeAppEventProxy, NativeAppLifecycle,
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativePtySpawner,
    NativeResizeGridMapper, NativeRuntimePerfSnapshot, NativeRuntimeStateSnapshot,
    NativeTerminalRuntime, NativeTerminalRuntimeConfig, NativeWindowMouseInput,
    NativeWindowSurface, RealNativePtySpawner, is_native_copy_shortcut, is_native_paste_shortcut,
    load_default_native_glyph_cache, render_and_present_terminal_glyph_frame_report,
};
use gromaq::font::RasterizedGlyphCache;
use gromaq::native_gpu::NativeGpuWindowSurface;
use gromaq::pty::{PtyConfig, ShellCommand};
use gromaq::renderer::{
    BackgroundQuadBatch, GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, GlyphBitmap, GlyphEntry,
    GlyphQuad, GlyphQuadBatch, GlyphVertex, RenderPlanner, RendererConfig, SurfaceGlyphFrame,
    SurfaceLifecycleAction, WgpuRenderer,
};
use gromaq::{
    GromaqError, KeyModifiers, MemoryClipboard, MouseButton, MouseEvent, MouseEventKind,
    SelectionRange, Terminal, TerminalConfig,
};
use winit::keyboard::{Key, KeyCode, ModifiersState, NamedKey, PhysicalKey};

#[path = "app/config.rs"]
mod config;
#[path = "app/support.rs"]
mod support;

use support::{
    MockFrameRenderer, MockPtySession, MockPtySpawner, MockSurfaceBackend, PresentedGlyphFrame,
    supported_surface_capabilities, system_mono_font,
};

#[test]
fn native_terminal_runtime_invalidates_clean_frame_for_redraw() {
    let mut runtime =
        NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig::default())
            .unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(!runtime.render_terminal_frame(&mut renderer).unwrap());
    runtime.invalidate_terminal_frame();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.render_attempts, 2);
    assert_eq!(metrics.clean_frame_skips, 1);
    assert_eq!(metrics.rendered_frames, 1);
}

#[test]
fn native_terminal_runtime_keeps_frame_dirty_after_renderer_error() {
    let mut runtime =
        NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig::default())
            .unwrap();
    runtime.invalidate_terminal_frame();
    let mut renderer = MockFrameRenderer {
        render_error: Some(GromaqError::GlyphAtlasInvariant {
            reason: "forced renderer failure",
        }),
        ..MockFrameRenderer::default()
    };

    let error = runtime.render_terminal_frame(&mut renderer).unwrap_err();

    assert_eq!(
        error.to_string(),
        "glyph atlas invariant violation: forced renderer failure"
    );
    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.render_attempts, 1);
    assert_eq!(metrics.rendered_frames, 0);
    assert_eq!(metrics.render_time_samples, 0);

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.render_attempts, 2);
    assert_eq!(metrics.rendered_frames, 1);
    assert_eq!(metrics.render_time_samples, 1);
    assert_eq!(renderer.frames.len(), 1);
}

#[test]
fn default_native_glyph_cache_loads_system_monospace_font() {
    let cache = load_default_native_glyph_cache().unwrap();

    assert!(cache.is_empty());
}

#[test]
fn default_native_glyph_cache_rasterizes_emoji_with_fallback_font() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("😀").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(24);
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();
    let mut cache = load_default_native_glyph_cache().unwrap();

    let batch = cache.rasterize_plan(&plan).unwrap();

    assert_eq!(batch.rasterized, 1);
    assert_eq!(batch.bitmaps.len(), 1);
    assert!(
        batch.bitmaps[0]
            .rgba
            .chunks_exact(4)
            .any(|pixel| pixel[3] > 0)
    );
}

#[test]
fn native_app_lifecycle_requests_window_redraw_and_exit_in_order() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());

    assert_eq!(lifecycle.on_resumed(), NativeAppAction::CreateWindow);
    lifecycle.on_window_created();
    assert_eq!(lifecycle.windows_created(), 1);
    assert!(lifecycle.has_window());

    assert_eq!(lifecycle.on_resumed(), NativeAppAction::None);
    assert_eq!(lifecycle.on_about_to_wait(), NativeAppAction::None);
    assert_eq!(lifecycle.redraw_requests(), 0);
    assert_eq!(
        lifecycle.on_terminal_output_ready(),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.redraw_requests(), 1);

    assert_eq!(lifecycle.on_redraw_requested(), NativeAppAction::None);
    assert_eq!(lifecycle.frames_presented(), 1);

    assert_eq!(lifecycle.on_close_requested(), NativeAppAction::Exit);
    assert!(lifecycle.close_requested());
    assert_eq!(lifecycle.on_destroyed(), NativeAppAction::Exit);
    assert!(!lifecycle.has_window());
}

#[test]
fn native_app_lifecycle_schedules_next_pty_pump_deadline() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());
    let now = std::time::Instant::now();

    assert_eq!(lifecycle.next_pty_pump_deadline(now), None);

    lifecycle.on_window_created();

    assert_eq!(
        lifecycle.next_pty_pump_deadline(now),
        Some(now + NativeAppConfig::default().target_frame_interval())
    );

    lifecycle.on_close_requested();

    assert_eq!(lifecycle.next_pty_pump_deadline(now), None);
}

#[test]
fn native_app_lifecycle_applies_reloaded_frame_cadence() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());
    lifecycle.on_window_created();
    let now = std::time::Instant::now();
    let config = NativeAppConfig {
        target_fps: 120,
        ..NativeAppConfig::default()
    };

    lifecycle.apply_config(config);

    assert_eq!(
        lifecycle.next_pty_pump_deadline(now),
        Some(now + Duration::from_nanos(8_333_333))
    );
}

#[test]
fn native_app_lifecycle_handles_pty_output_ready_user_event() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());

    assert_eq!(
        lifecycle.on_user_event(NativeAppEvent::PtyOutputReady),
        NativeAppAction::None
    );
    assert_eq!(lifecycle.redraw_requests(), 0);

    lifecycle.on_window_created();

    assert_eq!(
        lifecycle.on_user_event(NativeAppEvent::PtyOutputReady),
        NativeAppAction::RequestRedraw
    );
    assert_eq!(lifecycle.redraw_requests(), 1);

    lifecycle.on_close_requested();

    assert_eq!(
        lifecycle.on_user_event(NativeAppEvent::PtyOutputReady),
        NativeAppAction::Exit
    );
}

#[test]
fn real_native_pty_spawner_sends_output_ready_event_when_reader_receives_bytes() {
    let wakeups = Arc::new(AtomicUsize::new(0));
    let wakeups_for_proxy = Arc::clone(&wakeups);
    let proxy = NativeAppEventProxy::from_sender(move |event| {
        if event == NativeAppEvent::PtyOutputReady {
            wakeups_for_proxy.fetch_add(1, Ordering::Relaxed);
        }
    });
    let spawner = RealNativePtySpawner::with_event_proxy(proxy);
    let mut session = spawner
        .spawn(PtyConfig {
            rows: 8,
            cols: 40,
            pixel_width: 0,
            pixel_height: 0,
            shell: ShellCommand {
                program: "/bin/sh".into(),
                args: vec!["-lc".into(), "printf gromaq-proxy-wakeup".into()],
                cwd: None,
            },
        })
        .unwrap();

    let mut output = Vec::new();
    for _ in 0..30 {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains("gromaq-proxy-wakeup") {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    assert!(String::from_utf8_lossy(&output).contains("gromaq-proxy-wakeup"));
    assert!(wakeups.load(Ordering::Relaxed) > 0);
}

#[test]
fn native_mouse_grid_mapper_converts_window_pixels_to_terminal_cells() {
    let mapper = NativeMouseGridMapper::new(800, 400, 80, 20).unwrap();

    assert_eq!(
        mapper.mouse_event_at(25.0, 39.0, MouseEventKind::Press, MouseButton::Left),
        Some(MouseEvent::new(
            MouseEventKind::Press,
            MouseButton::Left,
            2,
            1
        ))
    );
    assert_eq!(
        mapper.mouse_event_at(799.0, 399.0, MouseEventKind::Release, MouseButton::Left),
        Some(MouseEvent::new(
            MouseEventKind::Release,
            MouseButton::Left,
            79,
            19
        ))
    );
    assert_eq!(
        mapper.mouse_event_at_with_modifiers(
            25.0,
            39.0,
            MouseEventKind::Press,
            MouseButton::Left,
            ModifiersState::SHIFT.union(ModifiersState::ALT)
        ),
        Some(
            MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1)
                .with_modifiers(KeyModifiers::SHIFT | KeyModifiers::ALT)
        )
    );
    assert_eq!(
        mapper.mouse_event_at(800.0, 399.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(-1.0, 10.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(f64::NAN, 10.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(
            10.0,
            f64::INFINITY,
            MouseEventKind::Press,
            MouseButton::Left
        ),
        None
    );
    assert_eq!(NativeMouseGridMapper::new(0, 400, 80, 20), None);
}

#[test]
fn native_mouse_button_tracker_reports_drag_only_while_button_is_pressed() {
    let mut tracker = NativeMouseButtonTracker::default();

    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Motion, MouseButton::None)
    );

    tracker.set_pressed(MouseButton::Left, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Left)
    );

    tracker.set_pressed(MouseButton::Left, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Motion, MouseButton::None)
    );
}

#[test]
fn native_mouse_button_tracker_reports_active_drag_button_priority() {
    let mut tracker = NativeMouseButtonTracker::default();

    tracker.set_pressed(MouseButton::Right, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Right)
    );

    tracker.set_pressed(MouseButton::Middle, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Middle)
    );

    tracker.set_pressed(MouseButton::Left, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Left)
    );

    tracker.set_pressed(MouseButton::None, true);
    tracker.set_pressed(MouseButton::WheelUp, true);
    tracker.set_pressed(MouseButton::WheelDown, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Left)
    );

    tracker.set_pressed(MouseButton::Left, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Middle)
    );

    tracker.set_pressed(MouseButton::Middle, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Right)
    );

    tracker.set_pressed(MouseButton::Right, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Motion, MouseButton::None)
    );
}

#[test]
fn native_resize_grid_mapper_scales_window_pixels_to_terminal_size() {
    let mapper = NativeResizeGridMapper::new(1280, 800, 120, 36).unwrap();

    assert_eq!(
        mapper.resize_for_window(1280, 800),
        Some(NativePtyResize {
            cols: 120,
            rows: 36,
            pixel_width: 1280,
            pixel_height: 800,
        })
    );
    assert_eq!(
        mapper.resize_for_window(640, 400),
        Some(NativePtyResize {
            cols: 60,
            rows: 18,
            pixel_width: 640,
            pixel_height: 400,
        })
    );
    assert_eq!(mapper.resize_for_window(0, 400), None);
    assert_eq!(NativeResizeGridMapper::new(0, 800, 120, 36), None);
}

#[test]
fn native_window_surface_configures_and_resizes_surface_backend() {
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());

    assert_eq!(
        surface.configure_initial(1280, 800).unwrap(),
        SurfaceLifecycleAction::Configure
    );
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );
    assert_eq!(surface.configured_size(), Some((1280, 800)));

    assert_eq!(
        surface.resize(1280, 800).unwrap(),
        SurfaceLifecycleAction::None
    );
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );

    assert_eq!(
        surface.resize(0, 800).unwrap(),
        SurfaceLifecycleAction::DeferZeroSize
    );
    assert!(surface.is_suspended());
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );

    assert_eq!(
        surface.resize(1440, 900).unwrap(),
        SurfaceLifecycleAction::Reconfigure
    );
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800), (1440, 900)]
    );
    assert_eq!(surface.configure_count(), 2);
}

#[test]
fn native_window_surface_presents_clear_frame_through_backend() {
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());

    surface.configure_initial(1280, 800).unwrap();
    surface.clear_and_present([0.02, 0.02, 0.025, 1.0]).unwrap();

    assert_eq!(
        surface.backend().presented_clear_colors.borrow().as_slice(),
        &[[0.02, 0.02, 0.025, 1.0]]
    );
}

#[test]
fn native_window_surface_presents_terminal_glyph_frame_through_backend() {
    let backend = MockSurfaceBackend::default();
    let mut surface = NativeWindowSurface::new(backend, supported_surface_capabilities());
    surface.configure_initial(1280, 800).unwrap();
    let atlas = GlyphAtlasImage::pack_rgba8(
        2,
        2,
        1,
        &[GlyphBitmap::try_solid_rgba8(
            GlyphEntry {
                slot: 0,
                generation: 0,
            },
            2,
            2,
            [255, 255, 255, 255],
        )
        .unwrap()],
    )
    .unwrap();
    let batch = GlyphQuadBatch {
        quads: vec![GlyphQuad {
            text: "A".to_owned(),
            ch: 'A',
            atlas_entry: GlyphEntry {
                slot: 0,
                generation: 0,
            },
            vertices: [
                GlyphVertex {
                    position: [0.0, 0.0],
                    uv: [0.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [2.0, 0.0],
                    uv: [1.0, 0.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [2.0, 2.0],
                    uv: [1.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
                GlyphVertex {
                    position: [0.0, 2.0],
                    uv: [0.0, 1.0],
                    foreground_rgba: [1.0, 1.0, 1.0, 1.0],
                },
            ],
        }],
        indices: vec![0, 1, 2, 0, 2, 3],
    };

    surface
        .present_glyph_frame(SurfaceGlyphFrame {
            atlas: &atlas,
            background_batch: &BackgroundQuadBatch::default(),
            batch: &batch,
            decoration_batch: &BackgroundQuadBatch::default(),
            cursor_batch: &BackgroundQuadBatch::default(),
            width: 2,
            height: 2,
            clear_color: [0.02, 0.02, 0.025, 1.0],
        })
        .unwrap();

    assert_eq!(
        surface.backend().presented_glyph_frames.borrow().as_slice(),
        &[PresentedGlyphFrame {
            width: 2,
            height: 2,
            quads: 1,
            atlas_pixels: 4,
        }]
    );
}

#[test]
fn native_window_surface_configures_from_gpu_surface_handoff() {
    let gpu_surface = NativeGpuWindowSurface::new(
        MockSurfaceBackend::default(),
        supported_surface_capabilities(),
    );

    let surface = NativeWindowSurface::from_gpu_surface(gpu_surface, 1280, 800).unwrap();

    assert_eq!(surface.configured_size(), Some((1280, 800)));
    assert_eq!(surface.configure_count(), 1);
    assert_eq!(
        surface.backend().configured_sizes.borrow().as_slice(),
        &[(1280, 800)]
    );
}

#[test]
fn native_terminal_runtime_pumps_output_before_scheduling_redraw() {
    let spawner = MockPtySpawner::default();
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());
    lifecycle.on_window_created();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();

    let action = runtime
        .pump_output_and_schedule_redraw(&mut lifecycle)
        .unwrap();

    assert_eq!(action, NativeAppAction::RequestRedraw);
    assert_eq!(lifecycle.redraw_requests(), 1);
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "hello");

    let idle_action = runtime
        .pump_output_and_schedule_redraw(&mut lifecycle)
        .unwrap();

    assert_eq!(idle_action, NativeAppAction::None);
    assert_eq!(lifecycle.redraw_requests(), 1);
}

#[test]
fn native_terminal_runtime_renders_dirty_terminal_frame_once() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    let mut renderer = MockFrameRenderer::default();

    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    assert_eq!(renderer.frames.len(), 1);
    assert_eq!(renderer.frames[0].first_line, "hello");
    assert_eq!(renderer.frames[0].cursor.row, 1);
    assert!(!renderer.frames[0].dirty_regions.is_empty());

    assert!(!runtime.render_terminal_frame(&mut renderer).unwrap());
    assert_eq!(renderer.frames.len(), 1);
}

#[test]
fn native_redraw_presents_dirty_runtime_frame_as_glyph_frame() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
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
fn native_terminal_runtime_starts_shell_pty_once_and_keeps_session() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 100,
        terminal_rows: 30,
        scrollback_lines: 2_000,
        pixel_width: 900,
        pixel_height: 600,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: vec!["-l".into()],
            cwd: None,
        },
    })
    .unwrap();

    assert_eq!(runtime.terminal().dump_grid().cols, 100);
    assert_eq!(runtime.terminal().dump_grid().rows, 30);

    runtime.start_shell(&spawner).unwrap();
    runtime.start_shell(&spawner).unwrap();

    let configs = spawner.configs.borrow();
    assert_eq!(configs.len(), 1);
    assert_eq!(configs[0].cols, 100);
    assert_eq!(configs[0].rows, 30);
    assert_eq!(configs[0].pixel_width, 900);
    assert_eq!(configs[0].pixel_height, 600);
    assert_eq!(configs[0].shell.program, "/bin/sh");
    assert_eq!(configs[0].shell.args, vec!["-l"]);
    assert!(runtime.has_shell_session());
}

#[test]
fn native_terminal_runtime_restarts_shell_with_updated_command() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 24,
        scrollback_lines: 1_000,
        pixel_width: 800,
        pixel_height: 480,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();

    runtime
        .restart_shell(
            ShellCommand {
                program: "/bin/zsh".into(),
                args: vec!["-l".into()],
                cwd: Some("/tmp".into()),
            },
            &spawner,
        )
        .unwrap();

    let configs = spawner.configs.borrow();
    assert_eq!(configs.len(), 2);
    assert_eq!(configs[1].cols, 80);
    assert_eq!(configs[1].rows, 24);
    assert_eq!(configs[1].pixel_width, 800);
    assert_eq!(configs[1].pixel_height, 480);
    assert_eq!(configs[1].shell.program, PathBuf::from("/bin/zsh"));
    assert_eq!(configs[1].shell.args, vec![PathBuf::from("-l")]);
    assert_eq!(configs[1].shell.cwd, Some(PathBuf::from("/tmp")));
    assert_eq!(runtime.config().shell, configs[1].shell);
    assert!(runtime.has_shell_session());
}

#[test]
fn native_terminal_runtime_pumps_pty_output_and_writes_input() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();

    let bytes = runtime.pump_pty_output().unwrap();
    runtime.send_pty_input(b"pwd\n").unwrap();

    assert_eq!(bytes, 7);
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "hello");
    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"pwd\n".to_vec()]);
}

#[test]
fn native_runtime_perf_metrics_track_io_resize_and_render_boundaries() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    assert_eq!(
        runtime.dump_runtime_perf_metrics(),
        NativeRuntimePerfSnapshot::default()
    );
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    runtime
        .send_winit_key_input(&Key::Character("c".into()), ModifiersState::CONTROL)
        .unwrap();
    runtime.send_paste_text("ab").unwrap();
    runtime.send_committed_text("é").unwrap();
    runtime
        .resize_terminal(NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        })
        .unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[6n".to_vec());
    runtime.pump_pty_output().unwrap();
    let mut renderer = MockFrameRenderer {
        render_delay: Duration::from_millis(1),
        ..MockFrameRenderer::default()
    };
    assert!(runtime.render_terminal_frame(&mut renderer).unwrap());
    assert!(!runtime.render_terminal_frame(&mut renderer).unwrap());
    let rendered_dirty_regions = renderer.frames[0].dirty_regions.len() as u64;
    let rendered_dirty_cells = renderer.frames[0]
        .dirty_regions
        .iter()
        .map(|region| u64::from(region.rows) * u64::from(region.cols))
        .sum::<u64>();

    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.pty_output_batches, 2);
    assert_eq!(metrics.pty_output_bytes, 11);
    assert_eq!(metrics.pty_response_writes, 1);
    assert!(!runtime.shell_session().unwrap().input.borrow()[3].is_empty());
    assert_eq!(
        metrics.pty_response_bytes,
        runtime.shell_session().unwrap().input.borrow()[3].len() as u64
    );
    assert_eq!(metrics.pty_input_writes, 3);
    assert_eq!(metrics.pty_input_bytes, 5);
    assert_eq!(metrics.native_key_inputs, 1);
    assert_eq!(metrics.paste_bytes, 2);
    assert_eq!(metrics.committed_text_bytes, 2);
    assert_eq!(metrics.resize_events, 1);
    assert_eq!(metrics.render_attempts, 2);
    assert_eq!(metrics.rendered_frames, 1);
    assert_eq!(metrics.rendered_dirty_regions, rendered_dirty_regions);
    assert_eq!(metrics.rendered_dirty_cells, rendered_dirty_cells);
    assert_eq!(metrics.rendered_dirty_cells_max, rendered_dirty_cells);
    assert_eq!(metrics.clean_frame_skips, 1);
    assert_eq!(metrics.render_time_samples, 1);
    assert!(metrics.render_time_total_ns >= 1_000_000);
    assert_eq!(metrics.render_time_avg_ns, metrics.render_time_total_ns);
    assert!(metrics.render_time_max_ns >= 1_000_000);
    assert!(metrics.render_time_p95_ns >= metrics.render_time_max_ns);
    assert!(metrics.render_time_total_ns >= metrics.render_time_max_ns);
    assert_eq!(metrics.input_to_render_samples, 1);
    assert_eq!(
        metrics.input_to_render_avg_ns,
        metrics.input_to_render_total_ns
    );
    assert!(metrics.input_to_render_total_ns >= metrics.input_to_render_max_ns);
    assert!(metrics.input_to_render_p95_ns >= metrics.input_to_render_max_ns);
}

#[test]
fn native_runtime_state_snapshot_reports_bounded_scrollback_footprint() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 8,
        terminal_rows: 2,
        scrollback_lines: 3,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();

    assert_eq!(
        runtime.dump_runtime_state_snapshot(),
        NativeRuntimeStateSnapshot {
            terminal_cols: 8,
            terminal_rows: 2,
            visible_cells: 16,
            scrollback_limit: 3,
            scrollback_lines: 0,
            scrollback_cell_rows: 0,
            scrollback_cells: 0,
            scrollback_cell_limit: 24,
        }
    );

    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\n".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    let state = runtime.dump_runtime_state_snapshot();
    assert_eq!(state.terminal_cols, 8);
    assert_eq!(state.terminal_rows, 2);
    assert_eq!(state.visible_cells, 16);
    assert_eq!(state.scrollback_limit, 3);
    assert_eq!(state.scrollback_lines, 3);
    assert_eq!(state.scrollback_cell_rows, 3);
    assert!(state.scrollback_cells <= state.scrollback_cell_limit);

    runtime
        .resize_terminal(NativePtyResize {
            cols: 5,
            rows: 4,
            pixel_width: 500,
            pixel_height: 320,
        })
        .unwrap();

    let resized = runtime.dump_runtime_state_snapshot();
    assert_eq!(resized.terminal_cols, 5);
    assert_eq!(resized.terminal_rows, 4);
    assert_eq!(resized.visible_cells, 20);
    assert_eq!(resized.scrollback_limit, 3);
    assert_eq!(resized.scrollback_cell_limit, 15);
    assert!(resized.scrollback_lines <= resized.scrollback_limit);
    assert!(resized.scrollback_cells <= resized.scrollback_cell_limit);
}

#[test]
fn native_terminal_runtime_writes_terminal_status_responses_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
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
        .push_back(b"\x1b[3;5H\x1b[6n\x1b[5n".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 14);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[3;5R\x1b[0n".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_writes_device_attribute_responses_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
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
        .push_back(b"\x1b[c".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 3);
    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1b[?1;2c".to_vec()]);
}

#[test]
fn native_terminal_runtime_writes_secondary_device_attribute_responses_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
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
        .push_back(b"\x1b[>c".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 4);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[>0;1;0c".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_resizes_terminal_and_pty_session() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .resize_terminal(NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        })
        .unwrap();

    assert_eq!(runtime.terminal().dump_grid().cols, 10);
    assert_eq!(runtime.terminal().dump_grid().rows, 6);
    assert_eq!(runtime.config().terminal_cols, 10);
    assert_eq!(runtime.config().terminal_rows, 6);
    assert_eq!(runtime.config().pixel_width, 800);
    assert_eq!(runtime.config().pixel_height, 480);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.resizes.borrow().as_slice(),
        &[NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        }]
    );
}

#[test]
fn native_terminal_runtime_updates_pixel_size_report_after_resize() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 900,
        pixel_height: 600,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    runtime
        .resize_terminal(NativePtyResize {
            cols: 10,
            rows: 6,
            pixel_width: 800,
            pixel_height: 480,
        })
        .unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[14t".to_vec());

    let bytes = runtime.pump_pty_output().unwrap();

    assert_eq!(bytes, 5);
    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[4;480;800t".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_encodes_winit_key_input_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();

    assert!(
        runtime
            .send_winit_key_input(&Key::Character("c".into()), ModifiersState::CONTROL)
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[vec![0x03]]);
}

#[test]
fn native_terminal_runtime_uses_application_cursor_key_mode_for_arrows() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty())
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1bOA".to_vec()]);
}

#[test]
fn native_terminal_runtime_returns_to_normal_cursor_key_mode_for_arrows() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1h\x1b[?1l".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::ArrowUp), ModifiersState::empty())
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1b[A".to_vec()]);
}

#[test]
fn native_terminal_runtime_uses_application_keypad_mode_for_numpad_keys() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?66h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_winit_key_event_input(
                &Key::Character("1".into()),
                Some(PhysicalKey::Code(KeyCode::Numpad1)),
                ModifiersState::empty(),
            )
            .unwrap()
    );
    assert!(
        runtime
            .send_winit_key_event_input(
                &Key::Named(NamedKey::Enter),
                Some(PhysicalKey::Code(KeyCode::NumpadEnter)),
                ModifiersState::empty(),
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1bOq".to_vec(), b"\x1bOM".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_sends_focus_reports_when_enabled() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(runtime.send_focus_event(true).unwrap());
    assert!(runtime.send_focus_event(false).unwrap());

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[I".to_vec(), b"\x1b[O".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_suppresses_focus_reports_when_disabled() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1004h\x1b[?1004l".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(!runtime.send_focus_event(true).unwrap());

    let session = runtime.shell_session().unwrap();
    assert!(session.input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_encodes_mouse_input_to_pty_when_reporting_is_enabled() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Press,
                MouseButton::Left,
                2,
                1
            ))
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2M"
    );
}

#[test]
fn native_terminal_runtime_encodes_default_mouse_protocol_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1000h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_mouse_input(MouseEvent::new(
                MouseEventKind::Press,
                MouseButton::Left,
                2,
                1,
            ))
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[M #\""
    );
}

#[test]
fn native_terminal_runtime_encodes_window_mouse_modifiers_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_window_mouse_input_event(NativeWindowMouseInput {
                x: 100.0,
                y: 150.0,
                window_width_px: 800,
                window_height_px: 400,
                kind: MouseEventKind::Press,
                button: MouseButton::Left,
                modifiers: ModifiersState::SHIFT.union(ModifiersState::CONTROL),
            })
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<20;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_window_mouse_input_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Press,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_window_mouse_drag_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1002h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Drag,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<32;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_drag_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1049halt\x1b[?1002h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Drag,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<32;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_press_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1049halt\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Press,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_release_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1049halt\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Release,
                MouseButton::Left,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<0;3;2m"
    );
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_wheel_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1049halt\x1b[?1000h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Press,
                MouseButton::WheelDown,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<65;3;2M"
    );
}

#[test]
fn native_terminal_runtime_scrolls_scrollback_on_unreported_wheel() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 6,
        terminal_rows: 3,
        scrollback_lines: 8,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"one\r\ntwo\r\nthree\r\nfour".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "two");
    assert_eq!(runtime.terminal().dump_grid().line_text(2), "four");

    assert!(
        runtime
            .send_window_mouse_input(
                5.0,
                5.0,
                60,
                30,
                MouseEventKind::Press,
                MouseButton::WheelUp,
            )
            .unwrap()
    );

    let scrolled = runtime.terminal().dump_grid();
    assert_eq!(scrolled.line_text(0), "one");
    assert_eq!(scrolled.line_text(1), "two");
    assert_eq!(scrolled.line_text(2), "three");
    assert!(!runtime.terminal().dump_cursor().visible);
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());

    assert!(
        runtime
            .send_window_mouse_input(
                5.0,
                5.0,
                60,
                30,
                MouseEventKind::Press,
                MouseButton::WheelDown,
            )
            .unwrap()
    );

    let live = runtime.terminal().dump_grid();
    assert_eq!(live.line_text(0), "two");
    assert_eq!(live.line_text(1), "three");
    assert_eq!(live.line_text(2), "four");
    assert!(runtime.terminal().dump_cursor().visible);
}

#[test]
fn native_terminal_runtime_scrolls_scrollback_on_shift_page_keys() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 6,
        terminal_rows: 3,
        scrollback_lines: 8,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"one\r\ntwo\r\nthree\r\nfour\r\nfive\r\nsix".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "four");
    assert_eq!(runtime.terminal().dump_grid().line_text(2), "six");

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
            .unwrap()
    );

    let scrolled = runtime.terminal().dump_grid();
    assert_eq!(scrolled.line_text(0), "two");
    assert_eq!(scrolled.line_text(1), "three");
    assert_eq!(scrolled.line_text(2), "four");
    assert!(!runtime.terminal().dump_cursor().visible);
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageDown), ModifiersState::SHIFT)
            .unwrap()
    );

    let live = runtime.terminal().dump_grid();
    assert_eq!(live.line_text(0), "four");
    assert_eq!(live.line_text(1), "five");
    assert_eq!(live.line_text(2), "six");
    assert!(runtime.terminal().dump_cursor().visible);
    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_sends_shift_page_keys_to_alternate_screen_apps() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1049halt".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1b[5;2~".to_vec()]);
}

#[test]
fn native_terminal_runtime_leaves_shift_page_keys_unhandled_when_primary_scrollback_cannot_move() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();

    assert!(
        !runtime
            .send_winit_key_input(&Key::Named(NamedKey::PageUp), ModifiersState::SHIFT)
            .unwrap()
    );

    assert!(runtime.shell_session().unwrap().input.borrow().is_empty());
}

#[test]
fn native_terminal_runtime_maps_alternate_screen_window_mouse_motion_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1049halt\x1b[?1003h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    assert_eq!(runtime.terminal().dump_grid().line_text(0), "alt");

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Motion,
                MouseButton::None,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<35;3;2M"
    );
}

#[test]
fn native_terminal_runtime_maps_window_mouse_motion_to_pty_report() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 80,
        terminal_rows: 20,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?1003h\x1b[?1006h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    assert!(
        runtime
            .send_window_mouse_input(
                25.0,
                39.0,
                800,
                400,
                MouseEventKind::Motion,
                MouseButton::None,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[<35;3;2M"
    );
}

#[test]
fn native_terminal_runtime_encodes_paste_text_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?2004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    runtime.send_paste_text("abc").unwrap();

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[200~abc\x1b[201~"
    );
}

#[test]
fn native_paste_shortcut_accepts_control_or_super_v() {
    assert!(is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::CONTROL
    ));
    assert!(is_native_paste_shortcut(
        &Key::Character("V".into()),
        ModifiersState::SUPER
    ));
    assert!(is_native_paste_shortcut(
        &Key::Character("V".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::empty()
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::CONTROL | ModifiersState::ALT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::SUPER | ModifiersState::ALT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::CONTROL | ModifiersState::SUPER
    ));
}

#[test]
fn native_copy_shortcut_accepts_super_c_or_control_shift_c_without_plain_control_c() {
    assert!(is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::SUPER
    ));
    assert!(is_native_copy_shortcut(
        &Key::Character("C".into()),
        ModifiersState::CONTROL.union(ModifiersState::SHIFT)
    ));
    assert!(is_native_copy_shortcut(
        &Key::Named(NamedKey::Copy),
        ModifiersState::empty()
    ));
    assert!(is_native_copy_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::CONTROL
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::empty()
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("v".into()),
        ModifiersState::SUPER
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::SUPER | ModifiersState::ALT
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL | ModifiersState::SUPER
    ));
}

#[test]
fn native_paste_shortcut_accepts_dedicated_paste_key() {
    assert!(is_native_paste_shortcut(
        &Key::Named(NamedKey::Paste),
        ModifiersState::empty()
    ));
    assert!(is_native_paste_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::SHIFT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::empty()
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::SHIFT | ModifiersState::CONTROL
    ));
}

#[test]
fn native_terminal_runtime_copies_selection_to_clipboard() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime.pump_pty_output().unwrap();
    runtime.set_selection(SelectionRange::new((0, 1), (0, 3)));
    let mut clipboard = MemoryClipboard::default();

    assert!(runtime.copy_selection_to_clipboard(&mut clipboard));
    assert_eq!(clipboard.read_text().as_deref(), Some("ell"));
}

#[test]
fn native_terminal_runtime_reads_clipboard_and_encodes_paste_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?2004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    let clipboard = MemoryClipboard::new("from clipboard");

    assert!(runtime.send_clipboard_paste(&clipboard).unwrap());

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        b"\x1b[200~from clipboard\x1b[201~"
    );
}

#[test]
fn native_terminal_runtime_does_not_count_clipboard_paste_without_session() {
    let mut runtime = NativeTerminalRuntime::<MockPtySession>::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    let clipboard = MemoryClipboard::new("from clipboard");

    assert!(!runtime.send_clipboard_paste(&clipboard).unwrap());

    let metrics = runtime.dump_runtime_perf_metrics();
    assert_eq!(metrics.clipboard_pastes, 0);
    assert_eq!(metrics.paste_bytes, 0);
    assert!(!runtime.has_shell_session());
}

#[test]
fn native_terminal_runtime_syncs_osc52_clipboard_text_to_host_clipboard() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b]52;c;ZnJvbSBvc2M1Mg==\x07".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();
    let mut clipboard = MemoryClipboard::default();

    assert!(runtime.sync_terminal_clipboard(&mut clipboard));
    assert_eq!(clipboard.read_text().as_deref(), Some("from osc52"));
    assert!(!runtime.sync_terminal_clipboard(&mut clipboard));
}

#[test]
fn native_terminal_runtime_writes_committed_text_to_pty() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?2004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    runtime.send_committed_text("olá").unwrap();

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().last().unwrap().as_slice(),
        "olá".as_bytes()
    );
}

#[test]
fn native_terminal_runtime_ignores_empty_pty_input_writes() {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();

    runtime.send_pty_input(b"").unwrap();
    runtime.send_paste_text("").unwrap();
    runtime.send_committed_text("").unwrap();

    let session = runtime.shell_session().unwrap();
    assert!(session.input.borrow().is_empty());
    assert_eq!(
        runtime.dump_runtime_perf_metrics(),
        NativeRuntimePerfSnapshot::default()
    );
}
