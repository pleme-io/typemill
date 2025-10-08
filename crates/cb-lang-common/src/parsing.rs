//! Common parsing utilities and patterns
//!
//! Provides helpers for implementing resilient parsing with fallback strategies.

use tracing::debug;

/// Execute a primary parser with automatic fallback on failure
///
/// This is a common pattern across language plugins where an AST-based
/// parser is attempted first, with a fallback to regex-based parsing
/// if the AST parser fails.
///
/// # Arguments
///
/// * `primary` - Primary parsing function (usually AST-based)
/// * `fallback` - Fallback parsing function (usually regex-based)
/// * `operation` - Description of the operation for logging
///
/// # Returns
///
/// Result from either the primary or fallback parser
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::parsing::parse_with_fallback;
///
/// let imports = parse_with_fallback(
///     || parse_imports_ast(source),
///     || parse_imports_regex(source),
///     "import parsing"
/// )?;
/// ```
pub fn parse_with_fallback<T, E>(
    primary: impl FnOnce() -> Result<T, E>,
    fallback: impl FnOnce() -> Result<T, E>,
    operation: &str,
) -> Result<T, E>
where
    E: std::fmt::Display,
{
    match primary() {
        Ok(result) => {
            debug!(
                operation = %operation,
                parser = "primary",
                "Primary parser succeeded"
            );
            Ok(result)
        }
        Err(e) => {
            debug!(
                error = %e,
                operation = %operation,
                parser = "fallback",
                "Primary parser failed, using fallback"
            );
            fallback()
        }
    }
}

/// Execute a parser with optional fallback
///
/// Similar to `parse_with_fallback` but allows for an optional fallback.
/// If fallback is `None`, returns the primary error.
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::parsing::parse_with_optional_fallback;
///
/// let imports = parse_with_optional_fallback(
///     || parse_imports_ast(source),
///     Some(|| parse_imports_regex(source)),
///     "import parsing"
/// )?;
/// ```
pub fn parse_with_optional_fallback<T, E>(
    primary: impl FnOnce() -> Result<T, E>,
    fallback: Option<impl FnOnce() -> Result<T, E>>,
    operation: &str,
) -> Result<T, E>
where
    E: std::fmt::Display,
{
    match primary() {
        Ok(result) => Ok(result),
        Err(primary_error) => {
            if let Some(fallback_fn) = fallback {
                debug!(
                    error = %primary_error,
                    operation = %operation,
                    "Primary parser failed, using fallback"
                );
                fallback_fn()
            } else {
                Err(primary_error)
            }
        }
    }
}

/// Try multiple parsers in sequence until one succeeds
///
/// Useful when you have more than two parsing strategies.
///
/// # Example
///
/// ```rust,ignore
/// use cb_lang_common::parsing::try_parsers;
///
/// let result = try_parsers(
///     vec![
///         Box::new(|| parse_native_ast(source)),
///         Box::new(|| parse_tree_sitter(source)),
///         Box::new(|| parse_regex(source)),
///     ],
///     "symbol extraction"
/// )?;
/// ```
pub fn try_parsers<T, E>(
    parsers: Vec<Box<dyn FnOnce() -> Result<T, E>>>,
    operation: &str,
) -> Result<T, E>
where
    E: std::fmt::Display,
{
    let parsers = parsers.into_iter();
    let mut last_error = None;

    for parser in parsers {
        match parser() {
            Ok(result) => return Ok(result),
            Err(e) => {
                debug!(
                    error = %e,
                    operation = %operation,
                    "Parser failed, trying next"
                );
                last_error = Some(e);
            }
        }
    }

    // All parsers failed
    Err(last_error.expect("At least one parser must be provided"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_with_fallback_primary_succeeds() {
        let result = parse_with_fallback(|| Ok::<_, String>("primary"), || Ok("fallback"), "test");

        assert_eq!(result.unwrap(), "primary");
    }

    #[test]
    fn test_parse_with_fallback_primary_fails() {
        let result = parse_with_fallback(
            || Err::<&str, _>("primary error"),
            || Ok("fallback"),
            "test",
        );

        assert_eq!(result.unwrap(), "fallback");
    }

    #[test]
    fn test_parse_with_fallback_both_fail() {
        let result = parse_with_fallback(
            || Err::<&str, _>("primary error"),
            || Err("fallback error"),
            "test",
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "fallback error");
    }

    #[test]
    fn test_parse_with_optional_fallback_some() {
        let result = parse_with_optional_fallback(
            || Err::<&str, _>("primary error"),
            Some(|| Ok("fallback")),
            "test",
        );

        assert_eq!(result.unwrap(), "fallback");
    }

    #[test]
    fn test_parse_with_optional_fallback_none() {
        let result = parse_with_optional_fallback(
            || Err::<&str, _>("primary error"),
            None::<fn() -> Result<&'static str, &'static str>>,
            "test",
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "primary error");
    }

    #[test]
    fn test_try_parsers_first_succeeds() {
        let result = try_parsers(
            vec![
                Box::new(|| Ok::<_, String>("first")),
                Box::new(|| Ok("second")),
                Box::new(|| Ok("third")),
            ],
            "test",
        );

        assert_eq!(result.unwrap(), "first");
    }

    #[test]
    fn test_try_parsers_second_succeeds() {
        let result = try_parsers(
            vec![
                Box::new(|| Err::<&str, _>("first error")),
                Box::new(|| Ok("second")),
                Box::new(|| Ok("third")),
            ],
            "test",
        );

        assert_eq!(result.unwrap(), "second");
    }

    #[test]
    fn test_try_parsers_all_fail() {
        let result = try_parsers(
            vec![
                Box::new(|| Err::<&str, _>("first error")),
                Box::new(|| Err("second error")),
                Box::new(|| Err("third error")),
            ],
            "test",
        );

        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "third error");
    }
}
