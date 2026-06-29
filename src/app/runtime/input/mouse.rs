use winit::keyboard::ModifiersState;

use crate::app::native_input::{
    NativeMouseGridMapper, NativeRenderedGridMetrics, NativeWindowMouseInput,
    NativeWindowMouseInputResult, clamp_u32_to_u16,
};
use crate::app::{NativeAppError, NativePtySessionIo};
use crate::mouse::{MouseButton, MouseEventKind};
use crate::{MouseEvent, SelectionPoint, SelectionRange};

use super::super::NativeTerminalRuntime;

impl<S> NativeTerminalRuntime<S>
where
    S: NativePtySessionIo,
{
    /// Map a native window mouse position to a terminal event and write its report to the PTY.
    pub fn send_window_mouse_input(
        &mut self,
        x: f64,
        y: f64,
        window_width_px: u32,
        window_height_px: u32,
        kind: MouseEventKind,
        button: MouseButton,
    ) -> Result<bool, NativeAppError> {
        self.send_window_mouse_input_event(NativeWindowMouseInput {
            x,
            y,
            window_width_px,
            window_height_px,
            cell_width_px: inferred_cell_size_px(window_width_px, self.terminal.dump_grid().cols),
            line_height_px: inferred_cell_size_px(window_height_px, self.terminal.dump_grid().rows),
            surface_padding_px: 0,
            cell_spacing_px: 0,
            kind,
            button,
            modifiers: ModifiersState::empty(),
        })
    }

    /// Map native window mouse input to a terminal event and write its report.
    pub fn send_window_mouse_input_event(
        &mut self,
        input: NativeWindowMouseInput,
    ) -> Result<bool, NativeAppError> {
        self.send_window_mouse_input_event_result(input)
            .map(|result| result.handled)
    }

    /// Map native window mouse input and report whether it changed visible terminal state.
    pub fn send_window_mouse_input_event_result(
        &mut self,
        input: NativeWindowMouseInput,
    ) -> Result<NativeWindowMouseInputResult, NativeAppError> {
        let grid = self.terminal.dump_grid();
        let Some(mapper) = NativeMouseGridMapper::new(
            input.window_width_px,
            input.window_height_px,
            NativeRenderedGridMetrics {
                cell_width_px: input.cell_width_px,
                line_height_px: input.line_height_px,
                surface_padding_px: input.surface_padding_px,
                cell_spacing_px: input.cell_spacing_px,
                cols: grid.cols,
                rows: grid.rows,
            },
        ) else {
            return Ok(NativeWindowMouseInputResult::default());
        };
        let Some(event) = mapper.mouse_event_at_with_modifiers(
            input.x,
            input.y,
            input.kind,
            input.button,
            input.modifiers,
        ) else {
            return Ok(NativeWindowMouseInputResult::default());
        };
        if self.send_mouse_input(event)? {
            self.selection_drag_anchor = None;
            return Ok(NativeWindowMouseInputResult {
                handled: true,
                needs_redraw: false,
            });
        }
        Ok(match (input.kind, input.button) {
            (MouseEventKind::Press, MouseButton::WheelUp) => self.scroll_display_up_result(1),
            (MouseEventKind::Press, MouseButton::WheelDown) => self.scroll_display_down_result(1),
            _ => self.apply_local_mouse_selection(event),
        })
    }

    fn scroll_display_up_result(&mut self, rows: u16) -> NativeWindowMouseInputResult {
        let changed = self.terminal.scroll_display_up(rows);
        NativeWindowMouseInputResult {
            handled: changed,
            needs_redraw: changed,
        }
    }

    fn scroll_display_down_result(&mut self, rows: u16) -> NativeWindowMouseInputResult {
        let changed = self.terminal.scroll_display_down(rows);
        NativeWindowMouseInputResult {
            handled: changed,
            needs_redraw: changed,
        }
    }

    fn apply_local_mouse_selection(&mut self, event: MouseEvent) -> NativeWindowMouseInputResult {
        match (event.kind, event.button) {
            (MouseEventKind::Press, MouseButton::Left) => {
                self.selection_drag_anchor = Some(selection_point(event));
                NativeWindowMouseInputResult {
                    handled: true,
                    needs_redraw: self.terminal.clear_selection(),
                }
            }
            (MouseEventKind::Drag, MouseButton::Left) => {
                let Some(anchor) = self.selection_drag_anchor else {
                    return NativeWindowMouseInputResult::default();
                };
                self.terminal
                    .set_selection(selection_range(anchor, selection_point(event)));
                NativeWindowMouseInputResult {
                    handled: true,
                    needs_redraw: true,
                }
            }
            (MouseEventKind::Release, MouseButton::Left) => NativeWindowMouseInputResult {
                handled: self.selection_drag_anchor.take().is_some(),
                needs_redraw: false,
            },
            _ => NativeWindowMouseInputResult::default(),
        }
    }
}

fn selection_point(event: MouseEvent) -> SelectionPoint {
    SelectionPoint {
        row: event.row,
        col: event.col,
    }
}

fn selection_range(start: SelectionPoint, end: SelectionPoint) -> SelectionRange {
    SelectionRange::new((start.row, start.col), (end.row, end.col))
}

fn inferred_cell_size_px(window_px: u32, cells: u16) -> u16 {
    if cells == 0 {
        return 0;
    }
    clamp_u32_to_u16((window_px / u32::from(cells)).max(1))
}
