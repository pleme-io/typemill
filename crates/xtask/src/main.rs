use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

mod check_duplicates;
mod check_features;
mod install;
mod new_lang;
mod utils;

#[derive(Parser)]
#[command(name = "xtask")]
#[command(about = "Codebuddy build automation tasks", long_about = None)]
#[command(version)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Install codebuddy and set up development environment
    Install(install::InstallArgs),

    /// Check for duplicate code in the codebase
    CheckDuplicates(check_duplicates::CheckDuplicatesArgs),

    /// Check cargo feature configurations
    CheckFeatures(check_features::CheckFeaturesArgs),

    /// Create a new language plugin scaffold
    NewLang(new_lang::NewLangArgs),

    /// Run all checks (fmt, clippy, test, deny)
    CheckAll,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Install(args) => install::run(args),
        Command::CheckDuplicates(args) => check_duplicates::run(args),
        Command::CheckFeatures(args) => check_features::run(args),
        Command::NewLang(args) => new_lang::run(args),
        Command::CheckAll => run_all_checks(),
    }
}

fn run_all_checks() -> Result<()> {
    println!("{}\n", "Running all checks...".bold());

    // Format check
    println!("{}", "Checking code formatting...".cyan());
    utils::run_cmd("cargo", &["fmt", "--check"])?;
    println!("{} Format check passed\n", "✓".green());

    // Clippy
    println!("{}", "Running clippy...".cyan());
    utils::run_cmd(
        "cargo",
        &[
            "clippy",
            "--workspace",
            "--all-targets",
            "--all-features",
            "--",
            "-D",
            "warnings",
        ],
    )?;
    println!("{} Clippy passed\n", "✓".green());

    // Tests
    println!("{}", "Running tests...".cyan());
    utils::run_cmd("cargo", &["nextest", "run", "--workspace"])?;
    println!("{} Tests passed\n", "✓".green());

    // Deny check
    println!("{}", "Running dependency audit...".cyan());
    utils::run_cmd("cargo", &["deny", "check"])?;
    println!("{} Dependency audit passed\n", "✓".green());

    println!("{}", "✓ All checks passed!".green().bold());
    Ok(())
}
