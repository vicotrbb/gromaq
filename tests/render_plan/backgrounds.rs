use gromaq::renderer::RenderPlanner;
use gromaq::{Color, DEFAULT_DIM_OPACITY, SelectionRange, Style, Terminal, TerminalConfig};

use super::support::{new_atlas, new_planner, plan_frame};

#[test]
fn render_plan_contains_dirty_glyphs_with_atlas_entries() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal.write_str("\x1b[31mA界").unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

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
fn render_plan_collects_styled_background_fills_for_dirty_cells() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());
    terminal
        .write_str("\x1b[48:2:1:2:3mAB \x1b[0mC\x1b[31;7mD")
        .unwrap();
    let dirty = terminal.take_dirty_regions();
    let mut atlas = new_atlas();
    let mut planner = new_planner(14);

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

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
    let mut atlas = new_atlas();
    let mut planner = RenderPlanner::with_visual_theme(
        14,
        [240, 240, 240],
        [[0, 0, 0]; 16],
        [9, 8, 7, 255],
        DEFAULT_DIM_OPACITY,
    );

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

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
    let mut atlas = new_atlas();
    let mut planner = RenderPlanner::with_visual_theme(
        14,
        [240, 240, 240],
        [[0, 0, 0]; 16],
        [9, 8, 7, 255],
        DEFAULT_DIM_OPACITY,
    );

    let plan = plan_frame(&terminal, &dirty, &mut atlas, &mut planner);

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
