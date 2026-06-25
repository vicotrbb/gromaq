use super::*;

#[test]
fn terminal_text_target_dimensions_reports_checked_size() {
    let dimensions = checked_terminal_text_target_dimensions(80, 24, 8, 16).unwrap();

    assert_eq!(dimensions, (640, 384));
}

#[test]
fn terminal_text_target_dimensions_rejects_overflowing_width() {
    let error = checked_terminal_text_target_dimensions(2, 1, u32::MAX, 1).unwrap_err();

    assert_eq!(
        error,
        GpuBootstrapError::SmokeReadback(
            "terminal text target width is too large to represent".to_owned()
        )
    );
}

#[test]
fn terminal_text_target_dimensions_rejects_overflowing_height() {
    let error = checked_terminal_text_target_dimensions(1, 2, 1, u32::MAX).unwrap_err();

    assert_eq!(
        error,
        GpuBootstrapError::SmokeReadback(
            "terminal text target height is too large to represent".to_owned()
        )
    );
}

#[test]
fn terminal_text_perf_average_reports_zero_without_samples() {
    assert_eq!(average_duration_ns(&[]), 0);
}

#[test]
fn terminal_text_perf_p95_reports_zero_without_samples() {
    assert_eq!(p95_duration_ns(&[]), 0);
}

#[test]
fn terminal_text_perf_p95_uses_inclusive_rank() {
    assert_eq!(p95_duration_ns(&[10, 20, 30, 40, 50]), 50);
}
