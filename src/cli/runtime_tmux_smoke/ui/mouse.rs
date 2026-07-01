//! Mouse proof helpers for the native tmux UI smoke.

use winit::keyboard::ModifiersState;

use crate::app::NativeWindowMouseInput;
use crate::{MouseButton, MouseEventKind};

const SMOKE_WINDOW_WIDTH_PX: u32 = 960;
const SMOKE_WINDOW_HEIGHT_PX: u32 = 100;
const SMOKE_CELL_WIDTH_PX: u16 = 10;
const SMOKE_LINE_HEIGHT_PX: u16 = 10;
const MANAGER_START_ROW: u16 = 2;
const MANAGER_WINDOW_ROW: u16 = MANAGER_START_ROW + 2;

pub(super) fn drive_mouse_focus(runtime: &mut super::SmokeRuntime) -> bool {
    runtime
        .send_window_mouse_input_event_result(NativeWindowMouseInput {
            x: 1.0,
            y: f64::from(MANAGER_WINDOW_ROW) * f64::from(SMOKE_LINE_HEIGHT_PX) + 1.0,
            window_width_px: SMOKE_WINDOW_WIDTH_PX,
            window_height_px: SMOKE_WINDOW_HEIGHT_PX,
            cell_width_px: SMOKE_CELL_WIDTH_PX,
            line_height_px: SMOKE_LINE_HEIGHT_PX,
            surface_padding_px: 0,
            cell_spacing_px: 0,
            kind: MouseEventKind::Press,
            button: MouseButton::Left,
            modifiers: ModifiersState::empty(),
        })
        .is_ok_and(|result| result.handled && result.needs_redraw)
}
