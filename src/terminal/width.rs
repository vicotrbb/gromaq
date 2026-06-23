use unicode_width::UnicodeWidthChar;

pub(super) fn visible_width(text: &str) -> usize {
    text.chars().map(char_width).sum()
}

pub(super) fn char_width(ch: char) -> usize {
    UnicodeWidthChar::width(ch).unwrap_or(0).min(2)
}

pub(super) fn metadata_id_for_index(index: usize) -> u16 {
    let id = index.saturating_add(1);
    if id > usize::from(u16::MAX) {
        return 0;
    }
    id as u16
}

pub(super) fn is_emoji_modifier(ch: char) -> bool {
    matches!(ch, '\u{1f3fb}'..='\u{1f3ff}')
}

pub(super) fn is_emoji_modifier_base_candidate(ch: char) -> bool {
    matches!(
        ch,
        '\u{2600}'..='\u{27bf}' | '\u{1f000}'..='\u{1faff}'
    )
}

pub(super) fn is_emoji_presentation_base_candidate(ch: char) -> bool {
    is_emoji_modifier_base_candidate(ch) || matches!(ch, '\u{00a9}' | '\u{00ae}' | '\u{2122}')
}

pub(super) fn is_variation_selector_16(ch: char) -> bool {
    ch == '\u{fe0f}'
}

pub(super) fn is_combining_enclosing_keycap(ch: char) -> bool {
    ch == '\u{20e3}'
}

pub(super) fn is_keycap_base_sequence(text: &str) -> bool {
    let mut chars = text.chars();
    let Some(base) = chars.next() else {
        return false;
    };
    if !matches!(base, '#' | '*' | '0'..='9') {
        return false;
    }
    matches!(chars.next(), None | Some('\u{fe0f}')) && chars.next().is_none()
}

pub(super) fn is_regional_indicator(ch: char) -> bool {
    matches!(ch, '\u{1f1e6}'..='\u{1f1ff}')
}

#[cold]
#[inline(never)]
pub(super) fn map_dec_special_graphics(ch: char) -> char {
    match ch {
        '`' => '◆',
        'a' => '▒',
        'f' => '°',
        'g' => '±',
        'h' => '␤',
        'i' => '␋',
        'j' => '┘',
        'k' => '┐',
        'l' => '┌',
        'm' => '└',
        'n' => '┼',
        'o' => '⎺',
        'p' => '⎻',
        'q' => '─',
        'r' => '⎼',
        's' => '⎽',
        't' => '├',
        'u' => '┤',
        'v' => '┴',
        'w' => '┬',
        'x' => '│',
        'y' => '≤',
        'z' => '≥',
        '{' => 'π',
        '|' => '≠',
        '}' => '£',
        '~' => '·',
        _ => ch,
    }
}
