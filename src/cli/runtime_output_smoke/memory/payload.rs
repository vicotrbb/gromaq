use super::{RUNTIME_LARGE_OUTPUT_LINES, RUNTIME_MEMORY_SMOKE_MEASURED_BATCHES};

pub(super) const RUNTIME_MEMORY_SMOKE_WARMUP_BATCHES: usize = 1;

pub(super) fn runtime_memory_payloads() -> Vec<Vec<u8>> {
    let total_batches = RUNTIME_MEMORY_SMOKE_WARMUP_BATCHES + RUNTIME_MEMORY_SMOKE_MEASURED_BATCHES;
    (0..total_batches)
        .map(|batch| {
            let start = batch * RUNTIME_LARGE_OUTPUT_LINES;
            let end = start + RUNTIME_LARGE_OUTPUT_LINES;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-memory-line-{line:04}\n").as_bytes());
            }
            payload
        })
        .collect()
}
