use gromaq::{Terminal, TerminalConfig, TerminalTestApi, TestKey};

#[test]
fn test_api_pastes_text_and_dumps_state() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());

    TerminalTestApi::paste_text(&mut terminal, "api").unwrap();

    assert_eq!(TerminalTestApi::dump_grid(&terminal).line_text(0), "api");
    assert_eq!(TerminalTestApi::dump_cursor(&terminal).col, 3);
    assert_eq!(
        TerminalTestApi::dump_perf_metrics(&terminal).parsed_bytes,
        3
    );
}

#[test]
fn test_api_perf_metrics_track_dirty_batches_and_resizes() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());

    TerminalTestApi::paste_text(&mut terminal, "abc").unwrap();
    let regions = terminal.take_dirty_regions();
    TerminalTestApi::resize(&mut terminal, 10, 3).unwrap();

    let metrics = TerminalTestApi::dump_perf_metrics(&terminal);
    assert_eq!(regions.len(), 1);
    assert_eq!(metrics.dirty_region_batches, 1);
    assert_eq!(metrics.resizes, 1);
}

#[test]
fn test_api_encodes_keys_without_mutating_grid() {
    let mut terminal = Terminal::new(TerminalConfig::new(12, 2).unwrap());

    let bytes = TerminalTestApi::send_keys(&mut terminal, &[TestKey::Char('x'), TestKey::Enter]);

    assert_eq!(bytes, b"x\r");
    assert_eq!(TerminalTestApi::dump_grid(&terminal).line_text(0), "");
}

#[test]
fn test_api_screenshot_captures_text_and_cursor_pixels() {
    let mut terminal = Terminal::new(TerminalConfig::new(4, 2).unwrap());
    TerminalTestApi::paste_text(&mut terminal, "A").unwrap();

    let screenshot = TerminalTestApi::screenshot(&terminal);

    assert_eq!(screenshot.width, 4);
    assert_eq!(screenshot.height, 2);
    assert_eq!(screenshot.rgba.len(), 4 * 2 * 4);
    assert_eq!(pixel(&screenshot, 0, 0), [255, 255, 255, 255]);
    assert_eq!(pixel(&screenshot, 1, 0), [64, 160, 255, 255]);
    assert_eq!(pixel(&screenshot, 2, 0), [0, 0, 0, 255]);
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
