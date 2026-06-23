use std::collections::{HashMap, VecDeque};

use thiserror::Error;

use crate::cell::{Color, Style, UnderlineStyle};
use crate::error::{GromaqError, Result};

const MAX_GLYPH_ATLAS_CAPACITY: usize = 65_536;

/// Glyph atlas configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GlyphAtlasConfig {
    capacity: usize,
}

impl GlyphAtlasConfig {
    /// Create a glyph atlas configuration.
    pub fn new(capacity: usize) -> Result<Self> {
        if capacity == 0 || capacity > MAX_GLYPH_ATLAS_CAPACITY {
            return Err(GromaqError::InvalidGlyphAtlasCapacity {
                minimum: 1,
                maximum: MAX_GLYPH_ATLAS_CAPACITY,
                actual: capacity,
            });
        }
        Ok(Self { capacity })
    }

    /// Maximum cached glyph entries.
    pub fn capacity(&self) -> usize {
        self.capacity
    }
}

/// Stable glyph cache text identity.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum GlyphKeyText {
    /// A single scalar value.
    Scalar(char),
    /// A multi-scalar terminal cell text cluster.
    Cluster(String),
}

/// Stable glyph cache key.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct GlyphKey {
    /// Text to render.
    pub text: GlyphKeyText,
    /// Cell style.
    pub style: Style,
    /// Font size in pixels.
    pub font_size_px: u16,
}

impl GlyphKey {
    /// Build a glyph cache key.
    pub fn new(ch: char, style: Style, font_size_px: u16) -> Self {
        Self {
            text: GlyphKeyText::Scalar(ch),
            style: glyph_raster_style(style),
            font_size_px,
        }
    }

    /// Build a glyph cache key for a full terminal cell text cluster.
    pub fn for_text(text: &str, first_char: char, style: Style, font_size_px: u16) -> Self {
        if text.len() == first_char.len_utf8() {
            Self::new(first_char, style, font_size_px)
        } else {
            Self {
                text: GlyphKeyText::Cluster(text.to_owned()),
                style: glyph_raster_style(style),
                font_size_px,
            }
        }
    }
}

fn glyph_raster_style(style: Style) -> Style {
    Style {
        foreground: Color::Default,
        background: Color::Default,
        dim: false,
        underline: false,
        underline_style: UnderlineStyle::Single,
        underline_color_id: 0,
        blink: false,
        hidden: false,
        inverse: false,
        overline: false,
        strikethrough: false,
        framed: false,
        encircled: false,
        ..style
    }
}

/// Glyph atlas entry handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphEntry {
    /// Stable slot index inside the atlas.
    pub slot: u32,
    /// Generation increments whenever a slot is reused.
    pub generation: u64,
}

/// One rasterized glyph bitmap ready for atlas packing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphBitmap {
    /// Atlas entry this bitmap belongs to.
    pub entry: GlyphEntry,
    /// Bitmap width in pixels.
    pub width: u32,
    /// Bitmap height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
}

