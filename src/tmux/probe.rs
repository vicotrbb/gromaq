//! tmux installation and environment probing.

use super::{SystemTmuxCommandRunner, TmuxCommandRunner, TmuxError};

/// Parsed tmux version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxVersion {
    /// Original trimmed `tmux -V` output.
    pub raw: String,
    /// Major version number.
    pub major: u16,
    /// Minor version number.
    pub minor: u16,
    /// Optional numeric patch version.
    pub patch: Option<u16>,
    /// Non-numeric suffix after major/minor/patch, such as `a` in `3.5a`.
    pub suffix: String,
}

/// Current tmux availability and environment status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TmuxProbeStatus {
    /// Whether tmux is installed.
    pub installed: bool,
    /// Parsed tmux version when installed and parseable.
    pub version: Option<TmuxVersion>,
    /// Whether this process appears to be running inside tmux.
    pub inside_tmux: bool,
    /// Whether attachable sessions were discovered.
    pub attachable_sessions: bool,
}

/// Testable tmux probe.
#[derive(Debug, Clone)]
pub struct TmuxProbe<R = SystemTmuxCommandRunner> {
    runner: R,
}

impl<R> TmuxProbe<R>
where
    R: TmuxCommandRunner,
{
    /// Create a probe backed by a tmux command runner.
    pub fn new(runner: R) -> Self {
        Self { runner }
    }

    /// Probe tmux installation and process environment.
    pub fn probe(&self) -> Result<TmuxProbeStatus, TmuxError> {
        let version = match self.runner.run_tmux(&["-V"]) {
            Ok(output) => Some(parse_tmux_version(&output.stdout)?),
            Err(TmuxError::Missing) => None,
            Err(error) => return Err(error),
        };
        let attachable_sessions = version
            .as_ref()
            .is_some_and(|_| self.attachable_sessions().unwrap_or(false));
        Ok(TmuxProbeStatus {
            installed: version.is_some(),
            version,
            inside_tmux: std::env::var_os("TMUX").is_some(),
            attachable_sessions,
        })
    }

    fn attachable_sessions(&self) -> Result<bool, TmuxError> {
        match self
            .runner
            .run_tmux(&["list-sessions", "-F", "#{session_name}"])
        {
            Ok(output) => Ok(!output.stdout.trim().is_empty()),
            Err(TmuxError::Command(_)) => Ok(false),
            Err(error) => Err(error),
        }
    }
}

impl TmuxProbe<SystemTmuxCommandRunner> {
    /// Parse `tmux -V` output.
    pub fn parse_version(output: &str) -> Result<TmuxVersion, TmuxError> {
        parse_tmux_version(output)
    }
}

fn parse_tmux_version(output: &str) -> Result<TmuxVersion, TmuxError> {
    let raw = output.trim();
    let Some(number) = raw.strip_prefix("tmux ") else {
        return Err(TmuxError::Parse {
            context: "tmux version",
            row: raw.to_owned(),
        });
    };
    let (major, rest) = parse_number_prefix(number, "tmux version")?;
    let rest = rest.strip_prefix('.').ok_or_else(|| TmuxError::Parse {
        context: "tmux version",
        row: raw.to_owned(),
    })?;
    let (minor, rest) = parse_number_prefix(rest, "tmux version")?;
    let (patch, suffix) = if let Some(after_dot) = rest.strip_prefix('.') {
        let (patch, suffix) = parse_number_prefix(after_dot, "tmux version")?;
        (Some(patch), suffix)
    } else {
        (None, rest)
    };
    Ok(TmuxVersion {
        raw: raw.to_owned(),
        major,
        minor,
        patch,
        suffix: suffix.to_owned(),
    })
}

fn parse_number_prefix<'a>(
    value: &'a str,
    context: &'static str,
) -> Result<(u16, &'a str), TmuxError> {
    let digits = value.chars().take_while(char::is_ascii_digit).count();
    if digits == 0 {
        return Err(TmuxError::Parse {
            context,
            row: value.to_owned(),
        });
    }
    let number = value[..digits].parse().map_err(|_| TmuxError::Parse {
        context,
        row: value.to_owned(),
    })?;
    Ok((number, &value[digits..]))
}
