---
name: Bug report
about: Report incorrect terminal behavior with reproducible evidence
title: "bug: "
labels: ["bug", "needs-triage"]
assignees: []
---

## Summary

Describe the incorrect behavior in one or two sentences.

## Reproduction

- Gromaq revision:
- OS and version:
- Shell or program:
- Viewport size:
- Command or input bytes:

```text
paste exact input, command, or escape sequence here
```

## Expected Behavior

Describe the expected grid, scrollback, cursor, response bytes, PTY output, or
rendered state.

## Observed Behavior

Include the smallest observed state that proves the bug:

- `dump_grid`:
- `dump_scrollback`:
- `dump_cursor`:
- response bytes:
- screenshot or GPU smoke output:

## Proof Boundary

Choose one:

- Deterministic test/API evidence
- Real PTY evidence
- Offscreen GPU evidence
- Live desktop/window evidence

## Checks Run

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench --bench parser_throughput -- --list
```

List any focused tests, smokes, or skipped checks.
