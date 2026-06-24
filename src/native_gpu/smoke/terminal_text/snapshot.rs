use super::super::super::GpuBootstrapError;

pub(super) fn terminal_text_ppm_bytes(
    width: u32,
    height: u32,
    pixels: &[u8],
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let expected_rgba_len =
        usize::try_from(u64::from(width) * u64::from(height) * 4).map_err(|_| {
            GpuBootstrapError::SmokeReadback("terminal text snapshot is too large".to_owned())
        })?;
    if pixels.len() != expected_rgba_len {
        return Err(GpuBootstrapError::SmokeReadback(format!(
            "terminal text snapshot expected {expected_rgba_len} RGBA bytes, got {}",
            pixels.len()
        )));
    }
    let header = format!("P6\n{width} {height}\n255\n");
    let rgb_len = usize::try_from(u64::from(width) * u64::from(height) * 3).map_err(|_| {
        GpuBootstrapError::SmokeReadback(
            "terminal text snapshot RGB buffer is too large".to_owned(),
        )
    })?;
    let mut snapshot = Vec::new();
    snapshot
        .try_reserve_exact(header.len() + rgb_len)
        .map_err(|_| GpuBootstrapError::SmokeReadback("snapshot allocation failed".to_owned()))?;
    snapshot.extend_from_slice(header.as_bytes());
    for pixel in pixels.chunks_exact(4) {
        snapshot.extend_from_slice(&pixel[..3]);
    }
    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_text_ppm_bytes_writes_binary_rgb_snapshot() {
        let snapshot = terminal_text_ppm_bytes(2, 1, &[255, 0, 0, 255, 4, 5, 6, 128]).unwrap();

        assert_eq!(snapshot, b"P6\n2 1\n255\n\xff\x00\x00\x04\x05\x06");
    }

    #[test]
    fn terminal_text_ppm_bytes_rejects_mismatched_rgba_len() {
        let error = terminal_text_ppm_bytes(2, 1, &[255, 0, 0, 255]).unwrap_err();

        assert_eq!(
            error,
            GpuBootstrapError::SmokeReadback(
                "terminal text snapshot expected 8 RGBA bytes, got 4".to_owned()
            )
        );
    }
}
