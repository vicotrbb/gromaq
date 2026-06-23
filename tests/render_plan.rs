use gromaq::renderer::{GlyphAtlas, GlyphAtlasConfig, RenderPlanner};
use gromaq::{Color, Style, Terminal, TerminalConfig};

#[test]
fn render_plan_contains_dirty_glyphs_with_atlas_entries() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("\x1b[31mA界").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(14);

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    assert_eq!(plan.viewport_cols, 8);
    assert_eq!(plan.viewport_rows, 3);
    assert_eq!(plan.cursor, terminal.dump_cursor());
    assert_eq!(plan.clear_regions, dirty);
    assert_eq!(plan.glyphs.len(), 2);
    assert_eq!(plan.glyphs[0].row, 0);
    assert_eq!(plan.glyphs[0].col, 0);
    assert_eq!(plan.glyphs[0].ch, 'A');
    assert_eq!(
        plan.glyphs[0].style,
        Style {
            foreground: Color::Ansi(1),
            ..Style::default()
        }
    );
    assert_eq!(plan.glyphs[1].row, 0);
    assert_eq!(plan.glyphs[1].col, 1);
    assert_eq!(plan.glyphs[1].ch, '界');
    assert!(plan.glyphs[1].is_wide);
    assert_eq!(atlas.metrics().entries, 2);
}

#[test]
fn render_plan_preserves_multi_codepoint_cell_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("👨\u{200d}👩").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(14);

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    assert_eq!(plan.glyphs.len(), 1);
    assert_eq!(plan.glyphs[0].text, "👨\u{200d}👩");
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_allocates_distinct_atlas_entries_for_different_cell_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("AA\u{0301}").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(14);

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    assert_eq!(plan.glyphs.len(), 2);
    assert_eq!(plan.glyphs[0].text, "A");
    assert_eq!(plan.glyphs[1].text, "A\u{0301}");
    assert_ne!(plan.glyphs[0].atlas_entry, plan.glyphs[1].atlas_entry);
    assert_eq!(atlas.metrics().entries, 2);
}

#[test]
fn render_plan_skips_whitespace_glyph_commands() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("A B").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(14);

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

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
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(16);

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    let planned: Vec<(u16, u16, char)> = plan
        .glyphs
        .iter()
        .map(|glyph| (glyph.row, glyph.col, glyph.ch))
        .collect();
    assert_eq!(planned, vec![(0, 2, 'X'), (0, 3, 'Y')]);
    assert_eq!(plan.clear_regions, dirty);
}