/// Errors produced while building or packing dense RGBA8 glyph images.
#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum GlyphImageError {
    /// A solid glyph bitmap could not reserve its pixel buffer.
    #[error("solid glyph bitmap is too large to allocate")]
    SolidGlyphAllocationTooLarge,
    /// A requested RGBA row cannot be represented in memory.
    #[error("rgba row dimensions are too large")]
    RgbaRowDimensionsTooLarge,
    /// A requested RGBA image cannot be represented in memory.
    #[error("rgba image dimensions are too large")]
    RgbaImageDimensionsTooLarge,
    /// A requested RGBA row offset cannot be represented in memory.
    #[error("rgba row offset is too large")]
    RgbaRowOffsetTooLarge,
    /// A requested RGBA image buffer could not be allocated.
    #[error("rgba image buffer is too large to allocate")]
    RgbaBufferAllocationTooLarge,
    /// A requested RGBA pixel offset cannot be represented in memory.
    #[error("rgba image offset is too large")]
    RgbaImageOffsetTooLarge,
    /// The padding target is smaller than the source glyph bitmap.
    #[error(
        "target {target_width}x{target_height} is smaller than glyph {glyph_width}x{glyph_height}"
    )]
    PaddingTargetTooSmall {
        /// Requested padded bitmap width.
        target_width: u32,
        /// Requested padded bitmap height.
        target_height: u32,
        /// Source glyph bitmap width.
        glyph_width: u32,
        /// Source glyph bitmap height.
        glyph_height: u32,
    },
    /// A source glyph bitmap does not contain the expected dense RGBA8 byte length.
    #[error("glyph slot {slot} expected {expected_len} rgba bytes before padding")]
    InvalidPaddingSourceLength {
        /// Atlas slot for the malformed glyph.
        slot: u32,
        /// Expected dense RGBA8 byte length.
        expected_len: usize,
    },
    /// Fixed-size atlas slot dimensions must be non-zero.
    #[error("slot dimensions and columns must be non-zero")]
    InvalidAtlasSlotLayout,
    /// The packed atlas width cannot be represented.
    #[error("glyph atlas width is too large")]
    AtlasWidthTooLarge,
    /// The packed atlas height cannot be represented.
    #[error("glyph atlas height is too large")]
    AtlasHeightTooLarge,
    /// A glyph bitmap does not match the requested fixed atlas slot size.
    #[error("glyph slot {slot} expected {expected_len} rgba bytes for {slot_width}x{slot_height}")]
    InvalidAtlasGlyphSize {
        /// Atlas slot for the malformed glyph.
        slot: u32,
        /// Expected dense RGBA8 byte length.
        expected_len: usize,
        /// Expected slot width.
        slot_width: u32,
        /// Expected slot height.
        slot_height: u32,
    },
    /// The packed atlas row offset cannot be represented.
    #[error("glyph atlas row offset is too large")]
    AtlasRowOffsetTooLarge,
    /// The packed atlas column offset cannot be represented.
    #[error("glyph atlas column offset is too large")]
    AtlasColumnOffsetTooLarge,
}

impl GlyphBitmap {
    /// Try to build a solid RGBA8 glyph bitmap without panicking on oversized dimensions.
    pub fn try_solid_rgba8(
        entry: GlyphEntry,
        width: u32,
        height: u32,
        rgba: [u8; 4],
    ) -> std::result::Result<Self, GlyphImageError> {
        let pixel_count = rgba_pixel_count(width, height)?;
        let mut pixels = Vec::new();
        pixels
            .try_reserve_exact(rgba_byte_len(width, height)?)
            .map_err(|_| GlyphImageError::SolidGlyphAllocationTooLarge)?;
        for _ in 0..pixel_count {
            pixels.extend_from_slice(&rgba);
        }
        Ok(Self {
            entry,
            width,
            height,
            rgba: pixels,
        })
    }

    /// Return this glyph copied into the top-left of a larger transparent bitmap.
    pub fn padded_to(
        &self,
        target_width: u32,
        target_height: u32,
    ) -> std::result::Result<Self, GlyphImageError> {
        if target_width < self.width || target_height < self.height {
            return Err(GlyphImageError::PaddingTargetTooSmall {
                target_width,
                target_height,
                glyph_width: self.width,
                glyph_height: self.height,
            });
        }
        if self.width == target_width && self.height == target_height {
            return Ok(self.clone());
        }

        let source_row_bytes = rgba_row_byte_len(self.width)?;
        let target_row_bytes = rgba_row_byte_len(target_width)?;
        let expected_source_len = rgba_byte_len(self.width, self.height)?;
        if self.rgba.len() != expected_source_len {
            return Err(GlyphImageError::InvalidPaddingSourceLength {
                slot: self.entry.slot,
                expected_len: expected_source_len,
            });
        }

        let source_height = usize::try_from(self.height)
            .map_err(|_| GlyphImageError::RgbaImageDimensionsTooLarge)?;
        let mut rgba = zeroed_rgba_buffer(target_width, target_height)?;
        for row in 0..source_height {
            let source_start = checked_rgba_row_offset(row, source_row_bytes)?;
            let target_start = checked_rgba_row_offset(row, target_row_bytes)?;
            rgba[target_start..target_start + source_row_bytes]
                .copy_from_slice(&self.rgba[source_start..source_start + source_row_bytes]);
        }

        Ok(Self {
            entry: self.entry,
            width: target_width,
            height: target_height,
            rgba,
        })
    }
}

fn rgba_row_byte_len(width: u32) -> std::result::Result<usize, GlyphImageError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| width.checked_mul(4))
        .ok_or(GlyphImageError::RgbaRowDimensionsTooLarge)
}

