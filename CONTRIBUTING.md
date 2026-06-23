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

## Required Checks

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo bench
```

## Pull Requests

Each pull request should state:

- What behavior changed
- What tests were added
- What benchmarks were run
- What is explicitly not proven yet
