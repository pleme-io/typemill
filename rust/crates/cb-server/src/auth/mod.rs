//! Authentication module

pub mod jwt;

pub use jwt::{generate_token, validate_token, validate_token_with_project, Claims};
