//! Mouse proof helpers for the native tmux UI smoke.

use winit::keyboard::ModifiersState;

use crate::app::NativeWindowMouseInput;
use crate::{MouseButton, MouseEventKind};

const SMOKE_WINDOW_WIDTH_PX: u32 = 2200;
const SMOKE_WINDOW_HEIGHT_PX: u32 = 100;
const SMOKE_CELL_WIDTH_PX: u16 = 10;
const SMOKE_LINE_HEIGHT_PX: u16 = 10;
const MANAGER_START_ROW: u16 = 2;
const MANAGER_WINDOW_ROW: u16 = MANAGER_START_ROW + 2;
const MANAGER_WORKSPACE_ROW: u16 = MANAGER_START_ROW + 4;
const MANAGER_ACTION_ROW: u16 = MANAGER_START_ROW + 5;
const KILL_WINDOW_ACTION_COL: u16 = 90;
const DOCS_WORKSPACE_COL: u16 = 95;

pub(super) fn drive_mouse_focus(runtime: &mut super::SmokeRuntime) -> bool {
    send_manager_mouse_press(runtime, 1, MANAGER_WINDOW_ROW)
}

pub(super) fn drive_mouse_action_selection(runtime: &mut super::SmokeRuntime) -> bool {
    send_manager_mouse_press(runtime, KILL_WINDOW_ACTION_COL, MANAGER_ACTION_ROW)
}

pub(super) fn drive_mouse_workspace_selection(runtime: &mut super::SmokeRuntime) -> bool {
    send_manager_mouse_press(runtime, DOCS_WORKSPACE_COL, MANAGER_WORKSPACE_ROW)
}

fn send_manager_mouse_press(runtime: &mut super::SmokeRuntime, col: u16, row: u16) -> bool {
    runtime
        .send_window_mouse_input_event_result(NativeWindowMouseInput {
            x: f64::from(col) * f64::from(SMOKE_CELL_WIDTH_PX) + 1.0,
            y: f64::from(row) * f64::from(SMOKE_LINE_HEIGHT_PX) + 1.0,
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
