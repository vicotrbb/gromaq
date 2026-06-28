use super::*;

#[test]
fn ansi_mode_reports_return_insert_mode_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b[4$p\x1b[4h\x1b[4$p\x1b[20$p\x1b[20h\x1b[20$p\x1b[999$p")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[4;2$y\x1b[4;1$y\x1b[20;2$y\x1b[20;1$y\x1b[999;0$y"
    );
}

#[test]
fn dec_private_mode_reports_return_mode_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?7$p\x1b[?7l\x1b[?7$p\x1b[?66$p\x1b=\x1b[?66$p\x1b>\x1b[?66$p\x1b[?66h\x1b[?66$p\x1b[?2004h\x1b[?2004$p\x1b[?999$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?7;1$y\x1b[?7;2$y\x1b[?66;2$y\x1b[?66;1$y\x1b[?66;2$y\x1b[?66;1$y\x1b[?2004;1$y\x1b[?999;0$y"
    );
}

#[test]
fn dec_private_mode_reports_include_alternate_screen_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?47$p\x1b[?1047$p\x1b[?1049$p\x1b[?1049h\x1b[?47$p\x1b[?1047$p\x1b[?1049$p\x1b[?1049l\x1b[?1049$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?47;2$y\x1b[?1047;2$y\x1b[?1049;2$y\x1b[?47;1$y\x1b[?1047;1$y\x1b[?1049;1$y\x1b[?1049;2$y"
    );
}

#[test]
fn dec_private_mode_reports_include_cursor_state_modes() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?1$p\x1b[?6$p\x1b[?12$p\x1b[?25$p\
             \x1b[?1h\x1b[?6h\x1b[?12h\x1b[?25l\
             \x1b[?1$p\x1b[?6$p\x1b[?12$p\x1b[?25$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?1;2$y\x1b[?6;2$y\x1b[?12;1$y\x1b[?25;1$y\
          \x1b[?1;1$y\x1b[?6;1$y\x1b[?12;1$y\x1b[?25;2$y"
    );
}

#[test]
fn dec_private_mode_reports_include_mouse_and_focus_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str(
            "\x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1006$p\
             \x1b[?1000h\x1b[?1002h\x1b[?1003h\x1b[?1004h\x1b[?1006h\
             \x1b[?1000$p\x1b[?1002$p\x1b[?1003$p\x1b[?1004$p\x1b[?1006$p",
        )
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?1000;2$y\x1b[?1002;2$y\x1b[?1003;2$y\x1b[?1004;2$y\x1b[?1006;2$y\
          \x1b[?1000;1$y\x1b[?1002;1$y\x1b[?1003;1$y\x1b[?1004;1$y\x1b[?1006;1$y"
    );
}

#[test]
fn dec_private_mode_reports_include_x10_mouse_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(8, 3).unwrap());

    terminal
        .write_str("\x1b[?9$p\x1b[?9h\x1b[?9$p\x1b[?9l\x1b[?9$p")
        .unwrap();

    assert_eq!(
        terminal.take_pending_response_bytes(),
        b"\x1b[?9;2$y\x1b[?9;1$y\x1b[?9;2$y"
    );
}
