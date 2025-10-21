use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::PathBuf;

#[derive(Args)]
pub struct InstallArgs {
    /// Install to a custom directory
    #[arg(long)]
    dest: Option<PathBuf>,

    /// Skip building, just copy existing binary
    #[arg(long)]
    skip_build: bool,

    /// Development install (don't optimize)
    #[arg(long)]
    dev: bool,
}

pub fn run(args: InstallArgs) -> Result<()> {
    println!("{}", "Installing codebuddy...\n".bold());

    // Build
    if !args.skip_build {
        let profile = if args.dev { "dev" } else { "release" };
        println!("Building in {} mode...", profile.cyan());

        let mut cmd = vec!["build", "-p", "codebuddy"];
        if !args.dev {
            cmd.push("--release");
        }

        crate::utils::run_cmd("cargo", &cmd)?;
        println!("{}", "✓ Build complete\n".green());
    }

    // Determine binary location
    let profile_dir = if args.dev { "debug" } else { "release" };
    let binary_name = if cfg!(windows) {
        "codebuddy.exe"
    } else {
        "codebuddy"
    };
    let binary_path = PathBuf::from("target").join(profile_dir).join(binary_name);

    if !binary_path.exists() {
        anyhow::bail!("Binary not found at {:?}", binary_path);
    }

    // Install
    let dest = args.dest.unwrap_or_else(default_install_dir);
    std::fs::create_dir_all(&dest).context("Failed to create install directory")?;

    let dest_binary = dest.join(binary_name);
    std::fs::copy(&binary_path, &dest_binary).context("Failed to copy binary")?;

    // Make executable on Unix
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&dest_binary)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&dest_binary, perms)?;
    }

    println!(
        "{} Installed to: {}",
        "✓".green(),
        dest_binary.display().to_string().cyan()
    );

    // Check if in PATH
    if !is_in_path(&dest) {
        println!("\n{}", "⚠️  Install directory not in PATH".yellow());
        println!("Add to your shell profile:");
        println!(
            "  {}",
            format!("export PATH=\"{}:$PATH\"", dest.display()).cyan()
        );
    } else {
        println!(
            "\n{} Installation complete! Run: {}",
            "✓".green(),
            "codebuddy --version".cyan()
        );
    }

    Ok(())
}

fn default_install_dir() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        PathBuf::from(home).join(".local/bin")
    } else if let Ok(userprofile) = std::env::var("USERPROFILE") {
        // Windows
        PathBuf::from(userprofile).join(".cargo").join("bin")
    } else {
        PathBuf::from("/usr/local/bin")
    }
}

fn is_in_path(dir: &PathBuf) -> bool {
    if let Ok(path_var) = std::env::var("PATH") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        path_var.split(separator).any(|p| PathBuf::from(p) == *dir)
    } else {
        false
    }
}
