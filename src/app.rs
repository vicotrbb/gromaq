//! Native `winit` application loop boundary.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, Instant};

use thiserror::Error;
use tracing::{debug, trace};
use winit::application::ApplicationHandler;
use winit::dpi::{LogicalSize, PhysicalPosition};
use winit::error::{EventLoopError, OsError};
use winit::event::{
    ElementState, Ime, MouseButton as WinitMouseButton, MouseScrollDelta, WindowEvent,
};
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy};
use winit::keyboard::{Key, ModifiersState, NamedKey, PhysicalKey};
use winit::window::{Window, WindowAttributes, WindowId};

use crate::clipboard::{HostClipboard, NativeClipboard};
use crate::config::{ConfigFileReloader, GromaqConfig};
use crate::font::{FontRasterError, RasterizedGlyphCache};
use crate::input::key_modifiers_from_winit;
use crate::mouse::{MouseButton, MouseEvent, MouseEventKind};
use crate::native_gpu::{
    GpuBootstrap, GpuBootstrapConfig, GpuBootstrapError, GpuSurfaceError, NativeGpuContext,
    NativeGpuWindowSurface,
};
use crate::pty::{PtyConfig, PtyError, PtySession, ShellCommand};
use crate::renderer::{
    GpuRenderer, PreparedSurfaceGlyphFrame, RendererConfig, SurfaceBackend, SurfaceConfigError,
    SurfaceConfigPlanner, SurfaceConfigurationController, SurfaceFrameBackend, SurfaceFrameError,
    SurfaceGlyphFrame, SurfaceLifecycleAction, WgpuRenderer, WgpuSurfaceBackend,
};
use crate::{SelectionRange, Terminal, TerminalConfig};

const NANOS_PER_SECOND: u64 = 1_000_000_000;
const RUNTIME_DURATION_BUCKETS_NS: [u64; 16] = [
    100_000,
    250_000,
    500_000,
    1_000_000,
    2_000_000,
    4_000_000,
    6_940_000,
    8_000_000,
    10_000_000,
    16_000_000,
    33_000_000,
    50_000_000,
    100_000_000,
    250_000_000,
    500_000_000,
    u64::MAX,
];

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
}

impl Default for NativeAppConfig {
    fn default() -> Self {
        Self {
            title: "Gromaq".to_owned(),
            width: 1280,
            height: 800,
            target_fps: 144,
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
}

/// Maps native window pixel positions to terminal grid-relative mouse events.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeMouseGridMapper {
    window_width_px: u32,
    window_height_px: u32,
    cols: u16,
    rows: u16,
}

/// Native window mouse input before terminal grid mapping.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeWindowMouseInput {
    /// Window-relative x coordinate in physical pixels.
    pub x: f64,
    /// Window-relative y coordinate in physical pixels.
    pub y: f64,
    /// Current window width in physical pixels.
    pub window_width_px: u32,
    /// Current window height in physical pixels.
    pub window_height_px: u32,
    /// Mouse event kind.
    pub kind: MouseEventKind,
    /// Mouse button identity.
    pub button: MouseButton,
    /// Active keyboard modifiers.
    pub modifiers: ModifiersState,
}

/// Tracks currently pressed native mouse buttons for cursor-move reporting.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeMouseButtonTracker {
    left: bool,
    middle: bool,
    right: bool,
}

impl NativeMouseButtonTracker {
    /// Record a native button press or release.
    pub fn set_pressed(&mut self, button: MouseButton, pressed: bool) {
        match button {
            MouseButton::Left => self.left = pressed,
            MouseButton::Middle => self.middle = pressed,
            MouseButton::Right => self.right = pressed,
            MouseButton::None | MouseButton::WheelUp | MouseButton::WheelDown => {}
        }
    }

    /// Mouse event kind and button identity to use for a cursor-move event.
    pub fn cursor_move_event(self) -> (MouseEventKind, MouseButton) {
        if self.left {
            (MouseEventKind::Drag, MouseButton::Left)
        } else if self.middle {
            (MouseEventKind::Drag, MouseButton::Middle)
        } else if self.right {
            (MouseEventKind::Drag, MouseButton::Right)
        } else {
            (MouseEventKind::Motion, MouseButton::None)
        }
    }
}

/// Terminal and PTY size requested by a native resize event.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativePtyResize {
    /// Terminal columns.
    pub cols: u16,
    /// Terminal rows.
    pub rows: u16,
    /// Pixel width of the PTY viewport.
    pub pixel_width: u16,
    /// Pixel height of the PTY viewport.
    pub pixel_height: u16,
}

/// Maps native window pixel sizes to terminal row/column counts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeResizeGridMapper {
    reference_width_px: u32,
    reference_height_px: u32,
    reference_cols: u16,
    reference_rows: u16,
}

/// Native window surface state owned by the app after a `wgpu` surface exists.
#[derive(Debug)]
pub struct NativeWindowSurface<B> {
    backend: B,
    capabilities: wgpu::SurfaceCapabilities,
    controller: SurfaceConfigurationController,
}

impl<B> NativeWindowSurface<B>
where
    B: SurfaceBackend,
{
    /// Create app-facing surface state for a concrete backend and capabilities.
    pub fn new(backend: B, capabilities: wgpu::SurfaceCapabilities) -> Self {
        Self {
            backend,
            capabilities,
            controller: SurfaceConfigurationController::new(SurfaceConfigPlanner::new()),
        }
    }

    /// Create and configure app-owned surface state from a GPU surface handoff.
    pub fn from_gpu_surface(
        gpu_surface: NativeGpuWindowSurface<B>,
        width: u32,
        height: u32,
    ) -> std::result::Result<Self, SurfaceConfigError> {
        let (backend, capabilities) = gpu_surface.into_parts();
        let mut surface = Self::new(backend, capabilities);
        surface.configure_initial(width, height)?;
        Ok(surface)
    }

    /// Configure the initial window surface size.
    pub fn configure_initial(
        &mut self,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.controller
            .configure(&mut self.backend, &self.capabilities, width, height)
    }

    /// Reconfigure the surface after a native resize when required.
    pub fn resize(
        &mut self,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.controller
            .resize(&mut self.backend, &self.capabilities, width, height)
    }

    /// Access the concrete surface backend.
    pub fn backend(&self) -> &B {
        &self.backend
    }

    /// Last configured non-zero surface size.
    pub fn configured_size(&self) -> Option<(u32, u32)> {
        self.controller.lifecycle().size()
    }

    /// Whether surface configuration is suspended for a zero-sized native window.
    pub fn is_suspended(&self) -> bool {
        self.controller.lifecycle().is_suspended()
    }

    /// Number of configure/reconfigure operations applied to the backend.
    pub fn configure_count(&self) -> u64 {
        self.controller.lifecycle().configure_count()
    }
}

impl<B> NativeWindowSurface<B>
where
    B: SurfaceFrameBackend,
{
    /// Clear the current native surface frame and present it.
    pub fn clear_and_present(
        &mut self,
        clear_color: [f64; 4],
    ) -> std::result::Result<(), SurfaceFrameError> {
        self.backend.clear_and_present(clear_color)
    }

    /// Render terminal glyph quads to the current native surface frame and present it.
    pub fn present_glyph_frame(
        &mut self,
        frame: SurfaceGlyphFrame<'_>,
    ) -> std::result::Result<(), SurfaceFrameError> {
        self.backend.present_glyph_frame(frame)
    }
}

