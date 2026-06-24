use super::GpuBootstrapError;
use crate::renderer::GlyphAtlasImage;

/// Deterministic RGBA8 upload pattern.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UploadPattern {
    /// Texture width in pixels.
    pub width: u32,
    /// Texture height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
}

/// Checked dense RGBA8 upload layout.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UploadPatternLayout {
    /// Dense bytes per row.
    pub row_bytes: u32,
    /// Expected total dense RGBA8 byte length.
    pub expected_len: usize,
}

impl UploadPattern {
    /// Build a 2x2 RGBA checker pattern.
    pub fn checker_rgba8_2x2() -> Self {
        Self {
            width: 2,
            height: 2,
            rgba: vec![
                255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255,
            ],
        }
    }

    /// Build an upload pattern from a packed glyph atlas image.
    pub fn from_glyph_atlas_image(image: &GlyphAtlasImage) -> Self {
        Self {
            width: image.width,
            height: image.height,
            rgba: image.rgba.clone(),
        }
    }

    /// Validate dimensions and return the dense RGBA8 upload layout.
    pub fn rgba8_layout(&self) -> std::result::Result<UploadPatternLayout, GpuBootstrapError> {
        if self.width == 0 || self.height == 0 {
            return Err(GpuBootstrapError::SmokeReadback(
                "upload pattern dimensions must be non-zero".to_owned(),
            ));
        }
        let row_bytes = self.width.checked_mul(4).ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("upload pattern row byte size is too large".to_owned())
        })?;
        let expected_len = usize::try_from(row_bytes)
            .ok()
            .and_then(|row_bytes| {
                usize::try_from(self.height)
                    .ok()
                    .and_then(|height| row_bytes.checked_mul(height))
            })
            .ok_or_else(|| {
                GpuBootstrapError::SmokeReadback("upload pattern byte size is too large".to_owned())
            })?;
        Ok(UploadPatternLayout {
            row_bytes,
            expected_len,
        })
    }
}
