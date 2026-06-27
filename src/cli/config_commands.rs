use std::path::PathBuf;

use super::CliExit;
use crate::config::{GromaqConfig, format_theme_preset};

mod formatting;
mod launch;

use formatting::{
    format_config_list, format_cursor_style, format_font_resolution_with_fallbacks,
    format_toml_string_array,
};
pub use launch::{
    NativeAppLaunchConfig, NativeAppLaunchError, NativeAppLauncher, RealNativeAppLauncher,
};

pub(super) fn launch_config_file_exit<A>(path: &str, app_launcher: &A) -> CliExit
where
    A: NativeAppLauncher,
{
    let config = match GromaqConfig::from_toml_file(path) {
        Ok(config) => config,
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!(
                    "config launch failed: {error}\nrun `gromaq --config-check {path}` before launch\n"
                ),
            };
        }
    };
    let launch_config = match NativeAppLaunchConfig::from_gromaq_config(&config) {
        Ok(mut launch_config) => {
            launch_config.config_path = Some(PathBuf::from(path));
            launch_config
        }
        Err(error) => {
            return CliExit {
                code: 1,
                stdout: String::new(),
                stderr: format!("{error}\n"),
            };
        }
    };
    launch_native_app_exit(app_launcher, launch_config)
}

pub(super) fn launch_native_app_exit<A>(app_launcher: &A, config: NativeAppLaunchConfig) -> CliExit
where
    A: NativeAppLauncher,
{
    match app_launcher.launch(config) {
        Ok(_) => CliExit {
            code: 0,
            stdout: String::new(),
            stderr: String::new(),
        },
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!("{error}\n"),
        },
    }
}

pub(super) fn config_check_exit(path: &str) -> CliExit {
    match GromaqConfig::from_toml_file(path) {
        Ok(config) => {
            let font_resolution = format_font_resolution_with_fallbacks(
                &config.font.family,
                &config.font.fallback_families,
            );
            CliExit {
                code: 0,
                stdout: format!(
                    "config check: ok\npath: {}\nterminal: {}x{}\nscrollback lines: {}\nshell: {}\nshell args: {}\nshell cwd: {}\nwelcome enabled: {}\nfont: {} {}px\nfont configured fallbacks: {}\nfont source: {}\nfont fallbacks: {}\ncell width: {}px\nline height: {}px\ntheme preset: {}\ntheme background: {}\ntheme foreground: {}\ntheme cursor: {}\ntheme selection: {}\ntheme background opacity: {}\ntheme cursor opacity: {}\ntheme selection opacity: {}\ntheme cursor style: {}\ntheme cursor blinking: {}\ntheme surface padding px: {}\ntheme cell spacing px: {}\ntheme dim opacity: {}\ntarget fps: {}\ndirty-region rendering: {}\n",
                    path,
                    config.terminal.cols,
                    config.terminal.rows,
                    config.terminal.scrollback_lines,
                    config.shell.program.as_deref().unwrap_or("<default>"),
                    format_config_list(&config.shell.args),
                    config.shell.cwd.as_deref().unwrap_or("<default>"),
                    config.welcome.enabled,
                    config.font.family,
                    config.font.size_px,
                    format_config_list(&config.font.fallback_families),
                    font_resolution.primary,
                    font_resolution.fallbacks,
                    config.font.renderer_cell_width_px(),
                    config.font.line_height_px,
                    format_theme_preset(config.theme.preset),
                    config.theme.background,
                    config.theme.foreground,
                    config.theme.cursor,
                    config.theme.selection,
                    config.theme.background_opacity,
                    config.theme.cursor_opacity,
                    config.theme.selection_opacity,
                    format_cursor_style(config.theme.cursor_style),
                    config.theme.cursor_blinking,
                    config.theme.surface_padding_px,
                    config.theme.cell_spacing_px,
                    config.theme.dim_opacity,
                    config.performance.target_fps,
                    config.performance.dirty_region_rendering
                ),
                stderr: String::new(),
            }
        }
        Err(error) => CliExit {
            code: 1,
            stdout: String::new(),
            stderr: format!(
                "config check failed: {error}\nrun `gromaq --config-check {path}` after editing\n"
            ),
        },
    }
}

pub(super) fn config_template_exit() -> CliExit {
    let config = GromaqConfig::default();
    CliExit {
        code: 0,
        stdout: format!(
            "# Gromaq configuration template\n\n[terminal]\ncols = {}\nrows = {}\nscrollback_lines = {}\n\n[shell]\n# program = \"/bin/zsh\"\n# args = [\"-l\"]\n# cwd = \"/tmp\"\n\n[welcome]\nenabled = {}\n\n[font]\nfamily = \"{}\"\n# fallback_families = [\"Apple Color Emoji\"]\nsize_px = {}\n# cell_width_px = {}\nline_height_px = {}\n\n[theme]\n# presets: gromaq-dark, gromaq-graphite, gromaq-ghostty\npreset = \"{}\"\nbackground = \"{}\"\nforeground = \"{}\"\ncursor = \"{}\"\nselection = \"{}\"\nbackground_opacity = {}\ncursor_opacity = {}\nselection_opacity = {}\ncursor_style = \"{}\"\ncursor_blinking = {}\nansi = {}\nsurface_padding_px = {}\ncell_spacing_px = {}\ndim_opacity = {}\n\n[performance]\ntarget_fps = {}\ndirty_region_rendering = {}\n",
            config.terminal.cols,
            config.terminal.rows,
            config.terminal.scrollback_lines,
            config.welcome.enabled,
            config.font.family,
            config.font.size_px,
            config.font.renderer_cell_width_px(),
            config.font.line_height_px,
            format_theme_preset(config.theme.preset),
            config.theme.background,
            config.theme.foreground,
            config.theme.cursor,
            config.theme.selection,
            config.theme.background_opacity,
            config.theme.cursor_opacity,
            config.theme.selection_opacity,
            format_cursor_style(config.theme.cursor_style),
            config.theme.cursor_blinking,
            format_toml_string_array(&config.theme.ansi),
            config.theme.surface_padding_px,
            config.theme.cell_spacing_px,
            config.theme.dim_opacity,
            config.performance.target_fps,
            config.performance.dirty_region_rendering
        ),
        stderr: String::new(),
    }
}
