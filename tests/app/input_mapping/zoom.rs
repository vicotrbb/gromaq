use gromaq::app::{NativeTextZoomAction, native_text_zoom_action, native_wheel_text_zoom_action};
use winit::dpi::PhysicalPosition;
use winit::event::MouseScrollDelta;
use winit::keyboard::{Key, ModifiersState, NamedKey};

#[test]
fn native_text_zoom_shortcuts_match_browser_controls() {
    assert_eq!(
        native_text_zoom_action(&Key::Character("+".into()), ModifiersState::SUPER),
        Some(NativeTextZoomAction::Increase)
    );
    assert_eq!(
        native_text_zoom_action(
            &Key::Character("+".into()),
            ModifiersState::SUPER | ModifiersState::SHIFT
        ),
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
        native_text_zoom_action(&Key::Named(NamedKey::ZoomIn), ModifiersState::empty()),
        Some(NativeTextZoomAction::Increase)
    );
    assert_eq!(
        native_text_zoom_action(&Key::Named(NamedKey::ZoomOut), ModifiersState::empty()),
        Some(NativeTextZoomAction::Decrease)
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
    assert_eq!(
        native_text_zoom_action(&Key::Named(NamedKey::ZoomIn), ModifiersState::CONTROL),
        None
    );
}

#[test]
fn native_wheel_text_zoom_shortcuts_match_browser_controls() {
    assert_eq!(
        native_wheel_text_zoom_action(
            &MouseScrollDelta::LineDelta(0.0, 1.0),
            ModifiersState::CONTROL
        ),
        Some(NativeTextZoomAction::Increase)
    );
    assert_eq!(
        native_wheel_text_zoom_action(
            &MouseScrollDelta::PixelDelta(PhysicalPosition::new(0.0, -10.0)),
            ModifiersState::SUPER
        ),
        Some(NativeTextZoomAction::Decrease)
    );
    assert_eq!(
        native_wheel_text_zoom_action(
            &MouseScrollDelta::LineDelta(0.0, 0.0),
            ModifiersState::CONTROL
        ),
        None
    );
    assert_eq!(
        native_wheel_text_zoom_action(
            &MouseScrollDelta::LineDelta(0.0, 1.0),
            ModifiersState::empty()
        ),
        None
    );
    assert_eq!(
        native_wheel_text_zoom_action(
            &MouseScrollDelta::LineDelta(0.0, 1.0),
            ModifiersState::CONTROL | ModifiersState::SUPER
        ),
        None
    );
    assert_eq!(
        native_wheel_text_zoom_action(
            &MouseScrollDelta::LineDelta(0.0, 1.0),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ),
        None
    );
}
