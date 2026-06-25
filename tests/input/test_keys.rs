use gromaq::{KeyModifiers, TestKey, encode_keys};

#[test]
fn encodes_common_terminal_keys_to_bytes() {
    let keys = [
        TestKey::Char('a'),
        TestKey::Enter,
        TestKey::Backspace,
        TestKey::ArrowUp,
        TestKey::ArrowRight,
    ];

    assert_eq!(encode_keys(&keys), b"a\r\x7f\x1b[A\x1b[C");
}

#[test]
fn encodes_control_modified_ascii_characters() {
    let keys = [TestKey::ModifiedChar {
        ch: 'c',
        modifiers: KeyModifiers::CTRL,
    }];

    assert_eq!(encode_keys(&keys), vec![0x03]);
}

#[test]
fn encodes_control_modified_ascii_punctuation() {
    let cases = [
        (' ', 0x00),
        ('@', 0x00),
        ('2', 0x00),
        ('[', 0x1b),
        ('3', 0x1b),
        (']', 0x1d),
        ('5', 0x1d),
        ('\\', 0x1c),
        ('4', 0x1c),
        ('^', 0x1e),
        ('6', 0x1e),
        ('_', 0x1f),
        ('/', 0x1f),
        ('7', 0x1f),
        ('?', 0x7f),
        ('8', 0x7f),
    ];

    for (ch, expected) in cases {
        let keys = [TestKey::ModifiedChar {
            ch,
            modifiers: KeyModifiers::CTRL,
        }];
        assert_eq!(encode_keys(&keys), vec![expected], "Ctrl+{ch}");
    }
}
