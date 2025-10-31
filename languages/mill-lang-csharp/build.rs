use std::process::Command;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");

    // Check for .NET SDK
    let output = Command::new("dotnet")
        .arg("--version")
        .output();

    match output {
        Ok(output) if output.status.success() => {
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            println!("cargo:warning=Found .NET SDK version: {}. C# parser can be built.", version);
        }
        _ => {
            println!("cargo:warning=---------------------------------------------------------------------");
            println!("cargo:warning= .NET SDK not found or 'dotnet' command failed.");
            println!("cargo:warning= The C# language plugin requires the .NET SDK to build its parser.");
            println!("cargo:warning= Please install the .NET SDK (>= 6.0) and ensure 'dotnet' is in your PATH.");
            println!("cargo:warning= For installation instructions, see: https://dotnet.microsoft.com/download");
            println!("cargo:warning=---------------------------------------------------------------------");
        }
    }
}