use crate::error::{ClientError, ClientResult};
use crate::formatting::Formatter;
use console::Term;
use dialoguer::{theme::ColorfulTheme, Confirm, FuzzySelect, Input, Password, Select};
use url::Url;

/// Interactive prompt utilities for CLI user interaction
#[derive(Default)]
pub struct Interactive {
    formatter: Formatter,
    theme: ColorfulTheme,
}

impl Interactive {
    /// Create a new interactive instance
    pub fn new() -> Self {
        Self::default()
    }

    /// Create an interactive instance with specific formatter
    pub fn with_formatter(formatter: Formatter) -> Self {
        Self {
            formatter,
            theme: ColorfulTheme::default(),
        }
    }

    /// Display an informational message
    pub fn info(&self, message: &str) {
        println!("{}", self.formatter.info(message));
    }

    /// Display a banner with title and optional description
    pub fn banner(&self, title: &str, description: Option<&str>) -> ClientResult<()> {
        println!("{}", self.formatter.title(title));
        if let Some(desc) = description {
            println!("{}", self.formatter.subtitle(desc));
        }
        println!();
        Ok(())
    }

    /// Display a progress message
    pub fn progress_message(&self, message: &str) {
        println!("{}", self.formatter.progress_message(message));
    }

    /// Display a success message
    pub fn success_message(&self, message: &str) {
        println!("{}", self.formatter.success_message(message));
    }

    /// Display an error message
    pub fn error_message(&self, message: &str) {
        println!("{}", self.formatter.error_message(message));
    }

    /// Display a warning message
    pub fn warning_message(&self, message: &str) {
        println!("{}", self.formatter.warning(message));
    }

    /// Prompt for a string input with validation
    pub fn input<F>(
        &self,
        prompt: &str,
        default: Option<&str>,
        validator: F,
    ) -> ClientResult<String>
    where
        F: Fn(&String) -> Result<(), String> + 'static,
    {
        let mut input_prompt = Input::with_theme(&self.theme);
        input_prompt = input_prompt.with_prompt(prompt);

        if let Some(default_value) = default {
            input_prompt = input_prompt.default(default_value.to_string());
        }

        input_prompt
            .validate_with(validator)
            .interact_text()
            .map_err(|e| ClientError::IoError(format!("Failed to get input: {}", e)))
    }

    /// Prompt for a URL with validation
    pub fn url_input(&self, prompt: &str, default: Option<&str>) -> ClientResult<String> {
        self.input(prompt, default, |input| {
            Url::parse(input.as_str())
                .map(|_| ())
                .map_err(|e| format!("Invalid URL: {}", e))
        })
    }

    /// Prompt for a non-empty string
    pub fn required_input(&self, prompt: &str, default: Option<&str>) -> ClientResult<String> {
        self.input(prompt, default, |input| {
            if input.trim().is_empty() {
                Err("This field is required".to_string())
            } else {
                Ok(())
            }
        })
    }

