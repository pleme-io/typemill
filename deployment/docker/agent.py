#!/usr/bin/env python3
"""
Lightweight HTTP agent for executing commands in workspace containers.

Listens on port 8000 and provides a single endpoint:
- POST /execute - Execute a shell command and return results
"""

import asyncio
import json
import time
from aiohttp import web

COMMAND_TIMEOUT_SECONDS = 30


async def execute_command(request):
    """Execute a shell command and return structured results."""
    try:
        data = await request.json()
    except json.JSONDecodeError:
        return web.json_response(
            {"error": "Invalid JSON"}, status=400
        )

    command = data.get("command")
    if not command:
        return web.json_response(
            {"error": "Missing 'command' field"}, status=400
        )

    print(f"Executing command: {command}", flush=True)
    start_time = time.time()

    try:
        # Execute command with timeout
        process = await asyncio.create_subprocess_shell(
            command,
            stdout=asyncio.subprocess.PIPE,
            stderr=asyncio.subprocess.PIPE,
        )

        try:
            stdout, stderr = await asyncio.wait_for(
                process.communicate(),
                timeout=COMMAND_TIMEOUT_SECONDS
            )
        except asyncio.TimeoutError:
            process.kill()
            await process.wait()
            return web.json_response({
                "error": f"Command timeout after {COMMAND_TIMEOUT_SECONDS}s",
                "exit_code": -1,
                "stdout": "",
                "stderr": "",
                "execution_time_ms": int((time.time() - start_time) * 1000)
            }, status=408)

        execution_time_ms = int((time.time() - start_time) * 1000)

        stdout_str = stdout.decode('utf-8', errors='replace')
        stderr_str = stderr.decode('utf-8', errors='replace')

        # Print output to container logs
        if stdout_str:
            print(stdout_str, flush=True)
        if stderr_str:
            print(f"STDERR: {stderr_str}", flush=True)

        result = {
            "exit_code": process.returncode,
            "stdout": stdout_str,
            "stderr": stderr_str,
            "execution_time_ms": execution_time_ms
        }

        print(f"Command completed with exit code {process.returncode} in {execution_time_ms}ms", flush=True)
        return web.json_response(result)

    except Exception as e:
        print(f"Error executing command: {e}", flush=True)
        return web.json_response(
            {
                "error": str(e),
                "exit_code": -1,
                "stdout": "",
                "stderr": "",
                "execution_time_ms": int((time.time() - start_time) * 1000)
            },
            status=500
        )


async def health_check(request):
    """Health check endpoint."""
    return web.json_response({"status": "healthy"})


def main():
    app = web.Application()
    app.router.add_post('/execute', execute_command)
    app.router.add_get('/health', health_check)

    print("Starting agent on 0.0.0.0:8000", flush=True)
    web.run_app(app, host='0.0.0.0', port=8000)


if __name__ == '__main__':
    main()
