use std::time::Duration;

use gromaq::pty::PtySession;

pub(crate) fn drain_until_contains(
    session: &mut PtySession,
    expected: &str,
    attempts: usize,
    pause: Duration,
) -> String {
    let mut output = Vec::new();
    for _ in 0..attempts {
        output.extend(session.drain_available_output().unwrap());
        if String::from_utf8_lossy(&output).contains(expected) {
            break;
        }
        std::thread::sleep(pause);
    }
    String::from_utf8_lossy(&output).into_owned()
}

pub(crate) fn drain_until_contains_stripped(
    session: &mut PtySession,
    expected: &str,
    attempts: usize,
    pause: Duration,
) -> String {
    let mut output = Vec::new();
    for _ in 0..attempts {
        output.extend(session.drain_available_output().unwrap());
        let normalized = strip_ansi_sequences(&String::from_utf8_lossy(&output));
        if normalized.contains(expected) {
            return normalized;
        }
        std::thread::sleep(pause);
    }
    strip_ansi_sequences(&String::from_utf8_lossy(&output))
}

pub(crate) fn drain_until_any_output(
    session: &mut PtySession,
    attempts: usize,
    pause: Duration,
) -> Vec<u8> {
    let mut output = Vec::new();
    for _ in 0..attempts {
        output.extend(session.drain_available_output().unwrap());
        if !output.is_empty() {
            break;
        }
        std::thread::sleep(pause);
    }
    output
}

pub(crate) fn strip_ansi_sequences(output: &str) -> String {
    let bytes = output.as_bytes();
    let mut stripped = Vec::with_capacity(bytes.len());
    let mut index = 0;

    while index < bytes.len() {
        if bytes[index] != 0x1b {
            stripped.push(bytes[index]);
            index += 1;
            continue;
        }

        index += 1;
        match bytes.get(index).copied() {
            Some(b'[') => {
                index += 1;
                while index < bytes.len() {
                    let byte = bytes[index];
                    index += 1;
                    if (0x40..=0x7e).contains(&byte) {
                        break;
                    }
                }
            }
            Some(b'(' | b')' | b'*' | b'+') => {
                index = (index + 2).min(bytes.len());
            }
            Some(_) => index += 1,
            None => {}
        }
    }

    String::from_utf8(stripped).unwrap()
}
