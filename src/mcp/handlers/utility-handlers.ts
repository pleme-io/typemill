import { existsSync, unlinkSync, writeFileSync } from 'node:fs';
import { mkdirSync } from 'node:fs';
import { resolve } from 'node:path';
import { dirname } from 'node:path';
import type { LSPClient } from '../../lsp-client.js';
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
export async function handleRestartServer(lspClient: LSPClient, args: { extensions?: string[] }) {
  const { extensions } = args;

  try {
    const result = await lspClient.restartServers(extensions);

    let response = result.message;

    if (result.restarted.length > 0) {
      response += `\n\nRestarted servers:\n${result.restarted.map((s) => `• ${s}`).join('\n')}`;
    }

    if (result.failed.length > 0) {
      response += `\n\nFailed to restart:\n${result.failed.map((s) => `• ${s}`).join('\n')}`;
    }

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
export async function handleRenameFile(
  lspClient: LSPClient,
  args: {
    old_path: string;
    new_path: string;
    dry_run?: boolean;
  }
) {
  const { old_path, new_path, dry_run = false } = args;

  try {
    const { renameFile } = await import('../../file-editor.js');
    const result = await renameFile(old_path, new_path, lspClient, { dry_run });

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
export async function handleCreateFile(
  _lspClient: LSPClient, // unused - kept for interface compatibility
  args: {
    file_path: string;
    content?: string;
    overwrite?: boolean;
  }
) {
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
export async function handleDeleteFile(
  _lspClient: LSPClient, // unused - kept for interface compatibility
  args: {
    file_path: string;
    force?: boolean;
  }
) {
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

    // Note: LSP notification for file deletion would require access to private methods
    // For now, file deletion works without LSP notification (filesystem operation only)
    // Future enhancement: Add public method for file operation notifications

    // Delete the file
    unlinkSync(absolutePath);

    // File successfully deleted (LSP notifications would require public API enhancement)

    return {
      content: [
        {
          type: 'text',
          text: `✅ Successfully deleted ${file_path}`,
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
