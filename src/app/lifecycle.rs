//! Native app configuration, user events, and lifecycle state.

use std::sync::Arc;
use std::time::{Duration, Instant};

use winit::dpi::LogicalSize;
use winit::event_loop::EventLoopProxy;
use winit::window::{Window, WindowAttributes};

use crate::config::GromaqConfig;

use super::NativeAppError;

mod report;

pub use report::NativeAppRunReport;

use report::{NativeAppRunReportInput, PresentedFrameIntervals};

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

/// Deterministic action requested by the native app lifecycle.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAppAction {
    /// No platform action is needed.
    None,
    /// Create the native window.
    CreateWindow,
    /// Request a redraw for the current native window.
    RequestRedraw,
    /// Exit the event loop.
    Exit,
}

/// User events sent into the native app event loop from background workers.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeAppEvent {
    /// The PTY background reader observed output and the app should pump it promptly.
    PtyOutputReady,
}

/// Clonable sender for native app user events.
#[derive(Clone)]
pub struct NativeAppEventProxy {
    sender: Arc<dyn Fn(NativeAppEvent) + Send + Sync>,
}

impl NativeAppEventProxy {
    /// Build a proxy from a custom sender.
    pub fn from_sender<F>(sender: F) -> Self
    where
        F: Fn(NativeAppEvent) + Send + Sync + 'static,
    {
        Self {
            sender: Arc::new(sender),
        }
    }

    /// Send one user event into the native app loop.
    pub fn send(&self, event: NativeAppEvent) {
        (self.sender)(event);
    }
}

impl std::fmt::Debug for NativeAppEventProxy {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("NativeAppEventProxy")
            .finish_non_exhaustive()
    }
}

impl From<EventLoopProxy<NativeAppEvent>> for NativeAppEventProxy {
    fn from(proxy: EventLoopProxy<NativeAppEvent>) -> Self {
        Self::from_sender(move |event| {
            let _ = proxy.send_event(event);
        })
    }
}

/// Testable native app lifecycle state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeAppLifecycle {
    config: NativeAppConfig,
    has_window: bool,
    close_requested: bool,
    windows_created: u64,
    redraw_requests: u64,
    frames_presented: u64,
    monitor_refresh_millihertz: Option<u32>,
    surface_present_mode: Option<&'static str>,
    window_width_px: Option<u32>,
    window_height_px: Option<u32>,
    window_scale_milliscale: Option<u32>,
    frame_intervals: PresentedFrameIntervals,
}

impl NativeAppLifecycle {
    /// Create lifecycle state for a native app configuration.
    pub fn new(config: NativeAppConfig) -> Self {
        Self {
            config,
            has_window: false,
            close_requested: false,
            windows_created: 0,
            redraw_requests: 0,
            frames_presented: 0,
            monitor_refresh_millihertz: None,
            surface_present_mode: None,
            window_width_px: None,
            window_height_px: None,
            window_scale_milliscale: None,
            frame_intervals: PresentedFrameIntervals::default(),
        }
    }

    /// Access native app configuration.
    pub fn config(&self) -> &NativeAppConfig {
        &self.config
    }

    /// Apply native app settings that can change without recreating the window.
    pub fn apply_config(&mut self, config: NativeAppConfig) {
        self.config = config;
    }

    /// Handle a platform resume notification.
    pub fn on_resumed(&mut self) -> NativeAppAction {
        if self.has_window {
            NativeAppAction::None
        } else {
            NativeAppAction::CreateWindow
        }
    }

    /// Record that the native window was created.
    pub fn on_window_created(&mut self) {
        self.on_window_created_with_monitor_refresh(None);
    }

    /// Record that the native window was created on a monitor with a known refresh rate.
    pub fn on_window_created_with_monitor_refresh(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
    ) {
        self.on_window_created_with_surface_report(monitor_refresh_millihertz, None);
    }

    /// Record that the native window was created with known monitor/surface metadata.
    pub fn on_window_created_with_surface_report(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
        surface_present_mode: Option<&'static str>,
    ) {
        self.on_window_created_with_full_report(
            monitor_refresh_millihertz,
            surface_present_mode,
            None,
            None,
            None,
        );
    }

    /// Record that the native window was created with known monitor, surface, and window metadata.
    pub fn on_window_created_with_full_report(
        &mut self,
        monitor_refresh_millihertz: Option<u32>,
        surface_present_mode: Option<&'static str>,
        window_width_px: Option<u32>,
        window_height_px: Option<u32>,
        window_scale_milliscale: Option<u32>,
    ) {
        self.has_window = true;
        self.windows_created += 1;
        self.monitor_refresh_millihertz = monitor_refresh_millihertz;
        self.surface_present_mode = surface_present_mode;
        self.window_width_px = window_width_px;
        self.window_height_px = window_height_px;
        self.window_scale_milliscale = window_scale_milliscale;
    }

