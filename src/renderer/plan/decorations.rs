//! Text-decoration extraction for render plans.

use crate::cell::{Color, Style, UnderlineStyle};
use crate::renderer::color::decoration_color_rgba8;

use super::types::{PlannedTextDecoration, TextDecorationKind};

pub(super) fn append_cell_decorations(
    decorations: &mut Vec<PlannedTextDecoration>,
    row: u16,
    col: u16,
    style: Style,
    underline_color: Color,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
) {
    if style.hidden {
        return;
    }
    if style.underline {
        append_underline_decorations(
            decorations,
            row,
            col,
            style,
            underline_color,
            default_foreground_rgb8,
            ansi_colors_rgb8,
        );
    }
    if style.overline {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Overline,
            decoration_color_rgba8(
                Color::Default,
                style,
                default_foreground_rgb8,
                ansi_colors_rgb8,
            ),
        );
    }
    if style.strikethrough {
        append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Strikethrough,
            decoration_color_rgba8(
                Color::Default,
                style,
                default_foreground_rgb8,
                ansi_colors_rgb8,
            ),
        );
    }
}

fn append_underline_decorations(
    decorations: &mut Vec<PlannedTextDecoration>,
    row: u16,
    col: u16,
    style: Style,
    underline_color: Color,
    default_foreground_rgb8: [u8; 3],
    ansi_colors_rgb8: [[u8; 3]; 16],
) {
    let color_rgba8 = decoration_color_rgba8(
        underline_color,
        style,
        default_foreground_rgb8,
        ansi_colors_rgb8,
    );
    match style.underline_style {
        UnderlineStyle::Single => append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::Underline,
            color_rgba8,
        ),
        UnderlineStyle::Double => {
            append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DoubleUnderlineTop,
                color_rgba8,
            );
            append_text_decoration(
                decorations,
                row,
                col,
                TextDecorationKind::DoubleUnderlineBottom,
                color_rgba8,
            );
        }
        UnderlineStyle::Curly => append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::CurlyUnderline,
            color_rgba8,
        ),
        UnderlineStyle::Dotted => append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::DottedUnderline,
            color_rgba8,
        ),
        UnderlineStyle::Dashed => append_text_decoration(
            decorations,
            row,
            col,
            TextDecorationKind::DashedUnderline,
            color_rgba8,
        ),
    }
}

fn append_text_decoration(
    decorations: &mut Vec<PlannedTextDecoration>,
    row: u16,
    col: u16,
    kind: TextDecorationKind,
    color_rgba8: [u8; 4],
) {
    if let Some(last) = decorations.iter_mut().rev().take(4).find(|last| {
        last.row == row
            && last.col.saturating_add(last.cols) == col
            && last.kind == kind
            && last.color_rgba8 == color_rgba8
    }) {
        last.cols = last.cols.saturating_add(1);
        return;
    }
    decorations.push(PlannedTextDecoration {
        row,
        col,
        cols: 1,
        kind,
        color_rgba8,
    });
}
