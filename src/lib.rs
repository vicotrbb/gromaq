#![forbid(unsafe_code)]
#![deny(missing_docs)]
//! Gromaq terminal emulator foundation.

pub mod app;
pub mod cell;
pub mod cli;
pub mod clipboard;
pub mod config;
pub mod dirty;
pub mod error;
pub mod font;
pub mod grid;
pub mod input;
pub mod mouse;
pub mod native_gpu;
pub mod pty;
pub mod renderer;
pub mod scrollback;
pub mod selection;
pub mod terminal;
pub mod test_api;

pub use cell::{Cell, CellSnapshot, Color, Style, UnderlineStyle};
pub use clipboard::{HostClipboard, MemoryClipboard, NativeClipboard};
pub use config::{
    ANSI_COLOR_COUNT, ConfigFileReloader, ConfigReload, CursorStyleSetting, DEFAULT_ANSI_COLORS,
    DEFAULT_ANSI_COLORS_RGB8, DEFAULT_BACKGROUND, DEFAULT_BACKGROUND_RGB8, DEFAULT_CURSOR,
    DEFAULT_CURSOR_RGB8, DEFAULT_DIM_OPACITY, DEFAULT_FONT_FAMILY, DEFAULT_FOREGROUND,
    DEFAULT_FOREGROUND_RGB8, DEFAULT_SELECTION, DEFAULT_SELECTION_RGB8, DEFAULT_SURFACE_PADDING_PX,
    DEFAULT_THEME_PRESET, FontSettings, GromaqConfig, MAX_CELL_WIDTH_PX, MAX_DIM_OPACITY,
    MAX_LINE_HEIGHT_PX, MIN_CELL_WIDTH_PX, MIN_DIM_OPACITY, MIN_LINE_HEIGHT_PX,
    PerformanceSettings, ShellSettings, TerminalSettings, ThemePresetSetting, ThemeSettings,
    format_theme_preset,
};
pub use dirty::{DirtyRegion, DirtyTracker};
pub use error::{GromaqError, Result};
pub use font::{FontRasterError, FontRasterizer};
pub use grid::GridSnapshot;
pub use input::{KeyModifiers, TestKey, encode_keys, encode_winit_key};
pub use mouse::{
    MouseButton, MouseEvent, MouseEventKind, MouseProtocol, MouseReportMode, MouseReportState,
};
pub use scrollback::ScrollbackSnapshot;
pub use selection::{SelectionPoint, SelectionRange};
pub use terminal::{
    CursorShape, CursorSnapshot, PerfSnapshot, Screenshot, Terminal, TerminalConfig,
};
pub use test_api::TerminalTestApi;
