pub(super) fn surface_present_mode_name(present_mode: wgpu::PresentMode) -> &'static str {
    match present_mode {
        wgpu::PresentMode::AutoVsync => "AutoVsync",
        wgpu::PresentMode::AutoNoVsync => "AutoNoVsync",
        wgpu::PresentMode::Fifo => "Fifo",
        wgpu::PresentMode::FifoRelaxed => "FifoRelaxed",
        wgpu::PresentMode::Immediate => "Immediate",
        wgpu::PresentMode::Mailbox => "Mailbox",
    }
}

pub(super) fn scale_factor_milliscale(scale_factor: f64) -> u32 {
    if scale_factor.is_finite() && scale_factor > 0.0 {
        (scale_factor * 1000.0)
            .round()
            .clamp(1.0, f64::from(u32::MAX)) as u32
    } else {
        1000
    }
}
