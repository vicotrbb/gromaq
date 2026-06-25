use crate::renderer::GlyphAtlasImage;

use super::super::super::surface_buffers::SurfaceGlyphAtlasLayout;

pub(super) struct SurfaceGlyphAtlasResources {
    pub(super) bind_group: wgpu::BindGroup,
    pub(super) bind_group_layout: wgpu::BindGroupLayout,
}

pub(super) fn prepare_surface_glyph_atlas(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    atlas_image: &GlyphAtlasImage,
    atlas_layout: SurfaceGlyphAtlasLayout,
) -> SurfaceGlyphAtlasResources {
    let atlas = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("gromaq-surface-glyph-atlas"),
        size: wgpu::Extent3d {
            width: atlas_image.width,
            height: atlas_image.height,
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
        atlas.as_image_copy(),
        &atlas_image.rgba,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(atlas_layout.row_bytes),
            rows_per_image: Some(atlas_image.height),
        },
        wgpu::Extent3d {
            width: atlas_image.width,
            height: atlas_image.height,
            depth_or_array_layers: 1,
        },
    );
    let atlas_view = atlas.create_view(&wgpu::TextureViewDescriptor::default());
    let sampler = device.create_sampler(&surface_glyph_sampler_descriptor());
    let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("gromaq-surface-glyph-bind-group-layout"),
        entries: &[
            wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Texture {
                    sample_type: wgpu::TextureSampleType::Float { filterable: true },
                    view_dimension: wgpu::TextureViewDimension::D2,
                    multisampled: false,
                },
                count: None,
            },
            wgpu::BindGroupLayoutEntry {
                binding: 1,
                visibility: wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                count: None,
            },
        ],
    });
    let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("gromaq-surface-glyph-bind-group"),
        layout: &bind_group_layout,
        entries: &[
            wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&atlas_view),
            },
            wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&sampler),
            },
        ],
    });

    SurfaceGlyphAtlasResources {
        bind_group,
        bind_group_layout,
    }
}

fn surface_glyph_sampler_descriptor() -> wgpu::SamplerDescriptor<'static> {
    wgpu::SamplerDescriptor {
        label: Some("gromaq-surface-glyph-sampler"),
        mag_filter: wgpu::FilterMode::Linear,
        min_filter: wgpu::FilterMode::Linear,
        mipmap_filter: wgpu::MipmapFilterMode::Nearest,
        ..Default::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surface_glyph_sampler_uses_linear_filtering_for_text() {
        let descriptor = surface_glyph_sampler_descriptor();

        assert_eq!(descriptor.mag_filter, wgpu::FilterMode::Linear);
        assert_eq!(descriptor.min_filter, wgpu::FilterMode::Linear);
        assert_eq!(descriptor.mipmap_filter, wgpu::MipmapFilterMode::Nearest);
    }
}