/// Render dirty terminal state into a prepared glyph frame and present it through a native surface.
pub fn render_and_present_terminal_glyph_frame<S, B>(
    runtime: &mut NativeTerminalRuntime<S>,
    renderer: &mut WgpuRenderer,
    glyph_cache: &mut RasterizedGlyphCache,
    surface: &mut NativeWindowSurface<B>,
) -> Result<bool, NativeGlyphFrameError>
where
    B: SurfaceFrameBackend,
{
    if !runtime.render_terminal_frame(renderer) {
        return Ok(false);
    }
    let clear_color = renderer.config().clear_color;
    let Some(plan) = renderer.last_plan() else {
        return Ok(false);
    };
    if plan.glyphs.is_empty() {
        surface.clear_and_present(clear_color)?;
        return Ok(false);
    }
    let glyphs = glyph_cache.rasterize_plan(plan)?;
    let prepared = PreparedSurfaceGlyphFrame::from_render_plan(plan, &glyphs.bitmaps, clear_color)?;
    surface.present_glyph_frame(prepared.as_surface_glyph_frame())?;
    Ok(true)
}

/// Build the default native glyph cache from a system monospace font.
pub fn load_default_native_glyph_cache() -> Result<RasterizedGlyphCache, NativeAppError> {
    for path in DEFAULT_MONOSPACE_FONT_CANDIDATES {
        if Path::new(path).exists() {
            let mut font_bytes = vec![
                std::fs::read(path).map_err(|error| NativeAppError::Runtime(error.to_string()))?,
            ];
            for fallback_path in DEFAULT_FALLBACK_FONT_CANDIDATES {
                if Path::new(fallback_path).exists() {
                    font_bytes.push(
                        std::fs::read(fallback_path)
                            .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
                    );
                }
            }
            return RasterizedGlyphCache::from_font_bytes(font_bytes).map_err(NativeAppError::from);
        }
    }
    Err(NativeAppError::Runtime(
        "no default monospace system font found".to_owned(),
    ))
}

const DEFAULT_MONOSPACE_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
];

const DEFAULT_FALLBACK_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Apple Color Emoji.ttc",
    "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
];

impl NativeResizeGridMapper {
    /// Create a mapper from a non-empty reference window and terminal size.
    pub fn new(
        reference_width_px: u32,
        reference_height_px: u32,
        reference_cols: u16,
        reference_rows: u16,
    ) -> Option<Self> {
        if reference_width_px == 0
            || reference_height_px == 0
            || reference_cols == 0
            || reference_rows == 0
        {
            return None;
        }
        Some(Self {
            reference_width_px,
            reference_height_px,
            reference_cols,
            reference_rows,
        })
    }

    /// Convert a native window size into a terminal and PTY resize request.
    pub fn resize_for_window(self, width_px: u32, height_px: u32) -> Option<NativePtyResize> {
        if width_px == 0 || height_px == 0 {
            return None;
        }
        let cols = scaled_cells(width_px, self.reference_width_px, self.reference_cols);
        let rows = scaled_cells(height_px, self.reference_height_px, self.reference_rows);
        Some(NativePtyResize {
            cols,
            rows,
            pixel_width: clamp_u32_to_u16(width_px),
            pixel_height: clamp_u32_to_u16(height_px),
        })
    }
}

fn scaled_cells(actual_px: u32, reference_px: u32, reference_cells: u16) -> u16 {
    let scaled = (u64::from(actual_px) * u64::from(reference_cells)) / u64::from(reference_px);
    u16::try_from(scaled.max(1)).unwrap_or(u16::MAX)
}

fn clamp_u32_to_u16(value: u32) -> u16 {
    u16::try_from(value).unwrap_or(u16::MAX)
}

impl NativeMouseGridMapper {
    /// Create a mapper for a non-empty window and terminal grid.
    pub fn new(window_width_px: u32, window_height_px: u32, cols: u16, rows: u16) -> Option<Self> {
        if window_width_px == 0 || window_height_px == 0 || cols == 0 || rows == 0 {
            return None;
        }
        Some(Self {
            window_width_px,
            window_height_px,
            cols,
            rows,
        })
    }

    /// Convert a window pixel position to a grid-relative terminal mouse event.
    pub fn mouse_event_at(
        self,
        x: f64,
        y: f64,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Option<MouseEvent> {
        if !x.is_finite()
            || !y.is_finite()
            || x < 0.0
            || y < 0.0
            || x >= f64::from(self.window_width_px)
            || y >= f64::from(self.window_height_px)
        {
            return None;
        }
        let col = ((x / f64::from(self.window_width_px)) * f64::from(self.cols)) as u16;
        let row = ((y / f64::from(self.window_height_px)) * f64::from(self.rows)) as u16;
        Some(MouseEvent::new(
            kind,
            button,
            col.min(self.cols - 1),
            row.min(self.rows - 1),
        ))
    }

    /// Convert a window pixel position to a grid-relative mouse event with modifiers.
    pub fn mouse_event_at_with_modifiers(
        self,
        x: f64,
        y: f64,
        kind: MouseEventKind,
        button: MouseButton,
        modifiers: ModifiersState,
    ) -> Option<MouseEvent> {
        self.mouse_event_at(x, y, kind, button)
            .map(|event| event.with_modifiers(key_modifiers_from_winit(modifiers)))
    }
}

/// Native terminal runtime configuration shared by the app and PTY boundary.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeTerminalRuntimeConfig {
    /// Startup terminal columns.
    pub terminal_cols: u16,
    /// Startup terminal rows.
    pub terminal_rows: u16,
    /// Scrollback line limit.
    pub scrollback_lines: usize,
    /// PTY pixel width, if known.
    pub pixel_width: u16,
    /// PTY pixel height, if known.
    pub pixel_height: u16,
    /// Shell command to spawn.
    pub shell: ShellCommand,
}

impl Default for NativeTerminalRuntimeConfig {
    fn default() -> Self {
        let config = GromaqConfig::default();
        Self {
            terminal_cols: config.terminal.cols,
            terminal_rows: config.terminal.rows,
            scrollback_lines: config.terminal.scrollback_lines,
            pixel_width: 0,
            pixel_height: 0,
            shell: ShellCommand::default_shell(),
        }
    }
}

impl NativeTerminalRuntimeConfig {
    /// Build runtime configuration from validated user configuration and shell command.
    pub fn from_gromaq_config(
        config: &GromaqConfig,
        shell: ShellCommand,
    ) -> Result<Self, NativeAppError> {
        config
            .validate()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let runtime_config = Self {
            terminal_cols: config.terminal.cols,
            terminal_rows: config.terminal.rows,
            scrollback_lines: config.terminal.scrollback_lines,
            pixel_width: 0,
            pixel_height: 0,
            shell,
        };
        runtime_config.terminal_config()?;
        Ok(runtime_config)
    }

    fn terminal_config(&self) -> Result<TerminalConfig, NativeAppError> {
        TerminalConfig::new(self.terminal_cols, self.terminal_rows)
            .and_then(|config| config.with_pixel_size(self.pixel_width, self.pixel_height))
            .and_then(|config| config.with_scrollback_limit(self.scrollback_lines))
            .map_err(|error| NativeAppError::Runtime(error.to_string()))
    }

    fn pty_config(&self) -> PtyConfig {
        PtyConfig {
            rows: self.terminal_rows,
            cols: self.terminal_cols,
            pixel_width: self.pixel_width,
            pixel_height: self.pixel_height,
            shell: self.shell.clone(),
        }
    }
}

