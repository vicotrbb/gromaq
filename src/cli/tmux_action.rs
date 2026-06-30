//! Native tmux action execution CLI surface.

use crate::cli::CliExit;
use crate::cli::args::usage;
use crate::tmux::{
    SystemTmuxCommandRunner, TmuxAction, TmuxActionRequest, TmuxActionResult, TmuxActionRunner,
};

pub(super) fn tmux_action_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let Some(action_id) = args.next() else {
        return usage_error("missing tmux action id for --tmux-action".to_owned());
    };
    let action_id = action_id.as_ref();
    let Some(action) = TmuxAction::by_stable_id(action_id) else {
        return usage_error(format!("unknown tmux action: {action_id}"));
    };
    let parsed = match parse_action_args(args) {
        Ok(parsed) => parsed,
        Err(message) => return usage_error(message),
    };
    let active_tmux = std::env::var_os("TMUX").is_some()
        || parsed.target.is_some()
        || action.can_run_outside_tmux;
    let mut request = TmuxActionRequest::new(action.id)
        .confirmed(parsed.confirmed)
        .active_tmux(active_tmux);
    if let Some(target) = parsed.target {
        request = request.target(target);
    }
    if let Some(name) = parsed.name {
        request = request.new_name(name);
    }
    render_action_result(
        action.stable_id,
        TmuxActionRunner::new(SystemTmuxCommandRunner).run(request),
    )
}

#[derive(Debug, Default, PartialEq, Eq)]
struct ParsedActionArgs {
    target: Option<String>,
    name: Option<String>,
    confirmed: bool,
}

fn parse_action_args<I, S>(args: &mut I) -> Result<ParsedActionArgs, String>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let mut parsed = ParsedActionArgs::default();
    for arg in args {
        let arg = arg.as_ref();
        if arg == "--confirm" {
            parsed.confirmed = true;
        } else if parsed.target.is_none() {
            parsed.target = Some(arg.to_owned());
        } else if parsed.name.is_none() {
            parsed.name = Some(arg.to_owned());
        } else {
            return Err(format!("unexpected extra tmux action argument: {arg}"));
        }
    }
    Ok(parsed)
}

fn render_action_result(action_id: &str, result: TmuxActionResult) -> CliExit {
    match result {
        TmuxActionResult::Success { teaching_hint, .. } => success(
            "tmux action: success",
            action_id,
            teaching_hint,
            String::new(),
        ),
        TmuxActionResult::ConfirmationRequired { teaching_hint, .. } => success(
            "tmux action: confirmation required",
            action_id,
            teaching_hint,
            "rerun with --confirm after checking the target\n".to_owned(),
        ),
        TmuxActionResult::NoActiveSession { teaching_hint, .. } => success(
            "tmux action: no active tmux session",
            action_id,
            teaching_hint,
            "provide a target or start/attach a tmux session first\n".to_owned(),
        ),
        TmuxActionResult::TmuxMissing { teaching_hint, .. } => success(
            "tmux action: tmux missing",
            action_id,
            teaching_hint,
            "install tmux or disable tmux workflows\n".to_owned(),
        ),
        TmuxActionResult::Skipped {
            reason,
            teaching_hint,
            ..
        } => success(
            "tmux action: skipped",
            action_id,
            teaching_hint,
            format!("reason: {reason}\n"),
        ),
        TmuxActionResult::CommandFailed {
            failure,
            teaching_hint,
            ..
        } => CliExit {
            code: 1,
            stdout: format!("tmux action: command failed\naction: {action_id}\n{teaching_hint}\n"),
            stderr: format!("tmux command failed: {}\n", failure.stderr.trim()),
        },
    }
}

fn success(status: &str, action_id: &str, teaching_hint: String, extra: String) -> CliExit {
    CliExit {
        code: 0,
        stdout: format!("{status}\naction: {action_id}\n{teaching_hint}\n{extra}"),
        stderr: String::new(),
    }
}

fn usage_error(message: String) -> CliExit {
    CliExit {
        code: 2,
        stdout: String::new(),
        stderr: format!("{}{message}\nrun `gromaq --help` for usage\n", usage()),
    }
}
