import * as readline from 'node:readline';
import type { AssistantInfo } from '../types.js';
import {
  findInstalledAssistants,
  getAllAssistantNames,
  getAssistantByName,
  removeMCPServer,
} from '../utils/assistant-utils.js';

interface UnlinkOptions {
  assistants?: string[];
  all?: boolean;
}

/**
 * Interactive prompt for assistant selection
 */
async function promptForAssistant(assistants: AssistantInfo[]): Promise<string[]> {
  const linkedAssistants = assistants.filter((a) => a.linked);

  if (linkedAssistants.length === 0) {
    console.log('No linked assistants found to unlink.');
    return [];
  }

  console.log('\nCurrently linked assistants:');
  linkedAssistants.forEach((a, i) => {
    console.log(`  ${i + 1}. ${a.displayName}`);
  });

  const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout,
  });

  return new Promise((resolve) => {
    rl.question(
      '\nSelect assistants to unlink (numbers separated by space, or "all"): ',
      (answer) => {
        rl.close();

        if (answer.toLowerCase() === 'all') {
          resolve(linkedAssistants.map((a) => a.name));
          return;
        }

        const indices = answer
          .split(/\s+/)
          .map((s) => Number.parseInt(s, 10) - 1)
          .filter((i) => !Number.isNaN(i) && i >= 0 && i < linkedAssistants.length);

        resolve(
          indices
            .map((i) => linkedAssistants[i]?.name)
            .filter((name): name is string => Boolean(name))
        );
      }
    );
  });
}

/**
 * Unlink codeflow-buddy from AI assistants
 */
export async function unlinkCommand(options: UnlinkOptions = {}): Promise<void> {
  try {
    const allAssistants = findInstalledAssistants();

    // Determine which assistants to unlink
    let targetAssistants: string[] = [];

    if (options.all) {
      // Unlink all linked assistants
      targetAssistants = allAssistants.filter((a) => a.linked).map((a) => a.name);
    } else if (options.assistants && options.assistants.length > 0) {
      // Unlink specific assistants
      targetAssistants = options.assistants;
    } else {
      // Interactive selection
      targetAssistants = await promptForAssistant(allAssistants);
    }

    if (targetAssistants.length === 0) {
      console.log('No assistants selected.');
      return;
    }

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

      if (!assistant.linked) {
        results.push({
          name: assistant.displayName,
          success: false,
          message: `Not linked to ${assistant.displayName}`,
        });
        continue;
      }

      try {
        const removed = removeMCPServer(assistant.configPath, 'codeflow-buddy');
        if (removed) {
          results.push({
            name: assistant.displayName,
            success: true,
            message: `✅ Unlinked from ${assistant.displayName}`,
          });
        } else {
          results.push({
            name: assistant.displayName,
            success: false,
            message: `Failed to unlink from ${assistant.displayName}`,
          });
        }
      } catch (error) {
        results.push({
          name: assistant.displayName,
          success: false,
          message: `Error unlinking from ${assistant.displayName}: ${error}`,
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

    // Show restart instructions if any were successfully unlinked
    const successfulUnlinks = results.filter((r) => r.success);
    if (successfulUnlinks.length > 0) {
      console.log('\n→ Restart the following to complete removal:');
      for (const result of successfulUnlinks) {
        console.log(`  • ${result.name}`);
      }
    }
  } catch (error) {
    console.error('Error unlinking assistants:', error);
    process.exit(1);
  }
}
