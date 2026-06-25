use super::*;
use crate::renderer::GlyphAtlasConfig;
use crate::terminal::{Terminal, TerminalConfig};

#[test]
fn render_planner_ignores_dirty_regions_outside_grid() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());
    terminal.write_str("AB").unwrap();
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let mut planner = RenderPlanner::new(14);
    let dirty = [DirtyRegion {
        row: 4,
        col: 0,
        rows: 1,
        cols: 1,
    }];

    let plan = planner
        .plan_frame(
            &terminal.dump_grid(),
            terminal.dump_cursor(),
            &dirty,
            &mut atlas,
        )
        .unwrap();

    assert!(plan.glyphs.is_empty());
    assert_eq!(atlas.metrics().entries, 0);
}
