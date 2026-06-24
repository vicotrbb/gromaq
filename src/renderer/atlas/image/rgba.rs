use super::GlyphImageError;

pub(super) fn rgba_row_byte_len(width: u32) -> std::result::Result<usize, GlyphImageError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or(GlyphImageError::RgbaRowDimensionsTooLarge)
}

pub(super) fn rgba_pixel_count(
    width: u32,
    height: u32,
) -> std::result::Result<usize, GlyphImageError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(GlyphImageError::RgbaImageDimensionsTooLarge)
}

pub(super) fn rgba_byte_len(
    width: u32,
    height: u32,
) -> std::result::Result<usize, GlyphImageError> {
    rgba_pixel_count(width, height)?
        .checked_mul(4)
        .ok_or(GlyphImageError::RgbaImageDimensionsTooLarge)
}

pub(super) fn checked_rgba_row_offset(
    row: usize,
    row_bytes: usize,
) -> std::result::Result<usize, GlyphImageError> {
    row.checked_mul(row_bytes)
        .ok_or(GlyphImageError::RgbaRowOffsetTooLarge)
}

pub(super) fn zeroed_rgba_buffer(
    width: u32,
    height: u32,
) -> std::result::Result<Vec<u8>, GlyphImageError> {
    let len = rgba_byte_len(width, height)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len)
        .map_err(|_| GlyphImageError::RgbaBufferAllocationTooLarge)?;
    rgba.resize(len, 0);
    Ok(rgba)
}

pub(super) fn rgba_offset(
    width: u32,
    x: u32,
    y: u32,
) -> std::result::Result<usize, GlyphImageError> {
    usize::try_from(y)
        .ok()
        .and_then(|y| {
            usize::try_from(width)
                .ok()
                .and_then(|width| y.checked_mul(width))
        })
        .and_then(|row_start| {
            usize::try_from(x)
                .ok()
                .and_then(|x| row_start.checked_add(x))
        })
        .and_then(|pixel_offset| pixel_offset.checked_mul(4))
        .ok_or(GlyphImageError::RgbaImageOffsetTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rgba_row_offset_uses_checked_multiplication() {
        assert_eq!(checked_rgba_row_offset(3, 8).unwrap(), 24);

        let error = checked_rgba_row_offset((usize::MAX / 8) + 1, 8).unwrap_err();

        assert_eq!(error, GlyphImageError::RgbaRowOffsetTooLarge);
    }
}
