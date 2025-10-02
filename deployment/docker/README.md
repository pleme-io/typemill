# Docker Deployment Guide

## Quick Start: Development

For local development with automatic hot-reloading, run:

```bash
# This will start the server and watch for file changes in ./rust
deployment/docker-compose up

# To rebuild the development image
deployment/docker-compose build
```

Any change saved to a `.rs` file on the host machine will automatically trigger a recompile and restart of the server inside the container.

## Production Deployment

For production, use the `deployment/docker-compose.production.yml` file. This uses a minimal, secure image without any development tools.

```bash
# Set a secure JWT secret for authentication
export JWT_SECRET="your-secure-secret-key"

# Start the production stack in the background
deployment/docker-compose -f deployment/docker-compose.production.yml up -d

# Check the health of the service (via nginx)
curl http://localhost/health
```

## Architecture

### Development (`deployment/docker-compose.yml`)
- **`codebuddy`**: The main application server running in a development image that includes the Rust toolchain and `cargo-watch`.
- **Source Code Mount**: The local `./rust` directory is mounted directly into the container, allowing `cargo-watch` to detect changes and trigger a rebuild.
- **Build Caching**: Volumes are used to cache the `cargo` registry and `target` build directory, persisting dependencies and build artifacts across restarts to speed up subsequent builds.

### Production (`deployment/docker-compose.production.yml`)
- **`codebuddy`**: The main application running from a minimal, hardened production image. It contains only the compiled binary and necessary runtime libraries.
- **`nginx`**: A reverse proxy that handles incoming traffic, provides SSL/TLS termination, and forwards requests to the `codebuddy` service.

## Configuration

Configuration is managed via environment variables in the `deployment/docker-compose.*.yml` files. For production, you can create a `.env` file to manage secrets like `JWT_SECRET`.

## Building

While `deployment/docker-compose up` builds the image automatically, you can force a rebuild:

```bash
# Rebuild the development image
deployment/docker-compose build --no-cache

# Rebuild the production image
deployment/docker-compose -f deployment/docker-compose.production.yml build --no-cache
```

## Logs

```bash
# View development logs
deployment/docker-compose logs -f

# View production logs
deployment/docker-compose -f deployment/docker-compose.production.yml logs -f
```
