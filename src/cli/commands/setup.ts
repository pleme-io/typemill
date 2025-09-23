#!/usr/bin/env node

import { spawn } from 'node:child_process';
import inquirer from 'inquirer';
import { LANGUAGE_SERVERS, generateConfig } from '../../core/configuration/language-presets.js';
import { scanDirectoryForExtensions } from '../../core/file-operations/scanner.js';
import * as DirectoryUtils from '../utils/directory-utils.js';
import { getPipCommand, runInstallCommand } from '../utils/install-utils.js';
import * as ServerUtils from '../utils/server-utils.js';
import { getCommandPath } from '../utils/server-utils.js';

interface ServerChoice {
  name: string;
  value: string;
  checked: boolean;
  disabled?: string;
}

interface SetupOptions {
  all?: boolean;
  force?: boolean;
  servers?: string[];
  installPrereqs?: boolean;
}

/**
 * Interactive setup command with checkbox selection
 */
export async function setupCommand(options: SetupOptions = {}): Promise<void> {
  // Handle Ctrl+C gracefully
  const originalHandler = process.listeners('SIGINT');
  process.removeAllListeners('SIGINT');
  process.on('SIGINT', () => {
    console.log('\n\n👋 Setup cancelled by user');
    process.exit(0);
  });

  try {
    console.clear();
    if (options.all) {
      console.log('🚀 CodeBuddy Auto Setup (--all)\n');
    } else {
      console.log('🚀 CodeBuddy Interactive Setup\n');
    }

    // Check if config already exists
    const existingConfig = DirectoryUtils.readConfigSilent();
    if (existingConfig) {
      if (options.all || options.force || options.servers) {
        console.log(
          '✨ Configuration already exists. Using non-interactive mode will overwrite it.\n'
        );
      } else {
        try {
          const { overwrite } = await inquirer.prompt([
            {
              type: 'confirm',
              name: 'overwrite',
              message: 'Configuration already exists. Overwrite?',
              default: false,
            },
          ]);

          if (!overwrite) {
            console.log('\n✨ Keeping existing configuration');
            return;
          }
        } catch (error) {
          if (
            error instanceof Error &&
            (error.message?.includes('SIGINT') || error.name === 'ExitPromptError')
          ) {
            console.log('\n\n👋 Setup cancelled by user');
            process.exit(0);
          }
          throw error;
        }
      }
    }

    // Auto-migrate old config if needed
    if (DirectoryUtils.migrateOldConfig()) {
      console.log('✅ Migrated existing configuration\n');
    }

    console.log('📂 Scanning project for file types...\n');

    // Scan for file extensions
    let detectedExtensions: Set<string>;
    try {
      detectedExtensions = await scanDirectoryForExtensions(process.cwd());

      if (detectedExtensions.size === 0) {
        console.log('No source files detected in project\n');
      } else {
        const extArray = Array.from(detectedExtensions);
        const displayExts = extArray
          .slice(0, 10)
          .map((ext) => `.${ext}`)
          .join(', ');
        const more = extArray.length > 10 ? ` (+${extArray.length - 10} more)` : '';
        console.log(`Found: ${displayExts}${more}\n`);
      }
    } catch (error) {
      console.log('Could not scan project files\n');
      detectedExtensions = new Set();
    }

    // Test which servers are already installed
    console.log('🔍 Checking installed language servers...\n');

    // Show ALL language servers available, no filtering
    const relevantServers = LANGUAGE_SERVERS;

    if (relevantServers.length === 0) {
      console.log('No language servers available for your project files.');
      return;
    }

    // Test availability and prepare choices
    const choices: ServerChoice[] = [];

    for (const server of relevantServers) {
      // For Go and Rust, use the full path to test
      const testCommand = [...server.command];
      if (testCommand[0] === 'gopls' || testCommand[0] === 'rust-analyzer') {
        testCommand[0] = getCommandPath(testCommand[0]);
      }

      const available = await ServerUtils.testCommand(testCommand);
      const fileTypes = server.extensions.map((ext) => `.${ext}`).join(', ');

      // Auto-check if server handles detected file extensions
      const hasDetectedFiles = server.extensions.some((ext) => detectedExtensions.has(ext));

      choices.push({
        name: `${server.displayName} (${fileTypes}) ${available ? '✓ installed' : '○ not installed'}`,
        value: server.name,
        checked: hasDetectedFiles, // Pre-select if project has these file types
      });
    }

    // Select servers (interactive or auto)
    let selectedServers: string[];

    if (options.all) {
      // Auto-select all servers
      selectedServers = relevantServers.map((server) => server.name);
      console.log(`📋 Auto-selecting ${selectedServers.length} language servers:\n`);

      for (const server of relevantServers) {
        const testCommand = [...server.command];
        if (testCommand[0] === 'gopls' || testCommand[0] === 'rust-analyzer') {
          testCommand[0] = getCommandPath(testCommand[0]);
        }
        const available = await ServerUtils.testCommand(testCommand);
        const status = available ? '✓ installed' : '○ will install';
        console.log(`   ${server.displayName} ${status}`);
      }
      console.log('');
    } else if (options.servers && options.servers.length > 0) {
      // Pre-selected servers mode
      const validServers = options.servers.filter((name) =>
        relevantServers.some((server) => server.name === name)
      );
      const invalidServers = options.servers.filter(
        (name) => !relevantServers.some((server) => server.name === name)
      );

      if (invalidServers.length > 0) {
        console.log(`❌ Unknown servers: ${invalidServers.join(', ')}`);
        console.log(`Available servers: ${relevantServers.map((s) => s.name).join(', ')}`);
        return;
      }

      selectedServers = validServers;
      console.log(`📋 Installing selected servers: ${selectedServers.join(', ')}\n`);

      // Show status for selected servers
      for (const serverName of selectedServers) {
        const server = relevantServers.find((s) => s.name === serverName)!;
        const testCommand = [...server.command];
        if (testCommand[0] === 'gopls' || testCommand[0] === 'rust-analyzer') {
          testCommand[0] = getCommandPath(testCommand[0]);
        }
        const available = await ServerUtils.testCommand(testCommand);
        const status = available ? '✓ installed' : '○ will install';
        console.log(`   ${server.displayName} ${status}`);
      }
      console.log('');
    } else {
      // Interactive selection
      try {
        const result = await inquirer.prompt([
          {
            type: 'checkbox',
            name: 'selectedServers',
            message: 'Select language servers to install (detected file types are pre-checked):',
            choices,
            pageSize: 15,
            loop: false,
            validate: (answer) => {
              if (answer.length < 1) {
                return 'You must choose at least one language server.';
              }
              return true;
            },
          },
        ]);
        selectedServers = result.selectedServers;

        if (selectedServers.length === 0) {
          console.log('\n❌ No servers selected. Setup cancelled.');
          return;
        }
      } catch (error) {
        if (
          error instanceof Error &&
          (error.message?.includes('SIGINT') || error.name === 'ExitPromptError')
        ) {
          console.log('\n\n👋 Setup cancelled by user');
          process.exit(0);
        }
        throw error;
      }
    }

    // Check for missing prerequisites and offer to install them
    await checkAndInstallPrerequisites(selectedServers, options);

    // Find which servers need installation
    const toInstall = [];
    const alreadyInstalled = [];

    for (const serverName of selectedServers) {
      const server = LANGUAGE_SERVERS.find((s) => s.name === serverName);
      if (!server) continue;

      // For Go and Rust, use the full path to test
      const testCommand = [...server.command];
      if (testCommand[0] === 'gopls' || testCommand[0] === 'rust-analyzer') {
        testCommand[0] = getCommandPath(testCommand[0]);
      }

      const available = await ServerUtils.testCommand(testCommand);
      if (available) {
        alreadyInstalled.push(server);
      } else {
        toInstall.push(server);
      }
    }

    // Install missing servers
    if (toInstall.length > 0) {
      console.log(
        `\n📦 Installing ${toInstall.length} language server${toInstall.length > 1 ? 's' : ''}...\n`
      );

      const installResults = [];
      const installedPackages = new Set<string>();

      for (const server of toInstall) {
        process.stdout.write(`Installing ${server.displayName}... `);

        if (!server.installCommand || server.installCommand.length === 0) {
          console.log('❌ No auto-install available');
          console.log(`  Manual install required: ${server.installInstructions}`);
          installResults.push({ server, success: false, manual: true });
          continue;
        }

        // Check for missing dependencies before attempting install
        const [baseCommand] = server.installCommand;
        if (baseCommand === 'go' && !(await ServerUtils.testCommand(['go', 'version']))) {
          console.log('❌ Failed');
          console.log(`    Error: Go is required but not installed`);
          if (process.platform === 'darwin') {
            console.log('    Install Go first: brew install go');
          } else {
            console.log('    Install Go first: https://golang.org/dl/');
          }
          installResults.push({ server, success: false, manual: true });
          continue;
        }

        const pipCommand = getPipCommand(['pip', '--version'])[0];
        if (
          (baseCommand === 'pip' || baseCommand === 'pip3') &&
          pipCommand &&
          !(await ServerUtils.testCommand([pipCommand, '--version']))
        ) {
          console.log('❌ Failed');
          console.log('    Error: Python pip is required but not available');
          console.log('    Install Python first: https://python.org/downloads/');
          console.log('    Or install pipx: brew install pipx');
          installResults.push({ server, success: false, manual: true });
          continue;
        }

        // Check if we've already installed this package
        const packageKey = server.installCommand.join(' ');
        if (installedPackages.has(packageKey)) {
          console.log('✅ (already installed in this session)');
          installResults.push({ server, success: true });
          continue;
        }

        // Handle pip commands specially
        let installCmd = server.installCommand;
        if (installCmd[0] === 'pip' || installCmd[0] === 'pip3') {
          installCmd = getPipCommand(installCmd);
        }

        const success = await runInstallCommand(installCmd, server.displayName);

        if (success) {
          installedPackages.add(packageKey);
        }

        if (success) {
          // For Go and Rust, use the full path to verify
          const testCommand = [...server.command];
          if (testCommand[0] === 'gopls' || testCommand[0] === 'rust-analyzer') {
            testCommand[0] = getCommandPath(testCommand[0]);
          }

          // Verify installation worked
          const nowAvailable = await ServerUtils.testCommand(testCommand);
          if (nowAvailable) {
            console.log('✅');
            installResults.push({ server, success: true });
          } else {
            console.log('⚠️ Installed but not detected');
            installResults.push({ server, success: false });
          }
        } else {
          console.log('❌ Failed');
          installResults.push({ server, success: false });
        }
      }

      // Show installation summary
      const successCount = installResults.filter((r) => r.success).length;
      const failedCount = installResults.filter((r) => !r.success && !r.manual).length;
      const manualCount = installResults.filter((r) => r.manual).length;

      console.log('\n📊 Installation Summary:');
      if (successCount > 0) {
        console.log(`  ✅ Successfully installed: ${successCount}`);
      }
      if (failedCount > 0) {
        console.log(`  ❌ Failed to install: ${failedCount}`);
      }
      if (manualCount > 0) {
        console.log(`  ⚠️  Manual installation required: ${manualCount}`);
      }

      // Show manual install instructions for failed ones
      const needsManual = installResults.filter((r) => !r.success);
      if (needsManual.length > 0) {
        console.log('\n📝 Manual installation instructions:');
        for (const { server } of needsManual) {
          console.log(`  ${server.displayName}: ${server.installInstructions}`);
        }
      }
    }

    // Verify which servers are actually working before saving config
    const workingServers = [];
    const workingServerNames = [];

    console.log('\n🔍 Verifying installed language servers...\n');

    for (const serverName of selectedServers) {
      const server = LANGUAGE_SERVERS.find((s) => s.name === serverName);
      if (server) {
        // For Go and Rust, use the full path to test
        const testCommand = [...server.command];
        if (testCommand[0] === 'gopls' || testCommand[0] === 'rust-analyzer') {
          testCommand[0] = getCommandPath(testCommand[0]);
        }

        if (await ServerUtils.testCommand(testCommand)) {
          workingServers.push(server);
          workingServerNames.push(serverName);
          console.log(`   ✓ ${server.displayName}`);
        } else {
          console.log(`   ✗ ${server.displayName} (not working - excluded from config)`);
        }
      }
    }

    // Create configuration with ONLY working servers
    const config = generateConfig(workingServerNames);

    // Save configuration
    DirectoryUtils.writeConfig(config);

    // Final summary
    console.log(`\n${'='.repeat(50)}`);
    console.log('\n✨ Setup Complete!\n');
    console.log(`📁 Configuration saved to: ${DirectoryUtils.getConfigPath()}`);
    console.log(`📋 Active servers: ${workingServers.length}/${selectedServers.length}`);

    // Check if this is a re-run with no changes
    const finalConfig = DirectoryUtils.readConfig();
    const configUnchanged = finalConfig && JSON.stringify(finalConfig) === JSON.stringify(config);

    if (configUnchanged) {
      console.log('Already set up—nothing changed.');
    } else {
      const lspNames = workingServers
        .map((s) => {
          const cmd = s.command[0] || 'unknown';
          const nameMap: Record<string, string> = {
            'typescript-language-server': 'TypeScript',
            pylsp: 'Python',
            gopls: 'Go',
            'rust-analyzer': 'Rust',
          };
          return nameMap[cmd] || cmd;
        })
        .join(', ');

      console.log(
        `\n✅ LSPs ready: ${lspNames || 'None'}. Next: \`codeflow-buddy link\` to choose assistants.`
      );

      if (workingServers.length < selectedServers.length) {
        const notWorking = selectedServers.length - workingServers.length;
        console.log(`⚠️  ${notWorking} server${notWorking !== 1 ? 's' : ''} excluded (not working)`);
        console.log('   💡 Tip: Install missing prerequisites and run setup again');
      }
    }
  } finally {
    // Restore original SIGINT handlers
    process.removeAllListeners('SIGINT');
    for (const handler of originalHandler) {
      process.on('SIGINT', handler);
    }
  }
}

