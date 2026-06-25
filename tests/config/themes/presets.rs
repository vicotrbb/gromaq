use gromaq::{ThemePresetSetting, format_theme_preset};

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
