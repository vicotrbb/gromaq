//! Native app snapshot artifact helpers.

use super::NativeGlyphFrameError;

pub(super) fn prepared_frame_ppm_bytes(
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<Vec<u8>, NativeGlyphFrameError> {
    let expected_rgba_len =
        usize::try_from(u64::from(width) * u64::from(height) * 4).map_err(|_| {
            NativeGlyphFrameError::Snapshot("native glyph frame snapshot is too large".to_owned())
        })?;
    if pixels.len() != expected_rgba_len {
        return Err(NativeGlyphFrameError::Snapshot(format!(
            "native glyph frame snapshot expected {expected_rgba_len} RGBA bytes, got {}",
            pixels.len()
        )));
    }
    let rgb_len = usize::try_from(u64::from(width) * u64::from(height) * 3).map_err(|_| {
        NativeGlyphFrameError::Snapshot(
            "native glyph frame snapshot RGB buffer is too large".to_owned(),
        )
    })?;
    let header = format!("P6\n{width} {height}\n255\n");
    let mut snapshot = Vec::new();
    snapshot
        .try_reserve_exact(header.len() + rgb_len)
        .map_err(|_| {
            NativeGlyphFrameError::Snapshot(
                "native glyph frame snapshot allocation failed".to_owned(),
            )
        })?;
    snapshot.extend_from_slice(header.as_bytes());
    for pixel in pixels.chunks_exact(4) {
        snapshot.extend_from_slice(&pixel[..3]);
    }
    Ok(snapshot)
}
