//! Offscreen GPU welcome splash image smoke.
//!
//! Renders the embedded welcome avatar as a real textured image quad centered
//! on the default theme background and reads it back, proving the GPU can draw
//! the avatar as a crisp image (not ANSI block characters) for the welcome
//! splash. The snapshot is written as a PPM artifact for inspection.

use std::path::Path;

use super::super::reports::{GpuWelcomeImageSnapshotReport, GpuWelcomeImageSnapshotRunner};
use super::super::{GpuBootstrapError, NativeGpuContext, UploadPattern};
use snapshot::{composite_over_background, ppm_bytes, welcome_image_report};

mod snapshot;

/// Embedded 320x320 RGBA8 avatar produced by `images/avatar/generate.mjs`
/// (`avatar-splash.rgba`), already contrast-floored for the dark background.
const AVATAR_SPLASH_RGBA: &[u8] = include_bytes!("../../../images/avatar/avatar-splash.rgba");
const AVATAR_SIZE: u32 = 320;
const TARGET_WIDTH: u32 = 480;
const TARGET_HEIGHT: u32 = 480;
const FIT_FRACTION: f32 = 0.8;
/// Default gromaq-ghostty background `#101216` in normalized RGBA.
const BACKGROUND_RGBA: [f32; 4] = [
    0x10 as f32 / 255.0,
    0x12 as f32 / 255.0,
    0x16 as f32 / 255.0,
    1.0,
];
const BACKGROUND_PIXEL: [u8; 4] = [0x10, 0x12, 0x16, 255];

impl GpuWelcomeImageSnapshotRunner for NativeGpuContext {
    fn run_welcome_image_snapshot(
        &self,
        path: &Path,
    ) -> std::result::Result<GpuWelcomeImageSnapshotReport, GpuBootstrapError> {
        let pattern = UploadPattern {
            width: AVATAR_SIZE,
            height: AVATAR_SIZE,
            // The offscreen image-quad pipeline draws with REPLACE blend (no
            // alpha blending), so composite the avatar over the theme
            // background here: figure pixels keep their color, transparent
            // padding becomes the background, and the result is fully opaque.
            rgba: composite_over_background(AVATAR_SPLASH_RGBA, BACKGROUND_PIXEL),
        };
        let pixels = self.draw_image_quad_and_readback(
            &pattern,
            TARGET_WIDTH,
            TARGET_HEIGHT,
            BACKGROUND_RGBA,
            FIT_FRACTION,
        )?;
        let report = welcome_image_report(&pixels, TARGET_WIDTH, TARGET_HEIGHT, BACKGROUND_PIXEL)?;

        let snapshot = ppm_bytes(TARGET_WIDTH, TARGET_HEIGHT, &pixels)?;
        std::fs::write(path, &snapshot).map_err(|error| {
            GpuBootstrapError::SmokeReadback(format!(
                "failed to write welcome image snapshot to {}: {error}",
                path.display()
            ))
        })?;

        Ok(GpuWelcomeImageSnapshotReport {
            width: TARGET_WIDTH,
            height: TARGET_HEIGHT,
            bytes_written: snapshot.len(),
            background_pixel: report.background_pixel,
            image_pixel: report.image_pixel,
            drawn_pixels: report.drawn_pixels,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_avatar_splash_rgba_is_320_square() {
        assert_eq!(
            AVATAR_SPLASH_RGBA.len(),
            usize::try_from(AVATAR_SIZE * AVATAR_SIZE * 4).unwrap()
        );
    }
}