/**
 * Check for missing prerequisites and offer to install them
 */
async function checkAndInstallPrerequisites(
  selectedServers: string[],
  options: SetupOptions
): Promise<void> {
  const missingPrereqs = await detectMissingPrerequisites(selectedServers);

  if (missingPrereqs.length === 0) {
    return; // All prerequisites available
  }

  console.log('\n🔍 Detected missing prerequisites:\n');

  for (const prereq of missingPrereqs) {
    console.log(`   ❌ ${prereq.name} - needed for ${prereq.servers.join(', ')}`);
    console.log(`      Install: ${prereq.installCommand}`);
  }

  // Skip prompt if force mode or installPrereqs flag
  if (options.force && options.installPrereqs) {
    console.log('\n🔧 Auto-installing prerequisites...\n');
    await installPrerequisites(missingPrereqs);
    return;
  }

  if (options.force) {
    console.log('\n⚠️  Prerequisites missing - some servers may fail to install');
    return;
  }

  // Interactive prompt to install prerequisites
  try {
    const { installPrereqs } = await inquirer.prompt([
      {
        type: 'confirm',
        name: 'installPrereqs',
        message: 'Install missing prerequisites automatically?',
        default: true,
      },
    ]);

    if (installPrereqs) {
      console.log('\n🔧 Installing prerequisites...\n');
      await installPrerequisites(missingPrereqs);
    } else {
      console.log('\n⚠️  Skipping prerequisite installation - some servers may fail');
    }
  } catch (error) {
    if (
      error instanceof Error &&
      (error.message?.includes('SIGINT') || error.name === 'ExitPromptError')
    ) {
      console.log('\n👋 Setup cancelled by user');
      process.exit(0);
    }
    throw error;
  }
}

