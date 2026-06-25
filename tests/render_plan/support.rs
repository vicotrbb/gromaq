use gromaq::renderer::{GlyphAtlas, GlyphAtlasConfig, RenderPlan, RenderPlanner};
use gromaq::{DirtyRegion, Terminal};

pub(super) fn new_atlas() -> GlyphAtlas {
    GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap())
}

pub(super) fn new_planner(font_size_px: u16) -> RenderPlanner {
    RenderPlanner::new(font_size_px)
}

pub(super) fn plan_frame(
    terminal: &Terminal,
    dirty: &[DirtyRegion],
    atlas: &mut GlyphAtlas,
    planner: &mut RenderPlanner,
) -> RenderPlan {
    planner
        .plan_frame(&terminal.dump_grid(), terminal.dump_cursor(), dirty, atlas)
        .unwrap()
}
