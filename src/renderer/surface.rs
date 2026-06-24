use thiserror::Error;

/// Errors produced when choosing a native `wgpu` surface configuration.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SurfaceConfigError {
    /// Surface dimensions must be non-zero.
    #[error("surface dimensions must be non-zero, got {width}x{height}")]
    InvalidSize {
        /// Requested surface width.
        width: u32,
        /// Requested surface height.
        height: u32,
    },
    /// The surface reported no supported texture formats.
    #[error("surface reported no supported texture formats")]
    NoSupportedFormats,
    /// The surface reported no supported presentation modes.
    #[error("surface reported no supported presentation modes")]
    NoSupportedPresentModes,
    /// The surface reported no supported alpha modes.
    #[error("surface reported no supported alpha modes")]
    NoSupportedAlphaModes,
    /// The surface cannot be rendered into.
    #[error("surface does not support render attachment usage")]
    MissingRenderAttachmentUsage,
}

/// Chooses deterministic `wgpu` surface configurations for the native renderer.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SurfaceConfigPlanner;

impl SurfaceConfigPlanner {
    /// Create a surface configuration planner.
    pub fn new() -> Self {
        Self
    }

    /// Build a `wgpu` surface configuration from adapter/surface capabilities.
    pub fn plan(
        &self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<wgpu::SurfaceConfiguration, SurfaceConfigError> {
        if width == 0 || height == 0 {
            return Err(SurfaceConfigError::InvalidSize { width, height });
        }
        if capabilities.formats.is_empty() {
            return Err(SurfaceConfigError::NoSupportedFormats);
        }
        if capabilities.present_modes.is_empty() {
            return Err(SurfaceConfigError::NoSupportedPresentModes);
        }
        if capabilities.alpha_modes.is_empty() {
            return Err(SurfaceConfigError::NoSupportedAlphaModes);
        }
        if !capabilities
            .usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
        {
            return Err(SurfaceConfigError::MissingRenderAttachmentUsage);
        }

        Ok(wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: choose_surface_format(&capabilities.formats),
            width,
            height,
            present_mode: choose_present_mode(&capabilities.present_modes),
            desired_maximum_frame_latency: 1,
            alpha_mode: choose_alpha_mode(&capabilities.alpha_modes),
            view_formats: Vec::new(),
        })
    }
}

/// Platform action required after a surface lifecycle transition.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceLifecycleAction {
    /// No surface action is required.
    None,
    /// Configure the surface for the first time.
    Configure,
    /// Reconfigure an already-created surface.
    Reconfigure,
    /// Defer configuration while the window is minimized or otherwise zero-sized.
    DeferZeroSize,
}

/// Surface endpoint that can receive an executable `wgpu` surface configuration.
pub trait SurfaceBackend {
    /// Apply `config` to the native surface boundary.
    fn configure(&mut self, config: &wgpu::SurfaceConfiguration);
}

/// Deterministic state for native `wgpu` surface configuration and resize handling.
#[derive(Debug, Clone)]
pub struct SurfaceLifecycle {
    planner: SurfaceConfigPlanner,
    current_config: Option<wgpu::SurfaceConfiguration>,
    current_size: Option<(u32, u32)>,
    suspended_for_zero_size: bool,
    configure_count: u64,
}

impl SurfaceLifecycle {
    /// Create surface lifecycle state using `planner`.
    pub fn new(planner: SurfaceConfigPlanner) -> Self {
        Self {
            planner,
            current_config: None,
            current_size: None,
            suspended_for_zero_size: false,
            configure_count: 0,
        }
    }

    /// Configure the surface for an initial non-zero size.
    pub fn configure(
        &mut self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.apply_size(capabilities, width, height)
    }

    /// Handle a native window resize.
    pub fn on_resized(
        &mut self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        self.apply_size(capabilities, width, height)
    }

    /// Return the current surface configuration.
    pub fn current_config(&self) -> Option<&wgpu::SurfaceConfiguration> {
        self.current_config.as_ref()
    }

    /// Return the current non-zero surface size.
    pub fn size(&self) -> Option<(u32, u32)> {
        self.current_size
    }

    /// Whether a valid surface configuration exists.
    pub fn is_configured(&self) -> bool {
        self.current_config.is_some()
    }

