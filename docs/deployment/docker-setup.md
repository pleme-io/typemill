# Docker Deployment Guide

This guide covers deploying CodeFlow Buddy as a containerized WebSocket service.

## Quick Start

### 1. Start the entire development environment:

```bash
docker-compose up -d
```

This will start:
- **codeflow-service**: WebSocket server on port 3000
- **frontend-dev**: Node.js development container
- **backend-dev**: Python development container
- **postgres-dev**: PostgreSQL database on port 5432

### 2. Check service health:

```bash
# Check all services
docker-compose ps

# Check service logs
docker-compose logs codeflow-service

# Test health endpoint
curl http://localhost:3000/healthz
```

### 3. Connect to development containers:

```bash
# Connect to frontend container
docker-compose exec frontend-dev bash

# Connect to backend container
docker-compose exec backend-dev bash

# Connect to database
docker-compose exec postgres-dev psql -U devuser -d devdb
```

## Service Configuration

### Environment Variables

**CodeFlow Service:**
- `PORT`: WebSocket server port (default: 3000)
- `MAX_CLIENTS`: Maximum concurrent clients (default: 50)
- `NODE_ENV`: Environment mode (development/production)
- `LOG_LEVEL`: Logging level (debug/info/warn/error)

**Authentication (Phase 3):**
- `JWT_SECRET`: JWT signing secret for authentication
- `JWT_EXPIRY`: Token expiry time (default: 24h)
- `JWT_ISSUER`: Token issuer (default: codeflow-buddy)
- `JWT_AUDIENCE`: Token audience (default: codeflow-clients)

**TLS/Security:**
- `TLS_KEY_PATH`: Path to TLS private key file
- `TLS_CERT_PATH`: Path to TLS certificate file
- `TLS_CA_PATH`: Path to CA certificate for client validation

