# Testing

Gromaq tests are designed to prove deterministic terminal behavior first, then
native integration boundaries. Keep new fixtures small, explicit, and close to
the behavior under test.

## Fixture Strategy

- Prefer inline byte strings for VT/ANSI control sequences so expected terminal
  state is visible in the test.
- Prefer generated payloads for large-output tests and benchmarks so fixture
  construction stays reproducible and easy to inspect.
- Use deterministic in-memory adapters for PTY, clipboard, surface, and app
  boundaries unless the test is explicitly proving a real OS or GPU boundary.
- Keep real-system tests conditional on tool availability when they validate
  external programs such as shells, editors, `tmux`, `kubectl`, or process
  monitors.
- Do not use recorded terminal output as broad proof without documenting the
  source command, viewport size, expected state, and unsupported assumptions.

## Fixture Locations

- `tests/parser.rs`: ANSI/SGR parser fixtures.
- `tests/vt_editing.rs`: VT editing, cursor, tab, scrolling, charset, and
  mode fixtures.
- `tests/reflow.rs`: visible-grid and scrollback resize fixtures.
- `tests/terminal_state/{basic,control,reports,scrollback,unicode}.rs`: core grid, cursor, Unicode, status-report, and
  scrollback fixtures.
- `tests/fixtures/README.md`: file-backed terminal golden fixture inventory,
  update workflow, and review rules.
- `tests/app/*.rs` and `tests/app/runtime_core/*.rs`: native runtime, PTY, input, mouse, resize, surface, and
  redraw boundary fixtures.
- `tests/pty.rs`: real PTY command and optional external-tool fixtures.
- `tests/tmux.rs` and `tests/tmux_manager.rs`: native tmux probe, parser,
  action metadata, action runner, state reader, manager snapshot, and
  workspace launcher fixtures using fake command runners.
- `tests/native_gpu.rs` and `tests/cli.rs`: GPU and CLI smoke fixtures.
- `benches/parser_throughput.rs`: reproducible benchmark payloads.

## Golden Fixture Review

File-backed terminal golden fixtures should capture a narrow behavior contract,
not a transcript dump. When adding one:

- Choose a small viewport that makes wrapping, scrollback, cursor, and metadata
  effects visible without long expected files.
- Put the exact control bytes in the test and keep the formatter scoped to the
  fields that prove the behavior.
- Include pending terminal response bytes only when the fixture is proving a
  response boundary such as status or mode reports.
- Run the focused golden test once with the expected fixture empty, inspect the
  rendered assertion output, then commit only the reviewed expected snapshot.
- Update `tests/fixtures/README.md` when adding or changing a file-backed
  fixture so reviewers know what the fixture is intended to prove.

## Required Local Checks

Run from the repository root before treating a code slice as complete:

```bash
scripts/prove-local-ci-parity.sh
cargo fmt --check
git diff --check
git diff --cached --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo test --test app tmux -- --nocapture
cargo test --test tmux
cargo run -- --tmux-assist
cargo run -- --tmux-manager
cargo run -- --tmux-action kill-session gromaq-test
cargo run -- --runtime-tmux-smoke
cargo run -- --runtime-tmux-ui-smoke
cargo run -- --window-tmux-manager-snapshot target/native-tmux-manual-proof/window-tmux-manager-snapshot.ppm
scripts/prove-native-tmux-default-snapshot.sh
scripts/prove-macos-native-tmux-manual.sh
cargo bench --bench parser_throughput -- --list
```

The parity helper is the default local proof command for CI-aligned slices. It
also runs shell syntax checks, font symbol fallback smoke, theme legibility and
preview proof, avatar asset freshness, welcome, README screenshot freshness,
GPU welcome image snapshot, GPU terminal text smoke, current-host compatibility,
frame scheduler smoke, and benchmark inventory proof helpers. Use the expanded
command list when you need to rerun or debug an individual gate.
After all steps pass, the helper writes
`target/local-ci-parity-proof/summary.txt` with the proof artifact handles.
Avatar asset freshness is part of local parity.

Run full Criterion benchmarks when changing parser, PTY pump, render planning,
glyph cache, rasterization, frame preparation, or other measured hot paths:

```bash
cargo bench
```

## Live Proof Boundary

Headless tests and offscreen GPU smokes do not prove live desktop behavior.
Compatibility rows marked as live or not proven require separate evidence from
a real windowed runtime, hardware-backed frame pacing, native clipboard paste,
or interactive workflows in tools such as `tmux`, editors, and pagers.

