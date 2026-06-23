//! Dirty-region tracking for render scheduling.

/// A rectangular terminal-grid region that needs rendering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirtyRegion {
    /// First row.
    pub row: u16,
    /// First column.
    pub col: u16,
    /// Region height in rows.
    pub rows: u16,
    /// Region width in columns.
    pub cols: u16,
}

impl DirtyRegion {
    /// Create a dirty region from inclusive start and exclusive end coordinates.
    pub fn from_bounds(row_start: u16, col_start: u16, row_end: u16, col_end: u16) -> Self {
        Self {
            row: row_start,
            col: col_start,
            rows: row_end.saturating_sub(row_start),
            cols: col_end.saturating_sub(col_start),
        }
    }

    fn union(self, other: Self) -> Self {
        let row_start = self.row.min(other.row);
        let col_start = self.col.min(other.col);
        let row_end = (self.row + self.rows).max(other.row + other.rows);
        let col_end = (self.col + self.cols).max(other.col + other.cols);
        Self::from_bounds(row_start, col_start, row_end, col_end)
    }

    fn contains(self, other: Self) -> bool {
        self.row <= other.row
            && self.col <= other.col
            && self.row + self.rows >= other.row + other.rows
            && self.col + self.cols >= other.col + other.cols
    }
}

/// Coalescing dirty-region tracker.
#[derive(Debug, Default, Clone)]
pub struct DirtyTracker {
    pending: Option<DirtyRegion>,
}

impl DirtyTracker {
    /// Mark a single cell dirty.
    pub fn mark_cell(&mut self, row: u16, col: u16) {
        self.mark_region(DirtyRegion {
            row,
            col,
            rows: 1,
            cols: 1,
        });
    }

    /// Mark a span on one row dirty.
    pub fn mark_span(&mut self, row: u16, col: u16, cols: u16) {
        if cols == 0 {
            return;
        }
        self.mark_region(DirtyRegion {
            row,
            col,
            rows: 1,
            cols,
        });
    }

    /// Mark an entire viewport dirty.
    pub fn mark_viewport(&mut self, rows: u16, cols: u16) {
        self.mark_region(DirtyRegion {
            row: 0,
            col: 0,
            rows,
            cols,
        });
    }

    /// Mark an arbitrary region dirty.
    pub fn mark_region(&mut self, region: DirtyRegion) {
        if region.rows == 0 || region.cols == 0 {
            return;
        }
        self.pending = Some(match self.pending {
            Some(existing) if existing.contains(region) => existing,
            Some(existing) if region.contains(existing) => region,
            Some(existing) => existing.union(region),
            None => region,
        });
    }

    /// Return true when a region is already covered by pending dirty state.
    pub fn contains_region(&self, region: DirtyRegion) -> bool {
        self.pending
            .is_some_and(|existing| existing.contains(region))
    }

    /// Return true when a row span is already covered by pending dirty state.
    pub fn contains_span(&self, row: u16, col: u16, cols: u16) -> bool {
        if cols == 0 {
            return true;
        }
        self.contains_region(DirtyRegion {
            row,
            col,
            rows: 1,
            cols,
        })
    }

    /// Drain pending dirty regions.
    pub fn take(&mut self) -> Vec<DirtyRegion> {
        self.pending.take().into_iter().collect()
    }
}
