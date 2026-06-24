use std::path::{Path, PathBuf};

use crate::config::DEFAULT_FONT_FAMILY;
use crate::font::RasterizedGlyphCache;

use super::NativeAppError;

/// Build a native glyph cache from a configured font path or the default system monospace font.
pub fn load_native_glyph_cache(font_family: &str) -> Result<RasterizedGlyphCache, NativeAppError> {
    let font_family = font_family.trim();
    if let Some(path) = configured_font_file_path(font_family)? {
        return load_glyph_cache_from_primary_font(path);
    }
    if is_default_font_family(font_family) {
        return load_default_native_glyph_cache();
    }
    if let Some(path) = resolve_named_font_file_path(font_family) {
        return load_glyph_cache_from_primary_font(&path);
    }
    Err(NativeAppError::Runtime(format!(
        "configured font family is not installed or supported by name: {font_family}; use an explicit font file path"
    )))
}

/// Build the default native glyph cache from a system monospace font.
pub fn load_default_native_glyph_cache() -> Result<RasterizedGlyphCache, NativeAppError> {
    if let Some(path) = first_existing_font_path(default_monospace_font_candidate_paths()) {
        return load_glyph_cache_from_primary_font(&path);
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
    font_family.is_empty() || font_family.eq_ignore_ascii_case(DEFAULT_FONT_FAMILY)
}

fn resolve_named_font_file_path(font_family: &str) -> Option<PathBuf> {
    first_existing_font_path(named_font_candidate_paths(font_family)?)
}

fn first_existing_font_path(candidates: Vec<PathBuf>) -> Option<PathBuf> {
    candidates.into_iter().find(|path| path.exists())
}

fn default_monospace_font_candidate_paths() -> Vec<PathBuf> {
    let mut candidates = font_search_candidates(DEFAULT_PREFERRED_MONO_FONT_FILES);
    candidates.extend(DEFAULT_MONOSPACE_FONT_CANDIDATES.iter().map(PathBuf::from));
    candidates
}

fn named_font_candidate_paths(font_family: &str) -> Option<Vec<PathBuf>> {
    let files = match normalized_font_family_name(font_family).as_str() {
        "jetbrainsmono" | "jetbrainsmononerdfont" => JETBRAINS_MONO_FONT_FILES,
        "cascadiamono" | "caskaydiacovenerdfont" => CASCADIA_MONO_FONT_FILES,
        "iosevkaterm" | "iosevka" => IOSEVKA_TERM_FONT_FILES,
        "sfmono" => SF_MONO_FONT_FILES,
        "menlo" => MENLO_FONT_FILES,
        _ => return None,
    };
    Some(font_search_candidates(files))
}

fn normalized_font_family_name(font_family: &str) -> String {
    font_family
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn font_search_candidates(file_names: &[&str]) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    for root in font_search_roots() {
        for file_name in file_names {
            candidates.push(root.join(file_name));
        }
    }
    candidates
}

fn font_search_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(PathBuf::from(home).join("Library/Fonts"));
    }
    roots.extend([
        PathBuf::from("/Library/Fonts"),
        PathBuf::from("/System/Library/Fonts"),
        PathBuf::from("/opt/homebrew/share/fonts"),
        PathBuf::from("/usr/local/share/fonts"),
        PathBuf::from("/usr/share/fonts/truetype"),
        PathBuf::from("/usr/share/fonts/opentype"),
    ]);
    roots
}

fn load_glyph_cache_from_primary_font(path: &Path) -> Result<RasterizedGlyphCache, NativeAppError> {
    let mut font_bytes =
        vec![std::fs::read(path).map_err(|error| NativeAppError::Runtime(error.to_string()))?];
    for fallback_path in DEFAULT_FALLBACK_FONT_CANDIDATES
        .iter()
        .map(Path::new)
        .filter(|path| path.exists())
    {
        font_bytes.push(
            std::fs::read(fallback_path)
                .map_err(|error| NativeAppError::Runtime(error.to_string()))?,
        );
    }
    RasterizedGlyphCache::from_font_bytes(font_bytes).map_err(NativeAppError::from)
}

const DEFAULT_PREFERRED_MONO_FONT_FILES: &[&str] = &[
    "JetBrainsMonoNerdFont-Regular.ttf",
    "JetBrainsMonoNLNerdFont-Regular.ttf",
    "JetBrainsMono-Regular.ttf",
    "CaskaydiaCoveNerdFont-Regular.ttf",
    "CascadiaMono.ttf",
    "IosevkaTerm-Regular.ttf",
    "SFNSMono.ttf",
    "Menlo.ttc",
];

const DEFAULT_MONOSPACE_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/SFNSMono.ttf",
    "/System/Library/Fonts/Menlo.ttc",
    "/System/Library/Fonts/Supplemental/Courier New.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSansMono.ttf",
    "/usr/share/fonts/dejavu-sans-fonts/DejaVuSansMono.ttf",
    "/usr/share/fonts/truetype/liberation2/LiberationMono-Regular.ttf",
    "/usr/share/fonts/liberation/LiberationMono-Regular.ttf",
    "/usr/share/fonts/truetype/noto/NotoSansMono-Regular.ttf",
];

const DEFAULT_FALLBACK_FONT_CANDIDATES: &[&str] = &[
    "/System/Library/Fonts/Apple Color Emoji.ttc",
    "/usr/share/fonts/truetype/noto/NotoColorEmoji.ttf",
];

const JETBRAINS_MONO_FONT_FILES: &[&str] = &[
    "JetBrainsMonoNerdFont-Regular.ttf",
    "JetBrainsMonoNLNerdFont-Regular.ttf",
    "JetBrainsMono-Regular.ttf",
];

const CASCADIA_MONO_FONT_FILES: &[&str] = &[
    "CaskaydiaCoveNerdFont-Regular.ttf",
    "CascadiaMono.ttf",
    "CascadiaCode.ttf",
];

const IOSEVKA_TERM_FONT_FILES: &[&str] = &[
    "IosevkaTerm-Regular.ttf",
    "IosevkaTermNerdFont-Regular.ttf",
    "Iosevka-Regular.ttc",
];

const SF_MONO_FONT_FILES: &[&str] = &["SFNSMono.ttf", "SFNSMonoItalic.ttf"];

const MENLO_FONT_FILES: &[&str] = &["Menlo.ttc"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_font_stack_prefers_polished_user_fonts_before_system_fallbacks() {
        let candidates = default_monospace_font_candidate_paths();
        let names = candidates
            .iter()
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy())
            .collect::<Vec<_>>();

        let jetbrains_index = names
            .iter()
            .position(|name| name == "JetBrainsMonoNerdFont-Regular.ttf")
            .unwrap();
        let sf_mono_index = names
            .iter()
            .position(|name| name == "SFNSMono.ttf")
            .unwrap();

        assert!(jetbrains_index < sf_mono_index);
    }

    #[test]
    fn named_font_resolution_normalizes_common_family_names() {
        let candidates = named_font_candidate_paths("JetBrains Mono Nerd Font").unwrap();
        let names = candidates
            .iter()
            .filter_map(|path| path.file_name())
            .map(|name| name.to_string_lossy())
            .collect::<Vec<_>>();

        assert!(names.contains(&"JetBrainsMonoNerdFont-Regular.ttf".into()));
        assert!(named_font_candidate_paths("Unmapped Mono").is_none());
    }
}
