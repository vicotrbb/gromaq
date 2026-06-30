use gromaq::app::{NativeTmuxAssistAction, native_tmux_assist_action};
use winit::keyboard::{Key, KeyCode, ModifiersState, PhysicalKey};

#[test]
fn native_tmux_assist_shortcut_uses_modified_t_without_plain_control_t() {
    assert_eq!(
        native_tmux_assist_action(
            &Key::Character("t".into()),
            None,
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ),
        Some(NativeTmuxAssistAction::ToggleManager)
    );
    assert_eq!(
        native_tmux_assist_action(
            &Key::Character("T".into()),
            None,
            ModifiersState::SUPER | ModifiersState::SHIFT
        ),
        Some(NativeTmuxAssistAction::ToggleManager)
    );
    assert_eq!(
        native_tmux_assist_action(&Key::Character("t".into()), None, ModifiersState::CONTROL),
        None
    );
    assert_eq!(
        native_tmux_assist_action(
            &Key::Character("t".into()),
            None,
            ModifiersState::CONTROL | ModifiersState::SHIFT | ModifiersState::ALT
        ),
        None
    );
}

#[test]
fn native_tmux_assist_shortcut_accepts_physical_t_when_logical_text_is_modified() {
    assert_eq!(
        native_tmux_assist_action(
            &Key::Character("\u{14}".into()),
            Some(PhysicalKey::Code(KeyCode::KeyT)),
            ModifiersState::CONTROL | ModifiersState::SHIFT
        ),
        Some(NativeTmuxAssistAction::ToggleManager)
    );
}
