use std::fs;
use std::time::Duration;

use crate::support::*;

#[test]
fn pty_session_runs_vim_version_when_available() {
    assert_program_outputs_when_available("vim", &["--version"], "VIM");
}

#[test]
fn pty_session_runs_vim_interactive_edit_when_available() {
    let Some(_program) = find_program("vim") else {
        eprintln!("skipping vim interactive PTY workflow test because vim is not on PATH");
        return;
    };
    let file = test_temp_path("vim-interactive.txt");
    let _ = fs::remove_file(&file);
    let command = format!(
        "TERM=xterm-256color vim -Nu NONE -n -i NONE -N --noplugin {}",
        shell_quote_path(&file)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b"igromaq-vim-interactive\x1b:wq\r")
        .unwrap();

    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let edited = fs::read_to_string(&file).unwrap();
    assert_eq!(edited, "gromaq-vim-interactive\n");
    let _ = fs::remove_file(file);
}

#[test]
fn pty_session_runs_vim_mouse_window_selection_when_available() {
    let Some(_program) = find_program("vim") else {
        eprintln!("skipping vim mouse PTY workflow test because vim is not on PATH");
        return;
    };
    let result_file = test_temp_path("vim-mouse-window.txt");
    let _ = fs::remove_file(&result_file);
    let command = "TERM=xterm-256color vim -Nu NONE -n -i NONE -N --noplugin".to_owned();
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b":set mouse=a ttymouse=sgr\r:vsplit\r:wincmd l\r")
        .unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));
    session.write_all(b"\x1b[<0;2;2M\x1b[<0;2;2m").unwrap();
    let command = format!(
        ":call writefile([string(winnr())], {})|qa!\r",
        shell_quote_path(&result_file)
    );
    session.write_all(command.as_bytes()).unwrap();

    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let selected_window = fs::read_to_string(&result_file).unwrap();
    assert_eq!(selected_window.trim(), "1");
    let _ = fs::remove_file(result_file);
}

#[test]
fn pty_session_runs_vim_alternate_screen_enter_exit_when_available() {
    assert_shell_command_enters_and_leaves_alternate_screen_when_available(
        "vim",
        "TERM=xterm-256color vim -Nu NONE -n -i NONE -N --noplugin".to_owned(),
        b":qa!\r",
    );
}

#[test]
fn pty_session_runs_nvim_version_when_available() {
    assert_program_outputs_when_available("nvim", &["--version"], "NVIM");
}

#[test]
fn pty_session_runs_nvim_interactive_edit_when_available() {
    let Some(_program) = find_program("nvim") else {
        eprintln!("skipping nvim interactive PTY workflow test because nvim is not on PATH");
        return;
    };
    let file = test_temp_path("nvim-interactive.txt");
    let _ = fs::remove_file(&file);
    let command = format!(
        "TERM=xterm-256color nvim -u NONE -n -i NONE -N --noplugin {}",
        shell_quote_path(&file)
    );
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b"igromaq-nvim-interactive\x1b:wq\r")
        .unwrap();

    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let edited = fs::read_to_string(&file).unwrap();
    assert_eq!(edited, "gromaq-nvim-interactive\n");
    let _ = fs::remove_file(file);
}

#[test]
fn pty_session_runs_nvim_mouse_window_selection_when_available() {
    let Some(_program) = find_program("nvim") else {
        eprintln!("skipping nvim mouse PTY workflow test because nvim is not on PATH");
        return;
    };
    let result_file = test_temp_path("nvim-mouse-window.txt");
    let _ = fs::remove_file(&result_file);
    let command = "TERM=xterm-256color nvim -u NONE -n -i NONE -N --noplugin".to_owned();
    let mut session = spawn_shell_pty_command(command);
    session.start_output_reader().unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));

    session
        .write_all(b":set mouse=a\r:vsplit\r:wincmd l\r")
        .unwrap();
    drain_until_any_output(&mut session, 50, Duration::from_millis(20));
    session.write_all(b"\x1b[<0;2;2M\x1b[<0;2;2m").unwrap();
    let command = format!(
        ":call writefile([string(winnr())], {})|qa!\r",
        shell_quote_path(&result_file)
    );
    session.write_all(command.as_bytes()).unwrap();

    assert!(
        session
            .wait_timeout(Duration::from_secs(5))
            .unwrap()
            .is_some()
    );
    let selected_window = fs::read_to_string(&result_file).unwrap();
    assert_eq!(selected_window.trim(), "1");
    let _ = fs::remove_file(result_file);
}

#[test]
fn pty_session_runs_nvim_alternate_screen_enter_exit_when_available() {
    assert_shell_command_enters_and_leaves_alternate_screen_when_available(
        "nvim",
        "TERM=xterm-256color nvim -u NONE -n -i NONE -N --noplugin".to_owned(),
        b":qa!\r",
    );
}
