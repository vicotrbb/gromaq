//! Structured tmux state model and stable-output parsers.

use super::TmuxError;

/// A tmux session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxSession {
    /// Session name.
    pub name: String,
    /// Whether at least one client is attached.
    pub attached: bool,
}

/// A tmux window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxWindow {
    /// Parent session name.
    pub session_name: String,
    /// Window index within the session.
    pub index: u16,
    /// Window name.
    pub name: String,
    /// Whether this is the active window.
    pub active: bool,
}

/// A tmux pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxPane {
    /// Parent session name.
    pub session_name: String,
    /// Parent window index.
    pub window_index: u16,
    /// Pane index within the window.
    pub index: u16,
    /// tmux pane id such as `%1`.
    pub id: String,
    /// Pane title.
    pub title: String,
    /// Current command reported by tmux.
    pub current_command: String,
    /// Whether this pane is active.
    pub active: bool,
    /// Pane width in cells when reported.
    pub width: Option<u16>,
    /// Pane height in cells when reported.
    pub height: Option<u16>,
}

/// Full read-only tmux state snapshot.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TmuxState {
    /// Sessions visible to tmux.
    pub sessions: Vec<TmuxSession>,
    /// Windows visible to tmux.
    pub windows: Vec<TmuxWindow>,
    /// Panes visible to tmux.
    pub panes: Vec<TmuxPane>,
}

/// Addressable tmux target.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TmuxTarget {
    /// Session by name.
    Session(String),
    /// Window by session name and index.
    Window {
        /// Session name.
        session_name: String,
        /// Window index.
        index: u16,
    },
    /// Pane by tmux pane id.
    PaneId(String),
}

impl TmuxState {
    /// Parse stable tab-delimited `list-*` output.
    pub fn parse(sessions: &str, windows: &str, panes: &str) -> Result<Self, TmuxError> {
        Ok(Self {
            sessions: parse_sessions(sessions)?,
            windows: parse_windows(windows)?,
            panes: parse_panes(panes)?,
        })
    }
}

fn parse_sessions(output: &str) -> Result<Vec<TmuxSession>, TmuxError> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = split_fields(line, 2, "session row")?;
            Ok(TmuxSession {
                name: fields[0].to_owned(),
                attached: parse_bool(fields[1], "session attached")?,
            })
        })
        .collect()
}

fn parse_windows(output: &str) -> Result<Vec<TmuxWindow>, TmuxError> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = split_fields(line, 4, "window row")?;
            Ok(TmuxWindow {
                session_name: fields[0].to_owned(),
                index: parse_u16(fields[1], "window index")?,
                name: fields[2].to_owned(),
                active: parse_bool(fields[3], "window active")?,
            })
        })
        .collect()
}

fn parse_panes(output: &str) -> Result<Vec<TmuxPane>, TmuxError> {
    output
        .lines()
        .filter(|line| !line.is_empty())
        .map(|line| {
            let fields = split_fields(line, 9, "pane row")?;
            Ok(TmuxPane {
                session_name: fields[0].to_owned(),
                window_index: parse_u16(fields[1], "pane window index")?,
                index: parse_u16(fields[2], "pane index")?,
                id: fields[3].to_owned(),
                title: fields[4].to_owned(),
                current_command: fields[5].to_owned(),
                active: parse_bool(fields[6], "pane active")?,
                width: parse_optional_u16(fields[7], "pane width")?,
                height: parse_optional_u16(fields[8], "pane height")?,
            })
        })
        .collect()
}

fn split_fields<'a>(
    line: &'a str,
    expected: usize,
    context: &'static str,
) -> Result<Vec<&'a str>, TmuxError> {
    let fields = line.split('\t').collect::<Vec<_>>();
    if fields.len() == expected {
        return Ok(fields);
    }
    Err(TmuxError::Parse {
        context,
        row: line.to_owned(),
    })
}

fn parse_bool(value: &str, context: &'static str) -> Result<bool, TmuxError> {
    match value {
        "0" => Ok(false),
        "1" => Ok(true),
        _ => Err(TmuxError::Parse {
            context,
            row: value.to_owned(),
        }),
    }
}

fn parse_u16(value: &str, context: &'static str) -> Result<u16, TmuxError> {
    value.parse().map_err(|_| TmuxError::Parse {
        context,
        row: value.to_owned(),
    })
}

fn parse_optional_u16(value: &str, context: &'static str) -> Result<Option<u16>, TmuxError> {
    if value.is_empty() {
        return Ok(None);
    }
    parse_u16(value, context).map(Some)
}
