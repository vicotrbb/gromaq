use gromaq::app::{NativeMouseButtonTracker, NativeMouseGridMapper, NativeRenderedGridMetrics};
use gromaq::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use winit::keyboard::ModifiersState;

#[test]
fn native_mouse_grid_mapper_converts_window_pixels_to_terminal_cells() {
    let mapper = NativeMouseGridMapper::new(800, 400, metrics(10, 20, 0, 0, 80, 20)).unwrap();

    assert_eq!(
        mapper.mouse_event_at(25.0, 39.0, MouseEventKind::Press, MouseButton::Left),
        Some(MouseEvent::new(
            MouseEventKind::Press,
            MouseButton::Left,
            2,
            1
        ))
    );
    assert_eq!(
        mapper.mouse_event_at(799.0, 399.0, MouseEventKind::Release, MouseButton::Left),
        Some(MouseEvent::new(
            MouseEventKind::Release,
            MouseButton::Left,
            79,
            19
        ))
    );
    assert_eq!(
        mapper.mouse_event_at_with_modifiers(
            25.0,
            39.0,
            MouseEventKind::Press,
            MouseButton::Left,
            ModifiersState::SHIFT.union(ModifiersState::ALT)
        ),
        Some(
            MouseEvent::new(MouseEventKind::Press, MouseButton::Left, 2, 1)
                .with_modifiers(KeyModifiers::SHIFT | KeyModifiers::ALT)
        )
    );
    assert_eq!(
        mapper.mouse_event_at(800.0, 399.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(-1.0, 10.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(f64::NAN, 10.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(
            10.0,
            f64::INFINITY,
            MouseEventKind::Press,
            MouseButton::Left
        ),
        None
    );
    assert_eq!(
        NativeMouseGridMapper::new(0, 400, metrics(10, 20, 0, 0, 80, 20)),
        None
    );
}

#[test]
fn native_mouse_grid_mapper_uses_rendered_padding_and_cell_metrics() {
    let mapper = NativeMouseGridMapper::new(1280, 800, metrics(16, 22, 16, 0, 78, 34)).unwrap();

    assert_eq!(
        mapper.mouse_event_at(16.0, 16.0, MouseEventKind::Press, MouseButton::Left),
        Some(MouseEvent::new(
            MouseEventKind::Press,
            MouseButton::Left,
            0,
            0
        ))
    );
    assert_eq!(
        mapper.mouse_event_at(48.0, 38.0, MouseEventKind::Press, MouseButton::Left),
        Some(MouseEvent::new(
            MouseEventKind::Press,
            MouseButton::Left,
            2,
            1
        ))
    );
    assert_eq!(
        mapper.mouse_event_at(1263.0, 763.0, MouseEventKind::Release, MouseButton::Left,),
        Some(MouseEvent::new(
            MouseEventKind::Release,
            MouseButton::Left,
            77,
            33
        ))
    );
    assert_eq!(
        mapper.mouse_event_at(15.0, 16.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(1264.0, 16.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
    assert_eq!(
        mapper.mouse_event_at(16.0, 764.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
}

#[test]
fn native_mouse_grid_mapper_accounts_for_cell_spacing() {
    let mapper = NativeMouseGridMapper::new(120, 120, metrics(10, 20, 2, 3, 4, 3)).unwrap();

    assert_eq!(
        mapper.mouse_event_at(15.0, 26.0, MouseEventKind::Press, MouseButton::Left),
        Some(MouseEvent::new(
            MouseEventKind::Press,
            MouseButton::Left,
            1,
            1
        ))
    );
    assert_eq!(
        mapper.mouse_event_at(50.0, 72.0, MouseEventKind::Press, MouseButton::Left),
        None
    );
}

#[test]
fn native_mouse_button_tracker_reports_drag_only_while_button_is_pressed() {
    let mut tracker = NativeMouseButtonTracker::default();

    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Motion, MouseButton::None)
    );

    tracker.set_pressed(MouseButton::Left, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Left)
    );

    tracker.set_pressed(MouseButton::Left, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Motion, MouseButton::None)
    );
}

#[test]
fn native_mouse_button_tracker_reports_active_drag_button_priority() {
    let mut tracker = NativeMouseButtonTracker::default();

    tracker.set_pressed(MouseButton::Right, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Right)
    );

    tracker.set_pressed(MouseButton::Middle, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Middle)
    );

    tracker.set_pressed(MouseButton::Left, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Left)
    );

    tracker.set_pressed(MouseButton::None, true);
    tracker.set_pressed(MouseButton::WheelUp, true);
    tracker.set_pressed(MouseButton::WheelDown, true);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Left)
    );

    tracker.set_pressed(MouseButton::Left, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Middle)
    );

    tracker.set_pressed(MouseButton::Middle, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Drag, MouseButton::Right)
    );

    tracker.set_pressed(MouseButton::Right, false);
    assert_eq!(
        tracker.cursor_move_event(),
        (MouseEventKind::Motion, MouseButton::None)
    );
}

fn metrics(
    cell_width_px: u16,
    line_height_px: u16,
    surface_padding_px: u16,
    cell_spacing_px: u16,
    cols: u16,
    rows: u16,
) -> NativeRenderedGridMetrics {
    NativeRenderedGridMetrics {
        cell_width_px,
        line_height_px,
        surface_padding_px,
        cell_spacing_px,
        cols,
        rows,
    }
}
