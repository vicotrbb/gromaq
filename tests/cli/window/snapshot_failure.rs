use std::cell::RefCell;
use std::fs;

use gromaq::app::NativeAppRunReport;
use gromaq::cli::{NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher};

use super::super::{MockBackend, run_with_backend_and_app, test_cli_config_path};

#[test]
fn window_tmux_manager_snapshot_failure_reports_missing_pane_command() {
    let backend = MockBackend {
        requests: RefCell::new(Vec::new()),
    };
    let app = MissingPaneCommandAppLauncher;
    let path = test_cli_config_path("window-tmux-manager-no-pane-command.ppm");
    let _ = fs::remove_file(&path);

    let exit = run_with_backend_and_app(
        [
            "gromaq",
            "--window-tmux-manager-snapshot",
            &path.to_string_lossy(),
        ],
        &backend,
        &app,
    );

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(exit.stderr.contains("window tmux manager snapshot failed"));
    assert!(exit.stderr.contains("snapshot written: true"));
    assert!(exit.stderr.contains("tmux status strip rendered: true"));
    assert!(
        exit.stderr
            .contains("tmux status pane command rendered: false")
    );
    assert!(exit.stderr.contains("tmux manager panel rendered: true"));
    assert!(!exit.stderr.contains("no snapshot was written"));
    assert!(path.exists());
    let _ = fs::remove_file(&path);
}

#[derive(Debug)]
struct MissingPaneCommandAppLauncher;

impl NativeAppLauncher for MissingPaneCommandAppLauncher {
    fn launch(
        &self,
        config: NativeAppLaunchConfig,
    ) -> Result<NativeAppRunReport, NativeAppLaunchError> {
        let snapshot_bytes = b"P6\n1 1\n255\n\x17\x1b$";
        if let Some(path) = &config.app.glyph_frame_snapshot_path {
            fs::write(path, snapshot_bytes)
                .map_err(|error| NativeAppLaunchError::new(error.to_string()))?;
        }
        Ok(NativeAppRunReport {
            glyph_frame_snapshot_written: true,
            glyph_frame_snapshot_bytes: snapshot_bytes.len(),
            glyph_frame_snapshot_width: 1,
            glyph_frame_snapshot_height: 1,
            tmux_status_strip_rendered: true,
            tmux_status_pane_command_rendered: false,
            tmux_manager_panel_rendered: true,
            glyph_frame_glyph_quads: 12,
            glyph_frame_background_quads: 1,
            ..NativeAppRunReport::default()
        })
    }
}