    /// Handle the event-loop idle boundary before waiting for more events.
    pub fn on_about_to_wait(&mut self) -> NativeAppAction {
        self.on_about_to_wait_at(Instant::now())
    }

    /// Handle the event-loop idle boundary at a deterministic instant.
    pub fn on_about_to_wait_at(&mut self, _now: Instant) -> NativeAppAction {
        if self.close_requested {
            NativeAppAction::Exit
        } else {
            NativeAppAction::None
        }
    }

    /// Record that terminal output changed the grid and a redraw should be scheduled.
    pub fn on_terminal_output_ready(&mut self) -> NativeAppAction {
        if self.has_window && !self.close_requested {
            self.redraw_requests += 1;
            NativeAppAction::RequestRedraw
        } else if self.close_requested {
            NativeAppAction::Exit
        } else {
            NativeAppAction::None
        }
    }

    /// Handle a native event-loop user event.
    pub fn on_user_event(&mut self, event: NativeAppEvent) -> NativeAppAction {
        match event {
            NativeAppEvent::PtyOutputReady => self.on_terminal_output_ready(),
        }
    }

    /// Next timer deadline for polling PTY output without forcing a redraw.
    pub fn next_pty_pump_deadline(&self, now: Instant) -> Option<Instant> {
        if self.has_window && !self.close_requested {
            Some(now + self.frame_interval_target_duration())
        } else {
            None
        }
    }

    /// Record that the native window requested application shutdown.
    pub fn on_close_requested(&mut self) -> NativeAppAction {
        self.close_requested = true;
        NativeAppAction::Exit
    }

    /// Record that the native window was destroyed.
    pub fn on_destroyed(&mut self) -> NativeAppAction {
        self.has_window = false;
        NativeAppAction::Exit
    }

    /// Record that a redraw was presented by the native app boundary.
    pub fn on_redraw_requested(&mut self) -> NativeAppAction {
        self.on_redraw_requested_at(Instant::now())
    }

    /// Record that a redraw was presented by the native app boundary at `presented_at`.
    pub fn on_redraw_requested_at(&mut self, presented_at: Instant) -> NativeAppAction {
        self.record_frame_presented_at(presented_at);
        self.frames_presented += 1;
        let Some(limit) = self.config.exit_after_presented_frames else {
            return NativeAppAction::None;
        };
        if self.frames_presented >= limit {
            self.close_requested = true;
            NativeAppAction::Exit
        } else if self.config.redraw_until_presented_frame_limit && self.has_window {
            self.redraw_requests += 1;
            NativeAppAction::RequestRedraw
        } else {
            NativeAppAction::None
        }
    }

    /// Whether the lifecycle currently owns a native window.
    pub fn has_window(&self) -> bool {
        self.has_window
    }

    /// Whether shutdown was requested.
    pub fn close_requested(&self) -> bool {
        self.close_requested
    }

    /// Count of native windows created by this lifecycle.
    pub fn windows_created(&self) -> u64 {
        self.windows_created
    }

    /// Count of redraw requests scheduled by this lifecycle.
    pub fn redraw_requests(&self) -> u64 {
        self.redraw_requests
    }

    /// Count of redraw events observed by this lifecycle.
    pub fn frames_presented(&self) -> u64 {
        self.frames_presented
    }

    /// Snapshot event-loop metrics after the native app exits.
    pub fn run_report(&self) -> NativeAppRunReport {
        self.frame_intervals.run_report(NativeAppRunReportInput {
            windows_created: self.windows_created,
            redraw_requests: self.redraw_requests,
            frames_presented: self.frames_presented,
            monitor_refresh_millihertz: self.monitor_refresh_millihertz,
            surface_present_mode: self.surface_present_mode,
            window_width_px: self.window_width_px,
            window_height_px: self.window_height_px,
            window_scale_milliscale: self.window_scale_milliscale,
            frame_interval_target_fps: self.frame_interval_target_fps(),
        })
    }

    fn record_frame_presented_at(&mut self, presented_at: Instant) {
        let target_fps = self.frame_interval_target_fps();
        self.frame_intervals
            .record_presented_at(presented_at, target_fps);
    }

    fn frame_interval_target_fps(&self) -> u32 {
        self.monitor_refresh_millihertz
            .map(refresh_millihertz_to_fps)
            .map(|refresh_fps| refresh_fps.min(self.config.target_fps.max(1)))
            .unwrap_or_else(|| self.config.target_fps.max(1))
    }

    fn frame_interval_target_duration(&self) -> Duration {
        Duration::from_nanos(NANOS_PER_SECOND / u64::from(self.frame_interval_target_fps()))
    }
}

fn refresh_millihertz_to_fps(refresh_millihertz: u32) -> u32 {
    refresh_millihertz
        .saturating_add(999)
        .saturating_div(1_000)
        .max(1)
}