    /// Prompt for an optional string (can be empty)
    pub fn optional_input(
        &self,
        prompt: &str,
        default: Option<&str>,
    ) -> ClientResult<Option<String>> {
        let result = self.input(prompt, default, |_| Ok(()))?;
        if result.trim().is_empty() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Prompt for a password
    pub fn password(&self, prompt: &str, confirm: bool) -> ClientResult<String> {
        let mut password_prompt = Password::with_theme(&self.theme);
        password_prompt = password_prompt.with_prompt(prompt);

        if confirm {
            password_prompt =
                password_prompt.with_confirmation("Confirm password", "Passwords don't match");
        }

        password_prompt
            .interact()
            .map_err(|e| ClientError::IoError(format!("Failed to get password: {}", e)))
    }

    /// Prompt for a confirmation (yes/no)
    pub fn confirm(&self, prompt: &str, default: Option<bool>) -> ClientResult<bool> {
        let confirm_prompt = Confirm::with_theme(&self.theme);
        let confirm_prompt = confirm_prompt.with_prompt(prompt);

        let confirm_prompt = if let Some(default_value) = default {
            confirm_prompt.default(default_value)
        } else {
            confirm_prompt
        };

        confirm_prompt
            .interact()
            .map_err(|e| ClientError::IoError(format!("Failed to get confirmation: {}", e)))
    }

    /// Prompt for selection from a list of options
    pub fn select(
        &self,
        prompt: &str,
        options: &[&str],
        default: Option<usize>,
    ) -> ClientResult<usize> {
        let mut select_prompt = Select::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(options);

        if let Some(default_index) = default {
            if default_index < options.len() {
                select_prompt = select_prompt.default(default_index);
            }
        }

        select_prompt
            .interact()
            .map_err(|e| ClientError::IoError(format!("Failed to get selection: {}", e)))
    }

    /// Prompt for fuzzy selection from a list of options
    pub fn fuzzy_select(
        &self,
        prompt: &str,
        options: &[&str],
        default: Option<usize>,
    ) -> ClientResult<usize> {
        let mut select_prompt = FuzzySelect::with_theme(&self.theme)
            .with_prompt(prompt)
            .items(options);

        if let Some(default_index) = default {
            if default_index < options.len() {
                select_prompt = select_prompt.default(default_index);
            }
        }

        select_prompt
            .interact()
            .map_err(|e| ClientError::IoError(format!("Failed to get selection: {}", e)))
    }

    /// Prompt for a numeric input
    pub fn number_input<T>(&self, prompt: &str, default: Option<T>) -> ClientResult<T>
    where
        T: std::str::FromStr + std::fmt::Display + Clone,
        T::Err: std::fmt::Display,
    {
        let default_str = default.as_ref().map(|v| v.to_string());
        let default_ref = default_str.as_deref();

        self.input(prompt, default_ref, |input| {
            input
                .parse::<T>()
                .map(|_| ())
                .map_err(|e| format!("Invalid number: {}", e))
        })?
        .parse()
        .map_err(|e| ClientError::IoError(format!("Failed to parse number: {}", e)))
    }

    /// Display a message and wait for user to press Enter
    pub fn pause(&self, message: &str) -> ClientResult<()> {
        println!("{}", message);
        let _ = Input::<String>::with_theme(&self.theme)
            .with_prompt("Press Enter to continue")
            .allow_empty(true)
            .interact_text()
            .map_err(|e| ClientError::IoError(format!("Failed to pause: {}", e)))?;
        Ok(())
    }

    /// Clear the screen
    pub fn clear_screen(&self) -> ClientResult<()> {
        Term::stdout()
            .clear_screen()
            .map_err(|e| ClientError::IoError(format!("Failed to clear screen: {}", e)))
    }

    /// Display an error and ask if user wants to retry
    pub fn retry_on_error(&self, error: &str, default_retry: bool) -> ClientResult<bool> {
        println!("{}", self.formatter.error(error));
        self.confirm("Would you like to try again?", Some(default_retry))
    }

    /// Multi-step wizard pattern
    pub fn wizard<T, F>(&self, title: &str, steps: Vec<(&str, F)>) -> ClientResult<Vec<T>>
    where
        F: Fn(&Self) -> ClientResult<T>,
    {
        self.banner(title, None)?;

        let mut results = Vec::new();

        for (i, (step_name, step_fn)) in steps.into_iter().enumerate() {
            println!(
                "{}",
                self.formatter
                    .info(&format!("Step {}: {}", i + 1, step_name))
            );
            println!();

            let result = step_fn(self)?;
            results.push(result);

            println!();
        }

        Ok(results)
    }

    /// Validate and get server URL
    pub fn get_server_url(&self, current_url: Option<&str>) -> ClientResult<String> {
        let default_url = current_url.unwrap_or("ws://localhost:3000");

        loop {
            match self.url_input("Server URL", Some(default_url)) {
                Ok(url) => {
                    // Additional validation for WebSocket URLs
                    if url.starts_with("ws://") || url.starts_with("wss://") {
                        return Ok(url);
                    } else if url.starts_with("http://") {
                        let ws_url = url.replace("http://", "ws://");
                        if self.confirm(
                            &format!("Convert to WebSocket URL: {}?", ws_url),
                            Some(true),
                        )? {
                            return Ok(ws_url);
                        }
                    } else if url.starts_with("https://") {
                        let wss_url = url.replace("https://", "wss://");
                        if self.confirm(
                            &format!("Convert to secure WebSocket URL: {}?", wss_url),
                            Some(true),
                        )? {
                            return Ok(wss_url);
                        }
                    } else {
                        // Assume it needs ws:// prefix
                        let ws_url = format!("ws://{}", url);
                        if self
                            .confirm(&format!("Add WebSocket protocol: {}?", ws_url), Some(true))?
                        {
                            return Ok(ws_url);
                        }
                    }
                }
                Err(e) => {
                    if !self.retry_on_error(&format!("Invalid URL: {}", e), true)? {
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Get timeout with validation
    pub fn get_timeout(&self, current_timeout: Option<u64>) -> ClientResult<u64> {
        let default_timeout = current_timeout.unwrap_or(30000);

        loop {
            match self.number_input("Request timeout (milliseconds)", Some(default_timeout)) {
                Ok(timeout) => {
                    if timeout == 0 {
                        if !self.retry_on_error("Timeout cannot be zero", true)? {
                            return Err(ClientError::ConfigError("Invalid timeout".to_string()));
                        }
                        continue;
                    }
                    if timeout > 300_000 {
                        if !self
                            .retry_on_error("Timeout cannot exceed 5 minutes (300000ms)", true)?
                        {
                            return Err(ClientError::ConfigError("Invalid timeout".to_string()));
                        }
                        continue;
                    }
                    return Ok(timeout);
                }
                Err(e) => {
                    if !self.retry_on_error(&format!("Invalid timeout: {}", e), true)? {
                        return Err(e);
                    }
                }
            }
        }
    }

    /// Display configuration summary and get confirmation
    pub fn confirm_config(&self, config_summary: &str) -> ClientResult<bool> {
        println!();
        println!("{}", self.formatter.header("Configuration Summary"));
        println!("{}", config_summary);
        println!();

        self.confirm("Save this configuration?", Some(true))
    }

    /// Show a list of available actions and get user choice
    pub fn action_menu(&self, title: &str, actions: &[&str]) -> ClientResult<usize> {
        println!();
        println!("{}", self.formatter.header(title));
        self.select("Choose an action", actions, Some(0))
    }
}

/// Create a default interactive instance
pub fn interactive() -> Interactive {
    Interactive::new()
}

/// Convenience function to get user confirmation
pub fn confirm(prompt: &str) -> ClientResult<bool> {
    interactive().confirm(prompt, None)
}

/// Convenience function to get user input
pub fn input(prompt: &str) -> ClientResult<String> {
    interactive().required_input(prompt, None)
}

/// Convenience function to get user selection
pub fn select(prompt: &str, options: &[&str]) -> ClientResult<usize> {
    interactive().select(prompt, options, None)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interactive_creation() {
        let _interactive = Interactive::new();
        // Test passes if creation doesn't panic
    }

    #[test]
    fn test_formatter_integration() {
        let formatter = Formatter::with_settings(false, false);
        let _interactive = Interactive::with_formatter(formatter);
        // Test passes if creation doesn't panic
    }

    // Note: Most interactive functions can't be easily unit tested
    // since they require user input. Integration tests would be better.
}
