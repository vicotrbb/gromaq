# Debugging

Gromaq debugging should produce reproducible evidence, not subjective terminal
behavior claims. Start with deterministic checks, then move outward to native OS
and GPU boundaries only when the failing behavior requires them.

## Required Baseline

Run from the repository root before treating a debugging session as conclusive:

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench --bench parser_throughput -- --list
```

Use `cargo test --test <name> <case>` for focused iteration, but finish with the
full baseline above before committing.

## Deterministic State Debugging

Prefer the structured test API and snapshots over visual inspection:

- `Terminal::dump_grid()` for visible text, styles, hyperlinks, cursor-adjacent
  cells, and wide-cell metadata.
- `Terminal::dump_scrollback()` for retained rows, hard-break flags,
  logical-line IDs, and retained rich cell metadata.
- `Terminal::dump_cursor()` for cursor position, shape, and visibility.
- `TerminalTestApi::screenshot()` for deterministic one-pixel-per-cell RGBA
  snapshots.
- `NativeTerminalRuntime::dump_runtime_perf_metrics()` for PTY, render,
  resize, clean-frame skip, render-time, and input-to-render counters.

Keep failing fixtures small. A good terminal-state reproduction includes the
input bytes, viewport size, expected grid or scrollback state, and any response
bytes expected to be written back to the PTY.

## Native Runtime Smokes

These commands exercise runtime boundaries without requiring a live window:

```bash
cargo run -- --runtime-perf-smoke
cargo run -- --runtime-idle-smoke
cargo run -- --runtime-large-output-smoke
cargo run -- --runtime-bounded-state-smoke
cargo run -- --runtime-continuous-output-smoke
cargo run -- --runtime-alternate-screen-smoke
cargo run -- --runtime-reflow-smoke
cargo run -- --runtime-glyph-frame-smoke
cargo run -- --runtime-clipboard-paste-smoke
```

Treat these as deterministic runtime evidence. They do not prove live desktop
frame pacing, live input latency, idle CPU, process memory growth, or windowed
glyph drawing.

## GPU and Surface Smokes

Use offscreen GPU smokes to isolate adapter, upload, atlas, textured draw, and
terminal text draw failures:

```bash
cargo run -- --gpu-info
cargo run -- --gpu-smoke
cargo run -- --gpu-upload-smoke
cargo run -- --gpu-glyph-atlas-smoke
cargo run -- --gpu-text-atlas-smoke
cargo run -- --gpu-textured-quad-smoke
cargo run -- --gpu-terminal-text-smoke
```

Surface lifecycle behavior is covered by `tests/surface_config.rs` and
`tests/app.rs`. A live window failure still needs separate evidence from a real
desktop session because offscreen readback does not prove surface acquisition,
presentation timing, compositor behavior, or screenshot correctness.

## Optional External Tools

`tests/pty.rs` uses real tools when they are available locally, including
shells, editors, `tmux`, pagers, process monitors, `ssh`, `kubectl`, and
`cargo`. If a tool is missing, the corresponding test reports a skip-like
success boundary instead of proving that workflow on the current machine.

When debugging a compatibility issue with an external tool, record:

- the tool and version
- viewport size
- exact command or interactive sequence
- expected terminal state
- observed grid, scrollback, cursor, response bytes, or PTY output
- whether the proof was deterministic, synthetic, or live desktop evidence

## Proof Boundaries

Do not turn partial evidence into broad claims. In particular:

- CPU-side render planning is not live GPU frame-time proof.
- Offscreen GPU readback is not windowed terminal proof.
- Deterministic runtime smokes are not live desktop latency proof.
- Clipboard write/read smokes are not the same as a user paste workflow.
- Conditional external-tool tests only prove workflows for tools present on the
  machine where they ran.

Update `COMPATIBILITY.md`, `BENCHMARKS.md`, or `README.md` when a debugging
session adds durable evidence or changes a known proof boundary.
