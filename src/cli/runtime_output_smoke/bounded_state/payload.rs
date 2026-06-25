use super::{RUNTIME_BOUNDED_STATE_BATCHES, RUNTIME_LARGE_OUTPUT_LINES};

pub(super) fn runtime_bounded_state_payloads() -> Vec<Vec<u8>> {
    (0..RUNTIME_BOUNDED_STATE_BATCHES)
        .map(|batch| {
            let start = batch * RUNTIME_LARGE_OUTPUT_LINES;
            let end = start + RUNTIME_LARGE_OUTPUT_LINES;
            let mut payload = Vec::new();
            for line in start..end {
                payload.extend_from_slice(format!("gromaq-bounded-line-{line:04}\n").as_bytes());
            }
            payload
        })
        .collect()
}
