//! Visible-grid text selection.

/// A zero-based visible-grid position.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct SelectionPoint {
    /// Row.
    pub row: u16,
    /// Column.
    pub col: u16,
}

impl From<(u16, u16)> for SelectionPoint {
    fn from((row, col): (u16, u16)) -> Self {
        Self { row, col }
    }
}

/// Inclusive normalized text selection over the visible grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectionRange {
    /// Start point after normalization.
    pub start: SelectionPoint,
    /// End point after normalization.
    pub end: SelectionPoint,
}

impl SelectionRange {
    /// Create a normalized selection range.
    pub fn new(start: (u16, u16), end: (u16, u16)) -> Self {
        let start = SelectionPoint::from(start);
        let end = SelectionPoint::from(end);
        let (start, end) = if start <= end {
            (start, end)
        } else {
            (end, start)
        };
        Self { start, end }
    }
}
