pub mod utils;

/// A simple greeting function for testing
pub fn say_hello() -> String {
    "Hello from source_crate!".to_string()
}

/// Another function to test consolidation
pub fn get_version() -> &'static str {
    "1.0.0"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_say_hello() {
        assert_eq!(say_hello(), "Hello from source_crate!");
    }
}
