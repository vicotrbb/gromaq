use gromaq::renderer::{GlyphAtlas, GlyphAtlasConfig, RenderPlanner};
use gromaq::{Color, DEFAULT_DIM_OPACITY, SelectionRange, Style, Terminal, TerminalConfig};

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
fn render_plan_collects_styled_background_fills_for_dirty_cells() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("\x1b[48:2:1:2:3mAB \x1b[0mC\x1b[31;7mD")
        .unwrap();
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

    assert_eq!(plan.backgrounds.len(), 2);
    assert_eq!(plan.backgrounds[0].row, 0);
    assert_eq!(plan.backgrounds[0].col, 0);
    assert_eq!(plan.backgrounds[0].cols, 3);
    assert_eq!(plan.backgrounds[0].color_rgba8, [1, 2, 3, 255]);
    assert_eq!(plan.backgrounds[1].row, 0);
    assert_eq!(plan.backgrounds[1].col, 4);
    assert_eq!(plan.backgrounds[1].cols, 1);
    assert_eq!(plan.backgrounds[1].color_rgba8, [255, 107, 122, 255]);
}

#[test]
fn render_plan_collects_themed_selection_backgrounds_for_dirty_cells() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("abcd\r\nefgh").unwrap();
    terminal.take_dirty_regions();
    terminal.set_selection(SelectionRange::new((0, 1), (1, 2)));
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::with_visual_theme(
        14,
        [240, 240, 240],
        [[0, 0, 0]; 16],
        [9, 8, 7, 255],
        DEFAULT_DIM_OPACITY,
    );

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    assert_eq!(plan.backgrounds.len(), 2);
    assert_eq!(plan.backgrounds[0].row, 0);
    assert_eq!(plan.backgrounds[0].col, 1);
    assert_eq!(plan.backgrounds[0].cols, 7);
    assert_eq!(plan.backgrounds[0].color_rgba8, [9, 8, 7, 255]);
    assert_eq!(plan.backgrounds[1].row, 1);
    assert_eq!(plan.backgrounds[1].col, 0);
    assert_eq!(plan.backgrounds[1].cols, 3);
    assert_eq!(plan.backgrounds[1].color_rgba8, [9, 8, 7, 255]);
}

#[test]
fn render_plan_selection_background_overrides_existing_cell_background() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    terminal.write_str("\x1b[48;2;1;2;3mabcd").unwrap();
    terminal.take_dirty_regions();
    terminal.set_selection(SelectionRange::new((0, 1), (0, 2)));
    let dirty = terminal.take_dirty_regions();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::with_visual_theme(
        14,
        [240, 240, 240],
        [[0, 0, 0]; 16],
        [9, 8, 7, 255],
        DEFAULT_DIM_OPACITY,
    );

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    assert_eq!(plan.backgrounds.len(), 3);
    assert_eq!(plan.backgrounds[0].col, 0);
    assert_eq!(plan.backgrounds[0].cols, 1);
    assert_eq!(plan.backgrounds[0].color_rgba8, [1, 2, 3, 255]);
    assert_eq!(plan.backgrounds[1].col, 1);
    assert_eq!(plan.backgrounds[1].cols, 2);
    assert_eq!(plan.backgrounds[1].color_rgba8, [9, 8, 7, 255]);
    assert_eq!(plan.backgrounds[2].col, 3);
    assert_eq!(plan.backgrounds[2].cols, 1);
    assert_eq!(plan.backgrounds[2].color_rgba8, [1, 2, 3, 255]);
}

#[test]
fn render_plan_preserves_modifier_on_zwj_joined_component_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("👨\u{200d}👩🏽").unwrap();
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
    assert_eq!(plan.glyphs[0].text, "👨\u{200d}👩🏽");
    assert!(plan.glyphs[0].is_wide);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn render_plan_preserves_multi_part_zwj_sequence_with_multiple_modifiers() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("👨🏽\u{200d}👩🏾\u{200d}👧🏼").unwrap();
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
fn render_plan_preserves_stacked_combining_mark_cell_text() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("A\u{0301}\u{0302}B").unwrap();
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
    assert_eq!(plan.glyphs[0].text, "A\u{0301}\u{0302}");
    assert_eq!(plan.glyphs[1].text, "B");
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
