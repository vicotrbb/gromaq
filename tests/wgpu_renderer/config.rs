use gromaq::GromaqConfig;
use gromaq::renderer::RendererConfig;

use crate::support::linear_clear_color;

#[test]
fn renderer_config_maps_validated_gromaq_settings() {
    let mut config = GromaqConfig::default();
    config.performance.target_fps = 120;
    config.performance.dirty_region_rendering = false;
    config.font.size_px = 16.5;
    config.font.cell_width_px = Some(9.5);
    config.font.line_height_px = 21.0;
    config.theme.background = "#1f2028".to_owned();
    config.theme.foreground = "#e8e2d6".to_owned();
    config.theme.cursor = "#f4c06a".to_owned();
    config.theme.selection = "#26364f".to_owned();
    config.theme.background_opacity = 0.42;
    config.theme.cursor_opacity = 0.5;
    config.theme.selection_opacity = 0.25;
    config.theme.ansi[1] = "#010203".to_owned();
    config.theme.surface_padding_px = 18;
    config.theme.cell_spacing_px = 2;
    config.theme.dim_opacity = 0.42;

    let renderer_config = RendererConfig::from_gromaq_config(&config).unwrap();

    assert_eq!(renderer_config.target_fps, 120);
    assert!(!renderer_config.dirty_regions);
    assert_eq!(renderer_config.font_size_px, 17);
    assert_eq!(renderer_config.cell_width_px, 10);
    assert_eq!(renderer_config.line_height_px, 21);
    let mut expected_clear = linear_clear_color(31, 32, 40);
    expected_clear[3] = f64::from(0.42f32);
    assert_eq!(renderer_config.clear_color, expected_clear);
    assert_eq!(renderer_config.default_foreground_rgb8, [232, 226, 214]);
    assert_eq!(renderer_config.ansi_colors_rgb8[1], [1, 2, 3]);
    assert_eq!(renderer_config.cursor_color_rgba8, [244, 192, 106, 128]);
    assert_eq!(renderer_config.selection_background_rgba8, [38, 54, 79, 64]);
    assert_eq!(renderer_config.surface_padding_px, 18);
    assert_eq!(renderer_config.cell_spacing_px, 2);
    assert_eq!(renderer_config.dim_opacity, 0.42);
}

#[test]
fn renderer_default_cell_width_is_compact_for_monospace_text() {
    let config = RendererConfig::default();

    assert_eq!(config.font_size_px, 32);
    assert_eq!(config.cell_width_px, 18);
    assert_eq!(config.line_height_px, 44);
    assert!(config.cell_width_px < config.font_size_px);
}

#[test]
fn renderer_default_theme_matches_default_gromaq_config() {
    let default_renderer = RendererConfig::default();
    let mapped_renderer = RendererConfig::from_gromaq_config(&GromaqConfig::default()).unwrap();

    assert_eq!(default_renderer.clear_color, mapped_renderer.clear_color);
    assert_eq!(
        default_renderer.default_foreground_rgb8,
        mapped_renderer.default_foreground_rgb8
    );
    assert_eq!(
        default_renderer.cursor_color_rgba8,
        mapped_renderer.cursor_color_rgba8
    );
    assert_eq!(
        default_renderer.selection_background_rgba8,
        mapped_renderer.selection_background_rgba8
    );
    assert_eq!(
        default_renderer.ansi_colors_rgb8,
        mapped_renderer.ansi_colors_rgb8
    );
    assert_eq!(
        default_renderer.surface_padding_px,
        mapped_renderer.surface_padding_px
    );
    assert_eq!(
        default_renderer.cell_spacing_px,
        mapped_renderer.cell_spacing_px
    );
    assert_eq!(default_renderer.dim_opacity, mapped_renderer.dim_opacity);
    assert_eq!(
        default_renderer.cell_width_px,
        mapped_renderer.cell_width_px
    );
}