/// Deterministic native runtime counters for validation and performance probes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeRuntimePerfSnapshot {
    /// Number of non-empty PTY output batches pumped into terminal state.
    pub pty_output_batches: u64,
    /// Total PTY output bytes pumped into terminal state.
    pub pty_output_bytes: u64,
    /// Number of terminal-generated response writes sent back to the PTY.
    pub pty_response_writes: u64,
    /// Total terminal-generated response bytes sent back to the PTY.
    pub pty_response_bytes: u64,
    /// Number of app-originated PTY input writes.
    pub pty_input_writes: u64,
    /// Total app-originated PTY input bytes.
    pub pty_input_bytes: u64,
    /// Number of native key inputs encoded and written to the PTY.
    pub native_key_inputs: u64,
    /// Number of terminal mouse inputs encoded and written to the PTY.
    pub mouse_inputs: u64,
    /// Number of focus inputs encoded and written to the PTY.
    pub focus_inputs: u64,
    /// Number of clipboard paste actions that wrote text to the PTY.
    pub clipboard_pastes: u64,
    /// Total pasted text bytes written through the terminal paste path.
    pub paste_bytes: u64,
    /// Total committed text bytes written to the PTY.
    pub committed_text_bytes: u64,
    /// Number of successful terminal resize operations through the native runtime.
    pub resize_events: u64,
    /// Number of render attempts made by the native runtime.
    pub render_attempts: u64,
    /// Number of dirty terminal frames rendered through the renderer boundary.
    pub rendered_frames: u64,
    /// Number of render attempts skipped because no dirty regions were pending.
    pub clean_frame_skips: u64,
    /// Number of rendered frames with measured render duration samples.
    pub render_time_samples: u64,
    /// Total measured render-frame duration in nanoseconds.
    pub render_time_total_ns: u64,
    /// Maximum measured render-frame duration in nanoseconds.
    pub render_time_max_ns: u64,
    /// Approximate p95 render-frame duration in nanoseconds, using fixed buckets.
    pub render_time_p95_ns: u64,
    /// Number of app-input-to-render latency samples.
    pub input_to_render_samples: u64,
    /// Total app-input-to-render latency in nanoseconds.
    pub input_to_render_total_ns: u64,
    /// Maximum app-input-to-render latency in nanoseconds.
    pub input_to_render_max_ns: u64,
    /// Approximate p95 app-input-to-render latency in nanoseconds, using fixed buckets.
    pub input_to_render_p95_ns: u64,
}

/// Fixed-size duration histogram for bounded live-performance probes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct RuntimeDurationHistogram {
    buckets: [u64; RUNTIME_DURATION_BUCKETS_NS.len()],
}

impl RuntimeDurationHistogram {
    fn record(&mut self, elapsed_ns: u64) {
        let bucket = RUNTIME_DURATION_BUCKETS_NS
            .iter()
            .position(|upper_bound| elapsed_ns <= *upper_bound)
            .unwrap_or(RUNTIME_DURATION_BUCKETS_NS.len() - 1);
        self.buckets[bucket] = self.buckets[bucket].saturating_add(1);
    }

    fn p95_upper_bound_ns(self, samples: u64) -> u64 {
        if samples == 0 {
            return 0;
        }
        let target_rank = percentile_rank(samples, 95);
        let mut cumulative = 0_u64;
        for (bucket, upper_bound) in self.buckets.iter().zip(RUNTIME_DURATION_BUCKETS_NS) {
            cumulative = cumulative.saturating_add(*bucket);
            if cumulative >= target_rank {
                return upper_bound;
            }
        }
        u64::MAX
    }
}

fn percentile_rank(samples: u64, percentile: u8) -> u64 {
    let samples = u128::from(samples);
    let percentile = u128::from(percentile);
    let rank = samples.saturating_mul(percentile).saturating_add(99) / 100;
    u64::try_from(rank).unwrap_or(u64::MAX)
}

fn saturating_usize_to_u64(value: usize) -> u64 {
    u64::try_from(value).unwrap_or(u64::MAX)
}

fn saturating_duration_nanos(duration: Duration) -> u64 {
    u64::try_from(duration.as_nanos()).unwrap_or(u64::MAX)
}

fn add_usize_counter(counter: &mut u64, value: usize) {
    *counter = (*counter).saturating_add(saturating_usize_to_u64(value));
}

/// Spawns PTY sessions for the native terminal runtime.
pub trait NativePtySpawner {
    /// Session handle kept alive by the runtime.
    type Session;

    /// Spawn a PTY session with `config`.
    fn spawn(&self, config: PtyConfig) -> Result<Self::Session, PtyError>;
}

/// Minimal I/O surface the native runtime needs from a live PTY session.
pub trait NativePtySessionIo {
    /// Drain currently available PTY output bytes without blocking.
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError>;

    /// Write terminal input bytes to the PTY.
    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError>;

    /// Resize the PTY backing the terminal session.
    fn resize(&mut self, size: NativePtyResize) -> Result<(), PtyError>;
}

/// Real native PTY spawner.
#[derive(Debug, Clone, Default)]
pub struct RealNativePtySpawner {
    event_proxy: Option<NativeAppEventProxy>,
}

impl RealNativePtySpawner {
    /// Build a real PTY spawner that wakes the native event loop when output arrives.
    pub fn with_event_proxy(event_proxy: NativeAppEventProxy) -> Self {
        Self {
            event_proxy: Some(event_proxy),
        }
    }
}

impl NativePtySpawner for RealNativePtySpawner {
    type Session = PtySession;

    fn spawn(&self, config: PtyConfig) -> Result<Self::Session, PtyError> {
        let mut session = PtySession::spawn(config)?;
        if let Some(event_proxy) = self.event_proxy.clone() {
            session.start_output_reader_with_wakeup(move || {
                event_proxy.send(NativeAppEvent::PtyOutputReady);
            })?;
        } else {
            session.start_output_reader()?;
        }
        Ok(session)
    }
}

impl NativePtySessionIo for PtySession {
    fn drain_output(&mut self) -> Result<Vec<u8>, PtyError> {
        self.drain_available_output()
    }

    fn write_input(&mut self, bytes: &[u8]) -> Result<(), PtyError> {
        self.write_all(bytes)
    }

    fn resize(&mut self, size: NativePtyResize) -> Result<(), PtyError> {
        PtySession::resize(
            self,
            portable_pty::PtySize {
                rows: size.rows,
                cols: size.cols,
                pixel_width: size.pixel_width,
                pixel_height: size.pixel_height,
            },
        )
    }
}

/// Runtime state owned by the native app after startup.
pub struct NativeTerminalRuntime<S> {
    config: NativeTerminalRuntimeConfig,
    terminal: Terminal,
    shell_session: Option<S>,
    last_synced_clipboard_text: Option<String>,
    perf: NativeRuntimePerfSnapshot,
    render_time_histogram: RuntimeDurationHistogram,
    input_to_render_histogram: RuntimeDurationHistogram,
    pending_input_to_render_started: Option<Instant>,
}

impl<S> NativeTerminalRuntime<S> {
    /// Create runtime state with terminal grid and shell settings.
    pub fn new(config: NativeTerminalRuntimeConfig) -> Result<Self, NativeAppError> {
        let terminal = Terminal::new(config.terminal_config()?);
        debug!(
            terminal_cols = config.terminal_cols,
            terminal_rows = config.terminal_rows,
            scrollback_lines = config.scrollback_lines,
            shell = ?config.shell.program,
            "created native terminal runtime"
        );
        Ok(Self {
            config,
            terminal,
            shell_session: None,
            last_synced_clipboard_text: None,
            perf: NativeRuntimePerfSnapshot::default(),
            render_time_histogram: RuntimeDurationHistogram::default(),
            input_to_render_histogram: RuntimeDurationHistogram::default(),
            pending_input_to_render_started: None,
        })
    }

