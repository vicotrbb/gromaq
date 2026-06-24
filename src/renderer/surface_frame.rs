use thiserror::Error;

use super::SurfaceGlyphFrame;
use super::surface::SurfaceBackend;
use glyph_pass::render_glyph_frame_to_view;

mod glyph_pass;
mod glyph_shader;
mod solid_draw;

/// Errors produced while acquiring or presenting a native surface frame.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum SurfaceFrameError {
    /// Surface acquisition timed out; the frame should be skipped.
    #[error("surface frame acquisition timed out")]
    Timeout,
    /// Surface is currently occluded; the frame should be skipped.
    #[error("surface is occluded")]
    Occluded,
    /// Surface configuration is outdated and must be refreshed.
    #[error("surface configuration is outdated")]
    Outdated,
    /// Surface was lost and must be recreated.
    #[error("surface was lost")]
    Lost,
    /// Surface acquisition hit a validation error.
    #[error("surface frame acquisition validation error")]
    Validation,
    /// A terminal glyph frame could not be rendered.
    #[error("invalid surface frame: {0}")]
    InvalidFrame(String),
}

/// Surface endpoint that can render and present a frame.
pub trait SurfaceFrameBackend {
    /// Clear the current surface frame to `clear_color` and present it.
    fn clear_and_present(
        &mut self,
        clear_color: [f64; 4],
    ) -> std::result::Result<(), SurfaceFrameError>;

    /// Render terminal glyph quads into the current surface frame and present it.
    fn present_glyph_frame(
        &mut self,
        frame: SurfaceGlyphFrame<'_>,
    ) -> std::result::Result<(), SurfaceFrameError>;
}

/// Concrete `wgpu` surface backend used by the native app once a window surface exists.
pub struct WgpuSurfaceBackend<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    current_format: Option<wgpu::TextureFormat>,
    current_size: Option<(u32, u32)>,
}

impl<'a> WgpuSurfaceBackend<'a> {
    /// Create a surface backend from a `wgpu` surface and device.
    pub fn new(surface: wgpu::Surface<'a>, device: &wgpu::Device, queue: &wgpu::Queue) -> Self {
        Self {
            surface,
            device: device.clone(),
            queue: queue.clone(),
            current_format: None,
            current_size: None,
        }
    }
}

impl SurfaceBackend for WgpuSurfaceBackend<'_> {
    fn configure(&mut self, config: &wgpu::SurfaceConfiguration) {
        self.surface.configure(&self.device, config);
        self.current_format = Some(config.format);
        self.current_size = Some((config.width, config.height));
    }
}

impl SurfaceFrameBackend for WgpuSurfaceBackend<'_> {
    fn clear_and_present(
        &mut self,
        clear_color: [f64; 4],
    ) -> std::result::Result<(), SurfaceFrameError> {
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout => return Err(SurfaceFrameError::Timeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Err(SurfaceFrameError::Occluded),
            wgpu::CurrentSurfaceTexture::Outdated => return Err(SurfaceFrameError::Outdated),
            wgpu::CurrentSurfaceTexture::Lost => return Err(SurfaceFrameError::Lost),
            wgpu::CurrentSurfaceTexture::Validation => return Err(SurfaceFrameError::Validation),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("gromaq-surface-clear-encoder"),
            });
        {
            encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gromaq-surface-clear-pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    depth_slice: None,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: clear_color[0],
                            g: clear_color[1],
                            b: clear_color[2],
                            a: clear_color[3],
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
                multiview_mask: None,
            });
        }
        self.queue.submit([encoder.finish()]);
        frame.present();
        Ok(())
    }

    fn present_glyph_frame(
        &mut self,
        glyph_frame: SurfaceGlyphFrame<'_>,
    ) -> std::result::Result<(), SurfaceFrameError> {
        let Some(format) = self.current_format else {
            return Err(SurfaceFrameError::InvalidFrame(
                "surface must be configured before drawing terminal glyphs".to_owned(),
            ));
        };
        let Some((target_width, target_height)) = self.current_size else {
            return Err(SurfaceFrameError::InvalidFrame(
                "surface target size is unknown before drawing terminal glyphs".to_owned(),
            ));
        };
        let frame = match self.surface.get_current_texture() {
            wgpu::CurrentSurfaceTexture::Success(frame)
            | wgpu::CurrentSurfaceTexture::Suboptimal(frame) => frame,
            wgpu::CurrentSurfaceTexture::Timeout => return Err(SurfaceFrameError::Timeout),
            wgpu::CurrentSurfaceTexture::Occluded => return Err(SurfaceFrameError::Occluded),
            wgpu::CurrentSurfaceTexture::Outdated => return Err(SurfaceFrameError::Outdated),
            wgpu::CurrentSurfaceTexture::Lost => return Err(SurfaceFrameError::Lost),
            wgpu::CurrentSurfaceTexture::Validation => return Err(SurfaceFrameError::Validation),
        };
        let view = frame
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());
        render_glyph_frame_to_view(
            &self.device,
            &self.queue,
            &view,
            format,
            target_width,
            target_height,
            glyph_frame,
        )?;
        frame.present();
        Ok(())
    }
}
