import { existsSync, readFileSync, unlinkSync, writeFileSync } from 'node:fs';
import { mkdirSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { dirname } from 'node:path';
import type { DiagnosticService } from '../../services/diagnostic-service.js';

// Handler for get_diagnostics tool
export async function handleGetDiagnostics(
  diagnosticService: DiagnosticService,
  args: { file_path: string }
) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);

  try {
    const diagnostics = await diagnosticService.getDiagnostics(absolutePath);

    // Handle undefined return (should not happen, but defensive coding)
    if (!diagnostics || !Array.isArray(diagnostics)) {
      return {
        content: [
          {
            type: 'text',
            text: `Error getting diagnostics: Diagnostic service returned invalid result (${typeof diagnostics})`,
          },
        ],
      };
    }

    if (diagnostics.length === 0) {
      return {
        content: [
          {
            type: 'text',
            text: `No diagnostics found for ${file_path}. The file has no errors, warnings, or hints.`,
          },
        ],
      };
    }

    const severityMap = {
      1: 'Error',
      2: 'Warning',
      3: 'Information',
      4: 'Hint',
    };

    const diagnosticMessages = diagnostics.map((diag) => {
      const severity = diag.severity ? severityMap[diag.severity] || 'Unknown' : 'Unknown';
      const code = diag.code ? ` [${diag.code}]` : '';
      const source = diag.source ? ` (${diag.source})` : '';
      const { start, end } = diag.range;

      return `• ${severity}${code}${source}: ${diag.message}\n  Location: Line ${start.line + 1}, Column ${start.character + 1} to Line ${end.line + 1}, Column ${end.character + 1}`;
    });

    return {
      content: [
        {
          type: 'text',
          text: `Found ${diagnostics.length} diagnostic${diagnostics.length === 1 ? '' : 's'} in ${file_path}:\n\n${diagnosticMessages.join('\n\n')}`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error getting diagnostics: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}

// Handler for restart_server tool
export async function handleRestartServer(
  newLspClient: import('../../lsp/client.js').LSPClient,
  args: { extensions?: string[] }
) {
  const { extensions } = args;

  try {
    // Clear failed servers to allow retry
    newLspClient.serverManager.clearFailedServers();

    const restartedServers = await newLspClient.restartServer(extensions);

    let response = `Successfully restarted ${restartedServers.length} LSP server(s)`;

    if (restartedServers.length > 0) {
      response += `\n\nRestarted servers:\n${restartedServers.map((s) => `• ${s}`).join('\n')}`;
    }

    response +=
      '\n\nNote: Any previously failed servers have been cleared and will be retried on next access.';

    return {
      content: [
        {
          type: 'text',
          text: response,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error restarting servers: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}

// Handler for rename_file tool
export async function handleRenameFile(args: {
  old_path: string;
  new_path: string;
  dry_run?: boolean;
}) {
  const { old_path, new_path, dry_run = false } = args;

  try {
    const { renameFile } = await import('../../file-editor.js');
    // Pass the workspace root directory to enable import detection
    // Don't use gitignore filtering to ensure all files are checked (including test/playground files)
    const rootDir = process.cwd(); // Use current working directory as root
    process.stderr.write(
      `[DEBUG handleRenameFile] rootDir: ${rootDir}, old_path: ${old_path}, new_path: ${new_path}, dry_run: ${dry_run}\n`
    );
    const result = await renameFile(old_path, new_path, undefined, {
      dry_run,
      rootDir,
      useGitignore: false, // Don't filter gitignored files so tests work
    });

    if (!result.success) {
      return {
        content: [
          {
            type: 'text',
            text: `Failed to rename file: ${result.error}`,
          },
        ],
      };
    }

    if (dry_run) {
      // In dry-run mode, show what would be changed
      const message = result.error || '[DRY RUN] No changes would be made';
      return {
        content: [
          {
            type: 'text',
            text: message,
          },
        ],
      };
    }

    // Success message
    const importCount = result.importUpdates
      ? Object.keys(result.importUpdates.changes || {}).length
      : 0;

    return {
      content: [
        {
          type: 'text',
          text: `✅ Successfully renamed ${old_path} to ${new_path}\n\nFiles modified: ${result.filesModified.length}\n${
            importCount > 0
              ? `Files with updated imports: ${importCount}`
              : 'No import updates needed'
          }`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error renaming file: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}

// Handler for create_file tool
export async function handleCreateFile(args: {
  file_path: string;
  content?: string;
  overwrite?: boolean;
}) {
  const { file_path, content = '', overwrite = false } = args;
  const absolutePath = resolve(file_path);

  try {
    // Check if file already exists
    if (existsSync(absolutePath) && !overwrite) {
      return {
        content: [
          {
            type: 'text',
            text: `File ${file_path} already exists. Use overwrite: true to replace it.`,
          },
        ],
      };
    }

    // Ensure parent directory exists
    const parentDir = dirname(absolutePath);
    if (!existsSync(parentDir)) {
      mkdirSync(parentDir, { recursive: true });
    }

    // Write file content
    writeFileSync(absolutePath, content, 'utf8');

    // Note: LSP notification for file creation would require access to private methods
    // For now, file creation works without LSP notification (filesystem operation only)
    // Future enhancement: Add public method for file operation notifications

    return {
      content: [
        {
          type: 'text',
          text: `✅ Successfully created ${file_path}${content ? ` with ${content.length} characters` : ' (empty file)'}`,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error creating file: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}

// Handler for delete_file tool
export async function handleDeleteFile(args: {
  file_path: string;
  force?: boolean;
}) {
  const { file_path, force = false } = args;
  const absolutePath = resolve(file_path);

  try {
    // Check if file exists
    if (!existsSync(absolutePath)) {
      return {
        content: [
          {
            type: 'text',
            text: `File ${file_path} does not exist.`,
          },
        ],
      };
    }

    // Import the project scanner for impact analysis
    const { projectScanner } = await import('../../utils/project-scanner.js');

    // Find all files that import this file
    process.stderr.write(`[DEBUG] Analyzing impact of deleting ${absolutePath}\n`);
    const importers = await projectScanner.findImporters(absolutePath);

    if (importers.length > 0 && !force) {
      // File is imported by other files - warn user
      const relativeImporters = importers.map((imp) => relative(process.cwd(), imp));

      return {
        content: [
          {
            type: 'text',
            text: `⚠️ Cannot delete ${file_path} - it is imported by ${importers.length} file${importers.length === 1 ? '' : 's'}:\n\n${relativeImporters.map((f) => `  • ${f}`).join('\n')}\n\n${importers.length === 1 ? 'This file depends' : 'These files depend'} on ${file_path}. Deleting it will cause import errors.\n\nTo force deletion despite broken imports, use:\n  force: true`,
          },
        ],
      };
    }

    // If force is true or no importers, proceed with deletion
    if (importers.length > 0 && force) {
      process.stderr.write(
        `[DEBUG] Force deleting ${absolutePath} despite ${importers.length} importers\n`
      );
    }

    // Delete the file
    unlinkSync(absolutePath);

    // Build success message
    let message = `✅ Successfully deleted ${file_path}`;

    if (importers.length > 0) {
      const relativeImporters = importers.map((imp) => relative(process.cwd(), imp));
      message += `\n\n⚠️ Warning: ${importers.length} file${importers.length === 1 ? ' has' : 's have'} broken imports:\n${relativeImporters.map((f) => `  • ${f}`).join('\n')}\n\nYou may need to update or remove these import statements.`;
    }

    return {
      content: [
        {
          type: 'text',
          text: message,
        },
      ],
    };
  } catch (error) {
    return {
      content: [
        {
          type: 'text',
          text: `Error deleting file: ${error instanceof Error ? error.message : String(error)}`,
        },
      ],
    };
  }
}