    /// Access the terminal state.
    pub fn terminal(&self) -> &Terminal {
        &self.terminal
    }

    /// Access runtime configuration.
    pub fn config(&self) -> &NativeTerminalRuntimeConfig {
        &self.config
    }

    /// Return deterministic native runtime counters.
    pub fn dump_runtime_perf_metrics(&self) -> NativeRuntimePerfSnapshot {
        self.perf
    }

    /// Render the current terminal frame when dirty regions are pending.
    pub fn render_terminal_frame<R>(&mut self, renderer: &mut R) -> bool
    where
        R: GpuRenderer,
    {
        self.perf.render_attempts += 1;
        let dirty_regions = self.terminal.take_dirty_regions();
        if dirty_regions.is_empty() {
            self.perf.clean_frame_skips += 1;
            trace!(
                render_attempts = self.perf.render_attempts,
                clean_frame_skips = self.perf.clean_frame_skips,
                "skipped clean native terminal frame"
            );
            return false;
        }
        let render_started = Instant::now();
        renderer.render_frame(
            &self.terminal.dump_grid(),
            self.terminal.dump_cursor(),
            &dirty_regions,
        );
        let elapsed_ns = saturating_duration_nanos(render_started.elapsed());
        self.perf.rendered_frames += 1;
        self.perf.render_time_samples += 1;
        self.perf.render_time_total_ns = self.perf.render_time_total_ns.saturating_add(elapsed_ns);
        self.perf.render_time_max_ns = self.perf.render_time_max_ns.max(elapsed_ns);
        self.render_time_histogram.record(elapsed_ns);
        self.perf.render_time_p95_ns = self
            .render_time_histogram
            .p95_upper_bound_ns(self.perf.render_time_samples);
        trace!(
            dirty_regions = dirty_regions.len(),
            render_time_ns = elapsed_ns,
            rendered_frames = self.perf.rendered_frames,
            render_time_p95_ns = self.perf.render_time_p95_ns,
            "rendered native terminal frame"
        );
        if let Some(input_started) = self.pending_input_to_render_started.take() {
            self.record_input_to_render_latency(saturating_duration_nanos(input_started.elapsed()));
        }
        true
    }

    /// Force the next renderer pass to cover the visible terminal viewport.
    pub fn invalidate_terminal_frame(&mut self) {
        self.terminal.invalidate_viewport();
    }

    fn record_input_to_render_latency(&mut self, elapsed_ns: u64) {
        self.perf.input_to_render_samples += 1;
        self.perf.input_to_render_total_ns = self
            .perf
            .input_to_render_total_ns
            .saturating_add(elapsed_ns);
        self.perf.input_to_render_max_ns = self.perf.input_to_render_max_ns.max(elapsed_ns);
        self.input_to_render_histogram.record(elapsed_ns);
        self.perf.input_to_render_p95_ns = self
            .input_to_render_histogram
            .p95_upper_bound_ns(self.perf.input_to_render_samples);
    }

    /// Write terminal-owned clipboard text, such as OSC 52 payloads, to a host clipboard.
    pub fn sync_terminal_clipboard<C>(&mut self, clipboard: &mut C) -> bool
    where
        C: HostClipboard,
    {
        let Some(text) = self.terminal.dump_clipboard_text() else {
            return false;
        };
        if self.last_synced_clipboard_text.as_deref() == Some(text.as_str()) {
            return false;
        }
        clipboard.write_text(&text);
        self.last_synced_clipboard_text = Some(text);
        true
    }

    /// Set the active visible-grid selection.
    pub fn set_selection(&mut self, selection: SelectionRange) {
        self.terminal.set_selection(selection);
    }

    /// Copy the active terminal selection into a host clipboard adapter.
    pub fn copy_selection_to_clipboard<C>(&self, clipboard: &mut C) -> bool
    where
        C: HostClipboard,
    {
        self.terminal
            .copy_selection_to_clipboard(clipboard)
            .is_some()
    }

    /// Access the retained shell session.
    pub fn shell_session(&self) -> Option<&S> {
        self.shell_session.as_ref()
    }

    /// Whether a shell session has been attached.
    pub fn has_shell_session(&self) -> bool {
        self.shell_session.is_some()
    }

    /// Start the shell PTY once and retain the live session handle.
    pub fn start_shell<P>(&mut self, spawner: &P) -> Result<(), NativeAppError>
    where
        P: NativePtySpawner<Session = S>,
    {
        if self.shell_session.is_some() {
            return Ok(());
        }
        let pty_config = self.config.pty_config();
        debug!(
            cols = pty_config.cols,
            rows = pty_config.rows,
            pixel_width = pty_config.pixel_width,
            pixel_height = pty_config.pixel_height,
            shell = ?pty_config.shell.program,
            "starting native shell PTY"
        );
        let session = spawner
            .spawn(pty_config)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.shell_session = Some(session);
        debug!("native shell PTY started");
        Ok(())
    }

    /// Update the shell command used for the next PTY spawn.
    pub fn set_shell_command(&mut self, shell: ShellCommand) {
        self.config.shell = shell;
    }

    /// Replace the configured shell and start a fresh PTY session.
    pub fn restart_shell<P>(
        &mut self,
        shell: ShellCommand,
        spawner: &P,
    ) -> Result<(), NativeAppError>
    where
        P: NativePtySpawner<Session = S>,
    {
        self.config.shell = shell;
        let pty_config = self.config.pty_config();
        debug!(
            cols = pty_config.cols,
            rows = pty_config.rows,
            pixel_width = pty_config.pixel_width,
            pixel_height = pty_config.pixel_height,
            shell = ?pty_config.shell.program,
            "restarting native shell PTY"
        );
        let session = spawner
            .spawn(pty_config)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.shell_session = Some(session);
        debug!("native shell PTY restarted");
        Ok(())
    }
}

