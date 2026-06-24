use std::path::PathBuf;

use super::CliExit;
use crate::app::resolve_native_font_paths;
use crate::config::{CursorStyleSetting, GromaqConfig, format_theme_preset};

mod launch;

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
                stderr: format!("config launch failed: {error}\n"),
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
            let font_resolution = format_font_resolution(&config.font.family);
            CliExit {
                code: 0,
                stdout: format!(
                    "config check: ok\npath: {}\nterminal: {}x{}\nscrollback lines: {}\nshell: {}\nshell args: {}\nshell cwd: {}\nfont: {} {}px\nfont source: {}\nfont fallbacks: {}\ncell width: {}px\nline height: {}px\ntheme preset: {}\ntheme background: {}\ntheme foreground: {}\ntheme cursor: {}\ntheme selection: {}\ntheme cursor style: {}\ntheme cursor blinking: {}\ntheme surface padding px: {}\ntheme dim opacity: {}\ntarget fps: {}\ndirty-region rendering: {}\n",
                    path,
                    config.terminal.cols,
                    config.terminal.rows,
                    config.terminal.scrollback_lines,
                    config.shell.program.as_deref().unwrap_or("<default>"),
                    format_config_list(&config.shell.args),
                    config.shell.cwd.as_deref().unwrap_or("<default>"),
                    config.font.family,
                    config.font.size_px,
                    font_resolution.primary,
                    font_resolution.fallbacks,
                    config.font.renderer_cell_width_px(),
                    config.font.line_height_px,
                    format_theme_preset(config.theme.preset),
                    config.theme.background,
                    config.theme.foreground,
                    config.theme.cursor,
                    config.theme.selection,
                    format_cursor_style(config.theme.cursor_style),
                    config.theme.cursor_blinking,
                    config.theme.surface_padding_px,
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
            stderr: format!("config check failed: {error}\n"),
        },
    }
}

struct FontResolutionText {
    primary: String,
    fallbacks: String,
}

fn format_font_resolution(font_family: &str) -> FontResolutionText {
    match resolve_native_font_paths(font_family) {
        Ok(resolution) => FontResolutionText {
            primary: resolution.primary_path.display().to_string(),
            fallbacks: if resolution.fallback_paths.is_empty() {
                "<none>".to_owned()
            } else {
                resolution
                    .fallback_paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect::<Vec<_>>()
                    .join(" ")
            },
        },
        Err(error) => FontResolutionText {
            primary: format!("<unresolved: {error}>"),
            fallbacks: "<unknown>".to_owned(),
        },
    }
}

pub(super) fn config_template_exit() -> CliExit {
    let config = GromaqConfig::default();
    CliExit {
        code: 0,
        stdout: format!(
            "# Gromaq configuration template\n\n[terminal]\ncols = {}\nrows = {}\nscrollback_lines = {}\n\n[shell]\n# program = \"/bin/zsh\"\n# args = [\"-l\"]\n# cwd = \"/tmp\"\n\n[font]\nfamily = \"{}\"\nsize_px = {}\n# cell_width_px = {}\nline_height_px = {}\n\n[theme]\npreset = \"{}\"\nbackground = \"{}\"\nforeground = \"{}\"\ncursor = \"{}\"\nselection = \"{}\"\ncursor_style = \"{}\"\ncursor_blinking = {}\nansi = {}\nsurface_padding_px = {}\ndim_opacity = {}\n\n[performance]\ntarget_fps = {}\ndirty_region_rendering = {}\n",
            config.terminal.cols,
            config.terminal.rows,
            config.terminal.scrollback_lines,
            config.font.family,
            config.font.size_px,
            config.font.renderer_cell_width_px(),
            config.font.line_height_px,
            format_theme_preset(config.theme.preset),
            config.theme.background,
            config.theme.foreground,
            config.theme.cursor,
            config.theme.selection,
            format_cursor_style(config.theme.cursor_style),
            config.theme.cursor_blinking,
            format_toml_string_array(&config.theme.ansi),
            config.theme.surface_padding_px,
            config.theme.dim_opacity,
            config.performance.target_fps,
            config.performance.dirty_region_rendering
        ),
        stderr: String::new(),
    }
}

fn format_cursor_style(style: CursorStyleSetting) -> &'static str {
    match style {
        CursorStyleSetting::Block => "block",
        CursorStyleSetting::Underline => "underline",
        CursorStyleSetting::Bar => "bar",
    }
}

fn format_config_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_owned()
    } else {
        values.join(" ")
    }
}

fn format_toml_string_array(values: &[String]) -> String {
    let entries = values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{entries}]")
}
