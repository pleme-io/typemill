import type { StatusOutput } from '../types.js';
import { findInstalledAssistants } from '../utils/assistant-utils.js';
import * as DirectoryUtils from '../utils/directory-utils.js';
import * as ServerUtils from '../utils/server-utils.js';

interface ServerInfo {
  name: string;
  extensions: string[];
  command: string[];
  available: boolean;
  running: boolean;
  pid?: number;
}

/**
 * Status command - shows what's working right now
 * Exactly what Bob specified
 */
export async function statusCommand(): Promise<void> {
  const args = process.argv.slice(3);
  const jsonMode = args.includes('--json');

  // Auto-migrate if needed
  DirectoryUtils.migrateOldConfig();

  if (!jsonMode) {
    console.log('Language Servers:');
  }

  const config = DirectoryUtils.readConfig();

  if (!config || !config.servers?.length) {
    if (!jsonMode) {
      console.log('  No configuration found');
      console.log('  Run: codeflow-buddy setup');
    } else {
      const output: StatusOutput = {
        lsps: [],
        assistants: [],
        server: { status: 'not-configured' },
      };
      console.log(JSON.stringify(output, null, 2));
    }
    return;
  }

  const state = DirectoryUtils.readState();
  const servers: ServerInfo[] = [];
  let activeCount = 0;
  let issueCount = 0;

  // Test each server
  for (const server of config.servers) {
    const serverKey = getServerKey(server);
    const serverState = state[serverKey];
    const running = serverState ? ServerUtils.isProcessRunning(serverState.pid) : false;

    // Test if server command is available
    const available = await ServerUtils.testCommand(server.command);

    const serverInfo: ServerInfo = {
      name: getServerName(server.command[0] || 'unknown'),
      extensions: server.extensions,
      command: server.command,
      available,
      running,
      pid: running ? serverState?.pid : undefined,
    };

    servers.push(serverInfo);

    if (available) {
      activeCount++;
    } else {
      issueCount++;
    }
  }

  // Get assistant status
  const assistants = findInstalledAssistants();

  if (jsonMode) {
    // JSON output mode
    const output: StatusOutput = {
      lsps: servers.map((s) => ({
        name: s.name,
        status: s.available ? 'ok' : 'error',
      })),
      assistants: assistants.map((a) => ({
        name: a.name,
        linked: a.linked,
      })),
      server: {
        status: 'running',
        uptime_sec: 0, // TODO: calculate actual uptime if needed
      },
    };
    console.log(JSON.stringify(output, null, 2));
    process.exit(issueCount > 0 ? 1 : 0);
  } else {
    // Normal display mode
    for (const server of servers) {
      const status = server.available ? '✓' : '✗';
      const extList = `(${server.extensions.map((ext) => `.${ext}`).join(' ')})`;
      const runningInfo = server.running ? ` [PID: ${server.pid}]` : '';
      const fixHint = server.available ? '' : " - run 'codeflow-buddy setup' to install";

      console.log(`  ${status} ${server.name}  ${extList}${runningInfo}${fixHint}`);
    }

    console.log('');
    console.log(`Active: ${activeCount} servers`);
    if (issueCount > 0) {
      console.log(`Issues: ${issueCount} - run 'codeflow-buddy setup' to install missing servers`);
    }

    // Show new compact status
    console.log('');
    console.log('──────────────────────────────────');
    const lspStatus = servers.map((s) => `${s.available ? '✓' : '✗'} ${s.name}`).join('  ');
    console.log(`LSPs:     ${lspStatus}`);

    const linkedStatus = assistants
      .filter((a) => a.installed)
      .map((a) => `${a.linked ? '✓' : '✗'} ${a.displayName}`)
      .join('  ');
    console.log(`Linked:   ${linkedStatus || 'None'}`);

    // TODO: Get actual server status
    console.log('Server:   Ready');

    // Show tips
    const unlinkedAssistants = assistants.filter((a) => a.installed && !a.linked);
    if (unlinkedAssistants.length > 0) {
      console.log('');
      console.log(
        `→ Tip: run \`codeflow-buddy link ${unlinkedAssistants[0]?.name}\` to add ${unlinkedAssistants[0]?.displayName}`
      );
    }
  }
}

function getServerKey(server: { command: string[] }): string {
  return JSON.stringify(server.command);
}

function getServerName(command: string): string {
  const nameMap: Record<string, string> = {
    npx: 'TypeScript',
    'typescript-language-server': 'TypeScript',
    pylsp: 'Python',
    gopls: 'Go',
    'rust-analyzer': 'Rust',
    clangd: 'C/C++',
    jdtls: 'Java',
    solargraph: 'Ruby',
    intelephense: 'PHP',
    'docker-langserver': 'Docker',
    'yaml-language-server': 'YAML',
    'bash-language-server': 'Shell',
    'vscode-json-language-server': 'JSON',
    'vscode-html-language-server': 'HTML',
    'vscode-css-language-server': 'CSS',
  };

  return nameMap[command] || command;
}
