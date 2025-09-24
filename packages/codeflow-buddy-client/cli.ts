#!/usr/bin/env node

import chalk from 'chalk';
import { Command } from 'commander';
import inquirer from 'inquirer';
import {
  deleteProfile,
  getConfig,
  listProfiles,
  loadConfig,
  saveConfig,
  saveProfile,
  setCurrentProfile,
} from './config.js';
import { createProxyServer } from './http-proxy.js';
import { MCPProxy } from './mcp-proxy.js';
import { WebSocketClient } from './websocket.js';

const program = new Command();

program
  .name('codeflow-client')
  .description('CLI client for Codeflow Buddy MCP server')
  .version('1.0.0');

// Global options
program
  .option('-u, --url <url>', 'WebSocket server URL')
  .option('-t, --token <token>', 'JWT authentication token')
  .option('-p, --profile <profile>', 'Use a named profile')
  .option('--timeout <ms>', 'Request timeout in milliseconds', '30000')
  .option('--no-color', 'Disable colored output');

// Configure command
program
  .command('configure')
  .description('Configure connection settings')
  .action(async () => {
    const answers = await inquirer.prompt([
      {
        type: 'input',
        name: 'url',
        message: 'Server URL:',
        default: 'ws://localhost:3000',
        validate: (input) => {
          try {
            new URL(input);
            return true;
          } catch {
            return 'Please enter a valid URL';
          }
        },
      },
      {
        type: 'password',
        name: 'token',
        message: 'Authentication token (optional):',
        mask: '*',
      },
      {
        type: 'confirm',
        name: 'saveAsProfile',
        message: 'Save as a profile?',
        default: false,
      },
    ]);

    if (answers.saveAsProfile) {
      const profileAnswers = await inquirer.prompt([
        {
          type: 'input',
          name: 'profileName',
          message: 'Profile name:',
          validate: (input) => input.length > 0 || 'Profile name is required',
        },
        {
          type: 'input',
          name: 'description',
          message: 'Profile description (optional):',
        },
        {
          type: 'confirm',
          name: 'setAsDefault',
          message: 'Set as default profile?',
          default: true,
        },
      ]);

      await saveProfile(profileAnswers.profileName, {
        url: answers.url,
        token: answers.token || undefined,
        description: profileAnswers.description || undefined,
      });

      if (profileAnswers.setAsDefault) {
        await setCurrentProfile(profileAnswers.profileName);
      }

      console.log(chalk.green(`✓ Profile '${profileAnswers.profileName}' saved`));
    } else {
      await saveConfig({
        url: answers.url,
        token: answers.token || undefined,
      });
      console.log(chalk.green('✓ Configuration saved'));
    }
  });

// Profile management commands
const profileCmd = program.command('profile').description('Manage connection profiles');

profileCmd
  .command('list')
  .description('List all profiles')
  .action(async () => {
    const profiles = await listProfiles();
    const config = await loadConfig();

    if (Object.keys(profiles).length === 0) {
      console.log(chalk.yellow('No profiles configured'));
      return;
    }

    console.log(chalk.bold('\nConfigured Profiles:'));
    for (const [name, profile] of Object.entries(profiles)) {
      const isCurrent = name === config.currentProfile;
      const marker = isCurrent ? chalk.green('* ') : '  ';
      console.log(`${marker}${chalk.bold(name)}`);
      console.log(`    URL: ${profile.url}`);
      if (profile.description) {
        console.log(`    ${chalk.gray(profile.description)}`);
      }
    }
  });

profileCmd
  .command('use <name>')
  .description('Set active profile')
  .action(async (name: string) => {
    await setCurrentProfile(name);
    console.log(chalk.green(`✓ Now using profile '${name}'`));
  });

profileCmd
  .command('delete <name>')
  .description('Delete a profile')
  .action(async (name: string) => {
    const confirm = await inquirer.prompt([
      {
        type: 'confirm',
        name: 'confirm',
        message: `Delete profile '${name}'?`,
        default: false,
      },
    ]);

    if (confirm.confirm) {
      await deleteProfile(name);
      console.log(chalk.green(`✓ Profile '${name}' deleted`));
    }
  });

