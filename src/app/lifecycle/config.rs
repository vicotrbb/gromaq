use std::path::PathBuf;
use std::time::Duration;

use winit::dpi::LogicalSize;
use winit::window::{Window, WindowAttributes};

use crate::config::GromaqConfig;

use super::super::NativeAppError;

const NANOS_PER_SECOND: u64 = 1_000_000_000;

/// Native window and frame-loop configuration.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAppConfig {
    /// Native window title.
    pub title: String,
    /// Initial window width in logical pixels.
    pub width: u32,
    /// Initial window height in logical pixels.
    pub height: u32,
    /// Target frames per second for redraw scheduling.
    pub target_fps: u32,
    /// Optional presented-frame limit after which the native app exits.
    pub exit_after_presented_frames: Option<u64>,
    /// Request redraws after presented frames until the configured frame limit is reached.
    pub redraw_until_presented_frame_limit: bool,
    /// Number of initial presented frames excluded from frame-interval performance metrics.
    pub frame_interval_warmup_frames: u64,
    /// Optional PPM artifact path for the first presented native glyph frame.
    pub glyph_frame_snapshot_path: Option<PathBuf>,
    /// Optional deterministic terminal text written before the native window presents.
    pub startup_text: Option<String>,
}

impl Default for NativeAppConfig {
    fn default() -> Self {
        Self {
            title: "Gromaq".to_owned(),
            width: 1280,
            height: 800,
            target_fps: 144,
            exit_after_presented_frames: None,
            redraw_until_presented_frame_limit: false,
            frame_interval_warmup_frames: 0,
            glyph_frame_snapshot_path: None,
            startup_text: None,
        }
    }
}

impl NativeAppConfig {
    /// Build native app configuration from validated user configuration.
    pub fn from_gromaq_config(config: &GromaqConfig) -> Result<Self, NativeAppError> {
        config
            .validate()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        Ok(Self {
            target_fps: config.performance.target_fps,
            ..Self::default()
        })
    }

    /// Build `winit` window attributes for the terminal window.
    pub fn window_attributes(&self) -> WindowAttributes {
        Window::default_attributes()
            .with_title(self.title.clone())
            .with_inner_size(LogicalSize::new(
                f64::from(self.width),
                f64::from(self.height),
            ))
            .with_visible(true)
            .with_resizable(true)
    }

    /// Target frame interval derived from `target_fps`.
    pub fn target_frame_interval(&self) -> Duration {
        Duration::from_nanos(NANOS_PER_SECOND / u64::from(self.target_fps.max(1)))
    }
}
