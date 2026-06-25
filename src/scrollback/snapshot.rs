use crate::cell::{CellSnapshot, Color};

pub(super) fn logical_line_ids_for(hard_breaks: &[bool]) -> Vec<usize> {
    let mut current_id = 0;
    let mut ids = Vec::with_capacity(hard_breaks.len());
    for hard_break in hard_breaks {
        ids.push(current_id);
        if *hard_break {
            current_id += 1;
        }
    }
    ids
}

/// Immutable scrollback snapshot used by tests and debug tooling.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScrollbackSnapshot {
    /// Scrollback lines from oldest to newest.
    pub lines: Vec<String>,
    /// Whether each scrollback row ended a hard line break instead of a soft wrap.
    pub hard_breaks: Vec<bool>,
    /// Stable logical-line group for each retained physical scrollback row.
    pub logical_line_ids: Vec<usize>,
    /// OSC 8 hyperlink URI table indexed by non-zero cell hyperlink identifiers.
    pub hyperlinks: Vec<String>,
    /// Underline color table indexed by non-zero style underline color identifiers.
    pub underline_colors: Vec<Color>,
    /// Styled scrollback cells from oldest to newest row.
    pub cells: Vec<Vec<CellSnapshot>>,
}
