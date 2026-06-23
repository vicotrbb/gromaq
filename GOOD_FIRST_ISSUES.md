# Good First Issue Candidates

These candidates are intentionally narrow. Each one should preserve the proof
boundary language in `COMPATIBILITY.md`, use the debugging workflow in
`DEBUGGING.md`, and finish with the required root checks from `CONTRIBUTING.md`.

## Documentation And Proof Hygiene

- Add a compatibility row evidence update after a focused deterministic test or
  smoke command adds new proof. Keep the row honest about what remains unproven.
- Add a small reference note to `DEBUGGING.md` for a newly introduced smoke
  command, including what it proves and what it does not prove.
- Expand `TESTING.md` with fixture guidance for one existing test file whose
  conventions are not yet obvious from the current list.
- Add a short troubleshooting note to `BENCHMARKS.md` for a benchmark dependency
  or skip condition observed on a real machine.

## Test Coverage

- Add one golden terminal-state fixture for a small ANSI, VT editing, OSC, or
  Unicode scenario that is already supported but not yet represented in
  `tests/fixtures/terminal_golden/`.
- Add one focused selection/copy test for an existing supported cell metadata
  case, such as copying from displayed scrollback with rich style metadata.
- Add one focused resize/reflow assertion for an existing supported cluster or
  metadata case, then update the relevant `COMPATIBILITY.md` evidence sentence.
- Add one CLI smoke output assertion for a metric or snapshot field that the
  smoke already computes but does not yet print.

## Compatibility Scouting

- Run one optional real-PTY workflow from `tests/pty.rs` on a machine with the
  required tool installed, record the version and proof boundary, and update
  `COMPATIBILITY.md` only if the evidence is durable.
- Compare one small deterministic escape-sequence fixture against a reference
  terminal and document the expected state in a test comment or fixture note.
- Add a tiny regression fixture for an issue-template reproduction that includes
  exact input bytes, viewport size, expected state, and proof boundary.

## Performance And Runtime Evidence

- Add one benchmark-list assertion or documentation update when a benchmark name
  changes, so `cargo bench --bench parser_throughput -- --list` remains
  discoverable.
- Improve one runtime smoke error message to include the measured value and
  expected threshold when the smoke already has both values available.
- Add one deterministic runtime metric assertion in `tests/cli.rs` for an
  existing smoke command without broadening it into live performance proof.

Avoid issues that require live 144Hz proof, live window screenshots, packaging,
or broad editor/multiplexer workflows until a maintainer confirms the hardware,
OS, and proof plan.
