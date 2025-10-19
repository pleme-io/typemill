use anyhow::Result;
use clap::Args;
use colored::Colorize;

#[derive(Args)]
pub struct CheckFeaturesArgs {
    /// Check a specific package
    #[arg(long)]
    package: Option<String>,
}

pub fn run(args: CheckFeaturesArgs) -> Result<()> {
    println!("{}", "Checking cargo features...\n".bold());

    // Build with different feature combinations to verify they work
    let checks = vec![
        ("default features", vec!["check", "--workspace"]),
        ("all features", vec!["check", "--workspace", "--all-features"]),
        ("no default features", vec!["check", "--workspace", "--no-default-features"]),
    ];

    for (desc, cmd_args) in checks {
        println!("Checking {}...", desc.cyan());

        let mut full_args = cmd_args.clone();
        if let Some(ref pkg) = args.package {
            full_args.push("-p");
            full_args.push(pkg.as_str());
        }

        crate::utils::run_cmd("cargo", &full_args)?;
        println!("{} {} passed", "✓".green(), desc);
    }

    println!("\n{}", "✓ All feature checks passed".green());
    Ok(())
}
