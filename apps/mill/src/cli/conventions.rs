//! Naming convention utilities for CLI operations
//!
//! This module provides string/filename conversion between naming conventions:
//! - kebab-case
//! - snake_case
//! - camelCase
//! - PascalCase

use std::path::Path;

/// Convert a filename from one naming convention to another
///
/// # Examples
///
/// ```
/// use mill::cli::conventions::convert_filename;
/// assert_eq!(convert_filename("user-profile.js", "kebab-case", "camelCase"), Some("userProfile.js".to_string()));
/// assert_eq!(convert_filename("user_name.rs", "snake_case", "camelCase"), Some("userName.rs".to_string()));
/// assert_eq!(convert_filename("UserData.ts", "PascalCase", "kebab-case"), Some("user-data.ts".to_string()));
/// ```
pub fn convert_filename(filename: &str, from: &str, to: &str) -> Option<String> {
    let path = Path::new(filename);
    let stem = path.file_stem()?.to_str()?;
    let extension = path.extension().and_then(|e| e.to_str());

    // Convert the stem (filename without extension)
    let converted_stem = convert_string(stem, from, to)?;

    // Rebuild filename with extension
    if let Some(ext) = extension {
        Some(format!("{}.{}", converted_stem, ext))
    } else {
        Some(converted_stem)
    }
}

/// Convert a string from one naming convention to another
fn convert_string(s: &str, from: &str, to: &str) -> Option<String> {
    // First, split into words based on the source convention
    let words = split_by_convention(s, from)?;

    // Then, join words using the target convention
    Some(join_by_convention(&words, to))
}

/// Split a string into words based on naming convention
fn split_by_convention(s: &str, convention: &str) -> Option<Vec<String>> {
    match convention {
        "kebab-case" => Some(s.split('-').map(|w| w.to_lowercase()).collect()),
        "snake_case" => Some(s.split('_').map(|w| w.to_lowercase()).collect()),
        "camelCase" => {
            // Split on capital letters: userName -> ["user", "Name"]
            let mut words = Vec::new();
            let mut current_word = String::new();

            for (i, ch) in s.chars().enumerate() {
                if ch.is_uppercase() && i > 0 {
                    if !current_word.is_empty() {
                        words.push(current_word.to_lowercase());
                    }
                    current_word = ch.to_string();
                } else {
                    current_word.push(ch);
                }
            }
            if !current_word.is_empty() {
                words.push(current_word.to_lowercase());
            }
            Some(words)
        }
        "PascalCase" => {
            // Split on capital letters: UserName -> ["User", "Name"]
            let mut words = Vec::new();
            let mut current_word = String::new();

            for ch in s.chars() {
                if ch.is_uppercase() && !current_word.is_empty() {
                    words.push(current_word.to_lowercase());
                    current_word = ch.to_string();
                } else {
                    current_word.push(ch);
                }
            }
            if !current_word.is_empty() {
                words.push(current_word.to_lowercase());
            }
            Some(words)
        }
        _ => None, // Unsupported convention
    }
}

/// Join words using a naming convention
fn join_by_convention(words: &[String], convention: &str) -> String {
    match convention {
        "kebab-case" => words.join("-"),
        "snake_case" => words.join("_"),
        "camelCase" => {
            if words.is_empty() {
                String::new()
            } else {
                let first = words[0].to_lowercase();
                let rest: String = words[1..].iter().map(|w| capitalize_first(w)).collect();
                format!("{}{}", first, rest)
            }
        }
        "PascalCase" => words.iter().map(|w| capitalize_first(w)).collect(),
        _ => words.join(""),
    }
}

/// Capitalize the first character of a string
fn capitalize_first(s: &str) -> String {
    let mut chars = s.chars();
    match chars.next() {
        None => String::new(),
        Some(first) => first.to_uppercase().chain(chars).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_convert_filename_kebab_to_camel() {
        assert_eq!(
            convert_filename("user-profile.js", "kebab-case", "camelCase"),
            Some("userProfile.js".to_string())
        );
    }

    #[test]
    fn test_convert_filename_snake_to_camel() {
        assert_eq!(
            convert_filename("user_name.rs", "snake_case", "camelCase"),
            Some("userName.rs".to_string())
        );
    }

    #[test]
    fn test_convert_filename_pascal_to_kebab() {
        assert_eq!(
            convert_filename("UserData.ts", "PascalCase", "kebab-case"),
            Some("user-data.ts".to_string())
        );
    }

    #[test]
    fn test_convert_filename_camel_to_snake() {
        assert_eq!(
            convert_filename("myFunction.py", "camelCase", "snake_case"),
            Some("my_function.py".to_string())
        );
    }

    #[test]
    fn test_convert_filename_no_extension() {
        assert_eq!(
            convert_filename("userProfile", "camelCase", "snake_case"),
            Some("user_profile".to_string())
        );
    }

    #[test]
    fn test_convert_filename_unsupported_convention() {
        assert_eq!(
            convert_filename("test.rs", "unknown-case", "snake_case"),
            None
        );
    }
}
