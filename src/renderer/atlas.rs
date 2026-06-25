use std::collections::{HashMap, VecDeque};

use crate::error::{GromaqError, Result};

const MAX_GLYPH_ATLAS_CAPACITY: usize = 65_536;

mod cache;
mod image;
mod key;
pub use image::{GlyphAtlasImage, GlyphBitmap, GlyphImageError};
pub use key::{GlyphKey, GlyphKeyText};

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

/// Glyph atlas entry handle.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GlyphEntry {
    /// Stable slot index inside the atlas.
    pub slot: u32,
    /// Generation increments whenever a slot is reused.
    pub generation: u64,
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
