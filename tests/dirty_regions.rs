use gromaq::{DirtyRegion, DirtyTracker, Terminal, TerminalConfig};
use proptest::prelude::*;

#[test]
fn printable_run_produces_single_dirty_region() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal.write_str("abc").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 0);
    assert_eq!(regions[0].col, 0);
    assert_eq!(regions[0].rows, 1);
    assert_eq!(regions[0].cols, 3);
    assert!(terminal.take_dirty_regions().is_empty());
}

#[test]
fn erase_line_marks_cleared_span_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("abcdef").unwrap();
    terminal.take_dirty_regions();

    terminal.write_str("\r\x1b[K").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 0);
    assert_eq!(regions[0].col, 0);
    assert_eq!(regions[0].rows, 1);
    assert_eq!(regions[0].cols, 8);
}

#[test]
fn erase_character_marks_repaired_wide_cell_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());
    terminal.write_str("A界B").unwrap();
    terminal.take_dirty_regions();

    terminal.write_str("\x1b[1;3H\x1b[X").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 0);
    assert_eq!(regions[0].col, 1);
    assert_eq!(regions[0].rows, 1);
    assert_eq!(regions[0].cols, 2);
}

#[test]
fn delete_character_marks_repaired_wide_cell_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());
    terminal.write_str("A界B").unwrap();
    terminal.take_dirty_regions();

    terminal.write_str("\x1b[1;3H\x1b[P").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 0);
    assert_eq!(regions[0].col, 1);
    assert_eq!(regions[0].rows, 1);
    assert_eq!(regions[0].cols, 5);
}

#[test]
fn insert_character_marks_repaired_wide_cell_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 2).unwrap());
    terminal.write_str("A界B").unwrap();
    terminal.take_dirty_regions();

    terminal.write_str("\x1b[1;3H\x1b[@").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 0);
    assert_eq!(regions[0].col, 1);
    assert_eq!(regions[0].rows, 1);
    assert_eq!(regions[0].cols, 5);
}

#[test]
fn resize_marks_entire_new_viewport_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("abc").unwrap();
    terminal.take_dirty_regions();

    terminal.resize(10, 4).unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 0);
    assert_eq!(regions[0].col, 0);
    assert_eq!(regions[0].rows, 4);
    assert_eq!(regions[0].cols, 10);
}

#[test]
fn linefeed_at_scroll_bottom_marks_entire_viewport_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 3).unwrap());
    terminal.write_str("\x1b[3;1H").unwrap();
    terminal.take_dirty_regions();

    terminal.write_str("\n").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 0);
    assert_eq!(regions[0].col, 0);
    assert_eq!(regions[0].rows, 3);
    assert_eq!(regions[0].cols, 6);
    assert_eq!(terminal.dump_perf_metrics().scrolls, 1);
}

#[test]
fn linefeed_at_scroll_margin_bottom_marks_only_scroll_region_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 4).unwrap());
    terminal.write_str("\x1b[2;3r\x1b[3;1H").unwrap();
    terminal.take_dirty_regions();

    terminal.write_str("\n").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 1);
    assert_eq!(regions[0].col, 0);
    assert_eq!(regions[0].rows, 2);
    assert_eq!(regions[0].cols, 6);
    assert_eq!(terminal.dump_perf_metrics().scrolls, 1);
}

#[test]
fn reverse_index_at_scroll_margin_top_marks_only_scroll_region_dirty() {
    let mut terminal = Terminal::new(TerminalConfig::new(6, 4).unwrap());
    terminal.write_str("\x1b[2;3r\x1b[2;1H").unwrap();
    terminal.take_dirty_regions();

    terminal.write_str("\x1bM").unwrap();

    let regions = terminal.take_dirty_regions();
    assert_eq!(regions.len(), 1);
    assert_eq!(regions[0].row, 1);
    assert_eq!(regions[0].col, 0);
    assert_eq!(regions[0].rows, 2);
    assert_eq!(regions[0].cols, 6);
    assert_eq!(terminal.dump_perf_metrics().scrolls, 1);
}

#[test]
fn dirty_tracker_contains_regions_at_u16_edges_without_overflow() {
    let mut dirty = DirtyTracker::default();
    let edge = DirtyRegion {
        row: u16::MAX,
        col: u16::MAX,
        rows: 1,
        cols: 1,
    };

    dirty.mark_region(edge);

    assert!(dirty.contains_region(edge));
}

#[test]
fn dirty_tracker_unions_adjacent_edge_regions_with_widened_bounds() {
    let mut dirty = DirtyTracker::default();
    dirty.mark_region(DirtyRegion {
        row: u16::MAX - 1,
        col: u16::MAX - 1,
        rows: 1,
        cols: 1,
    });
    dirty.mark_region(DirtyRegion {
        row: u16::MAX,
        col: u16::MAX,
        rows: 1,
        cols: 1,
    });

    let regions = dirty.take();

    assert_eq!(
        regions,
        vec![DirtyRegion {
            row: u16::MAX - 1,
            col: u16::MAX - 1,
            rows: 2,
            cols: 2,
        }]
    );
}

proptest! {
    #[test]
    fn dirty_tracker_coalesces_marked_regions_into_exact_covering_union(
        regions in prop::collection::vec((0u16..200, 0u16..200, 1u16..20, 1u16..20), 1..64)
    ) {
        let mut dirty = DirtyTracker::default();
        let mut expected_row_start = u16::MAX;
        let mut expected_col_start = u16::MAX;
        let mut expected_row_end = 0u32;
        let mut expected_col_end = 0u32;

        for (row, col, rows, cols) in &regions {
            dirty.mark_region(DirtyRegion {
                row: *row,
                col: *col,
                rows: *rows,
                cols: *cols,
            });
            expected_row_start = expected_row_start.min(*row);
            expected_col_start = expected_col_start.min(*col);
            expected_row_end = expected_row_end.max(u32::from(*row) + u32::from(*rows));
            expected_col_end = expected_col_end.max(u32::from(*col) + u32::from(*cols));
        }

        let expected = DirtyRegion {
            row: expected_row_start,
            col: expected_col_start,
            rows: (expected_row_end - u32::from(expected_row_start)) as u16,
            cols: (expected_col_end - u32::from(expected_col_start)) as u16,
        };

        for (row, col, rows, cols) in &regions {
            let region = DirtyRegion {
                row: *row,
                col: *col,
                rows: *rows,
                cols: *cols,
            };
            prop_assert!(dirty.contains_region(region));
        }

        let drained = dirty.take();
        prop_assert_eq!(drained, vec![expected]);
        prop_assert!(dirty.take().is_empty());
    }
}
