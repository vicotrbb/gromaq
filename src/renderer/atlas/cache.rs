use crate::error::{GromaqError, Result};

use super::{GlyphAtlas, GlyphAtlasConfig, GlyphAtlasMetrics, GlyphEntry, GlyphKey};

#[derive(Debug, Clone, Copy)]
struct GlyphSlot {
    entry: GlyphEntry,
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
            entries: std::collections::HashMap::with_capacity(config.capacity()),
            lru: std::collections::VecDeque::with_capacity(config.capacity()),
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
