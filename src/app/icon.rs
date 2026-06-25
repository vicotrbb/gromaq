use winit::window::Icon;

const GROMAQ_ICON_WIDTH: u32 = 128;
const GROMAQ_ICON_HEIGHT: u32 = 128;
const GROMAQ_ICON_RGBA: &[u8] = include_bytes!("../../images/logos/logo-icon-128.rgba");

pub(crate) fn gromaq_window_icon() -> Option<Icon> {
    Icon::from_rgba(
        GROMAQ_ICON_RGBA.to_vec(),
        GROMAQ_ICON_WIDTH,
        GROMAQ_ICON_HEIGHT,
    )
    .ok()
}