fn rgba_pixel_count(width: u32, height: u32) -> std::result::Result<usize, GlyphImageError> {
    usize::try_from(width)
        .ok()
        .and_then(|width| {
            usize::try_from(height)
                .ok()
                .and_then(|height| width.checked_mul(height))
        })
        .ok_or(GlyphImageError::RgbaImageDimensionsTooLarge)
}

fn rgba_byte_len(width: u32, height: u32) -> std::result::Result<usize, GlyphImageError> {
    rgba_pixel_count(width, height)?
        .checked_mul(4)
        .ok_or(GlyphImageError::RgbaImageDimensionsTooLarge)
}

fn checked_rgba_row_offset(
    row: usize,
    row_bytes: usize,
) -> std::result::Result<usize, GlyphImageError> {
    row.checked_mul(row_bytes)
        .ok_or(GlyphImageError::RgbaRowOffsetTooLarge)
}

fn zeroed_rgba_buffer(width: u32, height: u32) -> std::result::Result<Vec<u8>, GlyphImageError> {
    let len = rgba_byte_len(width, height)?;
    let mut rgba = Vec::new();
    rgba.try_reserve_exact(len)
        .map_err(|_| GlyphImageError::RgbaBufferAllocationTooLarge)?;
    rgba.resize(len, 0);
    Ok(rgba)
}

fn rgba_offset(width: u32, x: u32, y: u32) -> std::result::Result<usize, GlyphImageError> {
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

/// Packed RGBA8 glyph atlas image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlyphAtlasImage {
    /// Atlas image width in pixels.
    pub width: u32,
    /// Atlas image height in pixels.
    pub height: u32,
    /// Dense RGBA8 pixels in row-major order.
    pub rgba: Vec<u8>,
    /// Number of populated atlas slots.
    pub occupied_slots: usize,
}

impl GlyphAtlasImage {
    /// Pack fixed-size RGBA8 glyph bitmaps into slots.
    pub fn pack_rgba8(
        slot_width: u32,
        slot_height: u32,
        columns: u32,
        glyphs: &[GlyphBitmap],
    ) -> std::result::Result<Self, GlyphImageError> {
        if slot_width == 0 || slot_height == 0 || columns == 0 {
            return Err(GlyphImageError::InvalidAtlasSlotLayout);
        }
        let max_slot = glyphs
            .iter()
            .map(|glyph| glyph.entry.slot)
            .max()
            .unwrap_or(0);
        let rows = (max_slot / columns) + 1;
        let width = slot_width
            .checked_mul(columns)
            .ok_or(GlyphImageError::AtlasWidthTooLarge)?;
        let height = slot_height
            .checked_mul(rows)
            .ok_or(GlyphImageError::AtlasHeightTooLarge)?;
        let mut rgba = zeroed_rgba_buffer(width, height)?;

        for glyph in glyphs {
            let expected_len = rgba_byte_len(slot_width, slot_height)?;
            if glyph.width != slot_width
                || glyph.height != slot_height
                || glyph.rgba.len() != expected_len
            {
                return Err(GlyphImageError::InvalidAtlasGlyphSize {
                    slot: glyph.entry.slot,
                    expected_len,
                    slot_width,
                    slot_height,
                });
            }

            let slot_col = glyph.entry.slot % columns;
            let slot_row = glyph.entry.slot / columns;
            for y in 0..slot_height {
                let atlas_y = slot_row
                    .checked_mul(slot_height)
                    .and_then(|row_start| row_start.checked_add(y))
                    .ok_or(GlyphImageError::AtlasRowOffsetTooLarge)?;
                let atlas_x = slot_col
                    .checked_mul(slot_width)
                    .ok_or(GlyphImageError::AtlasColumnOffsetTooLarge)?;
                let atlas_start = rgba_offset(width, atlas_x, atlas_y)?;
                let glyph_start = rgba_offset(slot_width, 0, y)?;
                let row_bytes = rgba_row_byte_len(slot_width)?;
                rgba[atlas_start..atlas_start + row_bytes]
                    .copy_from_slice(&glyph.rgba[glyph_start..glyph_start + row_bytes]);
            }
        }

        Ok(Self {
            width,
            height,
            rgba,
            occupied_slots: glyphs.len(),
        })
    }

    /// Build a deterministic two-slot atlas image for GPU upload smoke tests.
    pub fn smoke_rgba8() -> std::result::Result<Self, GlyphImageError> {
        let red = GlyphBitmap::try_solid_rgba8(
            GlyphEntry {
                slot: 0,
                generation: 0,
            },
            2,
            2,
            [255, 0, 0, 255],
        )?;
        let green = GlyphBitmap::try_solid_rgba8(
            GlyphEntry {
                slot: 1,
                generation: 0,
            },
            2,
            2,
            [0, 255, 0, 255],
        )?;
        Self::pack_rgba8(2, 2, 2, &[red, green])
    }
}