impl<S> NativeTerminalRuntime<S>
where
    S: NativePtySessionIo,
{
    /// Drain available PTY output and feed it into the terminal parser.
    pub fn pump_pty_output(&mut self) -> Result<usize, NativeAppError> {
        let Some(session) = self.shell_session.as_mut() else {
            return Ok(0);
        };
        let output = session
            .drain_output()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        if output.is_empty() {
            return Ok(0);
        }
        self.perf.pty_output_batches += 1;
        add_usize_counter(&mut self.perf.pty_output_bytes, output.len());
        self.terminal
            .write_bytes(&output)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let response = self.terminal.take_pending_response_bytes();
        if !response.is_empty() {
            session
                .write_input(&response)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
            self.perf.pty_response_writes += 1;
            add_usize_counter(&mut self.perf.pty_response_bytes, response.len());
        }
        trace!(
            output_bytes = output.len(),
            response_bytes = response.len(),
            output_batches = self.perf.pty_output_batches,
            total_output_bytes = self.perf.pty_output_bytes,
            "pumped native PTY output"
        );
        Ok(output.len())
    }

    /// Write encoded terminal input bytes to the PTY session.
    pub fn send_pty_input(&mut self, bytes: &[u8]) -> Result<(), NativeAppError> {
        let Some(session) = self.shell_session.as_mut() else {
            return Ok(());
        };
        session
            .write_input(bytes)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.perf.pty_input_writes += 1;
        add_usize_counter(&mut self.perf.pty_input_bytes, bytes.len());
        trace!(
            input_bytes = bytes.len(),
            input_writes = self.perf.pty_input_writes,
            total_input_bytes = self.perf.pty_input_bytes,
            "wrote native PTY input"
        );
        if !bytes.is_empty() && self.pending_input_to_render_started.is_none() {
            self.pending_input_to_render_started = Some(Instant::now());
        }
        Ok(())
    }

    /// Encode a native logical key and write it to the PTY when it maps to terminal input.
    pub fn send_winit_key_input(
        &mut self,
        key: &Key,
        modifiers: ModifiersState,
    ) -> Result<bool, NativeAppError> {
        self.send_winit_key_event_input(key, None, modifiers)
    }

    /// Encode a native key event and write it to the PTY when it maps to terminal input.
    pub fn send_winit_key_event_input(
        &mut self,
        key: &Key,
        physical_key: Option<PhysicalKey>,
        modifiers: ModifiersState,
    ) -> Result<bool, NativeAppError> {
        let Some(bytes) = self
            .terminal
            .encode_winit_key_event_input(key, physical_key, modifiers)
        else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.native_key_inputs += 1;
        }
        Ok(true)
    }

    /// Encode a terminal mouse event and write it to the PTY when reporting is enabled.
    pub fn send_mouse_input(&mut self, event: MouseEvent) -> Result<bool, NativeAppError> {
        let Some(bytes) = self.terminal.encode_mouse_event(event) else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.mouse_inputs += 1;
        }
        Ok(true)
    }

    /// Encode a terminal focus event and write it to the PTY when reporting is enabled.
    pub fn send_focus_event(&mut self, focused: bool) -> Result<bool, NativeAppError> {
        let Some(bytes) = self.terminal.encode_focus_event(focused) else {
            return Ok(false);
        };
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            self.perf.focus_inputs += 1;
        }
        Ok(true)
    }

    /// Map a native window mouse position to a terminal event and write its report to the PTY.
    pub fn send_window_mouse_input(
        &mut self,
        x: f64,
        y: f64,
        window_width_px: u32,
        window_height_px: u32,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<bool, NativeAppError> {
        self.send_window_mouse_input_event(NativeWindowMouseInput {
            x,
            y,
            window_width_px,
            window_height_px,
            kind,
            button,
            modifiers: ModifiersState::empty(),
        })
    }

    /// Map native window mouse input to a terminal event and write its report.
    pub fn send_window_mouse_input_event(
        &mut self,
        input: NativeWindowMouseInput,
    ) -> Result<bool, NativeAppError> {
        let grid = self.terminal.dump_grid();
        let Some(mapper) = NativeMouseGridMapper::new(
            input.window_width_px,
            input.window_height_px,
            grid.cols,
            grid.rows,
        ) else {
            return Ok(false);
        };
        let Some(event) = mapper.mouse_event_at_with_modifiers(
            input.x,
            input.y,
            input.kind,
            input.button,
            input.modifiers,
        ) else {
            return Ok(false);
        };
        if self.send_mouse_input(event)? {
            return Ok(true);
        }
        Ok(match (input.kind, input.button) {
            (MouseEventKind::Press, MouseButton::WheelUp) => self.terminal.scroll_display_up(1),
            (MouseEventKind::Press, MouseButton::WheelDown) => self.terminal.scroll_display_down(1),
            _ => false,
        })
    }

    /// Encode pasted text according to terminal mode and write it to the PTY.
    pub fn send_paste_text(&mut self, text: &str) -> Result<(), NativeAppError> {
        let bytes = self.terminal.encode_paste_text(text);
        let had_session = self.shell_session.is_some();
        self.send_pty_input(&bytes)?;
        if had_session {
            add_usize_counter(&mut self.perf.paste_bytes, text.len());
        }
        Ok(())
    }

    /// Read text from a host clipboard and write it to the PTY as a terminal paste.
    pub fn send_clipboard_paste<C>(&mut self, clipboard: &C) -> Result<bool, NativeAppError>
    where
        C: HostClipboard,
    {
        if self.shell_session.is_none() {
            return Ok(false);
        }
        let Some(text) = clipboard.read_text().filter(|text| !text.is_empty()) else {
            return Ok(false);
        };
        self.send_paste_text(&text)?;
        self.perf.clipboard_pastes += 1;
        Ok(true)
    }

    /// Write committed platform text input to the PTY as typed UTF-8 text.
    pub fn send_committed_text(&mut self, text: &str) -> Result<(), NativeAppError> {
        let had_session = self.shell_session.is_some();
        self.send_pty_input(text.as_bytes())?;
        if had_session {
            add_usize_counter(&mut self.perf.committed_text_bytes, text.len());
        }
        Ok(())
    }

    /// Resize terminal state and notify the retained PTY session.
    pub fn resize_terminal(&mut self, size: NativePtyResize) -> Result<(), NativeAppError> {
        self.terminal
            .resize_with_pixel_size(size.cols, size.rows, size.pixel_width, size.pixel_height)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        self.config.terminal_cols = size.cols;
        self.config.terminal_rows = size.rows;
        self.config.pixel_width = size.pixel_width;
        self.config.pixel_height = size.pixel_height;
        if let Some(session) = self.shell_session.as_mut() {
            session
                .resize(size)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        }
        self.perf.resize_events += 1;
        Ok(())
    }

    /// Reconfigure terminal dimensions, pixel size, and scrollback retention without restarting the PTY.
    pub fn reconfigure_terminal(
        &mut self,
        config: NativeTerminalRuntimeConfig,
    ) -> Result<(), NativeAppError> {
        self.terminal
            .reconfigure(config.terminal_config()?)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let resize = NativePtyResize {
            cols: config.terminal_cols,
            rows: config.terminal_rows,
            pixel_width: config.pixel_width,
            pixel_height: config.pixel_height,
        };
        if let Some(session) = self.shell_session.as_mut() {
            session
                .resize(resize)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        }
        self.config = config;
        self.perf.resize_events += 1;
        Ok(())
    }

    /// Drain PTY output at the event-loop idle boundary and request redraw only when output changed.
    pub fn pump_output_and_schedule_redraw(
        &mut self,
        lifecycle: &mut NativeAppLifecycle,
    ) -> Result<NativeAppAction, NativeAppError> {
        let pumped_bytes = self.pump_pty_output()?;
        if pumped_bytes > 0 {
            Ok(lifecycle.on_terminal_output_ready())
        } else {
            Ok(lifecycle.on_about_to_wait())
        }
    }

    /// Drain PTY output, sync terminal clipboard state, and request redraw only when output changed.
    pub fn pump_output_sync_clipboard_and_schedule_redraw<C>(
        &mut self,
        lifecycle: &mut NativeAppLifecycle,
        clipboard: &mut C,
    ) -> Result<NativeAppAction, NativeAppError>
    where
        C: HostClipboard,
    {
        let pumped_bytes = self.pump_pty_output()?;
        if pumped_bytes > 0 {
            self.sync_terminal_clipboard(clipboard);
            Ok(lifecycle.on_terminal_output_ready())
        } else {
            Ok(lifecycle.on_about_to_wait())
        }
    }
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
        self.has_window = true;
        self.windows_created += 1;
    }

    /// Handle the event-loop idle boundary before waiting for more events.
    pub fn on_about_to_wait(&mut self) -> NativeAppAction {
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
            Some(now + self.config.target_frame_interval())
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
        self.frames_presented += 1;
        NativeAppAction::None
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
}

