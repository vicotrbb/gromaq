use crate::app::NativeTextZoomAction;
use crate::config::{
    MAX_CELL_WIDTH_PX, MAX_FONT_SIZE_PX, MAX_LINE_HEIGHT_PX, MIN_CELL_WIDTH_PX, MIN_FONT_SIZE_PX,
    MIN_LINE_HEIGHT_PX,
};
use crate::renderer::RendererConfig;

const TEXT_ZOOM_STEP: f32 = 1.15;

pub(super) fn renderer_config_for_text_zoom(
    config: &RendererConfig,
    action: NativeTextZoomAction,
) -> RendererConfig {
    match action {
        NativeTextZoomAction::Increase => scaled_renderer_font_metrics(config, TEXT_ZOOM_STEP),
        NativeTextZoomAction::Decrease => {
            scaled_renderer_font_metrics(config, 1.0 / TEXT_ZOOM_STEP)
        }
        NativeTextZoomAction::Reset => default_renderer_font_metrics(config),
    }
}

fn scaled_renderer_font_metrics(config: &RendererConfig, factor: f32) -> RendererConfig {
    let font_size_px = scaled_metric(
        config.font_size_px,
        factor,
        MIN_FONT_SIZE_PX,
        MAX_FONT_SIZE_PX,
    );
    let font_size_ratio = f32::from(font_size_px) / f32::from(config.font_size_px.max(1));
    let mut next = config.clone();
    next.font_size_px = font_size_px;
    next.cell_width_px = scaled_metric(
        config.cell_width_px,
        font_size_ratio,
        MIN_CELL_WIDTH_PX,
        MAX_CELL_WIDTH_PX,
    );
    next.line_height_px = scaled_metric(
        config.line_height_px,
        font_size_ratio,
        MIN_LINE_HEIGHT_PX.max(f32::from(next.font_size_px)),
        MAX_LINE_HEIGHT_PX,
    );
    next
}

fn default_renderer_font_metrics(config: &RendererConfig) -> RendererConfig {
    let defaults = RendererConfig::default();
    let mut next = config.clone();
    next.font_size_px = defaults.font_size_px;
    next.cell_width_px = defaults.cell_width_px;
    next.line_height_px = defaults.line_height_px;
    next
}

fn scaled_metric(value: u16, factor: f32, minimum: f32, maximum: f32) -> u16 {
    (f32::from(value) * factor).round().clamp(minimum, maximum) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn text_zoom_scaling_clamps_to_valid_renderer_metrics() {
        let config = RendererConfig {
            font_size_px: MAX_FONT_SIZE_PX as u16,
            cell_width_px: MAX_CELL_WIDTH_PX as u16,
            line_height_px: MAX_LINE_HEIGHT_PX as u16,
            ..RendererConfig::default()
        };

        let increased = renderer_config_for_text_zoom(&config, NativeTextZoomAction::Increase);

        assert_eq!(increased.font_size_px, MAX_FONT_SIZE_PX as u16);
        assert_eq!(increased.cell_width_px, MAX_CELL_WIDTH_PX as u16);
        assert_eq!(increased.line_height_px, MAX_LINE_HEIGHT_PX as u16);

        let config = RendererConfig {
            font_size_px: MIN_FONT_SIZE_PX as u16,
            cell_width_px: MIN_CELL_WIDTH_PX as u16,
            line_height_px: MIN_LINE_HEIGHT_PX as u16,
            ..RendererConfig::default()
        };

        let decreased = renderer_config_for_text_zoom(&config, NativeTextZoomAction::Decrease);

        assert_eq!(decreased.font_size_px, MIN_FONT_SIZE_PX as u16);
        assert_eq!(decreased.cell_width_px, MIN_CELL_WIDTH_PX as u16);
        assert_eq!(decreased.line_height_px, MIN_LINE_HEIGHT_PX as u16);
    }

    #[test]
    fn text_zoom_reset_keeps_non_metric_renderer_settings() {
        let config = RendererConfig {
            font_size_px: 42,
            cell_width_px: 24,
            line_height_px: 58,
            dirty_regions: false,
            surface_padding_px: 22,
            ..RendererConfig::default()
        };

        let reset = renderer_config_for_text_zoom(&config, NativeTextZoomAction::Reset);

        assert_eq!(reset.font_size_px, RendererConfig::default().font_size_px);
        assert_eq!(reset.cell_width_px, RendererConfig::default().cell_width_px);
        assert_eq!(
            reset.line_height_px,
            RendererConfig::default().line_height_px
        );
        assert!(!reset.dirty_regions);
        assert_eq!(reset.surface_padding_px, 22);
    }
}