/// Glyph atlas cache metrics.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct GlyphAtlasMetrics {
    /// Cache hits.
    pub hits: u64,
    /// Cache misses.
    pub misses: u64,
    /// Cache evictions.
    pub evictions: u64,
    /// Current entry count.
    pub entries: usize,
}

#[derive(Debug, Clone, Copy)]
struct GlyphSlot {
    entry: GlyphEntry,
}

/// Deterministic glyph atlas cache.
#[derive(Debug)]
pub struct GlyphAtlas {
    config: GlyphAtlasConfig,
    entries: HashMap<GlyphKey, GlyphEntry>,
    lru: VecDeque<GlyphKey>,
    free_slots: Vec<u32>,
    generations: Vec<u64>,
    metrics: GlyphAtlasMetrics,
}

impl GlyphAtlas {
    /// Create an empty glyph atlas.
    pub fn new(config: GlyphAtlasConfig) -> Self {
        let mut free_slots = Vec::with_capacity(config.capacity());
        for slot in (0..config.capacity()).rev() {
            free_slots.push(slot as u32);
        }
        Self {
            config,
            entries: HashMap::with_capacity(config.capacity()),
            lru: VecDeque::with_capacity(config.capacity()),
            free_slots,
            generations: vec![0; config.capacity()],
            metrics: GlyphAtlasMetrics::default(),
        }
    }

    /// Look up a glyph entry or allocate one.
    pub fn lookup_or_insert(&mut self, key: GlyphKey) -> Result<GlyphEntry> {
        if let Some(entry) = self.entries.get(&key).copied() {
            self.metrics.hits += 1;
            self.touch(key);
            return Ok(entry);
        }

        self.metrics.misses += 1;
        let entry = match self.free_slots.pop() {
            Some(slot) => GlyphEntry {
                slot,
                generation: self.generations[slot as usize],
            },
            None => {
                let evicted = self.evict_lru()?;
                let slot = evicted.entry.slot;
                let generation = evicted.entry.generation + 1;
                self.generations[slot as usize] = generation;
                GlyphEntry { slot, generation }
            }
        };
        self.entries.insert(key.clone(), entry);
        self.lru.push_back(key);
        self.metrics.entries = self.entries.len();
        Ok(entry)
    }

    /// Return glyph atlas metrics.
    pub fn metrics(&self) -> GlyphAtlasMetrics {
        GlyphAtlasMetrics {
            entries: self.entries.len(),
            ..self.metrics
        }
    }

    /// Maximum cached glyph entries.
    pub fn capacity(&self) -> usize {
        self.config.capacity()
    }

    fn touch(&mut self, key: GlyphKey) {
        self.lru.retain(|existing| *existing != key);
        self.lru.push_back(key);
    }

    fn evict_lru(&mut self) -> Result<GlyphSlot> {
        let key = self
            .lru
            .pop_front()
            .ok_or(GromaqError::GlyphAtlasInvariant {
                reason: "glyph atlas full with no LRU key",
            })?;
        let entry = self
            .entries
            .remove(&key)
            .ok_or(GromaqError::GlyphAtlasInvariant {
                reason: "glyph LRU key must exist in entries",
            })?;
        self.metrics.evictions += 1;
        Ok(GlyphSlot { entry })
    }
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

    #[test]
    fn glyph_atlas_eviction_reports_missing_lru_key_invariant() {
        let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(1).unwrap());
        atlas.free_slots.clear();

        let error = atlas.evict_lru().unwrap_err();

        assert_eq!(
            error,
            GromaqError::GlyphAtlasInvariant {
                reason: "glyph atlas full with no LRU key",
            }
        );
    }

    #[test]
    fn glyph_atlas_eviction_reports_lru_entry_map_mismatch() {
        let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(1).unwrap());
        atlas.free_slots.clear();
        atlas
            .lru
            .push_back(GlyphKey::new('A', Style::default(), 14));

        let error = atlas.evict_lru().unwrap_err();

        assert_eq!(
            error,
            GromaqError::GlyphAtlasInvariant {
                reason: "glyph LRU key must exist in entries",
            }
        );
    }
}