**Development Containers:**
- `MCP_SERVER`: WebSocket server URL (ws://codeflow-service:3000)
- `PROJECT_ID`: Unique project identifier
- `PROJECT_ROOT`: Container working directory

### Volume Mounts

- `./logs:/app/logs` - Service logs persistence
- `./examples/frontend:/workspace` - Frontend source code
- `./examples/backend:/workspace` - Backend source code
- `frontend-node-modules` - Node.js dependencies cache
- `postgres-data` - Database persistence

## Client Connection Example

### Basic Connection (No Auth)
```javascript
// Connect to CodeFlow service
const ws = new WebSocket('ws://codeflow-service:3000');

// Initialize session
ws.send(JSON.stringify({
  method: 'initialize',
  project: process.env.PROJECT_ID,
  projectRoot: process.env.PROJECT_ROOT
}));
```

### With JWT Authentication (Phase 3)
```javascript
// 1. Get authentication token
const authResponse = await fetch('http://codeflow-service:3000/auth', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    projectId: process.env.PROJECT_ID,
    secretKey: process.env.PROJECT_SECRET
  })
});
const { token } = await authResponse.json();

// 2. Connect with token
const ws = new WebSocket('ws://codeflow-service:3000');
ws.send(JSON.stringify({
  method: 'initialize',
  project: process.env.PROJECT_ID,
  projectRoot: process.env.PROJECT_ROOT,
  token: token
}));
```

### Using MCP Tools
```javascript
// Use MCP tools (same for both auth modes)
ws.send(JSON.stringify({
  method: 'find_definition',
  params: {
    file_path: '/workspace/src/index.ts',
    symbol_name: 'UserComponent'
  }
}));
```

## Production Deployment

### 1. Build service image:

```bash
docker build -f Dockerfile.service -t codeflow-buddy:latest .
```

### 2. Run in production mode:

**Basic Production:**
```bash
docker run -d \
  --name codeflow-service \
  -p 3000:3000 \
  -e NODE_ENV=production \
  -e MAX_CLIENTS=100 \
  codeflow-buddy:latest
```

**With JWT Authentication:**
```bash
docker run -d \
  --name codeflow-service \
  -p 3000:3000 \
  -e NODE_ENV=production \
  -e MAX_CLIENTS=100 \
  -e JWT_SECRET=your-super-secret-key \
  codeflow-buddy:latest serve --require-auth
```

**Enterprise with TLS/WSS:**
```bash
docker run -d \
  --name codeflow-service \
  -p 3000:3000 \
  -v /etc/ssl/certs:/app/certs:ro \
  -e NODE_ENV=production \
  -e MAX_CLIENTS=100 \
  -e JWT_SECRET=enterprise-secret \
  -e TLS_KEY_PATH=/app/certs/server.key \
  -e TLS_CERT_PATH=/app/certs/server.crt \
  codeflow-buddy:latest serve --require-auth --tls-key /app/certs/server.key --tls-cert /app/certs/server.crt
```

### 3. Using Docker Swarm:

```yaml
# docker-stack.yml
version: '3.8'
services:
  codeflow-service:
    image: codeflow-buddy:latest
    ports:
      - "3000:3000"
    environment:
      - NODE_ENV=production
      - MAX_CLIENTS=100
    deploy:
      replicas: 3
      restart_policy:
        condition: on-failure
      resources:
        limits:
          memory: 512M
        reservations:
          memory: 256M
```

```bash
docker stack deploy -c docker-stack.yml codeflow
```

## Monitoring & Health Checks

### Health Check Endpoint

```bash
# Basic health check
curl http://localhost:3000/healthz

# Expected response
{
  "status": "healthy",
  "timestamp": "2025-01-01T00:00:00Z",
  "version": "2.0.0",
  "uptime": 3600,
  "connections": {
    "active": 5,
    "disconnected": 2,
    "total": 7
  },
  "sessions": {
    "active": 5,
    "disconnected": 2,
    "projects": 3
  },
  "lspServers": {
    "active": 8,
    "crashed": 0,
    "projects": 3,
    "languages": 4
  },
  "cache": {
    "size": 42,
    "hitRate": 0.85,
    "totalHits": 1250,
    "totalMisses": 220
  },
  "deltaProcessor": {
    "diffTimeout": 1,
    "editCost": 4,
    "minDeltaSize": 1024,
    "maxPatchSizeRatio": 0.8
  },
  "authentication": {
    "enabled": true,
    "issuer": "codeflow-buddy",
    "audience": "codeflow-clients"
  },
  "security": {
    "tls": true,
    "protocol": "wss",
    "clientCertValidation": false
  }
}
```

### Authentication Endpoint (Phase 3)

```bash
# Get JWT token
curl -X POST http://localhost:3000/auth \
  -H "Content-Type: application/json" \
  -d '{"projectId": "my-project", "secretKey": "my-secret"}'

# Expected response
{
  "token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "expiresAt": "2025-01-02T00:00:00Z",
  "permissions": ["file:read", "file:write", "lsp:query", "lsp:symbol", "session:manage"]
}
```

### Prometheus Metrics

```bash
# Get metrics in Prometheus format
curl http://localhost:3000/metrics

# Sample metrics
codeflow_connections_active 5
codeflow_connections_disconnected 2
codeflow_projects_active 3
codeflow_lsp_servers_active 8
codeflow_uptime_seconds 3600
```

### Container Logs

```bash
# Follow service logs
docker-compose logs -f codeflow-service

# View connection events
docker-compose logs codeflow-service | grep "Client connected"

# View LSP server events
docker-compose logs codeflow-service | grep "LSP server"
```

### Resource Monitoring

```bash
# Check container resource usage
docker stats

# Check service-specific metrics
docker-compose exec codeflow-service cat /proc/meminfo
docker-compose exec codeflow-service ps aux
```

## Development Workflow

### 1. Local Development with Hot Reload

```bash
# Start only the service dependencies
docker-compose up -d postgres-dev

# Run service locally with hot reload
bun run dev
```

### 2. Testing Multi-Project Setup

```bash
# Start all containers
docker-compose up -d

# Connect to frontend container and test
docker-compose exec frontend-dev bash
cd /workspace && npm run dev

# In another terminal, connect to backend container
docker-compose exec backend-dev bash
cd /workspace && python main.py
```

### 3. Database Development

```bash
# Connect to database
docker-compose exec postgres-dev psql -U devuser -d devdb

# Run migrations
\i /docker-entrypoint-initdb.d/init.sql

# View tables
\dt
```

## Troubleshooting

### Service Won't Start

```bash
# Check build logs
docker-compose build --no-cache codeflow-service

# Check configuration
docker-compose config

# Check port conflicts
netstat -tulpn | grep :3000
```

### Connection Issues

```bash
# Test from container to service
docker-compose exec frontend-dev curl codeflow-service:3000/healthz

# Check network connectivity
docker network ls
docker network inspect codeflow-buddy_codeflow-network
```

### LSP Server Issues

```bash
# Check LSP server logs
docker-compose logs codeflow-service | grep "LSP"

# Restart service
docker-compose restart codeflow-service
```

## Security Considerations

### Production Hardening

1. **Use non-root user** (already implemented in Dockerfile.service)
2. **Limit container resources**:
   ```yaml
   deploy:
     resources:
       limits:
         memory: 512M
         cpus: 0.5
   ```
3. **Network security**:
   ```yaml
   networks:
     codeflow-network:
       driver: overlay
       encrypted: true
   ```
4. **Environment secrets**:
   ```bash
   # Use Docker secrets instead of environment variables
   docker secret create db_password password.txt
   ```

### Access Control

- Use reverse proxy (nginx/traefik) for SSL termination
- Implement authentication in WebSocket handshake
- Use network policies to restrict container communication
- Regular security updates for base images

## Scaling

### Horizontal Scaling

```yaml
# docker-compose.scale.yml
version: '3.8'
services:
  codeflow-service:
    # ... base config
    deploy:
      replicas: 3

  nginx-lb:
    image: nginx:alpine
    ports:
      - "80:80"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
    depends_on:
      - codeflow-service
```

### Load Balancing

Configure nginx for WebSocket load balancing:

```nginx
upstream codeflow {
    server codeflow-service_1:3000;
    server codeflow-service_2:3000;
    server codeflow-service_3:3000;
}

server {
    listen 80;
    location / {
        proxy_pass http://codeflow;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "upgrade";
    }
}
```