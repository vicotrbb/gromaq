use super::*;
use std::time::Duration;

#[test]
fn dropped_frames_for_interval_counts_missed_target_periods() {
    assert_eq!(dropped_frames_for_interval(6_944_444, 144), 0);
    assert_eq!(dropped_frames_for_interval(13_888_888, 144), 1);
    assert_eq!(dropped_frames_for_interval(27_777_776, 144), 3);
}

#[test]
fn presented_frame_intervals_report_dropped_frames() {
    let started_at = Instant::now();
    let mut intervals = PresentedFrameIntervals::default();

    intervals.record_presented_at(started_at, 144, 1, 0);
    intervals.record_presented_at(started_at + Duration::from_nanos(6_944_444), 144, 2, 0);
    intervals.record_presented_at(started_at + Duration::from_nanos(27_777_776), 144, 3, 0);

    let report = intervals.run_report(NativeAppRunReportInput {
        windows_created: 1,
        redraw_requests: 2,
        frames_presented: 3,
        frame_interval_target_fps: 144,
        ..NativeAppRunReportInput::default()
    });

    assert_eq!(report.frame_interval_samples, 2);
    assert_eq!(report.monitor_refresh_millihertz, None);
    assert_eq!(report.surface_present_mode, None);
    assert_eq!(report.window_width_px, None);
    assert_eq!(report.window_height_px, None);
    assert_eq!(report.window_scale_milliscale, None);
    assert_eq!(report.frame_interval_target_fps, 144);
    assert_eq!(report.frame_interval_warmup_frames, 0);
    assert_eq!(report.frame_interval_max_sample_index, 2);
    assert_eq!(report.frame_interval_p95_exact_ns, 20_833_332);
    assert_eq!(report.frame_intervals_over_target, 1);
    assert_eq!(report.frame_intervals_over_double_target, 1);
    assert_eq!(report.dropped_frames, 2);
    assert_eq!(report.first_dropped_frame_interval_sample, 2);
    assert_eq!(report.last_dropped_frame_interval_sample, 2);
}

#[test]
fn presented_frame_intervals_exact_p95_requires_full_bounded_sample_set() {
    let started_at = Instant::now();
    let mut intervals = PresentedFrameIntervals::default();

    for frame in 0..=PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY {
        intervals.record_presented_at(
            started_at + Duration::from_nanos(frame as u64),
            144,
            u64::try_from(frame + 1).unwrap(),
            0,
        );
    }

    let report = intervals.run_report(NativeAppRunReportInput {
        frames_presented: u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 1).unwrap(),
        frame_interval_target_fps: 144,
        ..NativeAppRunReportInput::default()
    });

    assert_eq!(
        report.frame_interval_samples,
        u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY).unwrap()
    );
    assert_eq!(report.frame_interval_p95_exact_ns, 1);

    intervals.record_presented_at(
        started_at
            + Duration::from_nanos(
                u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 1).unwrap(),
            ),
        144,
        u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 2).unwrap(),
        0,
    );
    let report = intervals.run_report(NativeAppRunReportInput {
        frames_presented: u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 2).unwrap(),
        frame_interval_target_fps: 144,
        ..NativeAppRunReportInput::default()
    });

    assert_eq!(
        report.frame_interval_samples,
        u64::try_from(PRESENTED_FRAME_INTERVAL_SAMPLE_CAPACITY + 1).unwrap()
    );
    assert_eq!(report.frame_interval_p95_exact_ns, 0);
}
