use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct CheckDuplicatesArgs {
    /// Minimum token threshold for duplication
    #[arg(long, default_value = "50")]
    min_tokens: usize,

    /// Output format (json, console)
    #[arg(long, default_value = "console")]
    format: String,
}

pub fn run(args: CheckDuplicatesArgs) -> Result<()> {
    println!("{}", "Checking for duplicate code...\n".bold());

    // Check if jscpd is installed
    if !is_jscpd_installed() {
        println!("{}", "⚠️  jscpd not found".yellow());
        println!("Install with: {}", "npm install -g jscpd".cyan());
        println!("\nSkipping duplicate code check...");
        return Ok(());
    }

    let output = crate::utils::run_cmd_output(
        "jscpd",
        &[
            ".",
            "--min-tokens",
            &args.min_tokens.to_string(),
            "--format",
            &args.format,
        ],
    )?;

    println!("{}", output);

    // Parse output to determine if duplicates found
    if output.contains("duplicates") || output.contains("Duplicates") {
        println!("{}", "\n⚠️  Duplicates detected".yellow());
        std::process::exit(1);
    } else {
        println!("{}", "✓ No significant duplicates found".green());
        Ok(())
    }
}

fn is_jscpd_installed() -> bool {
    std::process::Command::new("jscpd")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}
