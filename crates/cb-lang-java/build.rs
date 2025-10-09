use std::process::Command;

fn main() {
    check_command("mvn", "Maven is required to build the Java parser. Please install it and ensure it's in your PATH. You can run 'make check-parser-deps' for more details.");
    check_command("java", "A Java runtime is required to build the Java parser. Please install it and ensure it's in your PATH. You can run 'make check-parser-deps' for more details.");
}

fn check_command(command: &str, message: &str) {
    if !is_command_in_path(command) {
        println!("cargo:warning={}", message);
    }
}

fn is_command_in_path(command: &str) -> bool {
    Command::new(command)
        .arg("--version")
        .output()
        .is_ok()
}