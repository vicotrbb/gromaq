use super::*;

#[test]
fn runtime_memory_smoke_reports_bounded_rss_growth() {
    let mut rss = 10_000_u64;

    let exit = runtime_memory_smoke_exit_with_sampler(|| {
        rss = rss.saturating_add(128);
        Ok(rss)
    });

    assert_eq!(exit.code, 0, "{exit:?}");
    assert!(exit.stdout.contains("runtime memory smoke: ok"));
    assert!(exit.stdout.contains("warmup batches: 1"));
    assert!(exit.stdout.contains("measured batches: 8"));
    assert!(exit.stdout.contains("lines: 4608"));
    assert!(exit.stdout.contains("rss growth cap kib: 65536"));
    assert!(
        exit.stdout
            .contains("last visible line: gromaq-memory-line-4607")
    );
    assert!(exit.stderr.is_empty());
}

#[test]
fn runtime_memory_smoke_rejects_rss_growth_over_cap() {
    let mut samples = 0_u64;

    let exit = runtime_memory_smoke_exit_with_sampler(|| {
        samples = samples.saturating_add(1);
        if samples == 1 {
            Ok(10_000)
        } else {
            Ok(10_000 + RUNTIME_MEMORY_SMOKE_RSS_GROWTH_LIMIT_KIB + 1)
        }
    });

    assert_eq!(exit.code, 1);
    assert!(exit.stdout.is_empty());
    assert!(
        exit.stderr
            .contains("process rss growth exceeded configured cap")
    );
}
