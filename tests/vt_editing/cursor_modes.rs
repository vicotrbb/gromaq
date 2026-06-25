use gromaq::{CursorShape, Terminal, TerminalConfig};

#[test]
fn dec_private_cursor_visibility_mode_toggles_cursor_snapshot() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    assert!(terminal.dump_cursor().visible);

    terminal.write_str("\x1b[?25l").unwrap();
    assert!(!terminal.dump_cursor().visible);

    terminal.write_str("\x1b[?25h").unwrap();
    assert!(terminal.dump_cursor().visible);
}

#[test]
fn dec_private_cursor_blink_mode_toggles_cursor_snapshot() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());
    assert!(terminal.dump_cursor().blinking);

    terminal.write_str("\x1b[?12l").unwrap();
    assert!(!terminal.dump_cursor().blinking);

    terminal.write_str("\x1b[?12h").unwrap();
    assert!(terminal.dump_cursor().blinking);
}

#[test]
fn decscusr_sets_cursor_shape_and_blinking_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 2).unwrap());

    terminal.write_str("\x1b[3 q").unwrap();
    let blinking_underline = terminal.dump_cursor();
    assert_eq!(blinking_underline.shape, CursorShape::Underline);
    assert!(blinking_underline.blinking);

    terminal.write_str("\x1b[6 q").unwrap();
    let steady_bar = terminal.dump_cursor();
    assert_eq!(steady_bar.shape, CursorShape::Bar);
    assert!(!steady_bar.blinking);

    terminal.write_str("\x1b[0 q").unwrap();
    let default_block = terminal.dump_cursor();
    assert_eq!(default_block.shape, CursorShape::Block);
    assert!(default_block.blinking);
}
