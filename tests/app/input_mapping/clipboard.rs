use gromaq::app::{is_native_copy_shortcut, is_native_paste_shortcut};
use winit::keyboard::{Key, ModifiersState, NamedKey};

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
