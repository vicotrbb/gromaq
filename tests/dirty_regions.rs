use gromaq::{DirtyRegion, DirtyTracker, Terminal, TerminalConfig};

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
