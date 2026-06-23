use gromaq::{Terminal, TerminalConfig};

#[test]
fn scrollback_view_scrolls_history_into_visible_grid() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(6, 3)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal.write_str("one\r\ntwo\r\nthree\r\nfour").unwrap();
    assert_eq!(terminal.dump_grid().line_text(0), "two");
    assert_eq!(terminal.dump_grid().line_text(1), "three");
    assert_eq!(terminal.dump_grid().line_text(2), "four");
    assert!(terminal.dump_cursor().visible);

    assert!(terminal.scroll_display_up(1));

    let scrolled = terminal.dump_grid();
    assert_eq!(scrolled.line_text(0), "one");
    assert_eq!(scrolled.line_text(1), "two");
    assert_eq!(scrolled.line_text(2), "three");
    assert!(!terminal.dump_cursor().visible);

    assert!(terminal.scroll_display_down(1));

    let live = terminal.dump_grid();
    assert_eq!(live.line_text(0), "two");
    assert_eq!(live.line_text(1), "three");
    assert_eq!(live.line_text(2), "four");
    assert!(terminal.dump_cursor().visible);
}

#[test]
fn scrollback_view_screenshot_uses_displayed_history_rows() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(6, 3)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );

    terminal
        .write_str("\x1b[31mone\r\n\x1b[32mtwo\r\n\x1b[33mthree\r\n\x1b[34mfour")
        .unwrap();
    assert_eq!(pixel(&terminal.screenshot(), 0, 0), [13, 188, 121, 255]);

    assert!(terminal.scroll_display_up(1));

    let screenshot = terminal.screenshot();
    assert_eq!(pixel(&screenshot, 0, 0), [205, 49, 49, 255]);
    assert_eq!(pixel(&screenshot, 0, 1), [13, 188, 121, 255]);
    assert_ne!(pixel(&screenshot, 3, 2), [64, 160, 255, 255]);
}

#[test]
fn scrollback_view_navigation_updates_perf_and_dirty_viewport() {
    let mut terminal = Terminal::new(
        TerminalConfig::new(6, 3)
            .unwrap()
            .with_scrollback_limit(8)
            .unwrap(),
    );
    terminal.write_str("one\r\ntwo\r\nthree\r\nfour").unwrap();
    terminal.take_dirty_regions();
    let before = terminal.dump_perf_metrics();

    assert!(terminal.scroll_display_up(1));

    let dirty = terminal.take_dirty_regions();
    let after = terminal.dump_perf_metrics();
    assert_eq!(after.scrolls, before.scrolls + 1);
    assert_eq!(after.dirty_cells, before.dirty_cells + 18);
    assert_eq!(dirty.len(), 1);
    assert_eq!(dirty[0].row, 0);
    assert_eq!(dirty[0].col, 0);
    assert_eq!(dirty[0].rows, 3);
    assert_eq!(dirty[0].cols, 6);
}

fn pixel(screenshot: &gromaq::terminal::Screenshot, x: u32, y: u32) -> [u8; 4] {
    let index = usize::try_from((y * screenshot.width + x) * 4).unwrap();
    [
        screenshot.rgba[index],
        screenshot.rgba[index + 1],
        screenshot.rgba[index + 2],
        screenshot.rgba[index + 3],
    ]
}
