# TypeMill CI/CD Guide

This guide provides instructions for setting up and using TypeMill in a fully automated Continuous Integration and Continuous Deployment (CI/CD) environment. The goal is to enable non-interactive installation, configuration, and execution of TypeMill tools.

## GitHub Actions Workflow

Here is an example of a complete GitHub Actions workflow that installs, configures, and verifies TypeMill. You can use this as a template for your own projects.

```yaml
name: TypeMill CI Test

on: [push, pull_request]

jobs:
  test-mill-install:
    runs-on: ubuntu-latest
    name: Test TypeMill Installation from Scratch

    steps:
      - uses: actions/checkout@v4

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable
        with:
          components: rust-analyzer

      - name: Setup Node.js
        uses: actions/setup-node@v4
        with:
          node-version: '20.x'

      - name: Cache cargo
        uses: actions/cache@v4
        with:
          path: |
            ~/.cargo/bin/
            ~/.cargo/registry/index/
            ~/.cargo/registry/cache/
            ~/.cargo/git/db/
            target/
          key: ${{ runner.os }}-cargo-${{ hashFiles('**/Cargo.lock') }}

      - name: Install TypeMill via cargo
        run: |
          cargo install mill --locked
          mill --version

      - name: Install Language Servers
        run: |
          npm install -g typescript-language-server typescript

      - name: Configure TypeMill
        run: |
          mkdir -p .typemill
          cat > .typemill/config.json <<'EOF'
          {
            "servers": [
              {
                "extensions": ["ts", "tsx", "js", "jsx"],
                "command": ["typescript-language-server", "--stdio"]
              },
              {
                "extensions": ["rs"],
                "command": ["rust-analyzer"]
              }
            ]
          }
          EOF

      - name: Create dummy source file for testing
        run: |
          mkdir -p src
          echo 'function hello() { console.log("hello world"); }' > src/main.ts

      - name: Run TypeMill Tools to Verify Installation
        run: |
          echo "Running status and doctor..."
          mill status
          mill doctor

          echo "Running get_diagnostics..."
          # This command will test if the LSP server is working via mill
          mill tool get_diagnostics --file-path "src/main.ts"
```text
### Workflow Breakdown

1.  **Checkout Code**: The workflow begins by checking out your repository's code.
2.  **Install Rust and Node.js**: It sets up the necessary environments for Rust and Node.js. Node.js is required for the `typescript-language-server`.
3.  **Cache Dependencies**: Caching is used for `cargo` dependencies to speed up subsequent runs.
4.  **Install TypeMill**: TypeMill is installed non-interactively using `cargo install mill --locked`. The `--locked` flag ensures a reproducible build based on the `Cargo.lock` file.
5.  **Install Language Servers**: The `typescript-language-server` is installed globally using `npm`. `rust-analyzer` is installed as a rustup component.
6.  **Configure TypeMill**: A configuration file is created programmatically at `.typemill/config.json`. This avoids any interactive setup prompts.
7.  **Create a Test File**: A dummy `src/main.ts` file is created to have a target for the TypeMill tool commands.
8.  **Verify Installation**: The workflow runs `mill status`, `mill doctor`, and `mill tool get_diagnostics` to confirm that the installation is successful and the language servers are operational.

## Other CI/CD Environments

The principles from the GitHub Actions workflow can be applied to other CI/CD systems like GitLab CI, Jenkins, or CircleCI.

-   **Use a Docker Image**: For environments that support Docker, you can create a `Dockerfile` that encapsulates all the installation and configuration steps. This creates a portable and reproducible environment.
-   **Script the Steps**: For other environments, you can create a shell script that executes the same sequence of commands as in the GitHub Actions workflow.

The key is to ensure that all steps are non-interactive and that configuration is handled programmatically.