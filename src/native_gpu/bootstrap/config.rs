/// Power preference used when choosing a GPU adapter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GpuPowerPreference {
    /// Do not prefer a power class.
    None,
    /// Prefer lower power usage.
    LowPower,
    /// Prefer the highest performance adapter available.
    HighPerformance,
}

impl From<GpuPowerPreference> for wgpu::PowerPreference {
    fn from(value: GpuPowerPreference) -> Self {
        match value {
            GpuPowerPreference::None => Self::None,
            GpuPowerPreference::LowPower => Self::LowPower,
            GpuPowerPreference::HighPerformance => Self::HighPerformance,
        }
    }
}

/// Configuration for native GPU bootstrap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBootstrapConfig {
    /// Adapter power preference.
    pub power_preference: GpuPowerPreference,
    /// Whether software fallback adapters may be selected.
    pub force_fallback_adapter: bool,
    /// Debug label for the render device.
    pub device_label: &'static str,
}

impl GpuBootstrapConfig {
    /// Native defaults for the performance-first terminal renderer.
    pub fn native_default() -> Self {
        Self {
            power_preference: GpuPowerPreference::HighPerformance,
            force_fallback_adapter: false,
            device_label: "gromaq-render-device",
        }
    }
}

/// Fully resolved GPU bootstrap request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuBootstrapRequest {
    /// Adapter power preference.
    pub power_preference: GpuPowerPreference,
    /// Whether software fallback adapters may be selected.
    pub force_fallback_adapter: bool,
    /// Whether required feature set is empty.
    pub required_features_empty: bool,
    /// Debug label for the render device.
    pub device_label: &'static str,
}
