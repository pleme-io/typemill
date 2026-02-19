//! Centralized performance-related environment variable names and defaults.

pub const PERF_ASSERT_STRICT: &str = "TYPEMILL_PERF_ASSERT_STRICT";
pub const MATRIX_ARTIFACT_DIR: &str = "TYPEMILL_MATRIX_ARTIFACT_DIR";

pub const PERF_MAX_DIRECTORY_MOVE_DETECTOR_MS: &str =
    "TYPEMILL_PERF_MAX_DIRECTORY_MOVE_DETECTOR_MS";
pub const PERF_MAX_DIRECTORY_MOVE_DOC_REWRITE_MS: &str =
    "TYPEMILL_PERF_MAX_DIRECTORY_MOVE_DOC_REWRITE_MS";
pub const PERF_MAX_DIRECTORY_MOVE_CONVERT_MS: &str = "TYPEMILL_PERF_MAX_DIRECTORY_MOVE_CONVERT_MS";

pub const PRUNE_TIER_120_MAX_MS: &str = "TYPEMILL_PRUNE_TIER_120_MAX_MS";
pub const PRUNE_TIER_500_MAX_MS: &str = "TYPEMILL_PRUNE_TIER_500_MAX_MS";
pub const PRUNE_TIER_1000_MAX_MS: &str = "TYPEMILL_PRUNE_TIER_1000_MAX_MS";

pub const DEFAULT_DIRECTORY_MOVE_DETECTOR_MS: u128 = 100;
pub const DEFAULT_DIRECTORY_MOVE_DOC_REWRITE_MS: u128 = 1_500;
pub const DEFAULT_DIRECTORY_MOVE_CONVERT_MS: u128 = 300;

pub fn env_truthy(var_name: &str) -> bool {
    std::env::var(var_name)
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false)
}

pub fn env_u128(var_name: &str, default: u128) -> u128 {
    std::env::var(var_name)
        .ok()
        .and_then(|v| v.parse::<u128>().ok())
        .unwrap_or(default)
}
