# Architecture

Gromaq is organized as a native Rust terminal stack with explicit boundaries
between terminal correctness, PTY IO, native app lifecycle, rendering, GPU
handoff, and validation tooling.

## Module Map

- `terminal`: VT/ANSI parser integration, terminal state, modes, reports,
  editing, reset/cursor lifecycle, scrollback interaction, reflow,
  selection/copy, and snapshot APIs.
- `grid`, `cell`, `dirty`, `scrollback`, `selection`, `input`, and `mouse`:
  core data structures and protocol helpers used by the terminal and app
  layers.
- `pty`: shell command resolution, PTY session lifecycle, background output
  reading, writer access, and resize notification.
- `config`: TOML settings, validation, defaults, theme tokens, and config-file
  reload state.
- `font`: real-font rasterization, glyph image preparation, fallback handling,
  and rasterized glyph caching.
- `renderer`: CPU-side render planning, glyph atlas metadata, quad generation,
  prepared surface frames, surface lifecycle planning, and `wgpu` surface-frame
  drawing. Prepared-frame preview color conversion and blending live in a
  focused child module, and surface configuration choice/error policy lives in
  a dedicated planner child module.
- `native_gpu`: GPU bootstrap, offscreen smoke paths, surface creation, upload
  and draw/readback helpers, and structured GPU reports.
- `app`: native `winit` app wiring, lifecycle state, launch wrappers, config
  reload application, PTY bridge, input mapping, text zoom, font discovery,
  runtime rendering, and native window surface presentation. Handler actions,
  shortcut policy, resize mapping, lifecycle window ownership, lifecycle
  run-report data, frame-interval accounting, snapshot artifact helpers, and
  native font search policy live in focused child modules.
- `cli`: executable smoke commands, config utilities, GPU reports, runtime
  validation commands, and bounded live-window probes. GPU command context
  traits are isolated from GPU CLI output formatting, and config launch
  boundaries are isolated from config-check/template formatting.
- `test_api`: deterministic integration hooks used by tests and future debug
  tooling.

## Boundary Rules

- Terminal state code must not depend on `winit`, `wgpu`, native clipboard, or
  platform PTY types.
- Renderer planning must remain testable without a live GPU surface.
- `native_gpu` owns GPU device/surface primitives and readback-oriented smoke
  helpers; `renderer` owns terminal-specific draw data.
- `app` owns orchestration and native event-loop decisions, but protocol
  encoding, terminal mutation, render planning, and PTY behavior stay in their
  dedicated modules.
- CLI smoke commands should report structured, reproducible proof data instead
  of subjective output.

## Organization Policy

Prefer small vertical modules over large mixed-responsibility files. When a file
starts mixing durable data types, platform glue, and behavior, split stable data
types into a sibling module first, then move behavior only when tests show a
clear boundary. The repository policy test currently caps Rust source files at
225 lines, which makes module growth visible before it turns into a maintenance
problem. Avoid cosmetic reshuffles that do not reduce ownership confusion.
