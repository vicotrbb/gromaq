use super::super::SurfaceGlyphFrame;
use super::super::surface_buffers::validate_surface_glyph_frame;
use super::SurfaceFrameError;
use super::solid_draw::{SolidDrawLabels, prepare_solid_draw};

mod atlas;
mod buffers;
mod pipeline;

use atlas::prepare_surface_glyph_atlas;
use buffers::prepare_surface_glyph_draw_buffers;
use pipeline::create_surface_glyph_pipeline;

pub(super) fn render_glyph_frame_to_view(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    view: &wgpu::TextureView,
    format: wgpu::TextureFormat,
    target_width: u32,
    target_height: u32,
    frame: SurfaceGlyphFrame<'_>,
) -> std::result::Result<(), SurfaceFrameError> {
    let atlas_layout = validate_surface_glyph_frame(frame)?;
    let atlas = prepare_surface_glyph_atlas(device, queue, frame.atlas, atlas_layout);
    let pipeline = create_surface_glyph_pipeline(device, format, &atlas.bind_group_layout);
    let background_draw = prepare_solid_draw(
        device,
        queue,
        format,
        frame.background_batch,
        target_width,
        target_height,
        SolidDrawLabels {
            shader: "gromaq-surface-background-shader",
            pipeline_layout: "gromaq-surface-background-pipeline-layout",
            pipeline: "gromaq-surface-background-pipeline",
            vertex_buffer: "gromaq-surface-background-vertices",
            index_buffer: "gromaq-surface-background-indices",
        },
    )?;
    let glyph_draw = prepare_surface_glyph_draw_buffers(
        device,
        queue,
        frame.batch,
        target_width,
        target_height,
    )?;
    let decoration_draw = prepare_solid_draw(
        device,
        queue,
        format,
        frame.decoration_batch,
        target_width,
        target_height,
        SolidDrawLabels {
            shader: "gromaq-surface-decoration-shader",
            pipeline_layout: "gromaq-surface-decoration-pipeline-layout",
            pipeline: "gromaq-surface-decoration-pipeline",
            vertex_buffer: "gromaq-surface-decoration-vertices",
            index_buffer: "gromaq-surface-decoration-indices",
        },
    )?;
    let cursor_draw = prepare_solid_draw(
        device,
        queue,
        format,
        frame.cursor_batch,
        target_width,
        target_height,
        SolidDrawLabels {
            shader: "gromaq-surface-cursor-shader",
            pipeline_layout: "gromaq-surface-cursor-pipeline-layout",
            pipeline: "gromaq-surface-cursor-pipeline",
            vertex_buffer: "gromaq-surface-cursor-vertices",
            index_buffer: "gromaq-surface-cursor-indices",
        },
    )?;
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gromaq-surface-glyph-encoder"),
    });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gromaq-surface-glyph-pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: frame.clear_color[0],
                        g: frame.clear_color[1],
                        b: frame.clear_color[2],
                        a: frame.clear_color[3],
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
            multiview_mask: None,
        });
        if let Some(draw) = &background_draw {
            pass.set_pipeline(&draw.pipeline);
            pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
            pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
        if let Some(draw) = &glyph_draw {
            pass.set_pipeline(&pipeline);
            pass.set_bind_group(0, &atlas.bind_group, &[]);
            pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
            pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
        if let Some(draw) = &decoration_draw {
            pass.set_pipeline(&draw.pipeline);
            pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
            pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
        if let Some(draw) = &cursor_draw {
            pass.set_pipeline(&draw.pipeline);
            pass.set_vertex_buffer(0, draw.vertex_buffer.slice(..));
            pass.set_index_buffer(draw.index_buffer.slice(..), wgpu::IndexFormat::Uint32);
            pass.draw_indexed(0..draw.index_count, 0, 0..1);
        }
    }
    queue.submit([encoder.finish()]);
    Ok(())
}
