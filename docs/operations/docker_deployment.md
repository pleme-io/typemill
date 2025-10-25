# Docker Deployment

Everything you need to deploy TypeMill with Docker—from local development to production.

## Quick Start: Development

For local development with automatic hot-reloading:

```bash
# Start server with automatic rebuild on file changes
docker-compose -f deployment/docker/docker-compose.yml up

# Rebuild development image
docker-compose -f deployment/docker/docker-compose.yml build
```

Any change saved to a `.rs` file triggers automatic recompile and restart inside the container.

## Production Deployment

Production deployment uses a minimal, hardened image without development tools:

```bash
# Set secure JWT secret for authentication
export JWT_SECRET="your-secure-secret-key"

# Start production stack in background
docker-compose -f deployment/docker/docker-compose.yml \
  -f deployment/docker/docker-compose.production.yml up -d

# Check service health (via nginx)
curl http://localhost/health
```

## Architecture

### Development Stack (`docker-compose.yml`)

**Components:**
- **`mill`** - Application server with Rust toolchain and `cargo-watch`
- **Source Mount** - Local `./rust` directory mounted for live reloading
- **Build Cache** - Volumes for `cargo` registry and `target` directory

**Features:**
- Automatic rebuild on file changes
- Persistent build artifacts across restarts
- Fast incremental compilation

### Production Stack (`docker-compose.production.yml`)

**Components:**
- **`mill`** - Minimal hardened image (compiled binary only)
- **`nginx`** - Reverse proxy with SSL/TLS termination

**Features:**
- Security-hardened runtime
- SSL/TLS support via nginx
- Health check endpoints
- Production logging

## Configuration

Configuration is managed via environment variables in `docker-compose.*.yml` files.

### Environment Variables

**Required for Production:**
- `JWT_SECRET` - Secret key for JWT authentication (use strong random value)

**Optional:**
- `RUST_LOG` - Log level (debug/info/warn/error)
- `PORT` - Application port (default: 3000)

**Using .env File:**
```bash
# Create .env file for production secrets
echo "JWT_SECRET=$(openssl rand -hex 32)" > .env
docker-compose -f deployment/docker/docker-compose.yml \
  -f deployment/docker/docker-compose.production.yml up -d
```

## Building Images

Force rebuild with `--no-cache` flag:

```bash
# Rebuild development image
docker-compose -f deployment/docker/docker-compose.yml build --no-cache

# Rebuild production image
docker-compose -f deployment/docker/docker-compose.yml \
  -f deployment/docker/docker-compose.production.yml build --no-cache
```

## Viewing Logs

```bash
# Development logs (follow)
docker-compose -f deployment/docker/docker-compose.yml logs -f

# Production logs (follow)
docker-compose -f deployment/docker/docker-compose.yml \
  -f deployment/docker/docker-compose.production.yml logs -f

# Filter by service
docker-compose logs -f mill
docker-compose logs -f nginx
```

## Health Monitoring

### Health Check Endpoints

```bash
# Application health (direct)
curl http://localhost:3000/health

# Application health (via nginx - production)
curl http://localhost/health

# Detailed status
curl http://localhost:3000/api/v1/status
```

### Expected Responses

**Healthy:**
```json
{
  "status": "healthy",
  "uptime_seconds": 3600,
  "version": "1.0.0"
}
```

**Unhealthy:**
```json
{
  "status": "unhealthy",
  "error": "LSP server connection failed"
}
```

## Security Best Practices

### Production Deployment

1. **Always use strong JWT secret:**
   ```bash
   export JWT_SECRET="$(openssl rand -hex 32)"
   ```

2. **Enable SSL/TLS in nginx:**
   - Configure SSL certificates in `nginx.conf`
   - Use Let's Encrypt for free certificates

3. **Restrict network access:**
   - Use firewall rules to limit exposed ports
   - Only expose port 80/443 (nginx) to public

4. **Regular updates:**
   ```bash
   # Rebuild with latest base images
   docker-compose pull
   docker-compose build --no-cache
   ```

### FUSE Capabilities

⚠️ **FUSE is EXPERIMENTAL and development-only**

FUSE requires `SYS_ADMIN` capability which disables container security boundaries:

```yaml
# DO NOT USE IN PRODUCTION
services:
  mill-fuse:
    cap_add:
      - SYS_ADMIN  # Required for FUSE, disables security
```

**To disable FUSE:**
Set `"fuse": null` in `.typemill/config.json`

## Troubleshooting

### Common Issues

**Port already in use:**
```bash
# Check what's using the port
lsof -i :3000

# Use different port
PORT=3041 docker-compose up
```

**Permission denied:**
```bash
# Fix ownership (development)
sudo chown -R $USER:$USER target/
```

**Build cache issues:**
```bash
# Clear Docker build cache
docker builder prune

# Remove volumes
docker-compose down -v
```

**LSP server not starting:**
```bash
# Check LSP configuration
cat .typemill/config.json

# View detailed logs
RUST_LOG=debug docker-compose up
```

## Performance Tuning

### Development

**Faster builds:**
```yaml
# Use sccache in Dockerfile
ENV RUSTC_WRAPPER=sccache
```

**Reduce disk usage:**
```bash
# Clean build artifacts
docker-compose exec mill cargo clean

# Remove unused images
docker image prune
```

### Production

**Optimize image size:**
- Use multi-stage builds (already configured)
- Strip debug symbols from binary
- Use Alpine-based images where possible

**Resource limits:**
```yaml
services:
  mill:
    deploy:
      resources:
        limits:
          cpus: '2'
          memory: 2G
```

## See Also

- [Architecture Documentation](../architecture/overview.md) - System design
- [Configuration Guide](../../README.md#configuration) - Setup options
- [Security Policy](../../SECURITY.md) - Security practices
