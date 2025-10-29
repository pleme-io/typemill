# System Tools

Health monitoring and server diagnostics for production observability.

**Tool count:** 1 tool
**Related categories:** None (standalone monitoring)

## Table of Contents

- [Tools](#tools)
  - [health_check](#health_check)
- [Common Patterns](#common-patterns)
  - [Production Health Monitoring](#production-health-monitoring)
  - [Debugging Server State](#debugging-server-state)
  - [Uptime Tracking](#uptime-tracking)
  - [Plugin Verification](#plugin-verification)

---

## Tools

### health_check

**Purpose:** Get comprehensive server health status including uptime, plugin counts, workflow states, and system metrics.

**Parameters:**

| Name | Type | Required | Description |
|------|------|----------|-------------|
| (none) | - | - | No parameters required |

**Returns:**

JSON object with server health information:

- `status` (string): Health status - always "healthy" if server is responding
- `uptime` (object): Server uptime information
  - `seconds` (number): Total uptime in seconds
  - `minutes` (number): Total uptime in minutes
  - `hours` (number): Total uptime in hours
  - `formatted` (string): Human-readable uptime (e.g., "2h 15m 30s")
- `plugins` (object): Plugin system status
  - `loaded` (number): Count of loaded language plugins
- `workflows` (object): Workflow executor state
  - `paused` (number): Count of paused workflows awaiting continuation
- `system_status` (object): System operational status
  - `status` (string): System status - "ok" when operational
  - `uptime_seconds` (number): System uptime in seconds
  - `message` (string): Status message

**Example:**

```json
// MCP request
{
  "jsonrpc": "2.0",
  "id": "health-1",
  "method": "tools/call",
  "params": {
    "name": "health_check",
    "arguments": {}
  }
}

// Response
{
  "jsonrpc": "2.0",
  "id": "health-1",
  "result": {
    "status": "healthy",
    "uptime": {
      "seconds": 8430,
      "minutes": 140,
      "hours": 2,
      "formatted": "2h 20m 30s"
    },
    "plugins": {
      "loaded": 2
    },
    "workflows": {
      "paused": 0
    },
    "system_status": {
      "status": "ok",
      "uptime_seconds": 8430,
      "message": "System is operational"
    }
  }
}
```text
**Notes:**

- **No LSP dependency**: Works independently of language servers
- **Always responds**: If you get a response, the server is alive - `status` will always be "healthy"
- **Production monitoring**: Use this for health checks in production deployments
- **Workflow state**: `paused` workflows indicate long-running operations awaiting user input
- **Plugin count**: Reflects registered language plugins (TypeScript, Rust, etc.)
- **Multiple uptime formats**: Use `formatted` for display, numeric values for alerting thresholds
- **No authentication required**: Health endpoint accessible without JWT (for load balancer checks)

---

## Common Patterns

### Production Health Monitoring

Use `health_check` in production monitoring systems:

```bash
# Kubernetes liveness probe
curl -X POST http://localhost:3000/rpc \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "id": "k8s-liveness",
    "method": "tools/call",
    "params": {
      "name": "health_check",
      "arguments": {}
    }
  }'

# Check if response contains "healthy" status
```text
### Debugging Server State

Check server state during troubleshooting:

```javascript
// WebSocket client monitoring
async function monitorServerHealth() {
  const response = await client.callTool("health_check", {});

  console.log(`Server uptime: ${response.uptime.formatted}`);
  console.log(`Plugins loaded: ${response.plugins.loaded}`);
  console.log(`Paused workflows: ${response.workflows.paused}`);

  // Alert if workflows are stuck
  if (response.workflows.paused > 5) {
    console.warn("Too many paused workflows - investigate");
  }
}
```text
### Uptime Tracking

Track server availability over time:

```javascript
// Periodic health check with alerting
setInterval(async () => {
  const response = await client.callTool("health_check", {});

  // Alert if uptime is too short (recent restart)
  if (response.uptime.hours < 1) {
    console.warn(`Server restarted recently: ${response.uptime.formatted}`);
  }

  // Log metrics to monitoring system
  metrics.record({
    uptime_seconds: response.uptime.seconds,
    plugins_loaded: response.plugins.loaded,
    paused_workflows: response.workflows.paused
  });
}, 60000); // Check every minute
```text
### Plugin Verification

Verify expected plugins are loaded:

```javascript
// Verify language support is available
const health = await client.callTool("health_check", {});

const expectedPlugins = 2; // TypeScript + Rust
if (health.plugins.loaded < expectedPlugins) {
  console.error(`Expected ${expectedPlugins} plugins, only ${health.plugins.loaded} loaded`);
  // Investigation needed - check LSP server configuration
}
```text
---

**Last Updated:** 2025-10-22
**API Version:** 1.0.0-rc4