For native tmux manager app-window proof, use an isolated tmux target and record
the exact result. The required manual checklist is:

```bash
scripts/prove-macos-native-tmux-default-cargo-run.sh
scripts/prove-macos-native-tmux-manual.sh
```

`scripts/prove-macos-native-tmux-default-cargo-run.sh` opens the default app
path through plain `cargo run`, matching the no-argument developer workflow. It
prepares uniquely named tmux sessions, asks the tester to verify the default
startup shortcut copy, status strip, Control/Super Shift `T` manager panel,
visible tmux state, navigation, start-session, attach-session, safe action
dispatch, new-window, destructive confirmation against the disposable target,
shell input after closing the panel, and right-prompt legibility. It records the
exact `target/debug/gromaq` binary marker used by the `cargo run` launch and
fails if the old keyboard/mouse/paste startup copy remains in that binary, then
records git branch/dirty metadata plus confirmation files under
`target/macos-native-tmux-default-cargo-run-proof`.
After confirmations, it also reads tmux state directly and fails unless the
named started session exists, the target session has an added window, the target
session has an added pane from the safe split, and the disposable kill-session
target is absent.

`scripts/prove-native-tmux-default-snapshot.sh` is the non-interactive default
startup artifact proof. It runs `cargo run -- --window-tmux-manager-snapshot`,
requires the default startup content plus tmux status strip, status pane
command, and manager panel proof lines, records the rendered manager
session/window/pane counts from an isolated tmux session, writes a PPM artifact,
and writes a PNG beside it when `sips` is available.

When no packaged app bundle is selected, the helper builds and launches
`target/debug/gromaq` directly with its generated tmux proof config. Set
`GROMAQ_MANUAL_TMUX_APP=/path/to/Gromaq.app` to force an app-bundle run, or
`GROMAQ_MANUAL_TMUX_BINARY=/path/to/gromaq` to force a specific binary.
The configured helper also records git branch/dirty metadata beside the manual
confirmation files so the live proof can be tied back to the checkout under
test.
After the live prompts finish, it reads tmux state directly and requires the
named started session to exist, the target session to show the new window and
split pane, and the disposable kill-session target to be removed before writing
the summary.
By default, the generated tmux proof config starts with the manager closed so
the tester must open it with Control/Super Shift `T`; set
`GROMAQ_MANUAL_TMUX_OPEN_ON_START=true` to instead prove startup-open behavior.

- Launch `cargo run` or `scripts/prove-macos-native-tmux-manual.sh`.
- Verify the persistent tmux status strip is visible and legible; the harness
  records this as `tmux-status-strip-visible.txt`.
- Verify the manager remains visible when startup shell output fills the first window.
- In the default no-argument proof, if the manager is already visible on
  startup, close it with Esc first, then press Control/Super Shift `T` and
  verify a real manager panel opens again.
- In the configured manual proof, press Control/Super Shift `T` when the manager
  starts closed and verify a real manager panel opens.
- Inspect sessions, windows, panes, current target, and pane command text.
- Navigate with arrows or `h`/`j`/`k`/`l`.
- Click session/window/pane/action/workspace rows and verify selection moves;
  the harness records this as `tmux-navigation-checked.txt`.
- Check a right-prompt or long prompt layout for legible overlap behavior; the
  harness records this as `tmux-right-prompt-legible.txt`.
- Start a named session from the UI; the harness records this as
  `tmux-start-session.txt`.
- Attach the prepared session from the UI; the harness records this as
  `tmux-attach-session.txt` in the configured manual proof.
- Run a safe split-pane action from the UI; the harness records this as
  `tmux-safe-action.txt`.
- Create a tmux window from the UI; the harness records this as
  `tmux-new-window.txt`.
- Press `r` and verify the manager refreshes without sending shell input; the
  harness records this as `tmux-refresh-checked.txt`.
- Use `q` or another destructive shortcut and verify inline confirmation appears
  before tmux runs.
- Confirm a kill only against an isolated test target.
- Launch or inspect the configured workspace preset; the configured manual
  harness records this as `tmux-workspace-visible.txt`.
- Close the panel and verify normal shell input still reaches the terminal.

Until that manual checklist has been performed in the current session, native
tmux manager behavior is proven only by automated model/render/runtime smokes
and remains separate from live app-window proof.
