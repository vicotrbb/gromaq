use winit::keyboard::ModifiersState;

use crate::app::{
    NativeAppConfig, NativeAppError, NativeAppLifecycle, NativeMouseButtonTracker,
    NativeResizeGridMapper, NativeTerminalApp, NativeTerminalRuntime, NativeTerminalRuntimeConfig,
    RealNativePtySpawner, load_native_glyph_cache_with_fallbacks,
};
use crate::config::DEFAULT_FONT_FAMILY;
use crate::renderer::{RendererConfig, WgpuRenderer};

use super::welcome::default_welcome_text;

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
        Self::new_with_runtime_renderer_and_font_config(
            config,
            runtime_config,
            renderer_config,
            DEFAULT_FONT_FAMILY,
        )
    }

    /// Create a native terminal app with explicit runtime, renderer, and font configuration.
    pub fn new_with_runtime_renderer_and_font_config(
        config: NativeAppConfig,
        runtime_config: NativeTerminalRuntimeConfig,
        renderer_config: RendererConfig,
        font_family: impl Into<String>,
    ) -> Result<Self, NativeAppError> {
        Self::new_with_runtime_renderer_font_and_fallback_config(
            config,
            runtime_config,
            renderer_config,
            font_family,
            Vec::new(),
        )
    }

    /// Create a native terminal app with explicit runtime, renderer, primary font, and fallback fonts.
    pub fn new_with_runtime_renderer_font_and_fallback_config(
        config: NativeAppConfig,
        mut runtime_config: NativeTerminalRuntimeConfig,
        renderer_config: RendererConfig,
        font_family: impl Into<String>,
        font_fallback_families: Vec<String>,
    ) -> Result<Self, NativeAppError> {
        let font_family = font_family.into();
        if config.width == 0 || config.height == 0 {
            return Err(NativeAppError::Runtime(
                "native window dimensions must be non-zero".to_owned(),
            ));
        }
        let resize_mapper = NativeResizeGridMapper::new(
            renderer_config.cell_width_px,
            renderer_config.line_height_px,
            renderer_config.surface_padding_px,
            renderer_config.cell_spacing_px,
        )
        .ok_or_else(|| {
            NativeAppError::Runtime("native renderer cell dimensions must be non-zero".to_owned())
        })?;
        if let Some(resize) = resize_mapper.resize_for_window(config.width, config.height) {
            runtime_config.terminal_cols = resize.cols;
            runtime_config.terminal_rows = resize.rows;
            runtime_config.pixel_width = resize.pixel_width;
            runtime_config.pixel_height = resize.pixel_height;
        }
        let mut runtime = NativeTerminalRuntime::new(runtime_config)?;
        if let Some(startup_text) = config.startup_text.as_deref() {
            runtime.write_startup_text(startup_text)?;
        } else if config.welcome_screen {
            let startup_text =
                default_welcome_text(&config, runtime.config(), &renderer_config, &font_family);
            runtime.write_startup_text(&startup_text)?;
        }
        Ok(Self {
            lifecycle: NativeAppLifecycle::new(config),
            runtime,
            renderer: WgpuRenderer::new(renderer_config)?,
            glyph_cache: load_native_glyph_cache_with_fallbacks(
                &font_family,
                &font_fallback_families,
            )?,
            font_family,
            font_fallback_families,
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
}
