# Security Policy

Gromaq is alpha terminal-emulator software. Please do not use public issues for
vulnerability reports that include exploit details, private environment data, or
crash material that could expose local system information.

## Supported Versions

Only the current `main` branch and the latest public alpha/beta release are
actively assessed during alpha development. Older tags remain available as
historical packaging evidence but are not long-term support releases.

## Reporting a Vulnerability

Use GitHub private vulnerability reporting for this repository when available:

`https://github.com/vicotrbb/gromaq/security/advisories/new`

If private reporting is unavailable, open a minimal public issue that says a
private security report is needed, but do not include exploit details.

Helpful reports include:

- the Gromaq commit or release version
- operating system and architecture
- terminal workflow involved
- whether the issue requires a malicious local process, a remote host, a crafted
  escape sequence, clipboard content, or a file
- a minimized reproduction when it can be shared safely

## Scope

Security-sensitive areas include:

- PTY process lifecycle and shell spawning
- terminal escape parsing, OSC handling, hyperlinks, and clipboard sequences
- native clipboard access
- install and packaging scripts
- GPU/window surface initialization
- file paths accepted through configuration or CLI arguments

The project does not currently provide remote services, authentication,
networked accounts, or cloud-hosted infrastructure.
