# Docker Deployment Guide

## Quick Start: Development

For local development with automatic hot-reloading, run:

```bash
# This will start the server and watch for file changes in ./rust
docker-compose -f deployment/docker/docker-compose.yml up

# To rebuild the development image
docker-compose -f deployment/docker/docker-compose.yml build
```

Any change saved to a `.rs` file on the host machine will automatically trigger a recompile and restart of the server inside the container.

## Production Deployment

For production, use the `docker-compose -f deployment/docker/docker-compose.yml.production.yml` file. This uses a minimal, secure image without any development tools.

```bash
# Set a secure JWT secret for authentication
export JWT_SECRET="your-secure-secret-key"

# Start the production stack in the background
docker-compose -f deployment/docker/docker-compose.yml -f docker-compose -f deployment/docker/docker-compose.yml.production.yml up -d

# Check the health of the service (via nginx)
curl http://localhost/health
```

## Architecture

### Development (`docker-compose -f deployment/docker/docker-compose.yml.yml`)
- **`codebuddy`**: The main application server running in a development image that includes the Rust toolchain and `cargo-watch`.
- **Source Code Mount**: The local `./rust` directory is mounted directly into the container, allowing `cargo-watch` to detect changes and trigger a rebuild.
- **Build Caching**: Volumes are used to cache the `cargo` registry and `target` build directory, persisting dependencies and build artifacts across restarts to speed up subsequent builds.

### Production (`docker-compose -f deployment/docker/docker-compose.yml.production.yml`)
- **`codebuddy`**: The main application running from a minimal, hardened production image. It contains only the compiled binary and necessary runtime libraries.
- **`nginx`**: A reverse proxy that handles incoming traffic, provides SSL/TLS termination, and forwards requests to the `codebuddy` service.

## Configuration

Configuration is managed via environment variables in the `docker-compose -f deployment/docker/docker-compose.yml.*.yml` files. For production, you can create a `.env` file to manage secrets like `JWT_SECRET`.

## Building

While `docker-compose -f deployment/docker/docker-compose.yml up` builds the image automatically, you can force a rebuild:

```bash
# Rebuild the development image
docker-compose -f deployment/docker/docker-compose.yml build --no-cache

# Rebuild the production image
docker-compose -f deployment/docker/docker-compose.yml -f docker-compose -f deployment/docker/docker-compose.yml.production.yml build --no-cache
```

## Logs

```bash
# View development logs
docker-compose -f deployment/docker/docker-compose.yml logs -f

# View production logs
docker-compose -f deployment/docker/docker-compose.yml -f docker-compose -f deployment/docker/docker-compose.yml.production.yml logs -f
```
