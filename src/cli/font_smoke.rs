//! Native font fallback smoke commands.

use super::CliExit;
use crate::app::load_default_native_glyph_cache;
use crate::renderer::{GlyphAtlas, GlyphAtlasConfig, RenderPlanner};
use crate::terminal::{Terminal, TerminalConfig};

const SYMBOL_FALLBACK_SAMPLE: &str = "\u{28ff}";

pub(super) fn font_symbol_fallback_smoke_exit() -> CliExit {
    match run_font_symbol_fallback_smoke() {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "font symbol fallback smoke: ok\nsample: {}\nglyphs planned: {}\nglyphs rasterized: {}\nbitmap: {}x{}\nalpha pixels: {}\n",
                SYMBOL_FALLBACK_SAMPLE,
                report.planned_glyphs,
                report.rasterized_glyphs,
                report.bitmap_width,
                report.bitmap_height,
                report.alpha_pixels
            ),
            stderr: String::new(),
        },
        Err(message) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("font symbol fallback smoke failed: {message}\n"),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FontSymbolFallbackSmokeReport {
    planned_glyphs: usize,
    rasterized_glyphs: usize,
    bitmap_width: u32,
    bitmap_height: u32,
    alpha_pixels: usize,
}

fn run_font_symbol_fallback_smoke() -> Result<FontSymbolFallbackSmokeReport, String> {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).map_err(|error| error.to_string())?);
    terminal
        .write_str(SYMBOL_FALLBACK_SAMPLE)
        .map_err(|error| error.to_string())?;
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(4).map_err(|error| error.to_string())?);
    let mut planner = RenderPlanner::new(24);
    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .map_err(|error| error.to_string())?;
    if plan.glyphs.len() != 1 {
        return Err(format!(
            "expected exactly one planned symbol glyph, got {}",
            plan.glyphs.len()
        ));
    }
    let planned = &plan.glyphs[0];
    if planned.text != SYMBOL_FALLBACK_SAMPLE {
        return Err(format!(
            "expected planned glyph {SYMBOL_FALLBACK_SAMPLE:?}, got {:?}",
            planned.text
        ));
    }

    let mut cache = load_default_native_glyph_cache().map_err(|error| error.to_string())?;
    let batch = cache
        .rasterize_plan(&plan)
        .map_err(|error| error.to_string())?;
    if batch.rasterized != 1 {
        return Err(format!(
            "expected one rasterized fallback glyph, got {}",
            batch.rasterized
        ));
    }
    let Some(bitmap) = batch.bitmaps.first() else {
        return Err("rasterized batch did not include a bitmap".to_owned());
    };
    if bitmap.width == 0 || bitmap.height == 0 {
        return Err(format!(
            "rasterized bitmap had invalid dimensions {}x{}",
            bitmap.width, bitmap.height
        ));
    }
    let alpha_pixels = bitmap
        .rgba
        .chunks_exact(4)
        .filter(|pixel| pixel[3] > 0)
        .count();
    if alpha_pixels == 0 {
        return Err("rasterized bitmap contained no visible alpha pixels".to_owned());
    }

    Ok(FontSymbolFallbackSmokeReport {
        planned_glyphs: plan.glyphs.len(),
        rasterized_glyphs: batch.rasterized,
        bitmap_width: bitmap.width,
        bitmap_height: bitmap.height,
        alpha_pixels,
    })
}
