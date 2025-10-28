use std::env;
use std::path::PathBuf;

fn main() {
    // Tell cargo to rerun this build script if any file in docs/ changes
    let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    let docs_dir = PathBuf::from(&manifest_dir).join("../../docs");

    println!("cargo:rerun-if-changed={}", docs_dir.display());

    // Also rerun if the build script itself changes
    println!("cargo:rerun-if-changed=build.rs");

    // Walk the docs directory and tell cargo to watch all markdown files
    if docs_dir.exists() {
        for entry in walkdir::WalkDir::new(&docs_dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() {
                println!("cargo:rerun-if-changed={}", entry.path().display());
            }
        }
    }
}
