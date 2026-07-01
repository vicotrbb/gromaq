/// Structured result from preparing and presenting a native terminal glyph frame.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct NativeGlyphFramePresentation {
    /// Whether dirty terminal state was rendered through the renderer boundary.
    pub rendered: bool,
    /// Whether a glyph frame was presented through the native surface backend.
    pub glyph_frame_presented: bool,
    /// Whether the rendered frame included the retained native tmux status strip.
    pub tmux_status_strip_rendered: bool,
    /// Whether the rendered tmux status strip included the active pane command.
    pub tmux_status_pane_command_rendered: bool,
    /// Whether the rendered frame included the native tmux manager panel.
    pub tmux_manager_panel_rendered: bool,
    /// Session count in the rendered native tmux manager snapshot.
    pub tmux_manager_sessions: usize,
    /// Window count in the rendered native tmux manager snapshot.
    pub tmux_manager_windows: usize,
    /// Pane count in the rendered native tmux manager snapshot.
    pub tmux_manager_panes: usize,
    /// Whether the surface was cleared without a glyph frame.
    pub clear_presented: bool,
    /// Presented frame width in pixels.
    pub width: u32,
    /// Presented frame height in pixels.
    pub height: u32,
    /// Textured glyph quads prepared for presentation.
    pub glyph_quads: usize,
    /// Solid background quads prepared for presentation.
    pub background_quads: usize,
    /// Solid text-decoration quads prepared for presentation.
    pub decoration_quads: usize,
    /// Solid cursor quads prepared for presentation.
    pub cursor_quads: usize,
    /// Packed glyph atlas byte length.
    pub atlas_bytes: usize,
    /// Occupied glyph atlas slots.
    pub atlas_occupied_slots: usize,
    /// Whether a prepared glyph-frame snapshot artifact was written.
    pub snapshot_written: bool,
    /// Bytes written for the prepared glyph-frame snapshot artifact.
    pub snapshot_bytes: usize,
    /// Snapshot artifact width in pixels.
    pub snapshot_width: u32,
    /// Snapshot artifact height in pixels.
    pub snapshot_height: u32,
}
