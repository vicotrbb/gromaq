use std::sync::mpsc;

use super::GpuBootstrapError;

/// Padded RGBA8 texture readback layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReadbackLayout {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// Dense bytes per row without GPU copy padding.
    pub dense_bytes_per_row: u32,
    /// Padded bytes per row required by `wgpu` texture-to-buffer copies.
    pub padded_bytes_per_row: u32,
    /// Total readback buffer size.
    pub buffer_size: u64,
}

impl ReadbackLayout {
    /// Build a padded layout for an RGBA8 texture.
    pub fn rgba8(width: u32, height: u32) -> std::result::Result<Self, GpuBootstrapError> {
        let dense_bytes_per_row = width.checked_mul(4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("RGBA8 row byte size is too large".to_owned())
        })?;
        let align = wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
        let padded_bytes_per_row = dense_bytes_per_row
            .div_ceil(align)
            .checked_mul(align)
            .ok_or_else(|| {
                GpuBootstrapError::SmokeReadback(
                    "padded readback row byte size is too large".to_owned(),
                )
            })?;
        Ok(Self {
            width,
            height,
            dense_bytes_per_row,
            padded_bytes_per_row,
            buffer_size: u64::from(padded_bytes_per_row) * u64::from(height),
        })
    }
}

pub(super) fn last_rgba_pixel<'a>(
    pixels: &'a [u8],
    label: &'static str,
) -> std::result::Result<&'a [u8], GpuBootstrapError> {
    let start = pixels.len().checked_sub(4).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(format!("{label} is shorter than one RGBA pixel"))
    })?;
    pixels.get(start..).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(format!("{label} is shorter than one RGBA pixel"))
    })
}

pub(super) fn rgba_pixel_at<'a>(
    pixels: &'a [u8],
    pixel_index: usize,
    label: &'static str,
) -> std::result::Result<&'a [u8], GpuBootstrapError> {
    let start = pixel_index.checked_mul(4).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(format!("{label} byte offset is too large"))
    })?;
    let end = start.checked_add(4).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(format!("{label} byte offset is too large"))
    })?;
    pixels.get(start..end).ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(format!("{label} is missing from readback"))
    })
}

pub(super) fn read_texture_rgba8(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    texture: &wgpu::Texture,
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let layout = ReadbackLayout::rgba8(width, height)?;
    let readback = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gromaq-texture-readback"),
        size: layout.buffer_size,
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
        label: Some("gromaq-texture-readback-encoder"),
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &readback,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(layout.padded_bytes_per_row),
                rows_per_image: Some(height),
            },
        },
        wgpu::Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([encoder.finish()]);
    read_dense_rgba8_from_buffer(device, &readback, layout)
}

fn read_dense_rgba8_from_buffer(
    device: &wgpu::Device,
    readback: &wgpu::Buffer,
    layout: ReadbackLayout,
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let slice = readback.slice(..);
    let (sender, receiver) = mpsc::channel();
    slice.map_async(wgpu::MapMode::Read, move |result| {
        let _ = sender.send(result.map_err(|error| error.to_string()));
    });
    device
        .poll(wgpu::PollType::wait_indefinitely())
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    receiver
        .recv()
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?
        .map_err(GpuBootstrapError::SmokeReadback)?;

    let mapped = slice.get_mapped_range();
    let dense_len = usize::try_from(layout.dense_bytes_per_row)
        .ok()
        .and_then(|row_bytes| {
            usize::try_from(layout.height)
                .ok()
                .and_then(|height| row_bytes.checked_mul(height))
        })
        .ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("dense readback size is too large".to_owned())
        })?;
    let mut dense = Vec::new();
    dense.try_reserve_exact(dense_len).map_err(|_| {
        GpuBootstrapError::SmokeReadback("dense readback buffer is too large".to_owned())
    })?;
    for row in 0..layout.height {
        let start = usize::try_from(u64::from(row) * u64::from(layout.padded_bytes_per_row))
            .map_err(|_| {
                GpuBootstrapError::SmokeReadback("readback row offset is too large".to_owned())
            })?;
        let row_bytes = usize::try_from(layout.dense_bytes_per_row).map_err(|_| {
            GpuBootstrapError::SmokeReadback("readback row byte size is too large".to_owned())
        })?;
        let end = start.checked_add(row_bytes).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("readback row end offset is too large".to_owned())
        })?;
        if end > mapped.len() {
            return Err(GpuBootstrapError::SmokeReadback(
                "readback row exceeds mapped buffer".to_owned(),
            ));
        }
        dense.extend_from_slice(&mapped[start..end]);
    }
    drop(mapped);
    readback.unmap();
    Ok(dense)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn last_rgba_pixel_reports_checked_slice() {
        assert_eq!(
            last_rgba_pixel(&[1, 2, 3, 4], "readback").unwrap(),
            &[1, 2, 3, 4]
        );
        assert_eq!(
            last_rgba_pixel(&[1, 2, 3, 4, 5, 6, 7, 8], "readback").unwrap(),
            &[5, 6, 7, 8]
        );
    }

    #[test]
    fn last_rgba_pixel_rejects_short_buffers() {
        let error = last_rgba_pixel(&[1, 2, 3], "readback").unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback("readback is shorter than one RGBA pixel".to_owned())
        );
    }

    #[test]
    fn rgba_pixel_at_reports_checked_slice() {
        assert_eq!(
            rgba_pixel_at(&[1, 2, 3, 4, 5, 6, 7, 8], 1, "pixel").unwrap(),
            &[5, 6, 7, 8]
        );
    }

    #[test]
    fn rgba_pixel_at_rejects_missing_pixel() {
        let error = rgba_pixel_at(&[1, 2, 3, 4], 1, "pixel").unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback("pixel is missing from readback".to_owned())
        );
    }

    #[test]
    fn rgba_pixel_at_rejects_overflowing_offset() {
        let error = rgba_pixel_at(&[], usize::MAX, "pixel").unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback("pixel byte offset is too large".to_owned())
        );
    }
}
