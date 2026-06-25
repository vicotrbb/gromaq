/// Stable adapter metadata for diagnostics and tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GpuAdapterSnapshot {
    /// Adapter name.
    pub name: String,
    /// Backend name.
    pub backend: String,
    /// Device type name.
    pub device_type: String,
    /// Backend-specific vendor ID.
    pub vendor: u32,
    /// Backend-specific device ID.
    pub device: u32,
}
