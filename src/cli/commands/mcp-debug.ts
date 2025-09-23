import { existsSync, readFileSync } from 'node:fs';
import { join } from 'node:path';

/**
 * MCP Debug command - directly call MCP tools for testing and debugging
 * Provides low-level access to any tool with raw request/response logging
 */
export async function mcpDebugCommand(toolName: string, toolArgs: string): Promise<void> {
  // Validate arguments
  if (!toolName) {
    console.error('Error: tool_name is required');
    console.error('Usage: codeflow-buddy mcp-debug <tool_name> <tool_args_json>');
    console.error(
      'Example: codeflow-buddy mcp-debug find_definition \'{"file_path": "src/index.ts", "symbol_name": "main"}\''
    );
    process.exit(1);
  }

  if (!toolArgs) {
    console.error('Error: tool_args is required');
    console.error('Usage: codeflow-buddy mcp-debug <tool_name> <tool_args_json>');
    console.error(
      'Example: codeflow-buddy mcp-debug find_definition \'{"file_path": "src/index.ts", "symbol_name": "main"}\''
    );
    process.exit(1);
  }

  // Parse tool arguments
  let parsedArgs: Record<string, unknown>;
  try {
    parsedArgs = JSON.parse(toolArgs);
  } catch (error) {
    console.error('Error: Invalid JSON in tool_args');
    console.error('Arguments must be valid JSON string');
    console.error('Example: \'{"file_path": "src/index.ts", "symbol_name": "main"}\'');
    process.exit(1);
  }

  // Check if MCP server is running
  const PID_FILE = join('.codebuddy', 'server.pid');
  if (!existsSync(PID_FILE)) {
    console.error('Error: MCP server is not running');
    console.error('Start it with: codeflow-buddy start');
    process.exit(1);
  }

  const pidContent = readFileSync(PID_FILE, 'utf-8').trim();
  const serverPid = Number.parseInt(pidContent, 10);

  if (Number.isNaN(serverPid)) {
    console.error('Error: Invalid PID file content');
    console.error('Try restarting with: codeflow-buddy stop && codeflow-buddy start');
    process.exit(1);
  }

  // Check if the process is actually running
  const { isProcessRunning } = await import('../../utils/platform/process.js');
  if (!isProcessRunning(serverPid)) {
    console.error(`Error: MCP server process (PID: ${serverPid}) is not running`);
    console.error('Start it with: codeflow-buddy start');
    process.exit(1);
  }

  console.log(`🔧 MCP Debug Tool`);
  console.log(`📡 Connecting to server (PID: ${serverPid})`);
  console.log('');

  try {
    // Create MCP client and connect via stdio to the running server
    // Note: In a real implementation, we'd need to establish a connection
    // to the already running MCP server. For now, we'll simulate the request.

    // Construct the MCP request
    const mcpRequest = {
      method: 'tools/call',
      params: {
        name: toolName,
        arguments: parsedArgs,
        // Add trace flag for enhanced debugging
        trace: true,
      },
    };

    console.log('📤 REQUEST:');
    console.log(JSON.stringify(mcpRequest, null, 2));
    console.log('');

    // TODO: In a real implementation, send the request to the running server
    // For now, we'll show what the request would look like and provide guidance
    console.log('⚠️  Note: Direct server communication not yet implemented');
    console.log('');
    console.log('🔨 To test this tool manually:');
    console.log('1. Use this request with your MCP client:');
    console.log(`   Tool: ${toolName}`);
    console.log(`   Args: ${JSON.stringify(parsedArgs, null, 2)}`);
    console.log('');
    console.log('2. Or test via the batch_execute tool:');
    const batchRequest = {
      operations: [
        {
          tool: toolName,
          args: parsedArgs,
        },
      ],
      options: {
        dry_run: false,
        atomic: false,
      },
    };
    console.log(JSON.stringify(batchRequest, null, 2));
  } catch (error) {
    console.error('❌ Connection failed:', error);
    console.error('');
    console.error('Troubleshooting:');
    console.error('1. Ensure MCP server is running: codeflow-buddy status');
    console.error('2. Restart the server: codeflow-buddy stop && codeflow-buddy start');
    console.error('3. Check the tool name is valid: use an MCP client to list tools');
    process.exit(1);
  }
}
