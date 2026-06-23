# Test Fixtures

Gromaq keeps file-backed fixtures small and reviewable. A fixture update is a
behavior change unless the diff is only correcting stale expected output after a
deliberate code change.

## Terminal Golden Fixtures

`terminal_golden/` contains deterministic snapshots produced by
`tests/golden_fixtures.rs`.

Current fixtures:

- `ansi_scrollback_alternate.txt`: ANSI styling, scrollback, cursor state,
  performance counters, and alternate-screen restoration.
- `vt_unicode_osc.txt`: Unicode wide-cell and complex emoji cluster state plus
  OSC 8 hyperlink metadata.
- `vt_editing_status.txt`: VT editing, tab stops, DEC special graphics, cursor
  state, and status replies.
- `osc_clipboard_paste.txt`: OSC title/icon label state, OSC 52 clipboard text,
  bracketed paste encoding, and pending response bytes.
- `status_capability_reports.txt`: grouped terminal status, capability, mode,
  window, title, and icon-label response bytes.

## Update Workflow

1. Change the parser, terminal state, runtime plumbing, or formatter that owns
   the behavior.
2. Run the focused golden test and inspect the assertion diff:

   ```bash
   cargo test --test golden_fixtures
   ```

3. If the new output is correct, update only the affected fixture file.
4. Re-run the focused test, then run the repository gate:

   ```bash
   cargo fmt --check
   git diff --check
   cargo clippy --all-targets --all-features -- -D warnings
   cargo test --all
   cargo bench --bench parser_throughput -- --list
   ```

## Review Rules

- Do not bless fixture diffs without reading the rendered state and escaped
  response bytes.
- Keep viewport sizes, input sequences, and expected fields deterministic.
- Keep live-terminal claims out of these fixtures unless they were validated and
  documented separately.
- Prefer adding a focused fixture over expanding an existing one when the new
  behavior is unrelated to the fixture name.
