use gromaq::{Terminal, TerminalConfig};

use super::formatting::{
    format_dec_origin_scroll_region_golden, format_osc_clipboard_paste_golden,
    format_status_capability_reports_golden, format_terminal_golden,
    format_vt_editing_status_golden, format_vt_unicode_osc_golden,
};

#[test]
fn terminal_state_matches_ansi_scrollback_and_alternate_screen_golden() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(12, 4)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal
        .write_str(
            "\
plain-one\r\n\
\x1b[31;1mred-bold\x1b[0m\r\n\
\x1b]8;;https://gromaq.dev\x1b\\linked\x1b]8;;\x1b\\\r\n\
wide 界\r\n\
fifth\r\n\
\x1b[?1049halt-screen\x1b[?1049lrestored",
        )
        .unwrap();

    assert_eq!(
        format_terminal_golden(&terminal),
        include_str!("../fixtures/terminal_golden/ansi_scrollback_alternate.txt")
    );
}

#[test]
fn terminal_state_matches_vt_unicode_osc_golden() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(12, 4)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal
        .write_str(
            "\
\x1b[38;5;45;48:2:1:2:3;3;4:3;58:2:17:34:51mstyle-row\x1b[0m\r\n\
abcde\x1b[2DXY\r\n\
\x1b]8;;https://gromaq.dev/docs\x1b\\link\x1b]8;;\x1b\\\r\n\
emoji 👨\u{200d}👩🏽\r\n\
tag 🏴\u{e0067}\u{e0062}\u{e0065}\u{e006e}\u{e0067}\u{e007f} ok\r\n\
tail",
        )
        .unwrap();

    assert_eq!(
        format_vt_unicode_osc_golden(&terminal),
        include_str!("../fixtures/terminal_golden/vt_unicode_osc.txt")
    );
}

#[test]
fn terminal_state_matches_vt_editing_status_golden() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(12, 5)
            .unwrap()
            .with_scrollback_limit(6)
            .unwrap(),
    );

    terminal
        .write_str(
            "\
\x1b[6 q\x1b[?25l\
\x1b[3g\x1b[1;6H\x1bH\x1b[1;1H\tT\
\x1b[2;1Habcdef\x1b[2;3H\x1b[4hXY\x1b[4l\
\x1b[3;1Habcdef\x1b[3;3H\x1b[2P\
\x1b[4;1H\x1b(0lqk\x1b(BZ\
\x1b[5;1Hbottom\
\x1b[2;4r\x1b[5;7H\
\x1bP$q q\x1b\\\x1bP$qr\x1b\\\x1b[6n",
        )
        .unwrap();

    assert_eq!(
        format_vt_editing_status_golden(&mut terminal),
        include_str!("../fixtures/terminal_golden/vt_editing_status.txt")
    );
}

#[test]
fn terminal_state_matches_dec_origin_scroll_region_golden() {
    let mut terminal = Terminal::new(TerminalConfig::new(10, 5).unwrap());

    terminal
        .write_str(
            "\
\x1b[1;1Htop\
\x1b[2;1Hone\
\x1b[3;1Htwo\
\x1b[4;1Hthree\
\x1b[5;1Hbottom\
\x1b[2;4r\
\x1b[?6h\
\x1b[1;1HZ\
\x1b[3;1HQ\
\n\
\x1b[?6$p\
\x1b[?6l\
\x1b[1;1HV\
\x1b[?6$p",
        )
        .unwrap();

    assert_eq!(
        format_dec_origin_scroll_region_golden(&mut terminal),
        include_str!("../fixtures/terminal_golden/dec_origin_scroll_region.txt")
    );
}

#[test]
fn terminal_state_matches_osc_clipboard_paste_golden() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 3).unwrap());

    terminal
        .write_str(
            "\
\x1b]0;Icon Title\x1b\\\
\x1b]1;Icon Only\x1b\\\
\x1b]2;Window Title\x07\
\x1b]52;c;SGVsbG8=\x07\
\x1b[?2004h\
\x1b[21t\x1b[20t\
ok",
        )
        .unwrap();

    assert_eq!(
        format_osc_clipboard_paste_golden(&mut terminal),
        include_str!("../fixtures/terminal_golden/osc_clipboard_paste.txt")
    );
}

#[test]
fn terminal_state_matches_status_capability_reports_golden() {
    let config = TerminalConfig::new(12, 5)
        .unwrap()
        .with_pixel_size(960, 540)
        .unwrap();
    let mut terminal = Terminal::new(config);

    terminal
        .write_str(
            "\
\x1b]0;Window Title\x1b\\\
\x1b]1;Icon Label\x1b\\\
\x1b[2;4H\
\x1b[6n\x1b[?6n\x1b[5n\
\x1b[?15n\x1b[?25n\x1b[?26n\x1b[?53n\
\x1b[x\x1b[1x\
\x1b[11t\x1b[13t\x1b[14t\x1b[18t\x1b[19t\x1b[20t\x1b[21t\
\x1b[c\x1b[>c",
        )
        .unwrap();

    assert_eq!(
        format_status_capability_reports_golden(&mut terminal),
        include_str!("../fixtures/terminal_golden/status_capability_reports.txt")
    );
}
