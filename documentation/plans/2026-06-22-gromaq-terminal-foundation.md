# Gromaq Terminal Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the first production-quality Rust foundation slice for `gromaq`: terminal state, ANSI parsing, scrollback, resize behavior, input encoding, test API, benchmark harness, documentation, and a GPU-renderer boundary.

**Architecture:** Start with a small Rust workspace and a focused core library. Keep terminal emulation deterministic and separately testable from PTY and rendering. Put GPU work behind a narrow boundary so correctness tests can run headlessly while the application remains native Rust and GPU-oriented.

**Tech Stack:** Rust 2024, `vte`, `unicode-width`, `bitflags`, `thiserror`, `tracing`, `serde`, `toml`, `winit`, `wgpu`, `portable-pty`, `criterion`, `proptest`.

---

### Task 1: Workspace and Failing Core Tests

**Files:**
- Create: `Cargo.toml`
- Create: `src/lib.rs`
- Create: `src/main.rs`
- Create: `tests/terminal_state.rs`
- Create: `tests/parser.rs`
- Create: `tests/input.rs`
- Create: `tests/config.rs`

- [ ] **Step 1: Write failing tests**

Add tests that describe the required public API before implementation:
- `Terminal::write_str` places printable text into the grid.
- Wide Unicode characters occupy two cells.
- Newlines scroll into bounded scrollback.
- ANSI SGR sets and resets attributes.
- Resize preserves visible text.
- Input encoding turns structured keys into terminal byte sequences.
- Config validation rejects zero dimensions and invalid scrollback.

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test --all`

Expected: failure because public modules and types do not exist yet.

### Task 2: Terminal Core

**Files:**
- Create: `src/cell.rs`
- Create: `src/grid.rs`
- Create: `src/scrollback.rs`
- Create: `src/terminal.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Implement minimal state engine**

Define `Cell`, `Style`, `Cursor`, `Grid`, `Scrollback`, `Terminal`, snapshots, and structured errors. Support printable text, wide cells, newline, carriage return, backspace, clear screen, line wrapping, bounded scrollback, and resize preservation.

- [ ] **Step 2: Run targeted tests**

Run: `cargo test --test terminal_state`

Expected: pass without warnings.

### Task 3: ANSI Parser and Input

**Files:**
- Create: `src/parser.rs`
- Create: `src/input.rs`
- Modify: `src/terminal.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Implement ANSI parsing**

Use `vte::Parser` and `vte::Perform` to apply printable characters, control bytes, CSI cursor movement, erase commands, and SGR colors/styles to `Terminal`.

- [ ] **Step 2: Implement input encoding**

Define `TestKey`, `KeyModifiers`, and `encode_keys` for deterministic keyboard validation.

- [ ] **Step 3: Run parser and input tests**

Run: `cargo test --test parser --test input`

Expected: pass without warnings.

### Task 4: Configuration, PTY Boundary, Test API, Renderer Boundary

**Files:**
- Create: `src/config.rs`
- Create: `src/pty.rs`
- Create: `src/test_api.rs`
- Create: `src/renderer.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Implement config validation**

Define serializable `GromaqConfig` and strict validation for dimensions, scrollback, font size, and frame target.

- [ ] **Step 2: Implement PTY and renderer boundaries**

Define shell launch configuration and PTY abstraction using `portable-pty`. Define a GPU renderer trait and `wgpu`-oriented renderer configuration without putting rendering work in terminal-state hot paths.

- [ ] **Step 3: Implement structured test API**

Expose `TerminalTestApi` with `send_keys`, `paste_text`, `resize`, `dump_grid`, `dump_scrollback`, `dump_cursor`, `dump_perf_metrics`, and `screenshot`.

- [ ] **Step 4: Run targeted tests**

Run: `cargo test --test config`

Expected: pass without warnings.

### Task 5: Benchmarks and Repository Quality

**Files:**
- Create: `benches/parser_throughput.rs`
- Create: `README.md`
- Create: `ARCHITECTURE.md`
- Create: `CONTRIBUTING.md`
- Create: `BENCHMARKS.md`
- Create: `COMPATIBILITY.md`
- Create: `ROADMAP.md`
- Create: `LICENSE`
- Create: `.github/workflows/ci.yml`

- [ ] **Step 1: Add benchmark harness**

Benchmark parser throughput, scrolling throughput, and large-output ingestion using Criterion.

- [ ] **Step 2: Add open-source project documentation**

Document current proof boundaries: this first slice is a tested foundation, not a complete daily-driver terminal.

- [ ] **Step 3: Run required gates**

Run:
```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench
```

Expected: all pass, or any failure is explicitly fixed before claiming completion.

### Task 6: Completion Audit

**Files:**
- Inspect all files listed above.

- [ ] **Step 1: Audit against `documentation/goal.md`**

Mark each acceptance criterion as `implemented`, `partially implemented`, or `not yet implemented`.

- [ ] **Step 2: Keep the active goal open if any acceptance criterion is missing**

The full goal is complete only after PTY workflows, native GPU UI, compatibility matrix, screenshots/reference behavior, and performance acceptance criteria are implemented and proven.
