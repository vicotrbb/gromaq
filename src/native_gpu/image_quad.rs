//! Vertex/batch helpers for centered image-quad rendering (the welcome splash
//! avatar). Kept in its own module so `quad_bytes.rs` and `offscreen.rs` stay
//! under the source line-limit policy.

use super::GpuBootstrapError;
use super::quad_bytes::textured_quad_index_bytes;
use crate::renderer::{BackgroundQuad, BackgroundQuadBatch, BackgroundVertex};

/// Build the textured vertices + indices for a single image quad centered in the
/// render target, preserving the source aspect ratio and fitting it to
/// `fit_fraction` of the smaller target dimension. Positions are emitted in the
/// same pixel→NDC space the offscreen textured pipeline expects, UVs span the
/// full [0,1] source rect, and the foreground attribute is opaque white so the
/// textured shader passes the image RGB through unmodified.
pub(super) fn centered_image_quad(
    target_width: u32,
    target_height: u32,
    image_width: u32,
    image_height: u32,
    fit_fraction: f32,
) -> std::result::Result<(Vec<u8>, Vec<u8>), GpuBootstrapError> {
    if target_width == 0
        || target_height == 0
        || image_width == 0
        || image_height == 0
        || !(0.0 < fit_fraction && fit_fraction <= 1.0)
    {
        return Err(GpuBootstrapError::SmokeReadback(
            "image quad dimensions and fit fraction must be positive".to_owned(),
        ));
    }
    let target_width = target_width as f32;
    let target_height = target_height as f32;
    let image_width = image_width as f32;
    let image_height = image_height as f32;
    let scale = fit_fraction * target_width.min(target_height) / image_width.max(image_height);
    let draw_width = image_width * scale;
    let draw_height = image_height * scale;
    let x0 = (target_width - draw_width) / 2.0;
    let y0 = (target_height - draw_height) / 2.0;
    let corners = [
        (x0, y0, 0.0_f32, 0.0),
        (x0 + draw_width, y0, 1.0, 0.0),
        (x0 + draw_width, y0 + draw_height, 1.0, 1.0),
        (x0, y0 + draw_height, 0.0, 1.0),
    ];
    let mut vertices = Vec::with_capacity(4 * 8 * 4);
    for (pixel_x, pixel_y, uv_x, uv_y) in corners {
        let ndc_x = (pixel_x / target_width * 2.0) - 1.0;
        let ndc_y = 1.0 - (pixel_y / target_height * 2.0);
        for value in [ndc_x, ndc_y, uv_x, uv_y, 1.0, 1.0, 1.0, 1.0] {
            vertices.extend_from_slice(&value.to_le_bytes());
        }
    }
    Ok((vertices, textured_quad_index_bytes()))
}

/// A full-target solid background quad in pixel space, used as the splash
/// surface behind the centered image quad.
pub(super) fn full_target_background_batch(
    width: u32,
    height: u32,
    color_rgba: [f32; 4],
) -> BackgroundQuadBatch {
    let width = width as f32;
    let height = height as f32;
    let corners = [
        BackgroundVertex {
            position: [0.0, 0.0],
            color_rgba,
        },
        BackgroundVertex {
            position: [width, 0.0],
            color_rgba,
        },
        BackgroundVertex {
            position: [width, height],
            color_rgba,
        },
        BackgroundVertex {
            position: [0.0, height],
            color_rgba,
        },
    ];
    BackgroundQuadBatch {
        quads: vec![BackgroundQuad {
            row: 0,
            col: 0,
            cols: 1,
            vertices: corners,
        }],
        indices: vec![0, 1, 2, 0, 2, 3],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centered_image_quad_fits_and_centers_within_target() {
        let (vertices, indices) = centered_image_quad(480, 480, 320, 320, 0.8).unwrap();
        assert_eq!(vertices.len(), 4 * 8 * 4);
        assert_eq!(indices.len(), 6 * 2);

        // 320x320 image fit to 0.8 of the 480 target: scale 1.2 -> 384x384,
        // centered with a 48px margin. The top-left vertex sits at NDC
        // (-0.8, 0.8) (pixel 48,48) sampling UV (0,0).
        let ndc_x = f32::from_le_bytes(vertices[0..4].try_into().unwrap());
        let ndc_y = f32::from_le_bytes(vertices[4..8].try_into().unwrap());
        let uv_x = f32::from_le_bytes(vertices[8..12].try_into().unwrap());
        let uv_y = f32::from_le_bytes(vertices[12..16].try_into().unwrap());
        assert!((ndc_x - (-0.8)).abs() < 1e-4, "ndc_x {ndc_x}");
        assert!((ndc_y - 0.8).abs() < 1e-4, "ndc_y {ndc_y}");
        assert_eq!(uv_x, 0.0);
        assert_eq!(uv_y, 0.0);

        let error = centered_image_quad(0, 480, 320, 320, 0.8).unwrap_err();
        assert!(matches!(error, GpuBootstrapError::SmokeReadback(_)));
    }
}
