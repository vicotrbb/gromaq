use std::collections::{HashMap, VecDeque};

use crate::error::{GromaqError, Result};

const MAX_GLYPH_ATLAS_CAPACITY: usize = 65_536;

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
    use crate::cell::Style;

    use super::*;

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
