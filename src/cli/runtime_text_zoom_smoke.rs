use crate::app::{NativeAppConfig, NativeTerminalApp, NativeTextZoomAction};
use crate::cli::CliExit;

pub(in crate::cli) fn runtime_text_zoom_smoke_exit() -> CliExit {
    let mut app = match NativeTerminalApp::new(NativeAppConfig::default()) {
        Ok(app) => app,
        Err(error) => return runtime_text_zoom_smoke_error(error),
    };
    let default_metrics = renderer_metrics(&app);
    let default_grid = terminal_grid(&app);

    let zoomed = match app.apply_text_zoom_action(NativeTextZoomAction::Increase) {
        Ok(changed) => changed,
        Err(error) => return runtime_text_zoom_smoke_error(error),
    };
    let zoomed_metrics = renderer_metrics(&app);
    let zoomed_grid = terminal_grid(&app);
    let zoom_in_reduced_grid = zoomed_grid.0 < default_grid.0 && zoomed_grid.1 < default_grid.1;

    let reset = match app.apply_text_zoom_action(NativeTextZoomAction::Reset) {
        Ok(changed) => changed,
        Err(error) => return runtime_text_zoom_smoke_error(error),
    };
    let reset_metrics = renderer_metrics(&app);
    let reset_grid = terminal_grid(&app);
    let reset_restored_metrics = reset_metrics == default_metrics;
    let reset_restored_grid = reset_grid == default_grid;

    if !zoomed || !zoom_in_reduced_grid || !reset || !reset_restored_metrics || !reset_restored_grid
    {
        return runtime_text_zoom_smoke_error(format!(
            "unexpected zoom state: zoomed={zoomed}, zoom_in_reduced_grid={zoom_in_reduced_grid}, reset={reset}, reset_restored_metrics={reset_restored_metrics}, reset_restored_grid={reset_restored_grid}"
        ));
    }

    CliExit {
        code: 0,
        stdout: format!(
            "runtime text zoom smoke: ok\ndefault font size px: {}\ndefault cell width px: {}\ndefault line height px: {}\ndefault grid: {}x{}\nzoomed font size px: {}\nzoomed cell width px: {}\nzoomed line height px: {}\nzoomed grid: {}x{}\nzoom in reduced grid: {}\nreset restored metrics: {}\nreset restored grid: {}\n",
            default_metrics.0,
            default_metrics.1,
            default_metrics.2,
            default_grid.0,
            default_grid.1,
            zoomed_metrics.0,
            zoomed_metrics.1,
            zoomed_metrics.2,
            zoomed_grid.0,
            zoomed_grid.1,
            zoom_in_reduced_grid,
            reset_restored_metrics,
            reset_restored_grid
        ),
        stderr: String::new(),
    }
}

fn renderer_metrics(app: &NativeTerminalApp) -> (u16, u16, u16) {
    let config = app.renderer().config();
    (
        config.font_size_px,
        config.cell_width_px,
        config.line_height_px,
    )
}

fn terminal_grid(app: &NativeTerminalApp) -> (u16, u16) {
    let grid = app.runtime().terminal().dump_grid();
    (grid.cols, grid.rows)
}

fn runtime_text_zoom_smoke_error(error: impl std::fmt::Display) -> CliExit {
    CliExit {
        code: 1,
        stdout: String::new(),
        stderr: format!("runtime text zoom smoke failed: {error}\n"),
    }
}
