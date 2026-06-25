use std::process::Command;
use std::time::Duration;

pub(crate) struct TmuxServerGuard {
    socket_name: String,
}

impl TmuxServerGuard {
    pub(crate) fn new(socket_name: String) -> Self {
        Self { socket_name }
    }
}

impl Drop for TmuxServerGuard {
    fn drop(&mut self) {
        let _ = Command::new("tmux")
            .args(["-L", &self.socket_name, "kill-server"])
            .output();
    }
}

pub(crate) fn wait_for_tmux_active_pane_index(
    socket_name: &str,
    expected: &str,
    attempts: usize,
    pause: Duration,
) -> String {
    let mut active_pane = String::new();
    for _ in 0..attempts {
        active_pane = tmux_active_pane_index(socket_name);
        if active_pane.trim() == expected {
            break;
        }
        std::thread::sleep(pause);
    }
    active_pane
}

pub(crate) fn tmux_active_pane_index(socket_name: &str) -> String {
    let output = Command::new("tmux")
        .args(["-L", socket_name, "display-message", "-p", "#{pane_index}"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "tmux display-message failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}
