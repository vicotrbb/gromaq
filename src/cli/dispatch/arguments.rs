use crate::cli::CliExit;
use crate::cli::args::usage;

pub(super) fn required_path_arg<I, S>(args: &mut I, command: &str) -> Result<S, CliExit>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    args.next().ok_or_else(|| CliExit {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{}missing config path for {command}\nrun `gromaq --help` for usage\n",
            usage()
        ),
    })
}

pub(super) fn required_snapshot_path_arg<I, S>(args: &mut I, command: &str) -> Result<S, CliExit>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    args.next().ok_or_else(|| CliExit {
        code: 2,
        stdout: String::new(),
        stderr: format!(
            "{}missing snapshot path for {command}\nrun `gromaq --help` for usage\n",
            usage()
        ),
    })
}

pub(super) fn reject_extra_args<I, S>(args: &mut I) -> Result<(), CliExit>
where
    I: Iterator<Item = S>,
    S: AsRef<str>,
{
    if let Some(extra) = args.next() {
        return Err(CliExit {
            code: 2,
            stdout: String::new(),
            stderr: format!(
                "{}unexpected extra argument: {}\nrun `gromaq --help` for usage\n",
                usage(),
                extra.as_ref()
            ),
        });
    }
    Ok(())
}
