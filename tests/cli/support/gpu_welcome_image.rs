use std::fs;
use std::path::Path;

use gromaq::native_gpu::{
    GpuBootstrapError, GpuWelcomeImageSnapshotReport, GpuWelcomeImageSnapshotRunner,
};

use super::MockContext;

impl GpuWelcomeImageSnapshotRunner for MockContext {
    fn run_welcome_image_snapshot(
        &self,
        path: &Path,
    ) -> Result<GpuWelcomeImageSnapshotReport, GpuBootstrapError> {
        let snapshot = b"P6\n2 2\n255\n\x10\x12\x16\xea\xd6\xff\xea\xd6\xff\x10\x12\x16";
        fs::write(path, snapshot).map_err(|error| {
            GpuBootstrapError::SmokeReadback(format!("failed to write mock snapshot: {error}"))
        })?;
        Ok(GpuWelcomeImageSnapshotReport {
            width: 2,
            height: 2,
            bytes_written: snapshot.len(),
            background_pixel: [0x10, 0x12, 0x16, 255],
            image_pixel: [0xEA, 0xD6, 0xFF, 255],
            drawn_pixels: 2,
        })
    }
}
