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
}