/// Native terminal application handler for the `winit` event loop.
pub struct NativeTerminalApp {
    lifecycle: NativeAppLifecycle,
    runtime: NativeTerminalRuntime<PtySession>,
    renderer: WgpuRenderer,
    glyph_cache: Option<RasterizedGlyphCache>,
    pty_spawner: RealNativePtySpawner,
    gpu_context: Option<NativeGpuContext>,
    surface: Option<NativeWindowSurface<WgpuSurfaceBackend<'static>>>,
    modifiers: ModifiersState,
    cursor_position: Option<PhysicalPosition<f64>>,
    mouse_buttons: NativeMouseButtonTracker,
    resize_mapper: NativeResizeGridMapper,
    config_reloader: Option<ConfigFileReloader>,
    window: Option<Arc<Window>>,
    window_id: Option<WindowId>,
    startup_error: Option<String>,
}

impl NativeTerminalApp {
    /// Create a native terminal app handler.
    pub fn new(config: NativeAppConfig) -> Result<Self, NativeAppError> {
        Self::new_with_runtime_config(config, NativeTerminalRuntimeConfig::default())
    }

    /// Create a native terminal app handler with an explicit runtime configuration.
    pub fn new_with_runtime_config(
        config: NativeAppConfig,
        runtime_config: NativeTerminalRuntimeConfig,
    ) -> Result<Self, NativeAppError> {
        Self::new_with_runtime_and_renderer_config(
            config,
            runtime_config,
            RendererConfig::default(),
        )
    }

    /// Create a native terminal app handler with explicit runtime and renderer configuration.
    pub fn new_with_runtime_and_renderer_config(
        config: NativeAppConfig,
        runtime_config: NativeTerminalRuntimeConfig,
        renderer_config: RendererConfig,
    ) -> Result<Self, NativeAppError> {
        let resize_mapper = NativeResizeGridMapper::new(
            config.width,
            config.height,
            runtime_config.terminal_cols,
            runtime_config.terminal_rows,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime(
                "native window and terminal reference dimensions must be non-zero".to_owned(),
            )
        })?;
        let runtime = NativeTerminalRuntime::new(runtime_config)?;
        Ok(Self {
            lifecycle: NativeAppLifecycle::new(config),
            runtime,
            renderer: WgpuRenderer::new(renderer_config),
            glyph_cache: load_default_native_glyph_cache().ok(),
            pty_spawner: RealNativePtySpawner::default(),
            gpu_context: None,
            surface: None,
            modifiers: ModifiersState::empty(),
            cursor_position: None,
            mouse_buttons: NativeMouseButtonTracker::default(),
            resize_mapper,
            config_reloader: None,
            window: None,
            window_id: None,
            startup_error: None,
        })
    }

    /// Access lifecycle state.
    pub fn lifecycle(&self) -> &NativeAppLifecycle {
        &self.lifecycle
    }

    /// Access runtime state.
    pub fn runtime(&self) -> &NativeTerminalRuntime<PtySession> {
        &self.runtime
    }

    /// Access renderer state.
    pub fn renderer(&self) -> &WgpuRenderer {
        &self.renderer
    }

    /// Install a config-file reloader for live reloadable settings.
    pub fn set_config_reloader(&mut self, config_reloader: ConfigFileReloader) {
        self.config_reloader = Some(config_reloader);
    }

    /// Poll the installed config file and apply reloadable settings when it changed.
    pub fn reload_config_if_changed(&mut self) -> Result<bool, NativeAppError> {
        let Some(reload) = self
            .config_reloader
            .as_mut()
            .map(ConfigFileReloader::reload_if_changed)
            .transpose()
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?
        else {
            return Ok(false);
        };
        if !reload.changed {
            return Ok(false);
        }
        self.apply_reloadable_gromaq_config(&reload.config)?;
        Ok(true)
    }

    /// Apply validated user configuration fields that are reloadable without restarting the PTY.
    pub fn apply_reloadable_gromaq_config(
        &mut self,
        config: &GromaqConfig,
    ) -> Result<(), NativeAppError> {
        let app_config = NativeAppConfig::from_gromaq_config(config)?;
        let reloaded_shell = shell_command_from_gromaq_config(config);
        let shell_changed = self.runtime.config().shell != reloaded_shell;
        let mut runtime_config =
            NativeTerminalRuntimeConfig::from_gromaq_config(config, reloaded_shell.clone())?;
        let (reference_width_px, reference_height_px, pixel_width, pixel_height) =
            self.reload_reference_size(&app_config);
        runtime_config.pixel_width = pixel_width;
        runtime_config.pixel_height = pixel_height;
        let resize_mapper = NativeResizeGridMapper::new(
            reference_width_px,
            reference_height_px,
            runtime_config.terminal_cols,
            runtime_config.terminal_rows,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime(
                "native window and terminal reference dimensions must be non-zero".to_owned(),
            )
        })?;
        let renderer_config = RendererConfig::from_gromaq_config(config)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?;
        let clear_color = self.renderer.config().clear_color;
        let terminal_config_changed = self.runtime.config().terminal_cols
            != runtime_config.terminal_cols
            || self.runtime.config().terminal_rows != runtime_config.terminal_rows
            || self.runtime.config().scrollback_lines != runtime_config.scrollback_lines
            || self.runtime.config().pixel_width != runtime_config.pixel_width
            || self.runtime.config().pixel_height != runtime_config.pixel_height;
        if terminal_config_changed {
            self.runtime.reconfigure_terminal(runtime_config)?;
        }
        if shell_changed {
            if self.runtime.has_shell_session() {
                self.runtime
                    .restart_shell(reloaded_shell, &self.pty_spawner)?;
            } else {
                self.runtime.set_shell_command(reloaded_shell);
            }
        }
        self.resize_mapper = resize_mapper;
        self.lifecycle.apply_config(app_config);
        self.renderer.reconfigure(RendererConfig {
            clear_color,
            ..renderer_config
        });
        self.runtime.invalidate_terminal_frame();
        Ok(())
    }

    fn reload_reference_size(&self, app_config: &NativeAppConfig) -> (u32, u32, u16, u16) {
        if let Some(window) = &self.window {
            let size = window.inner_size();
            if size.width > 0 && size.height > 0 {
                return (
                    size.width,
                    size.height,
                    clamp_u32_to_u16(size.width),
                    clamp_u32_to_u16(size.height),
                );
            }
        }
        (
            app_config.width,
            app_config.height,
            self.runtime.config().pixel_width,
            self.runtime.config().pixel_height,
        )
    }

    /// Take a startup error captured from the event handler.
    pub fn take_startup_error(&mut self) -> Option<String> {
        self.startup_error.take()
    }

    /// Configure the user-event proxy used by the PTY background reader.
    pub fn set_event_proxy(&mut self, event_proxy: NativeAppEventProxy) {
        self.pty_spawner = RealNativePtySpawner::with_event_proxy(event_proxy);
    }
}

fn shell_command_from_gromaq_config(config: &GromaqConfig) -> ShellCommand {
    let mut shell = config
        .shell
        .program
        .as_ref()
        .map(|program| ShellCommand {
            program: program.into(),
            args: Vec::new(),
            cwd: None,
        })
        .unwrap_or_else(ShellCommand::default_shell);
    shell.args = config.shell.args.iter().map(Into::into).collect();
    shell.cwd = config.shell.cwd.as_ref().map(PathBuf::from);
    shell
}

