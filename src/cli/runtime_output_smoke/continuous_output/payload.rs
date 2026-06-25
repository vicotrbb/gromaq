use super::{RUNTIME_CONTINUOUS_OUTPUT_BATCHES, RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH};

pub(super) fn runtime_continuous_output_payloads() -> Vec<Vec<u8>> {
    (0..RUNTIME_CONTINUOUS_OUTPUT_BATCHES)
        .map(|batch| {
            let start = batch * RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let end = start + RUNTIME_CONTINUOUS_OUTPUT_LINES_PER_BATCH;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-continuous-line-{line:03}\n").as_bytes());
            }
            payload
        })
        .collect()
}
