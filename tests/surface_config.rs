use gromaq::renderer::{
    SurfaceBackend, SurfaceConfigError, SurfaceConfigPlanner, SurfaceConfigurationController,
    SurfaceLifecycle, SurfaceLifecycleAction,
};
use wgpu::{CompositeAlphaMode, PresentMode, SurfaceCapabilities, TextureFormat, TextureUsages};

#[derive(Debug, Default)]
struct MockSurfaceBackend {
    configured_sizes: Vec<(u32, u32)>,
}

impl SurfaceBackend for MockSurfaceBackend {
    fn configure(&mut self, config: &wgpu::SurfaceConfiguration) {
        self.configured_sizes.push((config.width, config.height));
    }
}

#[test]
fn surface_config_planner_prefers_srgb_mailbox_and_opaque_alpha() {
    let caps = SurfaceCapabilities {
        formats: vec![
            TextureFormat::Rgba8Unorm,
            TextureFormat::Bgra8UnormSrgb,
            TextureFormat::Rgba8UnormSrgb,
        ],
        present_modes: vec![
            PresentMode::Immediate,
            PresentMode::Fifo,
            PresentMode::Mailbox,
        ],
        alpha_modes: vec![
            CompositeAlphaMode::PreMultiplied,
            CompositeAlphaMode::Opaque,
        ],
        usages: TextureUsages::COPY_SRC | TextureUsages::RENDER_ATTACHMENT,
    };

    let config = SurfaceConfigPlanner::new().plan(&caps, 1280, 800).unwrap();

    assert_eq!(config.width, 1280);
    assert_eq!(config.height, 800);
    assert_eq!(config.format, TextureFormat::Bgra8UnormSrgb);
    assert_eq!(config.present_mode, PresentMode::Mailbox);
    assert_eq!(config.alpha_mode, CompositeAlphaMode::Opaque);
    assert_eq!(config.usage, TextureUsages::RENDER_ATTACHMENT);
    assert_eq!(config.desired_maximum_frame_latency, 1);
    assert!(config.view_formats.is_empty());
}