// Send command
program
  .command('send <tool> [params]')
  .description('Send a tool request to the server')
  .option('-i, --interactive', 'Interactive mode for parameters')
  .option('-f, --format <format>', 'Output format (json, pretty)', 'pretty')
  .action(async (tool: string, paramsJson?: string, options?: any) => {
    const globalOpts = program.opts();
    const config = await getConfig({
      url: globalOpts.url,
      token: globalOpts.token,
      currentProfile: globalOpts.profile,
    });

    if (!config.url) {
      console.error(chalk.red('Error: No server URL configured'));
      console.error('Run "codeflow-client configure" to set up connection');
      process.exit(1);
    }

    let params: any = {};

    if (options.interactive || !paramsJson) {
      // Interactive mode - fetch tool schema and prompt for parameters
      console.log(chalk.gray('Fetching tool schema...'));

      try {
        const client = new WebSocketClient(config.url, {
          token: config.token,
          requestTimeout: parseInt(globalOpts.timeout, 10),
        });

        await client.connect();
        const tools = await client.send<any>('tools/list');
        await client.disconnect();

        const toolInfo = tools?.tools?.find((t: any) => t.name === tool);
        if (!toolInfo) {
          console.error(chalk.red(`Tool '${tool}' not found`));
          process.exit(1);
        }

        // Build prompts from schema
        const prompts: any[] = [];
        if (toolInfo.inputSchema?.properties) {
          for (const [key, schema] of Object.entries(
            toolInfo.inputSchema.properties as Record<string, any>
          )) {
            const required = toolInfo.inputSchema.required?.includes(key);
            prompts.push({
              type: schema.type === 'boolean' ? 'confirm' : 'input',
              name: key,
              message: `${key}${required ? ' (required)' : ''}:`,
              when: () =>
                required ||
                inquirer
                  .prompt([
                    {
                      type: 'confirm',
                      name: 'include',
                      message: `Include ${key}?`,
                      default: false,
                    },
                  ])
                  .then((a: any) => a.include),
            });
          }
        }

        if (prompts.length > 0) {
          params = await inquirer.prompt(prompts as any);
        }
      } catch (error) {
        console.error(chalk.red('Failed to fetch tool schema:'), error);
        // Fall back to raw parameter input
        const answer = await inquirer.prompt([
          {
            type: 'editor',
            name: 'params',
            message: 'Enter parameters (JSON):',
            default: '{}',
          },
        ]);
        params = JSON.parse(answer.params);
      }
    } else {
      // Parse provided JSON
      try {
        params = JSON.parse(paramsJson);
      } catch (error) {
        console.error(chalk.red('Invalid JSON parameters:'), error);
        process.exit(1);
      }
    }

    // Send the request
    const spinner = chalk.gray('Sending request...');
    console.log(spinner);

    try {
      const proxy = new MCPProxy(config.url, {
        token: config.token,
        requestTimeout: parseInt(globalOpts.timeout, 10),
      });

      const result = await proxy.send({ method: tool, params });
      await proxy.disconnect();

      // Format output
      if (options.format === 'json') {
        console.log(JSON.stringify(result, null, 2));
      } else {
        // Pretty print
        console.log(chalk.green('\n✓ Success'));
        console.log(chalk.bold('Result:'));
        console.log(formatResult(result));
      }
    } catch (error: any) {
      console.error(chalk.red('\n✗ Error:'), error.message);
      if (error.data) {
        console.error(chalk.gray('Details:'), error.data);
      }
      process.exit(1);
    }
  });

// Proxy command
program
  .command('proxy')
  .description('Start HTTP proxy server')
  .option('-P, --port <port>', 'Proxy server port', '3001')
  .action(async (options) => {
    const globalOpts = program.opts();
    const config = await getConfig({
      url: globalOpts.url,
      token: globalOpts.token,
      currentProfile: globalOpts.profile,
    });

    if (!config.url) {
      console.error(chalk.red('Error: No server URL configured'));
      console.error('Run "codeflow-client configure" to set up connection');
      process.exit(1);
    }

    const proxy = new MCPProxy(config.url, {
      token: config.token,
      requestTimeout: parseInt(globalOpts.timeout, 10),
    });

    const server = createProxyServer(proxy, parseInt(options.port, 10));

    server.listen(parseInt(options.port, 10), () => {
      console.log(chalk.green(`\n✓ HTTP proxy server started`));
      console.log(`  Listening on: http://localhost:${options.port}`);
      console.log(`  WebSocket backend: ${config.url}`);
      console.log(chalk.gray('\n  Example usage:'));
      console.log(chalk.gray(`  curl -X POST http://localhost:${options.port}/rpc \\`));
      console.log(chalk.gray('    -H "Content-Type: application/json" \\'));
      console.log(chalk.gray('    -d \'{"method": "find_definition", "params": {...}}\''));
      console.log(chalk.gray('\n  Press Ctrl+C to stop'));
    });

    // Handle shutdown
    process.on('SIGINT', async () => {
      console.log(chalk.yellow('\nShutting down...'));
      server.close(() => {
        proxy.disconnect().then(() => process.exit(0));
      });
    });
  });

// Test command
program
  .command('test')
  .description('Test connection to server')
  .action(async () => {
    const globalOpts = program.opts();
    const config = await getConfig({
      url: globalOpts.url,
      token: globalOpts.token,
      currentProfile: globalOpts.profile,
    });

    if (!config.url) {
      console.error(chalk.red('Error: No server URL configured'));
      console.error('Run "codeflow-client configure" to set up connection');
      process.exit(1);
    }

    console.log(chalk.gray(`Testing connection to ${config.url}...`));

    try {
      const client = new WebSocketClient(config.url, {
        token: config.token,
        requestTimeout: 5000,
      });

      await client.connect();
      console.log(chalk.green('✓ Connected successfully'));

      // Try to list tools
      console.log(chalk.gray('Fetching available tools...'));
      const result = await client.send('tools/list');
      const tools = (result as any)?.tools || [];

      console.log(chalk.green(`✓ Server has ${tools.length} tools available`));

      await client.disconnect();
      console.log(chalk.green('✓ Connection test successful'));
    } catch (error: any) {
      console.error(chalk.red('✗ Connection failed:'), error.message);
      process.exit(1);
    }
  });

// Helper function to format results
function formatResult(result: any, indent: number = 0): string {
  const spaces = ' '.repeat(indent);

  if (result === null || result === undefined) {
    return chalk.gray('null');
  }

  if (typeof result === 'string') {
    return chalk.yellow(`"${result}"`);
  }

  if (typeof result === 'number' || typeof result === 'boolean') {
    return chalk.cyan(String(result));
  }

  if (Array.isArray(result)) {
    if (result.length === 0) {
      return chalk.gray('[]');
    }
    const items = result.map((item) => `${spaces}  - ${formatResult(item, indent + 4)}`).join('\n');
    return `\n${items}`;
  }

  if (typeof result === 'object') {
    const entries = Object.entries(result);
    if (entries.length === 0) {
      return chalk.gray('{}');
    }
    const items = entries
      .map(([key, value]) => {
        const formattedValue = formatResult(value, indent + 2);
        return `${spaces}  ${chalk.blue(key)}: ${formattedValue}`;
      })
      .join('\n');
    return `\n${items}`;
  }

  return String(result);
}

program.parse();