interface Prerequisite {
  name: string;
  command: string;
  installCommand: string;
  servers: string[];
  autoInstallable: boolean;
}

/**
 * Detect missing prerequisites for selected servers
 */
async function detectMissingPrerequisites(selectedServers: string[]): Promise<Prerequisite[]> {
  const prereqs = new Map<string, Prerequisite>();

  for (const serverName of selectedServers) {
    const server = LANGUAGE_SERVERS.find((s) => s.name === serverName);
    if (!server?.installCommand) continue;

    const [baseCommand] = server.installCommand;

    // Check for Go
    if (baseCommand === 'go' && !(await ServerUtils.testCommand(['go', 'version']))) {
      const existing = prereqs.get('go') || getPrerequisiteInfo('go', 'Go');
      existing.servers.push(server.displayName);
      prereqs.set('go', existing);
    }

    // Check for Python/pip
    if (baseCommand === 'pip' || baseCommand === 'pip3') {
      const pipCommand = getPipCommand(['pip', '--version'])[0];
      if (pipCommand && !(await ServerUtils.testCommand([pipCommand, '--version']))) {
        const existing = prereqs.get('python') || getPrerequisiteInfo('python', 'Python/pip');
        existing.servers.push(server.displayName);
        prereqs.set('python', existing);
      }
    }

    // Check for Ruby/gem
    if (baseCommand === 'gem' && !(await ServerUtils.testCommand(['gem', '--version']))) {
      const existing = prereqs.get('ruby') || getPrerequisiteInfo('ruby', 'Ruby/gem');
      existing.servers.push(server.displayName);
      prereqs.set('ruby', existing);
    }

    // Check for npm (should be rare since we're running in Node.js)
    if (baseCommand === 'npm' && !(await ServerUtils.testCommand(['npm', '--version']))) {
      const existing = prereqs.get('npm') || getPrerequisiteInfo('npm', 'npm');
      existing.servers.push(server.displayName);
      prereqs.set('npm', existing);
    }
  }

  return Array.from(prereqs.values());
}

