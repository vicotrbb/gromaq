use gromaq::app::{
    NativeMouseButtonTracker, NativeMouseGridMapper, NativePtyResize, NativeResizeGridMapper,
    NativeTextZoomAction, is_native_copy_shortcut, is_native_paste_shortcut,
    native_text_zoom_action,
};
use gromaq::{KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use winit::keyboard::{Key, ModifiersState, NamedKey};

#[test]
fn native_mouse_grid_mapper_converts_window_pixels_to_terminal_cells() {
    let mapper = NativeMouseGridMapper::new(800, 400, 10, 20, 0, 80, 20).unwrap();

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
    assert_eq!(NativeMouseGridMapper::new(0, 400, 10, 20, 0, 80, 20), None);
}

#[test]
fn native_mouse_grid_mapper_uses_rendered_padding_and_cell_metrics() {
    let mapper = NativeMouseGridMapper::new(1280, 800, 16, 22, 16, 78, 34).unwrap();

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

#[test]
fn native_resize_grid_mapper_fits_cells_inside_window_padding() {
    let mapper = NativeResizeGridMapper::new(14, 18, 14).unwrap();

    assert_eq!(
        mapper.resize_for_window(1280, 800),
        Some(NativePtyResize {
            cols: 89,
            rows: 42,
            pixel_width: 1280,
            pixel_height: 800,
        })
    );
    assert_eq!(
        mapper.resize_for_window(640, 400),
        Some(NativePtyResize {
            cols: 43,
            rows: 20,
            pixel_width: 640,
            pixel_height: 400,
        })
    );
    assert_eq!(
        mapper.resize_for_window(20, 20),
        Some(NativePtyResize {
            cols: 1,
            rows: 1,
            pixel_width: 20,
            pixel_height: 20,
        })
    );
    assert_eq!(mapper.resize_for_window(0, 400), None);
    assert_eq!(NativeResizeGridMapper::new(0, 18, 14), None);
}

#[test]
fn native_paste_shortcut_accepts_control_or_super_v() {
    assert!(is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::CONTROL
    ));
    assert!(is_native_paste_shortcut(
        &Key::Character("V".into()),
        ModifiersState::SUPER
    ));
    assert!(is_native_paste_shortcut(
        &Key::Character("V".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::empty()
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::CONTROL | ModifiersState::ALT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::SUPER | ModifiersState::ALT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Character("v".into()),
        ModifiersState::CONTROL | ModifiersState::SUPER
    ));
}

#[test]
fn native_copy_shortcut_accepts_super_c_or_control_shift_c_without_plain_control_c() {
    assert!(is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::SUPER
    ));
    assert!(is_native_copy_shortcut(
        &Key::Character("C".into()),
        ModifiersState::CONTROL.union(ModifiersState::SHIFT)
    ));
    assert!(is_native_copy_shortcut(
        &Key::Named(NamedKey::Copy),
        ModifiersState::empty()
    ));
    assert!(is_native_copy_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::CONTROL
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::empty()
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("v".into()),
        ModifiersState::SUPER
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::SUPER | ModifiersState::ALT
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT
    ));
    assert!(!is_native_copy_shortcut(
        &Key::Character("c".into()),
        ModifiersState::CONTROL | ModifiersState::SUPER
    ));
}

#[test]
fn native_text_zoom_shortcuts_match_browser_controls() {
    assert_eq!(
        native_text_zoom_action(&Key::Character("+".into()), ModifiersState::SUPER),
        Some(NativeTextZoomAction::Increase)
    );
    assert_eq!(
        native_text_zoom_action(&Key::Character("=".into()), ModifiersState::CONTROL),
        Some(NativeTextZoomAction::Increase)
    );
    assert_eq!(
        native_text_zoom_action(&Key::Character("-".into()), ModifiersState::SUPER),
        Some(NativeTextZoomAction::Decrease)
    );
    assert_eq!(
        native_text_zoom_action(&Key::Character("0".into()), ModifiersState::CONTROL),
        Some(NativeTextZoomAction::Reset)
    );
    assert_eq!(
        native_text_zoom_action(&Key::Character("+".into()), ModifiersState::empty()),
        None
    );
    assert_eq!(
        native_text_zoom_action(
            &Key::Character("+".into()),
            ModifiersState::CONTROL | ModifiersState::SUPER
        ),
        None
    );
    assert_eq!(
        native_text_zoom_action(
            &Key::Character("+".into()),
            ModifiersState::CONTROL | ModifiersState::ALT
        ),
        None
    );
}

#[test]
fn native_paste_shortcut_accepts_dedicated_paste_key() {
    assert!(is_native_paste_shortcut(
        &Key::Named(NamedKey::Paste),
        ModifiersState::empty()
    ));
    assert!(is_native_paste_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::SHIFT
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::empty()
    ));
    assert!(!is_native_paste_shortcut(
        &Key::Named(NamedKey::Insert),
        ModifiersState::SHIFT | ModifiersState::CONTROL
    ));
}
