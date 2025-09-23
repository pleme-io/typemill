#!/usr/bin/env node

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
      if (options.all) {
        console.log('✨ Configuration already exists. Using --all will overwrite it.\n');
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

    // Create configuration with all selected servers
    const config = generateConfig(selectedServers);

    // Save configuration
    DirectoryUtils.writeConfig(config);

    // Final summary
    console.log(`\n${'='.repeat(50)}`);
    console.log('\n✨ Setup Complete!\n');
    console.log(`📁 Configuration saved to: ${DirectoryUtils.getConfigPath()}`);

    // Count how many servers are actually working
    const workingServers = [];
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
        }
      }
    }

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
        console.log(
          `⚠️  ${notWorking} server${notWorking !== 1 ? 's' : ''} need manual installation`
        );
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
