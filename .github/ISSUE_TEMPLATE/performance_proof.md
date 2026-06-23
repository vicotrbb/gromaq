---
name: Performance proof gap
about: Report or request measured evidence for frame time, latency, CPU, memory, throughput, or cache behavior
title: "perf: "
labels: ["performance", "needs-proof"]
assignees: []
---

## Target

Choose the acceptance target:

- 144Hz frame pacing
- p95 frame time below 6.94 ms
- input latency p95 below 10 ms
- near-zero idle CPU
- bounded memory growth
- large-output throughput
- smooth scrollback
- glyph cache efficiency
- hot-path allocation reduction

## Measurement Environment

- Gromaq revision:
- OS and version:
- Hardware/GPU:
- Display refresh rate:
- Viewport size:
- Shell/program/workload:

## Command Or Scenario

```bash
paste exact benchmark, smoke, or live workflow command here
```

## Result

Include raw values and units:

- p95 frame time:
- input latency p95:
- dropped frames:
- idle CPU:
- process memory:
- throughput:
- glyph cache hit rate:

## Proof Boundary

Choose one:

- Criterion benchmark
- deterministic runtime smoke
- offscreen GPU smoke
- live desktop/window measurement

## Regression Baseline

Compare against the nearest previous result or state that no baseline is
available.
