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
- `tests/terminal_state.rs`: core grid, cursor, Unicode, status-report, and
  scrollback fixtures.
- `tests/app.rs`: native runtime, PTY, input, mouse, resize, surface, and
  redraw boundary fixtures.
- `tests/pty.rs`: real PTY command and optional external-tool fixtures.
- `tests/native_gpu.rs` and `tests/cli.rs`: GPU and CLI smoke fixtures.
- `benches/parser_throughput.rs`: reproducible benchmark payloads.

## Required Local Checks

Run from the repository root before treating a code slice as complete:

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench --bench parser_throughput -- --list
```

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
