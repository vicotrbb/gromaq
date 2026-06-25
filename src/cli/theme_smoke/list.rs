use crate::cli::CliExit;
use crate::config::{DEFAULT_THEME_PRESET, ThemePresetSetting, ThemeSettings, format_theme_preset};

const THEME_PRESETS: [ThemePresetSetting; 3] = [
    ThemePresetSetting::GromaqGhostty,
    ThemePresetSetting::GromaqDark,
    ThemePresetSetting::GromaqGraphite,
];

pub(in crate::cli) fn theme_list_exit() -> CliExit {
    let mut stdout = String::from("theme presets:\n");
    for preset in THEME_PRESETS {
        stdout.push_str(&theme_preset_summary(preset));
    }
    CliExit {
        code: 0,
        stdout,
        stderr: String::new(),
    }
}

fn theme_preset_summary(preset: ThemePresetSetting) -> String {
    let theme = ThemeSettings::from_preset(preset);
    let name = format_theme_preset(preset);
    let default_marker = if name == DEFAULT_THEME_PRESET {
        " default"
    } else {
        ""
    };
    format!(
        "- {name}{default_marker}\n  background: {}\n  foreground: {}\n  cursor: {}\n  selection: {}\n  background opacity: {}\n  surface padding px: {}\n  cell spacing px: {}\n  dim opacity: {}\n",
        theme.background,
        theme.foreground,
        theme.cursor,
        theme.selection,
        theme.background_opacity,
        theme.surface_padding_px,
        theme.cell_spacing_px,
        theme.dim_opacity
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn theme_preset_summary_marks_default_and_reports_core_tokens() {
        let summary = theme_preset_summary(ThemePresetSetting::GromaqGhostty);

        assert!(summary.contains("- gromaq-ghostty default"));
        assert!(summary.contains("background: #101216"));
        assert!(summary.contains("foreground: #eef4fb"));
        assert!(summary.contains("background opacity: 1"));
        assert!(summary.contains("surface padding px: 14"));
    }
}
