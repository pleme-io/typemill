# Using the Codebuddy CLI

This guide provides a comprehensive overview of how to install, configure, and use the `codebuddy` command-line interface.

## Installation

You can build and install the CLI directly from the source code using `cargo`.

1.  Navigate to the root of the Rust workspace:
    ```bash
    cd /path/to/codebuddy/rust
    ```

2.  Install the client binary:
    ```bash
    cargo install --path crates/cb-client
    ```

This will place the `codebuddy` executable in your Cargo binary directory (usually `~/.cargo/bin/`), which should be in your system's `PATH`.

## Configuration

The CLI can be configured from three sources, in the following order of precedence:

1.  **Command-line arguments:** (e.g., `--url <URL>`) - Highest precedence.
2.  **Environment variables:** (`CODEBUDDY_URL`, `CODEBUDDY_TOKEN`).
3.  **Configuration file:** (`~/.codebuddy/config.json`) - Lowest precedence.

### Environment Variables

You can configure the CLI by setting the following environment variables:

- `CODEBUDDY_URL`: The WebSocket URL of the Codebuddy server (e.g., `ws://localhost:3000`).
- `CODEBUDDY_TOKEN`: The authentication token for the server.

```bash
export CODEBUDDY_URL="ws://localhost:3000"
export CODEBUDDY_TOKEN="your-secret-token"
```

### Configuration File

You can also use the interactive setup wizard to create a configuration file:

```bash
codebuddy setup
```

This will guide you through the process and create a config file at `~/.codebuddy/config.json`.

## Shell Completions

To make the CLI easier to use, you can generate auto-completion scripts for your shell.

### Bash

Add the following to your `~/.bashrc` file:

```bash
source <(codebuddy completions bash)
```

### Zsh

Add the following to your `~/.zshrc` file:

```bash
source <(codebuddy completions zsh)
```

### Fish

Add the following to your `~/.config/fish/config.fish` file:

```fish
codebuddy completions fish | source
```

## Common Commands

Here are some of the most common commands and their usage.

### `status`

Check the client's status and verify connectivity to the server.

```bash
# Basic status check
codebuddy status

# Verbose status check with connection details
codebuddy status --verbose
```

### `call`

Execute a raw MCP tool on the server. This is useful for scripting and advanced operations.

```bash
# Read a file from the server's workspace
codebuddy call read_file '{"file_path":"/path/to/your/file.txt"}'

# List files in the root directory
codebuddy call list_files '{"recursive":true}'

# Output the result as raw JSON
codebuddy call get_hover '{"file_path":"/src/index.ts","line":10,"character":5}' --format json
```

### `connect`

Start an interactive session with the server (functionality to be expanded in future versions).

```bash
codebuddy connect --url ws://custom-server:4000
```
