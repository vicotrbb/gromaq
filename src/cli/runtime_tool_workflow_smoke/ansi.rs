//! ANSI normalization for PTY command output.

pub(super) fn strip_ansi_sequences(output: &str) -> String {
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

    String::from_utf8(stripped).expect("stripped ANSI output remains UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ansi_stripping_removes_csi_sequences() {
        assert_eq!(
            strip_ansi_sequences("\x1b[31mClient Version\x1b[0m"),
            "Client Version"
        );
    }
}
