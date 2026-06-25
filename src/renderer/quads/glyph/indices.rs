use super::GlyphQuadError;

pub(super) fn checked_glyph_quad_base_index(
    quad_index: usize,
) -> std::result::Result<u32, GlyphQuadError> {
    u32::try_from(quad_index)
        .ok()
        .and_then(|index| index.checked_mul(4))
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

pub(super) fn checked_glyph_quad_index_capacity(
    quad_count: usize,
) -> std::result::Result<usize, GlyphQuadError> {
    quad_count
        .checked_mul(6)
        .ok_or(GlyphQuadError::IndexCountTooLarge)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn glyph_quad_base_index_accepts_last_representable_quad() {
        let last_valid_quad = usize::try_from(u32::MAX / 4).unwrap();

        assert_eq!(
            checked_glyph_quad_base_index(last_valid_quad).unwrap(),
            u32::MAX - 3
        );
    }

    #[test]
    fn glyph_quad_base_index_rejects_overflowing_quad_count() {
        let first_invalid_quad = usize::try_from(u32::MAX / 4).unwrap() + 1;

        let error = checked_glyph_quad_base_index(first_invalid_quad).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
    }

    #[test]
    fn glyph_quad_index_capacity_uses_checked_multiplication() {
        assert_eq!(checked_glyph_quad_index_capacity(7).unwrap(), 42);

        let error = checked_glyph_quad_index_capacity((usize::MAX / 6) + 1).unwrap_err();

        assert_eq!(error, GlyphQuadError::IndexCountTooLarge);
    }
}