#[test]
fn surface_config_planner_uses_fifo_when_mailbox_is_unavailable() {
    let caps = SurfaceCapabilities {
        formats: vec![TextureFormat::Bgra8UnormSrgb],
        present_modes: vec![PresentMode::Immediate, PresentMode::Fifo],
        alpha_modes: vec![CompositeAlphaMode::Opaque],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    let config = SurfaceConfigPlanner::new().plan(&caps, 1280, 800).unwrap();

    assert_eq!(config.present_mode, PresentMode::Fifo);
}

#[test]
fn surface_config_planner_falls_back_to_first_supported_values() {
    let caps = SurfaceCapabilities {
        formats: vec![TextureFormat::Rgba8Unorm],
        present_modes: vec![PresentMode::Immediate],
        alpha_modes: vec![CompositeAlphaMode::Inherit],
        usages: TextureUsages::RENDER_ATTACHMENT,
    };

    let config = SurfaceConfigPlanner::new().plan(&caps, 640, 480).unwrap();

    assert_eq!(config.format, TextureFormat::Rgba8Unorm);
    assert_eq!(config.present_mode, PresentMode::Immediate);
    assert_eq!(config.alpha_mode, CompositeAlphaMode::Inherit);
}

#[test]
fn surface_config_planner_rejects_invalid_capabilities_and_size() {
    fn valid_capabilities() -> SurfaceCapabilities {
        SurfaceCapabilities {
            formats: vec![TextureFormat::Bgra8UnormSrgb],
            present_modes: vec![PresentMode::Fifo],
            alpha_modes: vec![CompositeAlphaMode::Opaque],
            usages: TextureUsages::RENDER_ATTACHMENT,
        }
    }

    let valid = valid_capabilities();
    assert_eq!(
        SurfaceConfigPlanner::new()
            .plan(&valid, 0, 480)
            .unwrap_err(),
        SurfaceConfigError::InvalidSize {
            width: 0,
            height: 480
        }
    );

    let no_formats = SurfaceCapabilities {
        formats: Vec::new(),
        ..valid_capabilities()
    };
    assert_eq!(
        SurfaceConfigPlanner::new()
            .plan(&no_formats, 640, 480)
            .unwrap_err(),
        SurfaceConfigError::NoSupportedFormats
    );

    let no_present_modes = SurfaceCapabilities {
        present_modes: Vec::new(),
        ..valid_capabilities()
    };
    assert_eq!(
        SurfaceConfigPlanner::new()
            .plan(&no_present_modes, 640, 480)
            .unwrap_err(),
        SurfaceConfigError::NoSupportedPresentModes
    );

    let no_alpha_modes = SurfaceCapabilities {
        alpha_modes: Vec::new(),
        ..valid_capabilities()
    };
    assert_eq!(
        SurfaceConfigPlanner::new()
            .plan(&no_alpha_modes, 640, 480)
            .unwrap_err(),
        SurfaceConfigError::NoSupportedAlphaModes
    );
}

#[test]
fn surface_lifecycle_configures_and_reconfigures_after_resize() {
    let caps = supported_capabilities();
    let mut lifecycle = SurfaceLifecycle::new(SurfaceConfigPlanner::new());

    assert_eq!(
        lifecycle.configure(&caps, 1280, 800).unwrap(),
        SurfaceLifecycleAction::Configure
    );
    assert!(lifecycle.is_configured());
    assert_eq!(lifecycle.size(), Some((1280, 800)));
    assert_eq!(lifecycle.configure_count(), 1);
    assert_eq!(lifecycle.current_config().unwrap().width, 1280);

    assert_eq!(
        lifecycle.on_resized(&caps, 1440, 900).unwrap(),
        SurfaceLifecycleAction::Reconfigure
    );
    assert_eq!(lifecycle.size(), Some((1440, 900)));
    assert_eq!(lifecycle.configure_count(), 2);
    assert_eq!(lifecycle.current_config().unwrap().height, 900);
}

#[test]
fn surface_lifecycle_defers_zero_size_and_reconfigures_when_visible_again() {
    let caps = supported_capabilities();
    let mut lifecycle = SurfaceLifecycle::new(SurfaceConfigPlanner::new());

    assert_eq!(
        lifecycle.configure(&caps, 1280, 800).unwrap(),
        SurfaceLifecycleAction::Configure
    );
    assert_eq!(
        lifecycle.on_resized(&caps, 0, 800).unwrap(),
        SurfaceLifecycleAction::DeferZeroSize
    );
    assert!(lifecycle.is_suspended());
    assert_eq!(lifecycle.configure_count(), 1);
    assert_eq!(lifecycle.current_config().unwrap().width, 1280);

    assert_eq!(
        lifecycle.on_resized(&caps, 1024, 768).unwrap(),
        SurfaceLifecycleAction::Reconfigure
    );
    assert!(!lifecycle.is_suspended());
    assert_eq!(lifecycle.size(), Some((1024, 768)));
    assert_eq!(lifecycle.configure_count(), 2);
}

#[test]
fn surface_configuration_controller_applies_configure_and_reconfigure_to_backend() {
    let caps = supported_capabilities();
    let mut backend = MockSurfaceBackend::default();
    let mut controller = SurfaceConfigurationController::new(SurfaceConfigPlanner::new());

    assert_eq!(
        controller
            .configure(&mut backend, &caps, 1280, 800)
            .unwrap(),
        SurfaceLifecycleAction::Configure
    );
    assert_eq!(backend.configured_sizes, vec![(1280, 800)]);

    assert_eq!(
        controller.resize(&mut backend, &caps, 1280, 800).unwrap(),
        SurfaceLifecycleAction::None
    );
    assert_eq!(backend.configured_sizes, vec![(1280, 800)]);

    assert_eq!(
        controller.resize(&mut backend, &caps, 0, 800).unwrap(),
        SurfaceLifecycleAction::DeferZeroSize
    );
    assert_eq!(backend.configured_sizes, vec![(1280, 800)]);

    assert_eq!(
        controller.resize(&mut backend, &caps, 1440, 900).unwrap(),
        SurfaceLifecycleAction::Reconfigure
    );
    assert_eq!(backend.configured_sizes, vec![(1280, 800), (1440, 900)]);
    assert_eq!(controller.lifecycle().configure_count(), 2);
}

fn supported_capabilities() -> SurfaceCapabilities {
    SurfaceCapabilities {
        formats: vec![TextureFormat::Bgra8UnormSrgb],
        present_modes: vec![PresentMode::Fifo],
        alpha_modes: vec![CompositeAlphaMode::Opaque],
        usages: TextureUsages::RENDER_ATTACHMENT,
    }
}
