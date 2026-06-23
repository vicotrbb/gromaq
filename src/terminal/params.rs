use vte::Params;

use crate::cell::{Color, Style, UnderlineStyle};

pub(super) fn first_value(params: &Params, index: usize) -> Option<u16> {
    params
        .iter()
        .nth(index)
        .and_then(|param| param.first().copied())
}

pub(super) fn first_values(params: &Params) -> impl Iterator<Item = u16> + '_ {
    params
        .iter()
        .map(|param| param.first().copied().unwrap_or(0))
}

pub(super) fn default_tab_stops(cols: u16) -> Vec<bool> {
    let mut tab_stops = vec![false; usize::from(cols)];
    for col in (8..usize::from(cols)).step_by(8) {
        tab_stops[col] = true;
    }
    tab_stops
}

pub(super) fn push_sgr_color_parameters(
    params: &mut Vec<String>,
    normal_base: u16,
    bright_base: u16,
    extended_prefix: u16,
    color: Color,
) {
    match color {
        Color::Default => {}
        Color::Ansi(index) if index < 8 => {
            params.push((normal_base + u16::from(index)).to_string());
        }
        Color::Ansi(index) if index < 16 => {
            params.push((bright_base + u16::from(index - 8)).to_string());
        }
        Color::Ansi(index) | Color::Indexed(index) => {
            params.push(format!("{extended_prefix}:5:{index}"));
        }
        Color::Rgb(red, green, blue) => {
            params.push(format!("{extended_prefix}:2:{red}:{green}:{blue}"));
        }
    }
}

pub(super) fn push_sgr_extended_color_parameter(
    params: &mut Vec<String>,
    prefix: u16,
    color: Color,
) {
    match color {
        Color::Default => {}
        Color::Ansi(index) | Color::Indexed(index) => {
            params.push(format!("{prefix}:5:{index}"));
        }
        Color::Rgb(red, green, blue) => {
            params.push(format!("{prefix}:2:{red}:{green}:{blue}"));
        }
    }
}

pub(super) fn parse_extended_color<I>(iter: &mut std::iter::Peekable<I>) -> Option<Color>
where
    I: Iterator<Item = u16>,
{
    match iter.next()? {
        5 => {
            let index = u8::try_from(iter.next()?).ok()?;
            Some(Color::Indexed(index))
        }
        2 => {
            let r = iter.next()?;
            let g = iter.next()?;
            let b = iter.next()?;
            let r = u8::try_from(r).ok()?;
            let g = u8::try_from(g).ok()?;
            let b = u8::try_from(b).ok()?;
            Some(Color::Rgb(r, g, b))
        }
        _ => {
            let _ = iter.next();
            None
        }
    }
}

pub(super) fn grouped_extended_color(param: &[u16]) -> Option<(u16, Color)> {
    match param {
        [target @ (38 | 48 | 58), 5, index] => {
            let index = u8::try_from(*index).ok()?;
            Some((*target, Color::Indexed(index)))
        }
        [target @ (38 | 48 | 58), 2, red, green, blue] => {
            let red = u8::try_from(*red).ok()?;
            let green = u8::try_from(*green).ok()?;
            let blue = u8::try_from(*blue).ok()?;
            Some((*target, Color::Rgb(red, green, blue)))
        }
        [target @ (38 | 48 | 58), 2, _colorspace, red, green, blue] => {
            let red = u8::try_from(*red).ok()?;
            let green = u8::try_from(*green).ok()?;
            let blue = u8::try_from(*blue).ok()?;
            Some((*target, Color::Rgb(red, green, blue)))
        }
        _ => None,
    }
}

pub(super) fn is_invalid_grouped_extended_color_param(param: &[u16]) -> bool {
    match param {
        [38 | 48 | 58] => false,
        [38 | 48 | 58, 5, index] => u8::try_from(*index).is_err(),
        [38 | 48 | 58, 2, red, green, blue] => {
            u8::try_from(*red).is_err()
                || u8::try_from(*green).is_err()
                || u8::try_from(*blue).is_err()
        }
        [38 | 48 | 58, 2, _colorspace, red, green, blue] => {
            u8::try_from(*red).is_err()
                || u8::try_from(*green).is_err()
                || u8::try_from(*blue).is_err()
        }
        [38 | 48 | 58, ..] => true,
        _ => false,
    }
}

pub(super) fn apply_grouped_sgr_param(style: &mut Style, param: &[u16]) -> bool {
    match param {
        [4, underline_style] => {
            match underline_style {
                0 => {
                    style.underline = false;
                    style.underline_style = UnderlineStyle::Single;
                }
                1 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Single;
                }
                2 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Double;
                }
                3 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Curly;
                }
                4 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Dotted;
                }
                5 => {
                    style.underline = true;
                    style.underline_style = UnderlineStyle::Dashed;
                }
                _ => {}
            }
            true
        }
        _ => false,
    }
}
