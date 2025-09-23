import * as readline from 'node:readline';
import type { AssistantInfo } from '../types.js';
import {
  addMCPServer,
  findInstalledAssistants,
  getAllAssistantNames,
  getAssistantByName,
  getMCPServerConfig,
} from '../utils/assistant-utils.js';

interface LinkOptions {
  assistants?: string[];
  all?: boolean;
}

/**
 * Interactive prompt for assistant selection
 */
async function promptForAssistant(assistants: AssistantInfo[]): Promise<string[]> {
  const availableAssistants = assistants.filter((a) => a.installed && !a.linked);

  if (availableAssistants.length === 0) {
    console.log('No unlinked assistants found to link.');
    return [];
  }

  console.log('\nAvailable AI assistants:');
  availableAssistants.forEach((a, i) => {
    console.log(`  ${i + 1}. ${a.displayName}`);
  });

  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  return new Promise((resolve) => {
    rl.question(
      '\nSelect assistants to link (numbers separated by space, or "all"): ',
      (answer) => {
        rl.close();

        if (answer.toLowerCase() === 'all') {
          resolve(availableAssistants.map((a) => a.name));
          return;
        }

        const indices = answer
          .split(/\s+/)
          .map((s) => Number.parseInt(s, 10) - 1)
          .filter((i) => !Number.isNaN(i) && i >= 0 && i < availableAssistants.length);

        resolve(
          indices
            .map((i) => availableAssistants[i]?.name)
            .filter((name): name is string => Boolean(name))
        );
      }
    );
  });
}

/**
 * Link codeflow-buddy to AI assistants
 */
export async function linkCommand(options: LinkOptions = {}): Promise<void> {
  try {
    const allAssistants = findInstalledAssistants();

    // Determine which assistants to link
    let targetAssistants: string[] = [];

    if (options.all) {
      // Link all available assistants
      targetAssistants = allAssistants.filter((a) => a.installed && !a.linked).map((a) => a.name);
    } else if (options.assistants && options.assistants.length > 0) {
      // Link specific assistants
      targetAssistants = options.assistants;
    } else {
      // Interactive selection
      targetAssistants = await promptForAssistant(allAssistants);
    }

    if (targetAssistants.length === 0) {
      console.log('No assistants selected.');
      return;
    }

    // Get the MCP server configuration
    const serverConfig = getMCPServerConfig();

    // Process each assistant
    const results: { name: string; success: boolean; message: string }[] = [];

    for (const assistantName of targetAssistants) {
      const assistant = getAssistantByName(assistantName);

      if (!assistant) {
        // Try to help with typos
        const allNames = getAllAssistantNames();
        const suggestion = allNames.find((n) =>
          n.toLowerCase().includes(assistantName.toLowerCase())
        );

        if (suggestion) {
          results.push({
            name: assistantName,
            success: false,
            message: `Unknown assistant "${assistantName}". Did you mean "${suggestion}"?`,
          });
        } else {
          results.push({
            name: assistantName,
            success: false,
            message: `Unknown assistant "${assistantName}". Available: ${allNames.join(', ')}`,
          });
        }
        continue;
      }

      if (!assistant.installed) {
        results.push({
          name: assistant.displayName,
          success: false,
          message: `${assistant.displayName} is not installed`,
        });
        continue;
      }

      if (assistant.linked) {
        results.push({
          name: assistant.displayName,
          success: true,
          message: `Already linked to ${assistant.displayName}`,
        });
        continue;
      }

      try {
        addMCPServer(assistant.configPath, 'codeflow-buddy', serverConfig);
        results.push({
          name: assistant.displayName,
          success: true,
          message: `✅ Linked to ${assistant.displayName}`,
        });
      } catch (error) {
        results.push({
          name: assistant.displayName,
          success: false,
          message: `Failed to link to ${assistant.displayName}: ${error}`,
        });
      }
    }

    // Display results
    console.log();
    for (const result of results) {
      if (result.success) {
        console.log(result.message);
      } else {
        console.error(result.message);
      }
    }

    // Show restart instructions if any were successfully linked
    const successfulLinks = results.filter(
      (r) => r.success && !r.message.includes('Already linked')
    );
    if (successfulLinks.length > 0) {
      console.log('\n→ Restart the following to activate:');
      for (const result of successfulLinks) {
        console.log(`  • ${result.name}`);
      }
    }
  } catch (error) {
    console.error('Error linking assistants:', error);
    process.exit(1);
  }
}
