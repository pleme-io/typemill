use std::path::{Path, PathBuf};

fn main() {
    // Compile the vendored tree-sitter-java grammar
    let dir: PathBuf = ["tree-sitter-java", "src"].iter().collect();

    cc::Build::new()
        .include(&dir)
        .file(dir.join("parser.c"))
        .compile("tree-sitter-java");

    println!("cargo:rerun-if-changed=tree-sitter-java/src");

    // Declare the custom cfg flag for conditional compilation
    println!("cargo::rustc-check-cfg=cfg(java_parser_jar_exists)");

    // Check if the JAR file exists (use CARGO_MANIFEST_DIR for correct path resolution)
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let jar_path = Path::new(&manifest_dir).join("resources/java-parser/target/java-parser-1.0.0.jar");
    if jar_path.exists() {
        println!("cargo:rustc-cfg=java_parser_jar_exists");
        println!("cargo:rerun-if-changed=resources/java-parser/target/java-parser-1.0.0.jar");
    } else {
        println!("cargo:warning=Java parser JAR not found. Import support will not work.");
        println!("cargo:warning=To build the JAR: cd resources/java-parser && mvn package");
    }
}
