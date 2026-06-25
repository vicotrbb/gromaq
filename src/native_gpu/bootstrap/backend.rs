use super::{
    GpuAdapterSnapshot, GpuBootstrapBackend, GpuBootstrapError, GpuBootstrapRequest,
    NativeGpuContext,
};

/// Real `wgpu` backend for native GPU bootstrap.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeWgpuBackend;

impl GpuBootstrapBackend for NativeWgpuBackend {
    type Context = NativeGpuContext;

    fn request_device(
        &self,
        request: &GpuBootstrapRequest,
    ) -> std::result::Result<Self::Context, GpuBootstrapError> {
        pollster::block_on(request_native_wgpu_context(request))
    }
}

async fn request_native_wgpu_context(
    request: &GpuBootstrapRequest,
) -> std::result::Result<NativeGpuContext, GpuBootstrapError> {
    let instance = wgpu::Instance::new(wgpu::InstanceDescriptor::new_without_display_handle());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            power_preference: request.power_preference.into(),
            force_fallback_adapter: request.force_fallback_adapter,
            compatible_surface: None,
        })
        .await
        .map_err(|error| GpuBootstrapError::AdapterUnavailable(error.to_string()))?;
    let info = adapter.get_info();
    let (device, queue) = adapter
        .request_device(&wgpu::DeviceDescriptor {
            label: Some(request.device_label),
            required_features: wgpu::Features::empty(),
            required_limits: wgpu::Limits::default(),
            experimental_features: wgpu::ExperimentalFeatures::disabled(),
            memory_hints: wgpu::MemoryHints::Performance,
            trace: wgpu::Trace::Off,
        })
        .await
        .map_err(|error| GpuBootstrapError::DeviceUnavailable(error.to_string()))?;

    Ok(NativeGpuContext {
        _instance: instance,
        _adapter: adapter,
        device,
        queue,
        adapter: GpuAdapterSnapshot {
            name: info.name,
            backend: format!("{:?}", info.backend),
            device_type: format!("{:?}", info.device_type),
            vendor: info.vendor,
            device: info.device,
        },
    })
}