impl ApplicationHandler<NativeAppEvent> for NativeTerminalApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.lifecycle.on_resumed() != NativeAppAction::CreateWindow {
            return;
        }
        match event_loop.create_window(self.lifecycle.config().window_attributes()) {
            Ok(window) => {
                let window = Arc::new(window);
                let size = window.inner_size();
                self.window_id = Some(window.id());
                if let Err(error) =
                    self.create_surface_for_window(Arc::clone(&window), size.width, size.height)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
                self.window = Some(window);
                self.lifecycle.on_window_created();
                if let Err(error) = self.runtime.start_shell(&self.pty_spawner) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            Err(error) => {
                self.startup_error = Some(error.to_string());
                event_loop.exit();
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        match self.reload_config_if_changed() {
            Ok(true) => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            Ok(false) => {}
            Err(error) => {
                self.startup_error = Some(error.to_string());
                event_loop.exit();
                return;
            }
        }
        if let Some(deadline) = self.lifecycle.next_pty_pump_deadline(Instant::now()) {
            event_loop.set_control_flow(ControlFlow::WaitUntil(deadline));
        }
        let mut clipboard = NativeClipboard::new();
        let action = self
            .runtime
            .pump_output_sync_clipboard_and_schedule_redraw(&mut self.lifecycle, &mut clipboard)
            .unwrap_or_else(|error| {
                self.startup_error = Some(error.to_string());
                NativeAppAction::Exit
            });
        match action {
            NativeAppAction::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            NativeAppAction::Exit => event_loop.exit(),
            NativeAppAction::None | NativeAppAction::CreateWindow => {}
        }
    }

    fn user_event(&mut self, event_loop: &ActiveEventLoop, event: NativeAppEvent) {
        let mut clipboard = NativeClipboard::new();
        let action = self
            .runtime
            .pump_output_sync_clipboard_and_schedule_redraw(&mut self.lifecycle, &mut clipboard)
            .unwrap_or_else(|error| {
                self.startup_error = Some(error.to_string());
                NativeAppAction::Exit
            });
        let action = match event {
            NativeAppEvent::PtyOutputReady => action,
        };
        match action {
            NativeAppAction::RequestRedraw => {
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            NativeAppAction::Exit => event_loop.exit(),
            NativeAppAction::None | NativeAppAction::CreateWindow => {}
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        if Some(window_id) != self.window_id {
            return;
        }
        match event {
            WindowEvent::CloseRequested => {
                self.lifecycle.on_close_requested();
                event_loop.exit();
            }
            WindowEvent::Destroyed => {
                self.lifecycle.on_destroyed();
                self.surface = None;
                self.gpu_context = None;
                self.window = None;
                self.window_id = None;
                event_loop.exit();
            }
            WindowEvent::RedrawRequested => {
                if let Some(window) = &self.window {
                    window.pre_present_notify();
                }
                if let Err(error) = self.present_redraw_frame() {
                    match error {
                        NativeGlyphFrameError::Surface(
                            SurfaceFrameError::Timeout | SurfaceFrameError::Occluded,
                        ) => {}
                        NativeGlyphFrameError::Surface(
                            SurfaceFrameError::Outdated
                            | SurfaceFrameError::Lost
                            | SurfaceFrameError::Validation
                            | SurfaceFrameError::InvalidFrame(_),
                        )
                        | NativeGlyphFrameError::Font(_) => {
                            self.startup_error = Some(error.to_string());
                            event_loop.exit();
                        }
                    }
                }
                self.lifecycle.on_redraw_requested();
            }
            WindowEvent::Resized(size) => {
                if let Some(surface) = &mut self.surface
                    && let Err(error) = surface.resize(size.width, size.height)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                    return;
                }
                if let Some(resize) = self
                    .resize_mapper
                    .resize_for_window(size.width, size.height)
                    && let Err(error) = self.runtime.resize_terminal(resize)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
                if let Some(window) = &self.window {
                    window.request_redraw();
                }
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state.is_pressed() {
                    let result = if is_native_copy_shortcut(&event.logical_key, self.modifiers) {
                        let mut clipboard = NativeClipboard::new();
                        self.runtime.copy_selection_to_clipboard(&mut clipboard);
                        Ok(())
                    } else if is_native_paste_shortcut(&event.logical_key, self.modifiers) {
                        let clipboard = NativeClipboard::new();
                        self.runtime.send_clipboard_paste(&clipboard).map(|_| ())
                    } else {
                        self.runtime
                            .send_winit_key_event_input(
                                &event.logical_key,
                                Some(event.physical_key),
                                self.modifiers,
                            )
                            .map(|_| ())
                    };
                    if let Err(error) = result {
                        self.startup_error = Some(error.to_string());
                        event_loop.exit();
                    }
                }
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                if let Err(error) = self.runtime.send_committed_text(&text) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.cursor_position = Some(position);
                let (kind, button) = self.mouse_buttons.cursor_move_event();
                if let Err(error) = self.send_current_mouse_input(kind, button) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::Focused(focused) => {
                if let Err(error) = self.runtime.send_focus_event(focused).map(|_| ()) {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(button) = native_mouse_button(button) {
                    let kind = if state == ElementState::Pressed {
                        self.mouse_buttons.set_pressed(button, true);
                        MouseEventKind::Press
                    } else {
                        MouseEventKind::Release
                    };
                    if let Err(error) = self.send_current_mouse_input(kind, button) {
                        self.startup_error = Some(error.to_string());
                        event_loop.exit();
                    }
                    if state == ElementState::Released {
                        self.mouse_buttons.set_pressed(button, false);
                    }
                }
            }
            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(button) = wheel_mouse_button(delta)
                    && let Err(error) = self.send_current_mouse_input(MouseEventKind::Press, button)
                {
                    self.startup_error = Some(error.to_string());
                    event_loop.exit();
                }
            }
            _ => {}
        }
    }
}

impl NativeTerminalApp {
    fn create_surface_for_window(
        &mut self,
        window: Arc<Window>,
        width: u32,
        height: u32,
    ) -> Result<(), NativeAppError> {
        let context = GpuBootstrap::new(GpuBootstrapConfig::native_default())
            .initialize_native()
            .map_err(NativeAppError::from)?;
        let gpu_surface = context
            .create_window_surface(window)
            .map_err(NativeAppError::from)?;
        let surface = NativeWindowSurface::from_gpu_surface(gpu_surface, width, height)
            .map_err(NativeAppError::from)?;
        self.gpu_context = Some(context);
        self.surface = Some(surface);
        Ok(())
    }

    fn send_current_mouse_input(
        &mut self,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<(), NativeAppError> {
        let (Some(position), Some(window)) = (self.cursor_position, self.window.as_ref()) else {
            return Ok(());
        };
        let size = window.inner_size();
        self.runtime
            .send_window_mouse_input_event(NativeWindowMouseInput {
                x: position.x,
                y: position.y,
                window_width_px: size.width,
                window_height_px: size.height,
                kind,
                button,
                modifiers: self.modifiers,
            })
            .map(|_| ())
    }

    fn present_redraw_frame(&mut self) -> Result<(), NativeGlyphFrameError> {
        let Some(surface) = &mut self.surface else {
            self.runtime.render_terminal_frame(&mut self.renderer);
            return Ok(());
        };
        if let Some(glyph_cache) = &mut self.glyph_cache {
            render_and_present_terminal_glyph_frame(
                &mut self.runtime,
                &mut self.renderer,
                glyph_cache,
                surface,
            )?;
        } else {
            self.runtime.render_terminal_frame(&mut self.renderer);
            surface.clear_and_present(self.renderer.config().clear_color)?;
        }
        Ok(())
    }
}

fn native_mouse_button(button: WinitMouseButton) -> Option<MouseButton> {
    match button {
        WinitMouseButton::Left => Some(MouseButton::Left),
        WinitMouseButton::Middle => Some(MouseButton::Middle),
        WinitMouseButton::Right => Some(MouseButton::Right),
        WinitMouseButton::Back | WinitMouseButton::Forward | WinitMouseButton::Other(_) => None,
    }
}

fn wheel_mouse_button(delta: MouseScrollDelta) -> Option<MouseButton> {
    let y = match delta {
        MouseScrollDelta::LineDelta(_, y) => y,
        MouseScrollDelta::PixelDelta(position) => position.y as f32,
    };
    if y > 0.0 {
        Some(MouseButton::WheelUp)
    } else if y < 0.0 {
        Some(MouseButton::WheelDown)
    } else {
        None
    }
}

/// Whether a native key event should copy the active terminal selection.
pub fn is_native_copy_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    matches!(key, Key::Named(NamedKey::Copy))
        || (matches!(key, Key::Named(NamedKey::Insert)) && modifiers.control_key())
        || (matches!(key, Key::Character(character) if character.eq_ignore_ascii_case("c"))
            && (modifiers.super_key() || (modifiers.control_key() && modifiers.shift_key())))
}

/// Whether a native key event should paste from the host clipboard.
pub fn is_native_paste_shortcut(key: &Key, modifiers: ModifiersState) -> bool {
    matches!(key, Key::Named(NamedKey::Paste))
        || (matches!(key, Key::Named(NamedKey::Insert)) && modifiers.shift_key())
        || (matches!(key, Key::Character(character) if character.eq_ignore_ascii_case("v"))
            && (modifiers.control_key() || modifiers.super_key()))
}

/// Errors from launching the native application loop.
#[derive(Debug, Error)]
pub enum NativeAppError {
    /// The event loop could not be created or executed.
    #[error("native event loop failed: {0}")]
    EventLoop(#[from] EventLoopError),
    /// The native window could not be created.
    #[error("native window creation failed: {0}")]
    WindowCreation(String),
    /// Native terminal runtime setup failed.
    #[error("native runtime failed: {0}")]
    Runtime(String),
    /// Native GPU setup failed.
    #[error("native GPU setup failed: {0}")]
    Gpu(String),
}

/// Errors while preparing or presenting a terminal glyph frame.
#[derive(Debug, Error)]
pub enum NativeGlyphFrameError {
    /// Font rasterization failed while building the glyph atlas image.
    #[error("native glyph rasterization failed: {0}")]
    Font(#[from] FontRasterError),
    /// Surface frame acquisition, drawing, or presentation failed.
    #[error("native glyph surface presentation failed: {0}")]
    Surface(#[from] SurfaceFrameError),
}

impl From<OsError> for NativeAppError {
    fn from(value: OsError) -> Self {
        Self::WindowCreation(value.to_string())
    }
}

impl From<GpuBootstrapError> for NativeAppError {
    fn from(value: GpuBootstrapError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<GpuSurfaceError> for NativeAppError {
    fn from(value: GpuSurfaceError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<SurfaceConfigError> for NativeAppError {
    fn from(value: SurfaceConfigError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<SurfaceFrameError> for NativeAppError {
    fn from(value: SurfaceFrameError) -> Self {
        Self::Gpu(value.to_string())
    }
}

impl From<FontRasterError> for NativeAppError {
    fn from(value: FontRasterError) -> Self {
        Self::Runtime(value.to_string())
    }
}

impl From<NativeGlyphFrameError> for NativeAppError {
    fn from(value: NativeGlyphFrameError) -> Self {
        match value {
            NativeGlyphFrameError::Font(error) => Self::Runtime(error.to_string()),
            NativeGlyphFrameError::Surface(error) => Self::Gpu(error.to_string()),
        }
    }
}

/// Run the native `winit` terminal application loop.
pub fn run_native_app(config: NativeAppConfig) -> Result<(), NativeAppError> {
    run_native_app_with_runtime_config(config, NativeTerminalRuntimeConfig::default())
}

/// Run the native `winit` terminal application loop with explicit runtime configuration.
pub fn run_native_app_with_runtime_config(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
) -> Result<(), NativeAppError> {
    run_native_app_with_runtime_and_renderer_config(
        config,
        runtime_config,
        RendererConfig::default(),
    )
}

/// Run the native `winit` terminal application loop with explicit runtime and renderer config.
pub fn run_native_app_with_runtime_and_renderer_config(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
    renderer_config: RendererConfig,
) -> Result<(), NativeAppError> {
    run_native_app_with_runtime_renderer_and_config_file(
        config,
        runtime_config,
        renderer_config,
        None,
    )
}

/// Run the native `winit` terminal application loop with explicit runtime, renderer, and config reload path.
pub fn run_native_app_with_runtime_renderer_and_config_file(
    config: NativeAppConfig,
    runtime_config: NativeTerminalRuntimeConfig,
    renderer_config: RendererConfig,
    config_path: Option<&Path>,
) -> Result<(), NativeAppError> {
    let event_loop = EventLoop::<NativeAppEvent>::with_user_event().build()?;
    let event_proxy = event_loop.create_proxy();
    let mut app = NativeTerminalApp::new_with_runtime_and_renderer_config(
        config,
        runtime_config,
        renderer_config,
    )?;
    if let Some(config_path) = config_path {
        app.set_config_reloader(
            ConfigFileReloader::from_file(config_path)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
        );
    }
    app.set_event_proxy(NativeAppEventProxy::from(event_proxy));
    event_loop.run_app(&mut app)?;
    if let Some(error) = app.take_startup_error() {
        return Err(NativeAppError::WindowCreation(error));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runtime_perf_counter_adds_usize_values_with_saturation() {
        let mut counter = u64::MAX - 1;

        add_usize_counter(&mut counter, 8);

        assert_eq!(counter, u64::MAX);
    }

    #[test]
    fn runtime_perf_duration_nanos_reports_u64_values() {
        let duration = Duration::from_nanos(42);

        assert_eq!(saturating_duration_nanos(duration), 42);
    }

    #[test]
    fn runtime_duration_histogram_reports_bucketed_p95_upper_bound() {
        let mut histogram = RuntimeDurationHistogram::default();
        for elapsed_ns in [
            50_000_u64, 120_000, 300_000, 900_000, 1_500_000, 3_000_000, 6_500_000, 7_500_000,
            9_500_000, 15_000_000,
        ] {
            histogram.record(elapsed_ns);
        }

        assert_eq!(histogram.p95_upper_bound_ns(10), 16_000_000);
    }

    #[test]
    fn runtime_duration_histogram_reports_zero_without_samples() {
        let histogram = RuntimeDurationHistogram::default();

        assert_eq!(histogram.p95_upper_bound_ns(0), 0);
    }

    #[test]
    fn percentile_rank_rounds_up() {
        assert_eq!(percentile_rank(1, 95), 1);
        assert_eq!(percentile_rank(20, 95), 19);
        assert_eq!(percentile_rank(21, 95), 20);
    }

    #[test]
    fn native_terminal_app_new_rejects_zero_window_reference_size() {
        let config = NativeAppConfig {
            width: 0,
            ..NativeAppConfig::default()
        };
        let error = match NativeTerminalApp::new(config) {
            Ok(_) => panic!("zero-width native app config should be rejected"),
            Err(error) => error,
        };

        assert_eq!(
            error.to_string(),
            "native runtime failed: native window and terminal reference dimensions must be non-zero"
        );
    }
}
