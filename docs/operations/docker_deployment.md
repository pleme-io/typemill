# Docker Deployment

> **Status:** Production Docker images are currently planned but not yet published.

## Current Recommendation

For production deployments, we recommend building from source using Cargo:

```bash
# Build release binary
cargo install mill --locked --path .

# Run setup
mill setup

# Start server
mill start
```

## Development

If you are looking for the development container environment, see **[../development/dev-container.md](../development/dev-container.md)**.
