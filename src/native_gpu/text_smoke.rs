//! Font-backed terminal text smoke fixtures for native GPU validation.

use std::path::{Path, PathBuf};

use crate::font::{RasterizedGlyphBatch, RasterizedGlyphCache};
use crate::renderer::{GlyphAtlas, GlyphAtlasConfig, GlyphAtlasImage, RenderPlan, RenderPlanner};
use crate::{Terminal, TerminalConfig};

use super::GpuBootstrapError;

#[derive(Debug)]
pub(super) struct TextAtlasSmokeFrame {
    pub(super) image: GlyphAtlasImage,
    pub(super) batch: RasterizedGlyphBatch,
    pub(super) plan: RenderPlan,
    pub(super) slot_width: u32,
    pub(super) slot_height: u32,
    pub(super) atlas_columns: u32,
}

pub(super) fn build_text_atlas_smoke_image()
-> std::result::Result<(GlyphAtlasImage, RasterizedGlyphBatch), GpuBootstrapError> {
    let frame = build_text_atlas_smoke_frame()?;
    Ok((frame.image, frame.batch))
}

pub(super) fn build_text_atlas_smoke_frame()
-> std::result::Result<TextAtlasSmokeFrame, GpuBootstrapError> {
    let mut terminal = Terminal::new(
        TerminalConfig::new(8, 2)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
    );
    terminal
        .write_str("\x1b[48:2:9:13:18;38:2:244:247:251m A😀A\x1b[0;4;31mB")
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(
        GlyphAtlasConfig::new(8)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
    );
    let mut planner = RenderPlanner::new(18);
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let mut cache = RasterizedGlyphCache::from_font_bytes(system_smoke_font_bytes()?)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let batch = cache
        .rasterize_plan(&plan)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    let slot_width = batch
        .bitmaps
        .iter()
        .map(|glyph| glyph.terminal_slot_width(0))
        .max()
        .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty text atlas batch".to_owned()))?;
    let slot_height = batch
        .bitmaps
        .iter()
        .map(|glyph| glyph.terminal_slot_height(0))
        .max()
        .ok_or_else(|| GpuBootstrapError::SmokeReadback("empty text atlas batch".to_owned()))?;
    let padded = batch
        .bitmaps
        .iter()
        .map(|glyph| {
            glyph
                .padded_to_terminal_slot(slot_width, slot_height)
                .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))
        })
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let image = GlyphAtlasImage::pack_rgba8(slot_width, slot_height, 2, &padded)
        .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?;
    Ok(TextAtlasSmokeFrame {
        image,
        batch,
        plan,
        slot_width,
        slot_height,
        atlas_columns: 2,
    })
}

fn system_mono_font_path() -> std::result::Result<PathBuf, GpuBootstrapError> {
    [
        "/System/Library/Fonts/SFNSMono.ttf",
        "/System/Library/Fonts/Menlo.ttc",
        "/System/Library/Fonts/Supplemental/Courier New.ttf",
        "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
        "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
        "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
        "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
        "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
    ]
    .into_iter()
    .map(Path::new)
    .find(|path| path.exists())
    .map(Path::to_path_buf)
    .ok_or_else(|| {
        GpuBootstrapError::SmokeReadback(
            "no supported system monospace font found for text atlas smoke".to_owned(),
        )
    })
}

fn system_smoke_font_bytes() -> std::result::Result<Vec<Vec<u8>>, GpuBootstrapError> {
    let mut font_bytes = vec![
        std::fs::read(system_mono_font_path()?)
            .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
    ];
    for fallback_path in [
        "/System/Library/Fonts/Apple Color Emoji.ttc",
        "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
    ] {
        let path = Path::new(fallback_path);
        if path.exists() {
            font_bytes.push(
                std::fs::read(path)
                    .map_err(|error| GpuBootstrapError::SmokeReadback(error.to_string()))?,
            );
        }
    }
    Ok(font_bytes)
}
