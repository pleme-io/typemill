# FUSE Quick Start Guide

Get your multi-tenant FUSE service running in production in minutes.

## Prerequisites

### 1. Install FUSE

#### Linux (Ubuntu/Debian)
```bash
sudo apt-get update
sudo apt-get install -y fuse libfuse-dev
```

#### Linux (RHEL/CentOS/Fedora)
```bash
sudo yum install -y fuse fuse-devel
# or
sudo dnf install -y fuse fuse-devel
```

#### macOS
Install macFUSE from: https://osxfuse.github.io/

#### Docker
No installation needed - included in our Docker image.

### 2. Verify FUSE Installation
```bash
# Check FUSE is available
fusermount -V

# Check kernel module is loaded
lsmod | grep fuse
```

## Quick Start

### Option 1: Docker (Recommended)

```bash
# Clone the repository
git clone https://github.com/goobits/codeflow-buddy.git
cd codeflow-buddy

# Start multi-tenant FUSE service
docker-compose up -d

# Service is now running on ws://localhost:3000
```

### Option 2: Direct Installation

```bash
# Install codeflow-buddy
npm install -g @goobits/codeflow-buddy

# Start the service with FUSE enabled
codeflow-buddy serve --enable-fuse --port 3000

# Or use the convenience script
./scripts/start-fuse-dev.sh
```

### Option 3: Production Docker

```bash
# Use production configuration
docker-compose -f docker-compose.production.yml up -d

# Scale for multiple clients
docker-compose -f docker-compose.production.yml up -d --scale fuse-service=3
```

## Basic Multi-Tenant Setup

### 1. Server Configuration

Create `.codebuddy/config.json`:
```json
{
  "servers": [
    {
      "extensions": ["ts", "tsx", "js", "jsx"],
      "command": ["typescript-language-server", "--stdio"]
    }
  ],
  "fuse": {
    "enabled": true,
    "workspaceBase": "/data/workspaces",
    "mountBase": "/data/mounts",
    "maxSessions": 100
  }
}
```

### 2. Connect Your First Client

```javascript
// client.js
const WebSocket = require('ws');

const ws = new WebSocket('ws://localhost:3000');

ws.on('open', () => {
  // Initialize session
  ws.send(JSON.stringify({
    method: 'initialize',
    projectId: 'my-project',
    params: {
      capabilities: {}
    }
  }));
});

ws.on('message', (data) => {
  const msg = JSON.parse(data);
  if (msg.type === 'workspace-ready') {
    console.log('FUSE mount ready at:', msg.mountPath);
    // Your isolated filesystem is now available
  }
});
```

### 3. Test File Operations

```javascript
// After workspace is ready
ws.send(JSON.stringify({
  type: 'fuse-write',
  path: '/test.txt',
  data: Buffer.from('Hello World').toString('base64')
}));

ws.send(JSON.stringify({
  type: 'fuse-read',
  path: '/test.txt'
}));
```

## Common Issues & Solutions

### Issue 1: "Permission denied" when mounting

**Solution:**
```bash
# Add user to fuse group
sudo usermod -a -G fuse $USER

# Logout and login again for group changes to take effect
```

### Issue 2: "fusermount: command not found"

**Solution:**
```bash
# Install FUSE utilities
sudo apt-get install fuse-utils  # Debian/Ubuntu
sudo yum install fuse-utils      # RHEL/CentOS
```

### Issue 3: Docker "Operation not permitted"

**Solution:**
```bash
# Run with privileged mode and device access
docker run --privileged --device /dev/fuse \
  -p 3000:3000 codeflow-buddy:latest
```

### Issue 4: "Transport endpoint is not connected"

**Solution:**
```bash
# Force unmount stale mount point
fusermount -uz /path/to/mount

# Or if that fails
sudo umount -l /path/to/mount
```

### Issue 5: ARM64 Architecture Issues

**Solution:**
```bash
# Rebuild with native dependencies
npm rebuild @cocalc/fuse-native

# Or use pre-built Docker image
docker pull goobits/codeflow-buddy:arm64
```

## Environment Variables

Configure the service via environment variables:

```bash
# Server configuration
WS_PORT=3000                     # WebSocket port
ENABLE_FUSE=true                 # Enable FUSE mounting
MAX_CLIENTS=100                  # Maximum concurrent clients

# Workspace configuration
WORKSPACE_BASE=/data/workspaces  # Base directory for workspaces
FUSE_MOUNT_BASE=/data/mounts     # Base directory for FUSE mounts
WORKSPACE_TIMEOUT=3600000         # Workspace timeout (1 hour)

# Security (optional)
AUTH_ENABLED=true                # Enable authentication
JWT_SECRET=your-secret-key       # JWT signing key
```

## Production Checklist

- [ ] FUSE kernel module loaded (`lsmod | grep fuse`)
- [ ] User has FUSE permissions (`groups | grep fuse`)
- [ ] Persistent storage mounted for workspaces
- [ ] Docker has privileged mode or appropriate capabilities
- [ ] Health check endpoint configured
- [ ] Monitoring and alerting set up
- [ ] Backup strategy for workspace data
- [ ] Rate limiting configured (if needed)
- [ ] SSL/TLS for WebSocket connections

## Health Check

```bash
# Check service health
curl http://localhost:8080/health

# Check FUSE mounts
mount | grep fuse

# Check active sessions
curl http://localhost:8080/stats
```

## Next Steps

1. **Security**: See [Security Guide](../production/security-guide.md) for authentication setup
2. **Scaling**: See [Scaling Guide](../production/scaling.md) for multi-instance deployment
3. **Monitoring**: See [Monitoring Guide](../production/monitoring.md) for observability

## Support

- GitHub Issues: https://github.com/goobits/codeflow-buddy/issues
- Documentation: https://github.com/goobits/codeflow-buddy/docs
- Docker Hub: https://hub.docker.com/r/goobits/codeflow-buddy

---

**Ready to go!** Your multi-tenant FUSE service should now be running. Each connected client gets an isolated filesystem mount for secure, session-based file operations.