    /// Whether configuration is suspended because the window is zero-sized.
    pub fn is_suspended(&self) -> bool {
        self.suspended_for_zero_size
    }

    /// Number of surface configuration transitions applied.
    pub fn configure_count(&self) -> u64 {
        self.configure_count
    }

    fn apply_size(
        &mut self,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError> {
        if width == 0 || height == 0 {
            self.suspended_for_zero_size = true;
            return Ok(SurfaceLifecycleAction::DeferZeroSize);
        }
        let config = self.planner.plan(capabilities, width, height)?;
        let action = if self.current_config.is_some() {
            if self.current_size == Some((width, height)) && !self.suspended_for_zero_size {
                SurfaceLifecycleAction::None
            } else {
                SurfaceLifecycleAction::Reconfigure
            }
        } else {
            SurfaceLifecycleAction::Configure
        };

        if action != SurfaceLifecycleAction::None {
            self.configure_count += 1;
        }
        self.current_size = Some((width, height));
        self.current_config = Some(config);
        self.suspended_for_zero_size = false;
        Ok(action)
    }
}

/// Applies planned surface lifecycle transitions to a concrete surface backend.
#[derive(Debug, Clone)]
pub struct SurfaceConfigurationController {
    lifecycle: SurfaceLifecycle,
}

impl SurfaceConfigurationController {
    /// Create a surface configuration controller.
    pub fn new(planner: SurfaceConfigPlanner) -> Self {
        Self {
            lifecycle: SurfaceLifecycle::new(planner),
        }
    }

    /// Access the underlying lifecycle state.
    pub fn lifecycle(&self) -> &SurfaceLifecycle {
        &self.lifecycle
    }

    /// Configure an initial surface size and apply the resulting config to `backend`.
    pub fn configure<B>(
        &mut self,
        backend: &mut B,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError>
    where
        B: SurfaceBackend,
    {
        let action = self.lifecycle.configure(capabilities, width, height)?;
        self.apply_action(backend, action);
        Ok(action)
    }

    /// Resize a configured surface and apply reconfiguration to `backend` when needed.
    pub fn resize<B>(
        &mut self,
        backend: &mut B,
        capabilities: &wgpu::SurfaceCapabilities,
        width: u32,
        height: u32,
    ) -> std::result::Result<SurfaceLifecycleAction, SurfaceConfigError>
    where
        B: SurfaceBackend,
    {
        let action = self.lifecycle.on_resized(capabilities, width, height)?;
        self.apply_action(backend, action);
        Ok(action)
    }

    fn apply_action<B>(&self, backend: &mut B, action: SurfaceLifecycleAction)
    where
        B: SurfaceBackend,
    {
        if matches!(
            action,
            SurfaceLifecycleAction::Configure | SurfaceLifecycleAction::Reconfigure
        ) && let Some(config) = self.lifecycle.current_config()
        {
            backend.configure(config);
        }
    }
}

fn choose_surface_format(formats: &[wgpu::TextureFormat]) -> wgpu::TextureFormat {
    [
        wgpu::TextureFormat::Bgra8UnormSrgb,
        wgpu::TextureFormat::Rgba8UnormSrgb,
    ]
    .into_iter()
    .find(|preferred| formats.contains(preferred))
    .unwrap_or_else(|| {
        formats
            .iter()
            .copied()
            .find(wgpu::TextureFormat::is_srgb)
            .unwrap_or(formats[0])
    })
}

fn choose_present_mode(present_modes: &[wgpu::PresentMode]) -> wgpu::PresentMode {
    if present_modes.contains(&wgpu::PresentMode::Mailbox) {
        wgpu::PresentMode::Mailbox
    } else if present_modes.contains(&wgpu::PresentMode::Fifo) {
        wgpu::PresentMode::Fifo
    } else {
        present_modes[0]
    }
}

fn choose_alpha_mode(alpha_modes: &[wgpu::CompositeAlphaMode]) -> wgpu::CompositeAlphaMode {
    if alpha_modes.contains(&wgpu::CompositeAlphaMode::Opaque) {
        wgpu::CompositeAlphaMode::Opaque
    } else {
        alpha_modes[0]
    }
}
