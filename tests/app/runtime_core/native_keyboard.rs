use super::*;

fn runtime_with_shell() -> NativeTerminalRuntime<MockPtySession> {
    let spawner = MockPtySpawner::default();
    let mut runtime = NativeTerminalRuntime::new(NativeTerminalRuntimeConfig {
        terminal_cols: 20,
        terminal_rows: 4,
        scrollback_lines: 100,
        pixel_width: 0,
        pixel_height: 0,
        cursor_shape: NativeTerminalRuntimeConfig::default().cursor_shape,
        cursor_blinking: NativeTerminalRuntimeConfig::default().cursor_blinking,
        shell: ShellCommand {
            program: "/bin/sh".into(),
            args: Vec::new(),
            cwd: None,
        },
    })
    .unwrap();
    runtime.start_shell(&spawner).unwrap();
    runtime
}

#[test]
fn native_terminal_runtime_routes_plain_printable_key_input() {
    let mut runtime = runtime_with_shell();

    assert!(
        runtime
            .send_native_key_event_input(
                &Key::Character("l".into()),
                None,
                ModifiersState::empty(),
                false,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"l".to_vec()]);
}

#[test]
fn native_terminal_runtime_defers_printable_key_input_during_active_ime_preedit() {
    let mut runtime = runtime_with_shell();

    assert!(
        !runtime
            .send_native_key_event_input(
                &Key::Character("l".into()),
                None,
                ModifiersState::empty(),
                true,
            )
            .unwrap()
    );
    runtime.send_committed_text("l").unwrap();

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"l".to_vec()]);
}

#[test]
fn native_terminal_runtime_routes_plain_typing_through_key_events() {
    let mut runtime = runtime_with_shell();

    for ch in ["l", "s"] {
        assert!(
            runtime
                .send_native_key_event_input(
                    &Key::Character(ch.into()),
                    None,
                    ModifiersState::empty(),
                    false,
                )
                .unwrap()
        );
    }

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"l".to_vec(), b"s".to_vec()]
    );
}

#[test]
fn native_terminal_runtime_preserves_control_character_input() {
    let mut runtime = runtime_with_shell();

    assert!(
        runtime
            .send_native_key_event_input(
                &Key::Character("c".into()),
                None,
                ModifiersState::CONTROL,
                false,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[vec![0x03]]);
}

#[test]
fn native_terminal_runtime_preserves_named_key_input() {
    let mut runtime = runtime_with_shell();

    let cases = [
        (Key::Named(NamedKey::Enter), b"\r".to_vec()),
        (Key::Named(NamedKey::Backspace), vec![0x7f]),
        (Key::Named(NamedKey::ArrowUp), b"\x1b[A".to_vec()),
        (Key::Named(NamedKey::Tab), b"\t".to_vec()),
        (Key::Named(NamedKey::PageUp), b"\x1b[5~".to_vec()),
        (Key::Named(NamedKey::PageDown), b"\x1b[6~".to_vec()),
        (Key::Named(NamedKey::F5), b"\x1b[15~".to_vec()),
    ];

    for (key, expected) in cases {
        assert!(
            runtime
                .send_native_key_event_input(&key, None, ModifiersState::empty(), false)
                .unwrap()
        );
        let session = runtime.shell_session().unwrap();
        assert_eq!(session.input.borrow().last().unwrap(), &expected);
    }
}

#[test]
fn native_terminal_runtime_preserves_alt_modified_printable_input() {
    let mut runtime = runtime_with_shell();

    assert!(
        runtime
            .send_native_key_event_input(
                &Key::Character("x".into()),
                None,
                ModifiersState::ALT,
                false,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"\x1bx".to_vec()]);
}

#[test]
fn native_terminal_runtime_routes_shift_printable_input_without_active_ime_preedit() {
    let mut runtime = runtime_with_shell();

    assert!(
        runtime
            .send_native_key_event_input(
                &Key::Character("L".into()),
                None,
                ModifiersState::SHIFT,
                false,
            )
            .unwrap()
    );

    let session = runtime.shell_session().unwrap();
    assert_eq!(session.input.borrow().as_slice(), &[b"L".to_vec()]);
}

#[test]
fn native_terminal_runtime_preserves_unicode_committed_text_input() {
    let mut runtime = runtime_with_shell();

    assert!(
        runtime
            .send_native_key_event_input(
                &Key::Character("界".into()),
                None,
                ModifiersState::empty(),
                false,
            )
            .unwrap()
    );
    runtime.send_committed_text("界\u{0301}🙂").unwrap();

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &["界".as_bytes().to_vec(), "界\u{0301}🙂".as_bytes().to_vec()]
    );
}

#[test]
fn native_terminal_runtime_keeps_paste_paths_separate_from_committed_text() {
    let mut runtime = runtime_with_shell();
    runtime
        .shell_session()
        .unwrap()
        .output
        .borrow_mut()
        .push_back(b"\x1b[?2004h".to_vec());
    runtime.pump_pty_output().unwrap();
    runtime.pump_pty_output().unwrap();

    runtime.send_paste_text("paste").unwrap();
    runtime.send_committed_text("typed").unwrap();

    let session = runtime.shell_session().unwrap();
    assert_eq!(
        session.input.borrow().as_slice(),
        &[b"\x1b[200~paste\x1b[201~".to_vec(), b"typed".to_vec(),]
    );
}
