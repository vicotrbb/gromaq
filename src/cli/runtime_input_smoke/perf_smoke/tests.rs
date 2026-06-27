use super::*;

#[test]
fn runtime_perf_budget_accepts_p95_values_within_limits() {
    let probe = RuntimePerfProbe {
        pumped_bytes: 1,
        expected_samples: 1,
        metrics: crate::app::NativeRuntimePerfSnapshot {
            render_time_p95_ns: RUNTIME_RENDER_P95_BUDGET_NS,
            input_to_render_p95_ns: RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS,
            ..Default::default()
        },
    };

    assert_eq!(runtime_perf_budget_failure(&probe), None);
}

#[test]
fn runtime_perf_budget_rejects_render_p95_over_144hz_budget() {
    let probe = RuntimePerfProbe {
        pumped_bytes: 1,
        expected_samples: 1,
        metrics: crate::app::NativeRuntimePerfSnapshot {
            render_time_p95_ns: RUNTIME_RENDER_P95_BUDGET_NS + 1,
            input_to_render_p95_ns: RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS,
            ..Default::default()
        },
    };

    assert_eq!(
        runtime_perf_budget_failure(&probe),
        Some(
            "render p95 exceeded 144Hz frame budget: measured 6940001 ns, budget 6940000 ns"
                .to_owned()
        )
    );
}

#[test]
fn runtime_perf_budget_rejects_input_to_render_p95_over_latency_budget() {
    let probe = RuntimePerfProbe {
        pumped_bytes: 1,
        expected_samples: 1,
        metrics: crate::app::NativeRuntimePerfSnapshot {
            render_time_p95_ns: RUNTIME_RENDER_P95_BUDGET_NS,
            input_to_render_p95_ns: RUNTIME_INPUT_TO_RENDER_P95_BUDGET_NS + 1,
            ..Default::default()
        },
    };

    assert_eq!(
        runtime_perf_budget_failure(&probe),
        Some(
            "input-to-render p95 exceeded latency budget: measured 10000001 ns, budget 10000000 ns"
                .to_owned()
        )
    );
}

#[test]
fn runtime_perf_probe_collects_repeated_samples() {
    let probe = run_runtime_perf_probe(4).unwrap();

    assert_eq!(probe.expected_samples, 4);
    assert_eq!(probe.pumped_bytes, 4);
    assert_eq!(probe.metrics.rendered_frames, 4);
    assert_eq!(probe.metrics.render_time_samples, 4);
    assert_eq!(probe.metrics.input_to_render_samples, 4);
}
