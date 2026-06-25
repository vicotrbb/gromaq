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
