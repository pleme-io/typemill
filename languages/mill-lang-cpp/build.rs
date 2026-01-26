use std::path::PathBuf;

fn main() {
    let dir: PathBuf = ["tree-sitter-cpp", "src"].iter().collect();
    let parser_file = dir.join("parser.c");

    // Check if the tree-sitter submodule is initialized
    if !parser_file.exists() {
        eprintln!();
        eprintln!("╔══════════════════════════════════════════════════════════════════════╗");
        eprintln!("║  ERROR: tree-sitter-cpp submodule not initialized                    ║");
        eprintln!("║                                                                      ║");
        eprintln!("║  The C++ language parser requires the tree-sitter-cpp git submodule.║");
        eprintln!("║  Please run the following command to initialize it:                 ║");
        eprintln!("║                                                                      ║");
        eprintln!("║    git submodule update --init --recursive                          ║");
        eprintln!("║                                                                      ║");
        eprintln!("║  Or run 'make first-time-setup' for complete setup.                 ║");
        eprintln!("╚══════════════════════════════════════════════════════════════════════╝");
        eprintln!();
        panic!("Missing git submodule: tree-sitter-cpp");
    }

    cc::Build::new()
        .include(&dir)
        .file(parser_file)
        .file(dir.join("scanner.c"))
        .compile("tree-sitter-cpp");
}
