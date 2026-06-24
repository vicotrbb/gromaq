//! Textured-quad offscreen texture construction.

use super::TexturedDrawInput;

pub(super) fn create_source_texture(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    input: &TexturedDrawInput<'_>,
) -> wgpu::Texture {
    let source = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-textured-quad-source"),
        size: wgpu::Extent3d {
            width: input.pattern.width,
            height: input.pattern.height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::COPY_DST | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    });
    queue.write_texture(
        source.as_image_copy(),
        &input.pattern.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(input.source_layout.row_bytes),
            rows_per_image: Some(input.pattern.height),
        },
        wgpu::Extent3d {
            width: input.pattern.width,
            height: input.pattern.height,
            depth_or_array_layers: 1,
        },
    );
    source
}

pub(super) fn create_target_texture(
    device: &wgpu::Device,
    width: u32,
    height: u32,
) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-textured-quad-target"),
        size: wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: wgpu::TextureFormat::Rgba8Unorm,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
