use std::cell::RefCell;

use gromaq::native_gpu::{
    GpuAdapterSnapshot, GpuBootstrap, GpuBootstrapBackend, GpuBootstrapConfig, GpuBootstrapError,
    GpuBootstrapRequest, GpuPowerPreference, NativeGpuWindowSurface, ReadbackLayout, UploadPattern,
};

#[derive(Debug)]
struct RecordingBackend {
    requests: RefCell<Vec<GpuBootstrapRequest>>,
    result: Result<GpuAdapterSnapshot, GpuBootstrapError>,
}

impl RecordingBackend {
    fn successful() -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
            result: Ok(GpuAdapterSnapshot {
                name: "Mock GPU".to_owned(),
                backend: "Mock".to_owned(),
                device_type: "DiscreteGpu".to_owned(),
                vendor: 123,
                device: 456,
            }),
        }
    }
}

impl GpuBootstrapBackend for RecordingBackend {
    type Context = GpuAdapterSnapshot;

    fn request_device(
        &self,
        request: &GpuBootstrapRequest,
    ) -> Result<Self::Context, GpuBootstrapError> {
        self.requests.borrow_mut().push(request.clone());
        self.result.clone()
    }
}

#[test]
fn native_gpu_bootstrap_requests_high_performance_defaults() {
    let backend = RecordingBackend::successful();
    let bootstrap = GpuBootstrap::new(GpuBootstrapConfig::native_default());

    let snapshot = bootstrap.initialize_with(&backend).unwrap();

    assert_eq!(snapshot.name, "Mock GPU");
    let requests = backend.requests.borrow();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].power_preference,
        GpuPowerPreference::HighPerformance
    );
    assert!(!requests[0].force_fallback_adapter);
    assert!(requests[0].required_features_empty);
    assert_eq!(requests[0].device_label, "gromaq-render-device");
}

#[test]
fn native_gpu_bootstrap_preserves_adapter_failure() {
    let backend = RecordingBackend {
        requests: RefCell::new(Vec::new()),
        result: Err(GpuBootstrapError::AdapterUnavailable(
            "no compatible adapter".to_owned(),
        )),
    };
    let bootstrap = GpuBootstrap::new(GpuBootstrapConfig::native_default());

    let error = bootstrap.initialize_with(&backend).unwrap_err();

    assert_eq!(
        error,
        GpuBootstrapError::AdapterUnavailable("no compatible adapter".to_owned())
    );
}

#[test]
fn native_gpu_bootstrap_preserves_device_failure() {
    let backend = RecordingBackend {
        requests: RefCell::new(Vec::new()),
        result: Err(GpuBootstrapError::DeviceUnavailable(
            "limits rejected".to_owned(),
        )),
    };
    let bootstrap = GpuBootstrap::new(GpuBootstrapConfig::native_default());

    let error = bootstrap.initialize_with(&backend).unwrap_err();

    assert_eq!(
        error,
        GpuBootstrapError::DeviceUnavailable("limits rejected".to_owned())
    );
}

#[test]
fn native_gpu_window_surface_preserves_backend_and_capabilities_for_app_handoff() {
    let surface = NativeGpuWindowSurface::new("surface-backend", supported_surface_capabilities());

    assert_eq!(
        surface.capabilities().formats,
        vec![wgpu::TextureFormat::Bgra8UnormSrgb]
    );
    assert_eq!(
        surface.capabilities().present_modes,
        vec![wgpu::PresentMode::Fifo]
    );

    let (backend, capabilities) = surface.into_parts();

    assert_eq!(backend, "surface-backend");
    assert_eq!(
        capabilities.alpha_modes,
        vec![wgpu::CompositeAlphaMode::Opaque]
    );
    assert!(
        capabilities
            .usages
            .contains(wgpu::TextureUsages::RENDER_ATTACHMENT)
    );
}

#[test]
fn readback_layout_aligns_rows_for_texture_copy() {
    let layout = ReadbackLayout::rgba8(3, 2);

    assert_eq!(layout.width, 3);
    assert_eq!(layout.height, 2);
    assert_eq!(layout.dense_bytes_per_row, 12);
    assert_eq!(layout.padded_bytes_per_row, 256);
    assert_eq!(layout.buffer_size, 512);
}

#[test]
fn upload_pattern_builds_deterministic_rgba_checker() {
    let pattern = UploadPattern::checker_rgba8_2x2();

    assert_eq!(pattern.width, 2);
    assert_eq!(pattern.height, 2);
    assert_eq!(
        pattern.rgba,
        vec![
            255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
        ]
    );
}

fn supported_surface_capabilities() -> wgpu::SurfaceCapabilities {
    wgpu::SurfaceCapabilities {
        formats: vec![wgpu::TextureFormat::Bgra8UnormSrgb],
        present_modes: vec![wgpu::PresentMode::Fifo],
        alpha_modes: vec![wgpu::CompositeAlphaMode::Opaque],
        usages: wgpu::TextureUsages::RENDER_ATTACHMENT,
    }
}
