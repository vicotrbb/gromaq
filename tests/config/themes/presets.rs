use gromaq::{ThemePresetSetting, format_theme_preset, parse_theme_preset};

#[test]
fn theme_preset_formatter_returns_documented_toml_names() {
    assert_eq!(
        format_theme_preset(ThemePresetSetting::GromaqDark),
        "gromaq-dark"
    );
    assert_eq!(
        format_theme_preset(ThemePresetSetting::GromaqGraphite),
        "gromaq-graphite"
    );
    assert_eq!(
        format_theme_preset(ThemePresetSetting::GromaqGhostty),
        "gromaq-ghostty"
    );
}

#[test]
fn theme_preset_parser_accepts_documented_toml_names() {
    assert_eq!(
        parse_theme_preset("gromaq-dark"),
        Some(ThemePresetSetting::GromaqDark)
    );
    assert_eq!(
        parse_theme_preset("gromaq-graphite"),
        Some(ThemePresetSetting::GromaqGraphite)
    );
    assert_eq!(
        parse_theme_preset("gromaq-ghostty"),
        Some(ThemePresetSetting::GromaqGhostty)
    );
    assert_eq!(parse_theme_preset("ghostty"), None);
}
