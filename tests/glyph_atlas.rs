use gromaq::renderer::{GlyphAtlas, GlyphAtlasConfig, GlyphKey};
use gromaq::{Color, Style};

#[test]
fn glyph_cache_reuses_existing_entry_for_same_key() {
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(4).unwrap());
    let key = GlyphKey::new('A', Style::default(), 14);

    let first = atlas.lookup_or_insert(key.clone()).unwrap();
    let second = atlas.lookup_or_insert(key).unwrap();

    assert_eq!(first, second);
    assert_eq!(atlas.metrics().misses, 1);
    assert_eq!(atlas.metrics().hits, 1);
    assert_eq!(atlas.metrics().entries, 1);
}

#[test]
fn glyph_cache_reuses_entry_for_color_only_style_changes() {
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let plain = GlyphKey::new('A', Style::default(), 14);
    let red_style = Style {
        foreground: Color::Ansi(1),
        background: Color::Ansi(4),
        underline_color_id: 7,
        ..Style::default()
    };
    let colored = GlyphKey::new('A', red_style, 14);

    let plain_entry = atlas.lookup_or_insert(plain).unwrap();
    let colored_entry = atlas.lookup_or_insert(colored).unwrap();

    assert_eq!(plain_entry, colored_entry);
    assert_eq!(atlas.metrics().entries, 1);
    assert_eq!(atlas.metrics().hits, 1);
}

#[test]
fn glyph_cache_reuses_entry_for_box_decoration_style_changes() {
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let plain = GlyphKey::new('A', Style::default(), 14);
    let decorated_style = Style {
        framed: true,
        encircled: true,
        overline: true,
        strikethrough: true,
        ..Style::default()
    };
    let decorated = GlyphKey::new('A', decorated_style, 14);

    let plain_entry = atlas.lookup_or_insert(plain).unwrap();
    let decorated_entry = atlas.lookup_or_insert(decorated).unwrap();

    assert_eq!(plain_entry, decorated_entry);
    assert_eq!(atlas.metrics().entries, 1);
    assert_eq!(atlas.metrics().hits, 1);
}

#[test]
fn glyph_cache_distinguishes_font_affecting_style_and_size() {
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(8).unwrap());
    let plain = GlyphKey::new('A', Style::default(), 14);
    let bold_style = Style {
        bold: true,
        ..Style::default()
    };
    let bold = GlyphKey::new('A', bold_style, 14);
    let larger = GlyphKey::new('A', Style::default(), 18);

    let plain_entry = atlas.lookup_or_insert(plain).unwrap();
    let bold_entry = atlas.lookup_or_insert(bold).unwrap();
    let larger_entry = atlas.lookup_or_insert(larger).unwrap();

    assert_ne!(plain_entry, bold_entry);
    assert_ne!(plain_entry, larger_entry);
    assert_eq!(atlas.metrics().entries, 3);
}

#[test]
fn glyph_cache_evicts_least_recently_used_entry_when_full() {
    let mut atlas = GlyphAtlas::new(GlyphAtlasConfig::new(2).unwrap());
    let a = GlyphKey::new('A', Style::default(), 14);
    let b = GlyphKey::new('B', Style::default(), 14);
    let c = GlyphKey::new('C', Style::default(), 14);

    let a_first = atlas.lookup_or_insert(a.clone()).unwrap();
    let b_entry = atlas.lookup_or_insert(b.clone()).unwrap();
    assert_eq!(atlas.lookup_or_insert(a).unwrap(), a_first);

    let c_entry = atlas.lookup_or_insert(c).unwrap();

    assert_ne!(b_entry, c_entry);
    assert_ne!(atlas.lookup_or_insert(b).unwrap(), b_entry);
    assert_eq!(atlas.metrics().evictions, 2);
    assert_eq!(atlas.metrics().entries, 2);
}

#[test]
fn invalid_glyph_atlas_capacity_is_rejected() {
    let error = GlyphAtlasConfig::new(0).unwrap_err();

    assert!(error.to_string().contains("glyph atlas capacity"));
}

#[test]
fn oversized_glyph_atlas_capacity_is_rejected() {
    let error = GlyphAtlasConfig::new(usize::MAX).unwrap_err();

    assert!(error.to_string().contains("65536"));
    assert!(error.to_string().contains(&usize::MAX.to_string()));
}
