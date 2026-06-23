pub fn terminal_fixture_value() -> u8 {
    42
}

#[cfg(test)]
mod tests {
    use super::terminal_fixture_value;

    #[test]
    fn fixture_test_passes() {
        assert_eq!(terminal_fixture_value(), 42);
    }

    #[test]
    fn fixture_emits_large_test_output() {
        for index in 0..256 {
            println!("gromaq-cargo-output-{index:03}");
        }

        assert_eq!(terminal_fixture_value(), 42);
    }
}
