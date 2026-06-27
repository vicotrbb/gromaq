//! Welcome splash avatar image snapshot CLI dispatch.

use std::path::Path;

use super::context::GpuCommandContext;
use crate::cli::CliExit;
use crate::native_gpu::{
    GpuBootstrap, GpuBootstrapBackend, GpuBootstrapConfig, GpuWelcomeImageSnapshotRunner,
};

pub(in crate::cli) fn gpu_welcome_image_snapshot_exit<B>(path: &str, backend: &B) -> CliExit
where
    B: GpuBootstrapBackend,
    B::Context: GpuCommandContext,
{
    let snapshot_path = Path::new(path);
    if snapshot_path.as_os_str().is_empty() {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: "snapshot path must not be empty\n".to_owned(),
        };
    }
    let bootstrap = GpuBootstrap::new(GpuBootstrapConfig::native_default());
    match bootstrap.initialize_with(backend) {
        Ok(context) => match context.run_welcome_image_snapshot(snapshot_path) {
            Ok(report) => CliExit {
                code: 0,
                stdout: format!(
                    "welcome image snapshot: ok\npath: {}\nsize: {}x{}\nbytes written: {}\nbackground pixel: {:?}\nimage pixel: {:?}\ndrawn pixels: {}\n",
                    snapshot_path.display(),
                    report.width,
                    report.height,
                    report.bytes_written,
                    report.background_pixel,
                    report.image_pixel,
                    report.drawn_pixels
                ),
                stderr: String::new(),
            },
            Err(error) => CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("welcome image snapshot failed: {error}\n"),
            },
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}
