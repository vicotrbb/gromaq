//! tmux commands rendered for entry through the terminal PTY.

/// Structured tmux command that can be typed into the retained shell PTY.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxTerminalCommand {
    args: Vec<String>,
}

impl TmuxTerminalCommand {
    /// Build a command that attaches the terminal shell to an existing session.
    pub fn attach_session(session: impl Into<String>) -> Self {
        Self {
            args: vec![
                "tmux".into(),
                "attach-session".into(),
                "-t".into(),
                session.into(),
            ],
        }
    }

    /// Render the command as PTY input ending with carriage return.
    pub fn to_pty_input(&self) -> Vec<u8> {
        let mut line = self
            .args
            .iter()
            .map(|arg| shell_quote(arg))
            .collect::<Vec<_>>()
            .join(" ");
        line.push('\r');
        line.into_bytes()
    }
}

pub(crate) fn shell_quote(arg: &str) -> String {
    if arg
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | '/' | ':' | '%'))
    {
        return arg.to_owned();
    }
    format!("'{}'", arg.replace('\'', "'\\''"))
}
