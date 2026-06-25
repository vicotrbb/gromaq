use std::path::{Path, PathBuf};

use crate::config::DEFAULT_FONT_FAMILY;
use crate::font::RasterizedGlyphCache;

use super::NativeAppError;

mod search;

use search::{
    default_monospace_font_candidate_paths, fallback_font_paths, first_existing_font_path,
    named_font_candidate_paths,
};

/// Resolved native font files used for glyph rasterization.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NativeFontResolution {
    /// Primary mono font file used for ordinary terminal glyphs.
    pub primary_path: PathBuf,
    /// Existing fallback font files used for symbols and emoji.
    pub fallback_paths: Vec<PathBuf>,
}

/// Build a native glyph cache from a configured font path or the default system monospace font.
pub fn load_native_glyph_cache(font_family: &str) -> Result<RasterizedGlyphCache, NativeAppError> {
    let resolution = resolve_native_font_paths(font_family)?;
    load_glyph_cache_from_resolution(&resolution)
}

/// Resolve the configured native font family or file path without loading font bytes.
pub fn resolve_native_font_paths(
    font_family: &str,
) -> Result<NativeFontResolution, NativeAppError> {
    let font_family = font_family.trim();
    if let Some(path) = configured_font_file_path(font_family)? {
        return Ok(NativeFontResolution {
            primary_path: path.to_path_buf(),
            fallback_paths: fallback_font_paths(),
        });
    }
    if is_default_font_family(font_family) {
        return default_native_font_resolution();
    }
    if let Some(path) = resolve_named_font_file_path(font_family) {
        return Ok(NativeFontResolution {
            primary_path: path,
            fallback_paths: fallback_font_paths(),
        });
    }
    Err(NativeAppError::Runtime(format!(
        "configured font family is not installed or supported by name: {font_family}; use an explicit font file path"
    )))
}

/// Build the default native glyph cache from a system monospace font.
pub fn load_default_native_glyph_cache() -> Result<RasterizedGlyphCache, NativeAppError> {
    let resolution = default_native_font_resolution()?;
    load_glyph_cache_from_resolution(&resolution)
}

fn default_native_font_resolution() -> Result<NativeFontResolution, NativeAppError> {
    if let Some(path) = first_existing_font_path(default_monospace_font_candidate_paths()) {
        return Ok(NativeFontResolution {
            primary_path: path,
            fallback_paths: fallback_font_paths(),
        });
    }
    Err(NativeAppError::Runtime(
        "no default monospace system font found".to_owned(),
    ))
}

fn configured_font_file_path(font_family: &str) -> Result<Option<&Path>, NativeAppError> {
    let path = Path::new(font_family);
    if path.is_file() {
        return Ok(Some(path));
    }
    if path.is_absolute() || font_family.contains('/') || font_family.contains('\\') {
        return Err(NativeAppError::Runtime(format!(
            "configured font file does not exist: {font_family}"
        )));
    }
    Ok(None)
}

fn is_default_font_family(font_family: &str) -> bool {
    font_family.is_empty()
        || font_family.eq_ignore_ascii_case(DEFAULT_FONT_FAMILY)
        || font_family.eq_ignore_ascii_case("monospace")
}

fn resolve_named_font_file_path(font_family: &str) -> Option<PathBuf> {
    first_existing_font_path(named_font_candidate_paths(font_family)?)
}

fn load_glyph_cache_from_resolution(
    resolution: &NativeFontResolution,
) -> Result<RasterizedGlyphCache, NativeAppError> {
    let mut font_bytes = vec![
        std::fs::read(&resolution.primary_path)
            .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
    ];
    for fallback_path in &resolution.fallback_paths {
        font_bytes.push(
            std::fs::read(fallback_path)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
        );
    }
    RasterizedGlyphCache::from_font_bytes(font_bytes).map_err(NativeAppError::from)
}
