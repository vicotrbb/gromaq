//! Terminal cell and style types.

use serde::{Deserialize, Serialize};

/// Terminal color representation used by snapshots and the core grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Color {
    /// Inherit the terminal default color.
    Default,
    /// ANSI color index in the 16-color palette.
    Ansi(u8),
    /// Indexed color from a 256-color palette.
    Indexed(u8),
    /// True-color RGB value.
    Rgb(u8, u8, u8),
}

/// Text style attributes for a terminal cell.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Style {
    /// Foreground color.
    pub foreground: Color,
    /// Background color.
    pub background: Color,
    /// Bold intensity flag.
    pub bold: bool,
    /// Faint/dim intensity flag.
    pub dim: bool,
    /// Italic presentation flag.
    pub italic: bool,
    /// Underline presentation flag.
    pub underline: bool,
    /// Underline presentation style.
    pub underline_style: UnderlineStyle,
    /// Internal underline color identifier. Zero means the default underline color.
    pub underline_color_id: u16,
    /// Blink presentation flag.
    pub blink: bool,
    /// Hidden/conceal presentation flag.
    pub hidden: bool,
    /// Inverse-video presentation flag.
    pub inverse: bool,
    /// Overline presentation flag.
    pub overline: bool,
    /// Strikethrough presentation flag.
    pub strikethrough: bool,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            foreground: Color::Default,
            background: Color::Default,
            bold: false,
            dim: false,
            italic: false,
            underline: false,
            underline_style: UnderlineStyle::Single,
            underline_color_id: 0,
            blink: false,
            hidden: false,
            inverse: false,
            overline: false,
            strikethrough: false,
        }
    }
}

/// Supported terminal underline styles.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum UnderlineStyle {
    /// Single straight underline.
    #[default]
    Single,
    /// Double straight underline.
    Double,
    /// Curly underline.
    Curly,
    /// Dotted underline.
    Dotted,
    /// Dashed underline.
    Dashed,
}

/// A single terminal grid cell.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cell {
    /// Display text for the cell. Empty cells use an empty string to avoid hot-path allocations.
    pub text: String,
    /// Style applied to the cell.
    pub style: Style,
    /// Internal OSC 8 hyperlink identifier. Zero means no hyperlink.
    pub hyperlink_id: u16,
    /// Whether this cell is the leading cell of a wide grapheme.
    pub is_wide_leading: bool,
    /// Whether this cell is the trailing placeholder for a wide grapheme.
    pub is_wide_trailing: bool,
}

impl Cell {
    /// Return a blank cell using the supplied style.
    pub fn blank(style: Style) -> Self {
        Self {
            text: String::new(),
            style,
            hyperlink_id: 0,
            is_wide_leading: false,
            is_wide_trailing: false,
        }
    }

    /// Return `true` when this cell has no visible glyph.
    pub fn is_blank(&self) -> bool {
        self.text.is_empty() && !self.is_wide_trailing
    }
}

impl Default for Cell {
    fn default() -> Self {
        Self::blank(Style::default())
    }
}

/// Serializable cell snapshot for tests and debug tooling.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CellSnapshot {
    /// Display text for the cell.
    pub text: String,
    /// Style applied to the cell.
    pub style: Style,
    /// OSC 8 hyperlink identifier. Zero means no hyperlink.
    pub hyperlink_id: u16,
    /// Whether this cell is the leading cell of a wide grapheme.
    pub is_wide_leading: bool,
    /// Whether this cell is the trailing placeholder for a wide grapheme.
    pub is_wide_trailing: bool,
}

impl From<&Cell> for CellSnapshot {
    fn from(value: &Cell) -> Self {
        Self {
            text: value.text.clone(),
            style: value.style,
            hyperlink_id: value.hyperlink_id,
            is_wide_leading: value.is_wide_leading,
            is_wide_trailing: value.is_wide_trailing,
        }
    }
}
