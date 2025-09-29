# Contributing to Codeflow Buddy (Rust)

First off, thank you for considering contributing! It's people like you that make Codeflow Buddy such a great tool.

## Getting Started

### Prerequisites

- **Rust Toolchain:** This project is built with Rust. If you don't have it installed, you can get it from [rustup.rs](https://rustup.rs/).

### Setup

1.  **Clone the repository:**
    ```bash
    git clone https://github.com/goobits/codebuddy.git
    cd codebuddy/rust
    ```

2.  **Build the project:**
    ```bash
    cargo build
    ```
    This will download all dependencies and compile the Rust crates.

## Running Tests

We have a comprehensive test suite to ensure code quality and prevent regressions.

To run all tests for the entire Rust workspace:
```bash
cargo test --workspace
```

## Code Style and Linting

We use the standard Rust formatting and linting tools to maintain a consistent codebase.

- **Formatting:** Before committing your changes, please format your code with `cargo fmt`.
  ```bash
  cargo fmt --all
  ```

- **Linting:** We use `clippy` for catching common mistakes and improving code quality.
  ```bash
  cargo clippy --all-targets -- -D warnings
  ```

## Pull Request Process

1.  **Create a Feature Branch:**
    ```bash
    git checkout -b your-feature-name
    ```

2.  **Commit Your Changes:** Make your changes and commit them with a descriptive message.
    ```bash
    git commit -m "feat: Add new feature" -m "Detailed description of the changes."
    ```

3.  **Ensure Tests Pass:** Run the full test suite one last time to make sure everything is working correctly.
    ```bash
    cargo test --workspace
    ```

4.  **Push to Your Branch:**
    ```bash
    git push origin your-feature-name
    ```

5.  **Open a Pull Request:** Go to the repository on GitHub and open a new pull request. Provide a clear title and description of your changes.
