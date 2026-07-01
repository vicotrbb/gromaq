# Roadmap

## Phase 1: Terminal Foundation

- Deterministic terminal grid/state engine
- ANSI parser integration
- VT editing/navigation subset
- OSC title handling
- Bracketed paste mode encoding
- Scrollback
- Visible-grid resize reflow with styles and wide cells
- Alternate-screen restoration
- Dirty-region tracking
- Visible-grid selection/copy
- SGR mouse reporting mode and press/release/drag/motion event encoding
- Real PTY shell-command lifecycle test
- Input encoding
- Config validation
- Test API
- PTY and renderer boundaries
- Deterministic 144Hz frame scheduler
- Deterministic glyph atlas cache
- Benchmark harness

## Phase 2: Core Terminal Correctness

- Alternate screen
- Tab stops
- Insert/delete character and line operations
- Full SGR coverage
- Host clipboard integration and explicit OSC 52 policy
- End-to-end mouse workflows in `tmux`, editors, and alternate-screen apps
- System clipboard integration
- Full scrollback reflow during resize
- Golden fixtures and reference-terminal comparisons

## Phase 3: Native Application

- `winit` event loop
- `wgpu` device and swapchain setup
- Glyph rasterization and GPU atlas upload
- Text shaping and rasterization
- Dirty-region renderer
- Hardware-backed frame scheduler integration
- Input-to-render latency metrics

## Phase 4: Compatibility and Performance Proof

- Shell PTY lifecycle tests
- `vim`, `nvim`, `tmux`, `ssh`, and pager scenarios
- Native tmux assist and manager proof, including action discovery, isolated
  action execution, state reading, native manager action/name-entry paths, and
  workspace preset launchers
- Default native tmux snapshot artifact proof for the startup manager/status
  strip frame with isolated live tmux state
- Screenshot/reference comparisons
- 144Hz frame pacing proof
- Idle CPU and memory-growth validation
- Documented compatibility matrix

## Phase 5: Daily-Driver Hardening

- Crash recovery
- Config files and reload
- Cross-platform packaging
- Accessibility review
- Release automation

## Post-0.2.0 Proof Work

- Accepted live desktop screenshot proof for the default native window
- Hardware-backed 144 Hz proof on a 144 Hz-capable display
- Developer ID signed and notarized macOS release
- Broader macOS and Linux host compatibility matrix
- Live desktop menu, Dock/Finder, Linux menu UI, and OS paste-menu workflows
- Manual app-window proof for the full native tmux manager workflow, including
  keyboard/mouse interaction in the real `cargo run` window and
  `live-app-window-proof.txt = completed`
- v1 daily-driver hardening for stability, recovery, accessibility, and
  long-session behavior
