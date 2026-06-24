use crate::dirty::DirtyRegion;
use crate::grid::GridSnapshot;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct ClippedDirtyRegion {
    pub(super) row_start: u16,
    pub(super) row_end: u16,
    pub(super) col_start: u16,
    pub(super) col_end: u16,
}

impl ClippedDirtyRegion {
    pub(super) fn rows(self) -> u16 {
        self.row_end - self.row_start
    }

    pub(super) fn cols(self) -> u16 {
        self.col_end - self.col_start
    }
}

pub(super) fn clipped_dirty_region(
    region: &DirtyRegion,
    grid: &GridSnapshot,
) -> Option<ClippedDirtyRegion> {
    let row_start = region.row.min(grid.rows);
    let col_start = region.col.min(grid.cols);
    let row_end = (u32::from(region.row) + u32::from(region.rows)).min(u32::from(grid.rows));
    let col_end = (u32::from(region.col) + u32::from(region.cols)).min(u32::from(grid.cols));
    let row_end = u16::try_from(row_end).ok()?;
    let col_end = u16::try_from(col_end).ok()?;
    if row_start >= row_end || col_start >= col_end {
        return None;
    }
    Some(ClippedDirtyRegion {
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn empty_grid_snapshot(rows: u16, cols: u16) -> GridSnapshot {
        GridSnapshot {
            rows,
            cols,
            hyperlinks: Vec::new(),
            underline_colors: Vec::new(),
            cells: Vec::new(),
        }
    }

    #[test]
    fn clipped_dirty_region_uses_widened_bounds_at_u16_edges() {
        let grid = empty_grid_snapshot(u16::MAX, u16::MAX);
        let region = DirtyRegion {
            row: u16::MAX - 1,
            col: u16::MAX - 2,
            rows: 8,
            cols: 9,
        };

        assert_eq!(
            clipped_dirty_region(&region, &grid),
            Some(ClippedDirtyRegion {
                row_start: u16::MAX - 1,
                row_end: u16::MAX,
                col_start: u16::MAX - 2,
                col_end: u16::MAX,
            })
        );
    }

    #[test]
    fn clipped_dirty_region_rejects_regions_outside_grid() {
        let grid = empty_grid_snapshot(10, 10);
        let region = DirtyRegion {
            row: 12,
            col: 0,
            rows: 1,
            cols: 1,
        };

        assert_eq!(clipped_dirty_region(&region, &grid), None);
    }
}
