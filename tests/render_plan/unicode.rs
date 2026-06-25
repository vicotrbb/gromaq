use gromaq::{Terminal, TerminalConfig};

use super::support::{new_atlas, new_planner, plan_frame};

#[test]
fn render_plan_preserves_multi_codepoint_cell_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("👨\u{200d}👩").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(plan.glyphs[0].text, "👨\u{200d}👩");
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_preserves_modifier_on_zwj_joined_component_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("👨\u{200d}👩🏽").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(plan.glyphs[0].text, "👨\u{200d}👩🏽");
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_preserves_multi_part_zwj_sequence_with_multiple_modifiers() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("👨🏽\u{200d}👩🏾\u{200d}👧🏼").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(plan.glyphs[0].text, "👨🏽\u{200d}👩🏾\u{200d}👧🏼");
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_preserves_zwj_sequence_with_internal_emoji_variation_selector() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨")
        .unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(
        plan.glyphs[0].text,
        "👩\u{200d}❤\u{fe0f}\u{200d}💋\u{200d}👨"
    );
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_preserves_rainbow_flag_zwj_sequence_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("🏳️\u{200d}🌈").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(plan.glyphs[0].text, "🏳️\u{200d}🌈");
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_preserves_tag_sequence_emoji_flag_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}")
        .unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(
        plan.glyphs[0].text,
        "🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f}"
    );
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_allocates_distinct_atlas_entries_for_different_cell_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("AA\u{0301}").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 2);
    assert_eq!(plan.glyphs[0].text, "A");
    assert_eq!(plan.glyphs[1].text, "A\u{0301}");
    assert_ne!(plan.glyphs[0].atlas_entry, plan.glyphs[1].atlas_entry);
    assert_eq!(atlas.metrics().entries, 2);
}

#[test]
fn render_plan_preserves_stacked_combining_mark_cell_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("A\u{0301}\u{0302}B").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

    assert_eq!(plan.glyphs.len(), 2);
    assert_eq!(plan.glyphs[0].text, "A\u{0301}\u{0302}");
    assert_eq!(plan.glyphs[1].text, "B");
    assert_eq!(atlas.metrics().entries, 2);
}
