# Contributing

Gromaq is early. Contributions must preserve correctness, determinism, and performance evidence.

## Standards

- Use native Rust only.
- Do not add Electron, webviews, React, or JavaScript frontend runtimes.
- Do not add dependencies casually.
- Do not introduce `unsafe`.
- Treat warnings and Clippy findings as failures.
- Add tests before behavior changes.
- Document benchmark commands and proof boundaries.
- Follow the fixture guidance in `TESTING.md`.
- Follow the debugging workflow in `DEBUGGING.md` when investigating failures
  or adding compatibility evidence.
- Use the label taxonomy in `.github/labels.yml` and the issue templates under
  `.github/ISSUE_TEMPLATE/` when shaping contributor work.

## Required Checks

```bash
cargo fmt --check
git diff --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench --bench parser_throughput -- --list
```

Run full Criterion benchmarks with `cargo bench` when changing parser, PTY pump,
render planning, glyph cache, rasterization, frame preparation, or other
measured hot paths.

## Pull Requests

Each pull request should state:

- What behavior changed
- What tests were added
- What benchmarks were run
- What is explicitly not proven yet
- Any screenshots or release artifacts affected by the change
