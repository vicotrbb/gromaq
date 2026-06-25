//! Config-oriented CLI argument commands.

use crate::cli::args::usage;
use crate::cli::config_commands::{
    config_check_exit, config_template_exit, launch_config_file_exit,
};
use crate::cli::dispatch::arguments::{reject_extra_args, required_path_arg};
use crate::cli::{CliExit, NativeAppLauncher};

pub(super) fn config_check_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    let path = match required_path_arg(args, "--config-check") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    config_check_exit(path.as_ref())
}

pub(super) fn config_template_command<I, S>(args: &mut I) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    config_template_exit()
}

pub(super) fn config_file_command<I, S, A>(args: &mut I, app_launcher: Option<&A>) -> CliExit
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
    A: NativeAppLauncher,
{
    let path = match required_path_arg(args, "--config") {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    if let Err(exit) = reject_extra_args(args) {
        return exit;
    }
    let Some(app_launcher) = app_launcher else {
        return CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!("{}native app launch unavailable for --config\n", usage()),
        };
    };
    launch_config_file_exit(path.as_ref(), app_launcher)
}
