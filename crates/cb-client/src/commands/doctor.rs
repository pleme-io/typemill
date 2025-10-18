use crate::commands::{Command, GlobalArgs};
use crate::ClientResult;
use codebuddy_config::config::AppConfig;

pub struct DoctorCommand;

impl Default for DoctorCommand {
    fn default() -> Self {
        Self::new()
    }
}

impl DoctorCommand {
    pub fn new() -> Self {
        Self
    }

    /// Main execution function for the doctor command.
    async fn execute_inner(&self, _args: &GlobalArgs) -> ClientResult<()> {
        println!("🩺 Running Codebuddy Doctor...");

        // 1. Check for and validate the configuration file.
        self.check_config_file();

        // 2. Load the config to check the language servers.
        if let Ok(config) = AppConfig::load() {
            self.check_language_servers(&config.lsp.servers).await;
        }

        // Add more checks here in the future...

        println!("\n✨ Doctor's checkup complete.");
        Ok(())
    }

    /// Checks if the config file exists and is valid.
    fn check_config_file(&self) {
        print!("Checking for configuration file... ");
        match AppConfig::load() {
            Ok(_) => println!("[✓] Found and parsed successfully."),
            Err(e) => {
                println!("[✗] Error: {}", e);
                println!("  > Run `codebuddy setup` to create a new configuration file.");
            }
        }
    }

    /// Checks for the existence of configured LSP servers.
    async fn check_language_servers(&self, servers: &[codebuddy_config::config::LspServerConfig]) {
        println!("\nChecking language servers:");
        for server in servers {
            let cmd = &server.command[0];
            print!(
                "  - Checking for '{}' (for {})... ",
                cmd,
                server.extensions.join(", ")
            );
            if self.command_exists(cmd) {
                println!("[✓] Found in PATH.");
            } else {
                println!("[✗] Not found in PATH.");
                println!(
                    "    > Please install '{}' and ensure it is available in your system's PATH.",
                    cmd
                );
            }
        }
    }

    /// Helper to check if a command exists on the system's PATH.
    fn command_exists(&self, cmd: &str) -> bool {
        codebuddy_core::utils::system::command_exists(cmd)
    }
}

#[async_trait::async_trait]
impl Command for DoctorCommand {
    async fn execute(&self, args: &GlobalArgs) -> ClientResult<()> {
        self.execute_inner(args).await
    }

    fn name(&self) -> &'static str {
        "doctor"
    }

    fn description(&self) -> &'static str {
        "Check client configuration and diagnose potential problems"
    }
}