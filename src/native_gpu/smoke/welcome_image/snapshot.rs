use super::super::super::GpuBootstrapError;

#[derive(Debug)]
pub(super) struct WelcomeImagePixels {
    pub(super) background_pixel: [u8; 4],
    pub(super) image_pixel: [u8; 4],
    pub(super) drawn_pixels: usize,
}

pub(super) fn welcome_image_report(
    pixels: &[u8],
    width: u32,
    height: u32,
    background: [u8; 4],
) -> std::result::Result<WelcomeImagePixels, GpuBootstrapError> {
    let background_pixel = pixel_at(pixels, width, 0, 0)?;
    if background_pixel != background {
        return Err(GpuBootstrapError::SmokeReadback(format!(
            "welcome image background pixel {background_pixel:?} did not match {background:?}"
        )));
    }
    let image_pixel = pixel_at(pixels, width, width / 2, height / 2)?;
    if image_pixel == background {
        return Err(GpuBootstrapError::SmokeReadback(
            "welcome image center pixel matched the background; avatar did not render".to_owned(),
        ));
    }
    let drawn_pixels = pixels
        .chunks_exact(4)
        .filter(|pixel| *pixel != background)
        .count();
    Ok(WelcomeImagePixels {
        background_pixel,
        image_pixel,
        drawn_pixels,
    })
}

fn pixel_at(
    pixels: &[u8],
    width: u32,
    x: u32,
    y: u32,
) -> std::result::Result<[u8; 4], GpuBootstrapError> {
    let index = usize::try_from(y)
        .ok()
        .and_then(|row| row.checked_mul(width as usize))
        .and_then(|offset| offset.checked_add(x as usize))
        .and_then(|pixel_index| pixel_index.checked_mul(4))
        .filter(|start| pixels.get(*start..*start + 4).is_some())
        .ok_or_else(|| {
            GpuBootstrapError::SmokeReadback("welcome image pixel out of range".to_owned())
        })?;
    let [a, b, c, d] = pixels[index..index + 4].try_into().unwrap();
    Ok([a, b, c, d])
}

pub(super) fn composite_over_background(rgba: &[u8], background: [u8; 4]) -> Vec<u8> {
    rgba.chunks_exact(4)
        .flat_map(|pixel| {
            let alpha = f32::from(pixel[3]) / 255.0;
            let blend = |figure: u8, bg: u8| {
                (f32::from(figure) * alpha + f32::from(bg) * (1.0 - alpha))
                    .round()
                    .clamp(0.0, 255.0) as u8
            };
            [
                blend(pixel[0], background[0]),
                blend(pixel[1], background[1]),
                blend(pixel[2], background[2]),
                255,
            ]
        })
        .collect()
}

pub(super) fn ppm_bytes(
    width: u32,
    height: u32,
    pixels: &[u8],
) -> std::result::Result<Vec<u8>, GpuBootstrapError> {
    let expected = usize::try_from(u64::from(width) * u64::from(height) * 4)
        .map_err(|_| GpuBootstrapError::SmokeReadback("welcome image too large".to_owned()))?;
    if pixels.len() != expected {
        return Err(GpuBootstrapError::SmokeReadback(format!(
            "welcome image expected {expected} RGBA bytes, got {}",
            pixels.len()
        )));
    }
    let mut out = Vec::with_capacity(pixels.len() / 4 * 3 + 32);
    out.extend_from_slice(format!("P6\n{width} {height}\n255\n").as_bytes());
    for pixel in pixels.chunks_exact(4) {
        out.extend_from_slice(&pixel[0..3]);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BG: [u8; 4] = [10, 20, 30, 255];

    #[test]
    fn composite_over_background_resolves_alpha_to_opaque() {
        let src = [200, 100, 50, 255, 200, 100, 50, 128, 0, 255, 0, 0];
        let out = composite_over_background(&src, BG);
        assert_eq!(out[0..4], [200, 100, 50, 255]);
        assert_eq!(out[8..12], BG);
        assert!(out.chunks_exact(4).all(|pixel| pixel[3] == 255));
    }

    #[test]
    fn welcome_image_report_counts_avatar_coverage() {
        const WIDTH: u32 = 4;
        const HEIGHT: u32 = 4;
        let mut pixels = vec![0u8; (WIDTH * HEIGHT * 4) as usize];
        for chunk in pixels.chunks_exact_mut(4) {
            chunk.copy_from_slice(&BG);
        }
        let center = ((HEIGHT / 2) * WIDTH + WIDTH / 2) as usize * 4;
        pixels[center..center + 4].copy_from_slice(&[200, 180, 240, 255]);

        let report = welcome_image_report(&pixels, WIDTH, HEIGHT, BG).unwrap();
        assert_eq!(report.background_pixel, BG);
        assert_eq!(report.image_pixel, [200, 180, 240, 255]);
        assert_eq!(report.drawn_pixels, 1);
    }

    #[test]
    fn welcome_image_report_rejects_unrendered_avatar() {
        const WIDTH: u32 = 4;
        const HEIGHT: u32 = 4;
        let mut filled = Vec::with_capacity((WIDTH * HEIGHT * 4) as usize);
        for _ in 0..(WIDTH * HEIGHT) {
            filled.extend_from_slice(&BG);
        }
        let error = welcome_image_report(&filled, WIDTH, HEIGHT, BG).unwrap_err();
        assert!(matches!(error, GpuBootstrapError::SmokeReadback(_)));
    }
}
