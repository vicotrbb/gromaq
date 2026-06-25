use crate::app::resolve_native_font_paths;
use crate::config::CursorStyleSetting;

pub(super) struct FontResolutionText {
    pub(super) primary: String,
    pub(super) fallbacks: String,
}

pub(super) fn format_font_resolution(font_family: &str) -> FontResolutionText {
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

pub(super) fn format_cursor_style(style: CursorStyleSetting) -> &'static str {
    match style {
        CursorStyleSetting::Block => "block",
        CursorStyleSetting::Underline => "underline",
        CursorStyleSetting::Bar => "bar",
    }
}

pub(super) fn format_config_list(values: &[String]) -> String {
    if values.is_empty() {
        "<none>".to_owned()
    } else {
        values.join(" ")
    }
}

pub(super) fn format_toml_string_array(values: &[String]) -> String {
    let entries = values
        .iter()
        .map(|value| format!("\"{value}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!("[{entries}]")
}
