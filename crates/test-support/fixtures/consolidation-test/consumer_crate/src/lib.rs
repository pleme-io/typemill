/// Consumer crate - uses source_crate
use source_crate::say_hello;
use source_crate::get_version;

pub fn greet() -> String {
    // Bug #5 test: This inline fully-qualified path should be updated
    let msg = source_crate::say_hello();
    format!("{} (version: {})", msg, get_version())
}

pub fn format_user_greeting(name: &str) -> String {
    // Bug #5 test: Another inline reference that should be updated
    source_crate::utils::format_greeting(name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greet() {
        let greeting = greet();
        assert!(greeting.contains("Hello"));
    }
}
