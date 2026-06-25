use std::fs;
use std::path::Path;

use crate::cli::CliExit;
use crate::config::{ThemeSettings, format_theme_preset, parse_theme_preset};

pub(in crate::cli) fn theme_export_exit(preset: &str, path: &str) -> CliExit {
    match export_theme_preset(preset, path) {
        Ok(report) => CliExit {
            code: 0,
            stdout: format!(
                "theme export: ok\npreset: {}\npath: {}\nbytes written: {}\n",
                report.preset, path, report.bytes_written
            ),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("theme export failed: {error}\n"),
        },
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ThemeExportReport {
    preset: &'static str,
    bytes_written: usize,
}

fn export_theme_preset(preset: &str, path: &str) -> Result<ThemeExportReport, String> {
    let preset = parse_theme_preset(preset)
        .ok_or_else(|| format!("unknown theme preset `{preset}`; run `gromaq --theme-list`"))?;
    let contents = theme_preset_toml(preset)?;
    fs::write(Path::new(path), contents.as_bytes())
        .map_err(|error| format!("failed to write theme export: {error}"))?;
    Ok(ThemeExportReport {
        preset: format_theme_preset(preset),
        bytes_written: contents.len(),
    })
}

fn theme_preset_toml(preset: crate::config::ThemePresetSetting) -> Result<String, String> {
    let theme = ThemeSettings::from_preset(preset);
    let mut contents = toml::to_string_pretty(&ThemeExportToml { theme })
        .map_err(|error| format!("failed to serialize theme preset: {error}"))?;
    if !contents.ends_with('\n') {
        contents.push('\n');
    }
    Ok(contents)
}

#[derive(serde::Serialize)]
struct ThemeExportToml {
    theme: ThemeSettings,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::ThemePresetSetting;

    #[test]
    fn theme_preset_toml_exports_parseable_theme_block() {
        let toml = theme_preset_toml(ThemePresetSetting::GromaqGraphite).unwrap();
        let parsed = crate::config::GromaqConfig::from_toml_str(&toml).unwrap();

        assert!(toml.starts_with("[theme]\n"));
        assert!(toml.contains("preset = \"gromaq-graphite\""));
        assert!(toml.contains("background = \"#0b0f14\""));
        assert_eq!(
            parsed.theme,
            ThemeSettings::from_preset(ThemePresetSetting::GromaqGraphite)
        );
    }
}
