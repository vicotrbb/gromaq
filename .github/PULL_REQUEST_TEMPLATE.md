## Summary

-

## Proof Boundary

Choose the strongest evidence this pull request provides:

- deterministic test/API evidence
- real PTY evidence
- offscreen GPU evidence
- release packaging evidence
- live desktop/window evidence

## Commands Run

```bash
cargo fmt --check
git diff --check
git diff --cached --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all
cargo test --test project_policy
```

## Screenshots Or Artifacts

Add links or paths for visual, packaging, compatibility, or release-facing
changes.

## Known Limitations

-
