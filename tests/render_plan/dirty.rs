use gromaq::{Terminal, TerminalConfig};

use super::support::{new_atlas, new_planner, plan_frame};

#[test]
fn render_plan_skips_whitespace_glyph_commands() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("A B").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    let planned: Vec<(u16, u16, char)> = plan
        .glyphs
        .iter()
        .map(|glyph| (glyph.row, glyph.col, glyph.ch))
        .collect();
    assert_eq!(planned, vec![(0, 0, 'A'), (0, 2, 'B')]);
    assert_eq!(plan.clear_regions, dirty);
    assert_eq!(atlas.metrics().entries, 2);
}

#[test]
fn render_plan_limits_work_to_dirty_regions() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("abcdef").unwrap();
    terminal.take_dirty_regions();
    terminal.write_str("\r\x1b[2CXY").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(16);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    let planned: Vec<(u16, u16, char)> = plan
        .glyphs
        .iter()
        .map(|glyph| (glyph.row, glyph.col, glyph.ch))
        .collect();
    assert_eq!(planned, vec![(0, 2, 'X'), (0, 3, 'Y')]);
    assert_eq!(plan.clear_regions, dirty);
}
