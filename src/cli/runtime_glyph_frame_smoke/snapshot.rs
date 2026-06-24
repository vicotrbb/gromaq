use std::fs;
use std::path::Path;

use super::{
    prepare_runtime_glyph_frame_smoke, runtime_glyph_frame_smoke_error,
    runtime_glyph_frame_smoke_failure,
};
use crate::cli::CliExit;

pub(in crate::cli) fn runtime_glyph_frame_snapshot_exit(path: &str) -> CliExit {
    let prepared = match prepare_runtime_glyph_frame_smoke() {
        Ok(prepared) => prepared,
        Err(exit) => return exit,
    };
    let preview = match prepared.prepared.preview_rgba8() {
        Ok(preview) => preview,
        Err(error) => return runtime_glyph_frame_smoke_error(error),
    };
    let snapshot = match runtime_glyph_frame_ppm_bytes(preview.width, preview.height, &preview.rgba)
    {
        Ok(snapshot) => snapshot,
        Err(exit) => return exit,
    };
    if let Err(error) = fs::write(Path::new(path), &snapshot) {
        return runtime_glyph_frame_smoke_error(format!(
            "failed to write runtime glyph frame snapshot: {error}"
        ));
    }
    let surface_frame = prepared.prepared.as_surface_glyph_frame();

    CliExit {
        code: 0,
        stdout: format!(
            "runtime glyph frame snapshot: ok\npath: {path}\nbytes written: {}\nframe size: {}x{}\npreview pixels: {}\nprepared quads: {}\nbackground quads: {}\ncursor quads: {}\natlas bytes: {}\n",
            snapshot.len(),
            preview.width,
            preview.height,
            preview.rgba.len() / 4,
            surface_frame.batch.quads.len(),
            surface_frame.background_batch.quads.len(),
            surface_frame.cursor_batch.quads.len(),
            surface_frame.atlas.rgba.len()
        ),
        stderr: String::new(),
    }
}

fn runtime_glyph_frame_ppm_bytes(
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Result<Vec<u8>, CliExit> {
    let expected_rgba_len = match usize::try_from(u64::from(width) * u64::from(height) * 4) {
        Ok(len) => len,
        Err(_) => {
            return Err(runtime_glyph_frame_smoke_failure(
                "runtime glyph frame snapshot is too large",
            ));
        }
    };
    if pixels.len() != expected_rgba_len {
        return Err(runtime_glyph_frame_smoke_failure(&format!(
            "runtime glyph frame snapshot expected {expected_rgba_len} RGBA bytes, got {}",
            pixels.len()
        )));
    }
    let header = format!("P6\n{width} {height}\n255\n");
    let rgb_len = match usize::try_from(u64::from(width) * u64::from(height) * 3) {
        Ok(len) => len,
        Err(_) => {
            return Err(runtime_glyph_frame_smoke_failure(
                "runtime glyph frame snapshot RGB buffer is too large",
            ));
        }
    };
    let mut snapshot = Vec::new();
    if snapshot.try_reserve_exact(header.len() + rgb_len).is_err() {
        return Err(runtime_glyph_frame_smoke_failure(
            "runtime glyph frame snapshot allocation failed",
        ));
    }
    snapshot.extend_from_slice(header.as_bytes());
    for pixel in pixels.chunks_exact(4) {
        snapshot.extend_from_slice(&pixel[..3]);
    }
    Ok(snapshot)
}