/**
 * Get platform-specific prerequisite information
 */
function getPrerequisiteInfo(command: string, displayName: string): Prerequisite {
  const platform = process.platform;

  const prereqInfo: Record<
    string,
    {
      darwin: { cmd: string; autoInstallable: boolean };
      linux: { cmd: string; autoInstallable: boolean };
      win32: { cmd: string; autoInstallable: boolean };
    }
  > = {
    go: {
      darwin: { cmd: 'brew install go', autoInstallable: true },
      linux: { cmd: 'sudo apt install golang-go', autoInstallable: true },
      win32: { cmd: 'Download from https://golang.org/dl/', autoInstallable: false },
    },
    python: {
      darwin: { cmd: 'brew install python', autoInstallable: true },
      linux: { cmd: 'sudo apt install python3 python3-pip', autoInstallable: true },
      win32: { cmd: 'Download from https://python.org/downloads/', autoInstallable: false },
    },
    ruby: {
      darwin: { cmd: 'brew install ruby', autoInstallable: true },
      linux: { cmd: 'sudo apt install ruby ruby-dev', autoInstallable: true },
      win32: { cmd: 'Download from https://rubyinstaller.org/', autoInstallable: false },
    },
    npm: {
      darwin: { cmd: 'brew install node', autoInstallable: true },
      linux: { cmd: 'sudo apt install nodejs npm', autoInstallable: true },
      win32: { cmd: 'Download from https://nodejs.org/', autoInstallable: false },
    },
  };

  const info = prereqInfo[command];
  if (!info) {
    return {
      name: displayName,
      command,
      installCommand: `Install ${displayName}`,
      servers: [],
      autoInstallable: false,
    };
  }

  const platformInfo = info[platform as keyof typeof info] || info.win32;

  return {
    name: displayName,
    command,
    installCommand: platformInfo.cmd,
    servers: [],
    autoInstallable: platformInfo.autoInstallable,
  };
}

/**
 * Install prerequisites using system package managers
 */
async function installPrerequisites(prereqs: Prerequisite[]): Promise<void> {
  for (const prereq of prereqs) {
    if (!prereq.autoInstallable) {
      console.log(`⚠️  ${prereq.name} requires manual installation: ${prereq.installCommand}`);
      continue;
    }

    console.log(`🔧 Installing ${prereq.name}...`);

    const success = await runSystemCommand(prereq.installCommand);

    if (success) {
      console.log(`✅ ${prereq.name} installed successfully`);
    } else {
      console.log(`❌ Failed to install ${prereq.name}`);
      console.log(`   Manual installation: ${prereq.installCommand}`);
    }
  }
}

/**
 * Run a system command (like brew install)
 */
async function runSystemCommand(command: string): Promise<boolean> {
  return new Promise((resolve) => {
    const [cmd, ...args] = command.split(' ');

    if (!cmd) {
      resolve(false);
      return;
    }

    const proc = spawn(cmd, args, {
      stdio: 'pipe',
    });

    proc.on('error', () => {
      resolve(false);
    });

    proc.on('close', (code: number | null) => {
      resolve(code === 0);
    });
  });
}
