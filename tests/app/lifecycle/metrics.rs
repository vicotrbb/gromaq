use std::time::{Duration, Instant};

use gromaq::app::{NativeAppConfig, NativeAppLifecycle, NativeGlyphFramePresentation};

#[test]
fn native_app_lifecycle_reports_dropped_presented_frame_intervals() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        exit_after_presented_frames: Some(3),
        redraw_until_presented_frame_limit: true,
        ..NativeAppConfig::default()
    });
    let first_presented_at = Instant::now();

    lifecycle.on_window_created();
    lifecycle.on_redraw_requested_at(first_presented_at);
    lifecycle.on_redraw_requested_at(first_presented_at + Duration::from_nanos(6_944_444));
    lifecycle.on_redraw_requested_at(first_presented_at + Duration::from_nanos(27_777_776));

    let report = lifecycle.run_report();

    assert_eq!(report.frames_presented, 3);
    assert_eq!(report.frame_interval_samples, 2);
    assert_eq!(report.frame_interval_p95_exact_ns, 20_833_332);
    assert_eq!(report.frame_interval_max_sample_index, 2);
    assert_eq!(report.frame_intervals_over_target, 1);
    assert_eq!(report.frame_intervals_over_double_target, 1);
    assert_eq!(report.dropped_frames, 2);
    assert_eq!(report.first_dropped_frame_interval_sample, 2);
    assert_eq!(report.last_dropped_frame_interval_sample, 2);
}

#[test]
fn native_app_lifecycle_excludes_configured_warmup_frame_intervals() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        exit_after_presented_frames: Some(4),
        redraw_until_presented_frame_limit: true,
        frame_interval_warmup_frames: 2,
        ..NativeAppConfig::default()
    });
    let first_presented_at = Instant::now();

    lifecycle.on_window_created();
    lifecycle.on_redraw_requested_at(first_presented_at);
    lifecycle.on_redraw_requested_at(first_presented_at + Duration::from_nanos(50_000_000));
    lifecycle.on_redraw_requested_at(first_presented_at + Duration::from_nanos(56_944_444));
    lifecycle.on_redraw_requested_at(first_presented_at + Duration::from_nanos(63_888_888));

    let report = lifecycle.run_report();

    assert_eq!(report.frames_presented, 4);
    assert_eq!(report.frame_interval_warmup_frames, 2);
    assert_eq!(report.frame_interval_samples, 2);
    assert_eq!(report.frame_interval_max_sample_index, 1);
    assert_eq!(report.dropped_frames, 0);
    assert_eq!(report.first_dropped_frame_interval_sample, 0);
    assert_eq!(report.last_dropped_frame_interval_sample, 0);
}

#[test]
fn native_app_lifecycle_accounts_frame_intervals_against_monitor_refresh() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig {
        target_fps: 144,
        exit_after_presented_frames: Some(3),
        redraw_until_presented_frame_limit: true,
        ..NativeAppConfig::default()
    });
    let first_presented_at = Instant::now();
    let monitor_frame_interval = Duration::from_nanos(16_666_666);

    lifecycle.on_window_created_with_surface_report(Some(60_000), Some("Mailbox"));
    lifecycle.on_redraw_requested_at(first_presented_at);
    lifecycle.on_redraw_requested_at(first_presented_at + monitor_frame_interval);
    lifecycle.on_redraw_requested_at(first_presented_at + monitor_frame_interval * 2);

    let report = lifecycle.run_report();

    assert_eq!(report.monitor_refresh_millihertz, Some(60_000));
    assert_eq!(report.surface_present_mode, Some("Mailbox"));
    assert_eq!(report.frame_interval_target_fps, 60);
    assert_eq!(report.frame_interval_samples, 2);
    assert_eq!(report.frame_intervals_over_target, 0);
    assert_eq!(report.frame_intervals_over_double_target, 0);
    assert_eq!(report.dropped_frames, 0);
}

#[test]
fn native_app_lifecycle_reports_window_surface_size_and_scale() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());

    lifecycle.on_window_created_with_full_report(
        Some(120_000),
        Some("Fifo"),
        Some(2560),
        Some(1600),
        Some(2000),
    );

    let report = lifecycle.run_report();

    assert_eq!(report.monitor_refresh_millihertz, Some(120_000));
    assert_eq!(report.surface_present_mode, Some("Fifo"));
    assert_eq!(report.window_width_px, Some(2560));
    assert_eq!(report.window_height_px, Some(1600));
    assert_eq!(report.window_scale_milliscale, Some(2000));
}

#[test]
fn native_app_lifecycle_reports_last_glyph_frame_presentation() {
    let mut lifecycle = NativeAppLifecycle::new(NativeAppConfig::default());

    lifecycle.record_glyph_frame_presentation(NativeGlyphFramePresentation {
        rendered: true,
        glyph_frame_presented: true,
        tmux_status_strip_rendered: true,
        tmux_status_pane_command_rendered: true,
        tmux_manager_panel_rendered: true,
        tmux_manager_sessions: 2,
        tmux_manager_windows: 3,
        tmux_manager_panes: 4,
        clear_presented: false,
        width: 2560,
        height: 1600,
        glyph_quads: 12,
        background_quads: 1,
        decoration_quads: 0,
        cursor_quads: 1,
        atlas_bytes: 4096,
        atlas_occupied_slots: 8,
        snapshot_written: true,
        snapshot_bytes: 42,
        snapshot_width: 80,
        snapshot_height: 24,
    });

    let report = lifecycle.run_report();

    assert!(report.glyph_frame_presented);
    assert!(report.tmux_status_strip_rendered);
    assert!(report.tmux_status_pane_command_rendered);
    assert!(report.tmux_manager_panel_rendered);
    assert_eq!(report.tmux_manager_sessions, 2);
    assert_eq!(report.tmux_manager_windows, 3);
    assert_eq!(report.tmux_manager_panes, 4);
    assert_eq!(report.glyph_frame_width, 2560);
    assert_eq!(report.glyph_frame_height, 1600);
    assert_eq!(report.glyph_frame_glyph_quads, 12);
    assert_eq!(report.glyph_frame_background_quads, 1);
    assert_eq!(report.glyph_frame_decoration_quads, 0);
    assert_eq!(report.glyph_frame_cursor_quads, 1);
    assert_eq!(report.glyph_frame_atlas_bytes, 4096);
    assert_eq!(report.glyph_frame_atlas_occupied_slots, 8);
    assert!(report.glyph_frame_snapshot_written);
    assert_eq!(report.glyph_frame_snapshot_bytes, 42);
    assert_eq!(report.glyph_frame_snapshot_width, 80);
    assert_eq!(report.glyph_frame_snapshot_height, 24);
}
