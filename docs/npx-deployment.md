# NPX Docker Deployment Guide

Zero-installation deployment of CodeFlow Buddy using NPX + Docker for always-fresh, multi-architecture support.

## Quick Start

### Option 1: Ultra-Quick Deploy (Single Command)
```bash
# Download and run quick deployment script
curl -fsSL https://raw.githubusercontent.com/goobits/codeflow-buddy/main/scripts/quick-npx.sh | bash

# Or clone and run locally
git clone https://github.com/goobits/codeflow-buddy.git
cd codeflow-buddy
./scripts/quick-npx.sh
```

### Option 2: Full Docker Compose Deploy
```bash
# Clone repository
git clone https://github.com/goobits/codeflow-buddy.git
cd codeflow-buddy

# Deploy with compose
./scripts/deploy-npx.sh

# Or manually
docker-compose -f docker-compose.npx.yml up --build -d
```

### Option 3: Direct Docker Run
```bash
# Always uses latest from npm
docker run -d \
  --name codeflow-buddy \
  -p 3000:3000 \
  -v ./workspaces:/workspace \
  --device /dev/fuse \
  --cap-add SYS_ADMIN \
  --security-opt apparmor:unconfined \
  node:22-alpine npx @goobits/codeflow-buddy@latest serve --enable-fuse
```

## Architecture Benefits

### NPX Advantages
- **Zero Installation**: No global installs or version management
- **Always Fresh**: Pulls latest version from npm every time
- **Multi-Architecture**: Works on ARM64, x64, and other platforms
- **Docker Security**: Isolated from host system
- **Automatic Updates**: Each deployment uses the newest version

### FUSE Integration
- **Native Support**: Uses `@cocalc/fuse-native` for ARM64 compatibility
- **Workspace Isolation**: Each tenant gets isolated filesystem access
- **Real File Operations**: Not mocked - actual FUSE mount operations
- **Multi-Tenant**: Supports multiple isolated workspaces

## Configuration

### Environment Variables
```bash
PORT=3000                # Service port
MAX_CLIENTS=50          # Maximum concurrent clients
NODE_ENV=production     # Runtime environment
```

### Docker Requirements
```bash
# Required Docker capabilities
--device /dev/fuse              # FUSE device access
--cap-add SYS_ADMIN            # FUSE mount permissions
--security-opt apparmor:unconfined  # Container FUSE access
```

### Volume Mounts
```bash
# Workspace isolation
-v ./workspaces:/workspace     # Host directory for tenant workspaces
```

## Service Management

### Using Deploy Script
```bash
./scripts/deploy-npx.sh         # Deploy
./scripts/deploy-npx.sh stop    # Stop
./scripts/deploy-npx.sh logs    # View logs
./scripts/deploy-npx.sh status  # Check status
./scripts/deploy-npx.sh restart # Restart
```

### Manual Docker Commands
```bash
# View logs
docker-compose -f docker-compose.npx.yml logs -f

# Stop service
docker-compose -f docker-compose.npx.yml down

# Restart service
docker-compose -f docker-compose.npx.yml restart

# Shell access
docker exec -it codeflow-npx sh
```

## Production Setup

### With Traefik Proxy
```bash
# Start with reverse proxy
docker-compose -f docker-compose.npx.yml --profile proxy up -d

# Access via:
# - Service: http://codeflow.localhost
# - Dashboard: http://localhost:8080
```

### Health Monitoring
```bash
# Health check endpoint
curl http://localhost:3000/health

# Container health status
docker-compose -f docker-compose.npx.yml ps
```

## Client Connection

### WebSocket Client Example
```javascript
import WebSocket from 'ws';

const ws = new WebSocket('ws://localhost:3000');

ws.on('open', () => {
  // Initialize tenant session
  ws.send(JSON.stringify({
    type: 'auth',
    tenantId: 'my-tenant',
    apiKey: 'my-secret-key'
  }));
});

ws.on('message', (data) => {
  const msg = JSON.parse(data.toString());
  if (msg.type === 'workspace-ready') {
    console.log(`FUSE mount ready: ${msg.mountPath}`);
    // Now you can access files via FUSE
  }
});
```

### FUSE Operations
```javascript
// Read file through FUSE
const content = await client.readFile('/project/src/index.ts');

// Write file through FUSE
await client.writeFile('/project/README.md', Buffer.from('# My Project'));
```

## Troubleshooting

### FUSE Issues
```bash
# Check FUSE availability
ls -la /dev/fuse

# Enable user_allow_other (if needed)
echo "user_allow_other" | sudo tee -a /etc/fuse.conf

# Check container FUSE mount
docker exec -it codeflow-npx mount | grep fuse
```

### Container Issues
```bash
# Check container status
docker ps | grep codeflow

# View detailed logs
docker logs codeflow-npx

# Check resource usage
docker stats codeflow-npx
```

### Network Issues
```bash
# Test service connectivity
curl -v http://localhost:3000/health

# Check port binding
netstat -tlnp | grep 3000

# Test WebSocket connection
wscat -c ws://localhost:3000
```

## Security Notes

### Container Security
- Non-root user execution (`codeflow:codeflow`)
- Minimal Alpine Linux base image
- Only required capabilities (`SYS_ADMIN` for FUSE)
- Isolated workspace volumes

### Network Security
- No external network dependencies at runtime
- Health check endpoints only
- WebSocket authentication supported
- TLS/WSS support available

### FUSE Security
- User-space filesystem isolation
- Per-tenant workspace separation
- No host filesystem access outside mounted volumes
- Automatic cleanup on disconnect

## Performance

### Resource Usage
- **Memory**: ~50MB base + workspace data
- **CPU**: Minimal when idle, scales with LSP operations
- **Storage**: Workspace volumes only
- **Network**: WebSocket connections only

### Scaling
- **Horizontal**: Multiple container instances
- **Vertical**: Increase MAX_CLIENTS per container
- **Load Balancing**: Traefik proxy included
- **Monitoring**: Health checks and metrics available