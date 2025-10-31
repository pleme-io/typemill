//! System-level utilities

/// Check if a command exists on the system's PATH
pub fn command_exists(cmd: &str) -> bool {
    std::process::Command::new(if cfg!(target_os = "windows") {
        "where"
    } else {
        "command"
    })
    .arg("-v")
    .arg(cmd)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .status()
    .is_ok_and(|status| status.success())
}

/// Test if a command works by running it with --version
///
/// Returns (success, version_or_error_message)
///
/// # Arguments
/// * `cmd` - The command to test
/// * `args` - Additional arguments to pass before --version
///
/// # Examples
/// ```
/// use mill_foundation::core::utils::system;
/// let (success, output) = system::test_command_with_version("rustc", &[]);
/// assert!(success);
/// ```
pub fn test_command_with_version(cmd: &str, args: &[&str]) -> (bool, String) {
    use std::process::Command;

    // Try: cmd args --version
    let mut full_args = args.to_vec();
    full_args.push("--version");

    match Command::new(cmd).args(&full_args).output() {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            (true, version)
        }
        Ok(output) => {
            // Try with -v instead
            let mut args_v = args.to_vec();
            args_v.push("-v");
            match Command::new(cmd).args(&args_v).output() {
                Ok(out2) if out2.status.success() => {
                    let version = String::from_utf8_lossy(&out2.stdout).trim().to_string();
                    (true, version)
                }
                _ => {
                    let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    (false, err)
                }
            }
        }
        Err(e) => (false, format!("Failed to execute: {}", e)),
    }
}

/// Resolve command to full path if in PATH
///
/// Returns absolute path if found, None otherwise
///
/// # Arguments
/// * `cmd` - The command to resolve
///
/// # Examples
/// ```
/// use mill_foundation::core::utils::system;
/// let path = system::resolve_command_path("rustc");
/// assert!(path.is_some());
/// ```
pub fn resolve_command_path(cmd: &str) -> Option<std::path::PathBuf> {
    // Try using which crate
    which::which(cmd).ok()
}

/// Check if running in CI environment
///
/// Detects common CI environment variables from:
/// - Generic CI
/// - GitHub Actions
/// - GitLab CI
/// - CircleCI
/// - Jenkins
/// - Travis CI
///
/// # Examples
/// ```
/// use mill_foundation::core::utils::system;
/// let in_ci = system::is_ci();
/// ```
pub fn is_ci() -> bool {
    std::env::var("CI").is_ok()
        || std::env::var("GITHUB_ACTIONS").is_ok()
        || std::env::var("GITLAB_CI").is_ok()
        || std::env::var("CIRCLECI").is_ok()
        || std::env::var("JENKINS_URL").is_ok()
        || std::env::var("TRAVIS").is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ci_detects_ci_env() {
        // Save original value if it exists
        let original_ci = std::env::var("CI").ok();

        // Test with CI env var set
        std::env::set_var("CI", "true");
        assert!(is_ci());

        // Clean up
        match original_ci {
            Some(val) => std::env::set_var("CI", val),
            None => std::env::remove_var("CI"),
        }
    }

    #[test]
    fn test_is_ci_detects_github_actions() {
        let original = std::env::var("GITHUB_ACTIONS").ok();

        std::env::set_var("GITHUB_ACTIONS", "true");
        assert!(is_ci());

        match original {
            Some(val) => std::env::set_var("GITHUB_ACTIONS", val),
            None => std::env::remove_var("GITHUB_ACTIONS"),
        }
    }

    #[test]
    fn test_is_ci_without_ci_env() {
        // Ensure no CI vars are set for this test
        let vars = [
            "CI",
            "GITHUB_ACTIONS",
            "GITLAB_CI",
            "CIRCLECI",
            "JENKINS_URL",
            "TRAVIS",
        ];
        let saved: Vec<_> = vars.iter().map(|v| (*v, std::env::var(v).ok())).collect();

        // Remove all CI vars
        for var in &vars {
            std::env::remove_var(var);
        }

        assert!(!is_ci());

        // Restore original values
        for (var, val) in saved {
            match val {
                Some(v) => std::env::set_var(var, v),
                None => std::env::remove_var(var),
            }
        }
    }

    #[test]
    fn test_resolve_command_path_finds_rustc() {
        // rustc should be available since we're running cargo
        let path = resolve_command_path("rustc");
        assert!(path.is_some(), "rustc should be found in PATH");

        if let Some(p) = path {
            assert!(p.is_absolute(), "Path should be absolute");
        }
    }

    #[test]
    fn test_resolve_command_path_missing_command() {
        // Use a command that definitely doesn't exist
        let path = resolve_command_path("this-command-does-not-exist-12345");
        assert!(path.is_none());
    }

    #[test]
    fn test_test_command_with_version_rustc() {
        // Test with rustc which should be available
        let (success, output) = test_command_with_version("rustc", &[]);
        assert!(success, "rustc --version should succeed");
        assert!(!output.is_empty(), "Version output should not be empty");
        assert!(output.contains("rustc"), "Output should contain 'rustc'");
    }

    #[test]
    fn test_test_command_with_version_missing() {
        // Test with non-existent command
        let (success, _output) =
            test_command_with_version("this-command-does-not-exist-12345", &[]);
        assert!(!success, "Non-existent command should fail");
    }

    // Note: Full testing of test_command_with_version with LSP servers
    // requires those servers to be installed, so we test with rustc here
}
