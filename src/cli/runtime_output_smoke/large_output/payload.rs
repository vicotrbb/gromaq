pub(super) fn runtime_large_output_payload(lines: usize) -> Vec<u8> {
    let mut payload = Vec::new();
    for line in 0..lines {
        payload.extend_from_slice(format!("gromaq-runtime-line-{line:03}\n").as_bytes());
    }
    payload
}
