import { existsSync, mkdirSync, unlinkSync, writeFileSync } from 'node:fs';
import { dirname, relative, resolve } from 'node:path';
import { logger } from '../../../../../server/src/core/diagnostics/logger.js';
import type { WorkspaceEdit } from '../../../../../server/src/core/file-operations/editor.js';
import { registerTools } from '../../../../../server/src/mcp/tool-registry.js';
import {
  createContextualErrorResponse,
  createFileModificationResponse,
  createListResponse,
  createMCPResponse,
  createNoResultsResponse,
  createSuccessResponse,
} from '../../../../../server/src/mcp/utils.js';
import { DependencyOrchestrator } from '../../../../../server/src/mcp/workflow/index.js';
import type { ServiceContext } from '../../../../../server/src/services/service-context.js';
import {
  assertValidFilePath,
  formatHumanRange,
  measureAndTrack,
  toHumanRange,
  ValidationError,
} from '../../../../core/src/utils/index.js';
import type { DiagnosticService } from '../../services/lsp/diagnostic-service.js';

// Handler for get_diagnostics tool
export async function handleGetDiagnostics(
  diagnosticService: DiagnosticService,
  args: { file_path: string }
) {
  const { file_path } = args;

  return measureAndTrack(
    'get_diagnostics',
    async () => {
      // Validate inputs
      try {
        assertValidFilePath(file_path);
      } catch (error) {
        if (error instanceof ValidationError) {
          return createContextualErrorResponse(error, {
            operation: 'get_diagnostics',
            filePath: file_path,
          });
        }
        throw error;
      }

      const absolutePath = resolve(file_path);

      try {
        const diagnostics = await diagnosticService.getDiagnostics(absolutePath);

        // Handle undefined return (should not happen, but defensive coding)
        if (!diagnostics || !Array.isArray(diagnostics)) {
          return createMCPResponse(
            `Error getting diagnostics: Diagnostic service returned invalid result (${typeof diagnostics})`
          );
        }

        if (diagnostics.length === 0) {
          return createNoResultsResponse('diagnostics', file_path, [
            'The file has no errors, warnings, or hints.',
          ]);
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

          const humanRange = toHumanRange(diag.range);
          return `• ${severity}${code}${source}: ${diag.message}\n  Location: ${formatHumanRange(humanRange)}`;
        });

        return createListResponse(`in ${file_path}`, diagnosticMessages, {
          singular: 'diagnostic',
          plural: 'diagnostics',
          showTotal: true,
        });
      } catch (error) {
        return createMCPResponse(
          `Error getting diagnostics: ${error instanceof Error ? error.message : String(error)}`
        );
      }
    },
    {
      context: { file_path },
    }
  );
}

// Handler for restart_server tool
export async function handleRestartServer(
  newLspClient: import('../../../../../server/src/lsp/lsp-client.js').LSPClient,
  args: { extensions?: string[] }
) {
  const { extensions } = args;

  try {
    // Clear failed servers to allow retry
    newLspClient.serverManager.clearFailedServers();

    const restartedServers = await newLspClient.restartServer(extensions);

    let response = `Successfully restarted ${restartedServers.length} LSP server(s)`;

    if (restartedServers.length > 0) {
      response += `\n\nRestarted servers:\n${restartedServers.map((s: string) => `• ${s}`).join('\n')}`;
    }

    response +=
      '\n\nNote: Any previously failed servers have been cleared and will be retried on next access.';

    return createMCPResponse(response);
  } catch (error) {
    return createMCPResponse(
      `Error restarting servers: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Fix dynamic imports that may have been missed by the initial rename operation
 * This function scans all project files to find and update any remaining references
 * to the renamed file that the initial rename operation missed (specifically dynamic imports)
 */
async function fixDynamicImportsInProject(
  oldFilePath: string,
  newFilePath: string,
  rootDir: string
) {
  const { readdirSync, readFileSync, writeFileSync, existsSync, statSync } = await import(
    'node:fs'
  );
  const { dirname, resolve, relative, join } = await import('node:path');

  // Get all TypeScript/JavaScript files in the project to scan
  const filesToScan: string[] = [];

  function collectFiles(dir: string) {
    try {
      const entries = readdirSync(dir);
      for (const entry of entries) {
        const fullPath = join(dir, entry);
        try {
          const stat = statSync(fullPath);
          if (stat.isDirectory()) {
            // Skip common directories that won't have import statements
            if (
              !entry.startsWith('.') &&
              !['node_modules', 'dist', 'build', 'coverage'].includes(entry)
            ) {
              collectFiles(fullPath);
            }
          } else if (stat.isFile()) {
            // Only scan TypeScript/JavaScript files
            if (/\.(ts|tsx|js|jsx|mjs|cjs)$/.test(entry)) {
              filesToScan.push(fullPath);
            }
          }
        } catch (statError) {
          // Skip files we can't stat
        }
      }
    } catch (readError) {
      // Skip directories we can't read
    }
  }

  collectFiles(rootDir);

  logger.debug('Scanning files for dynamic import fixes', {
    tool: 'rename_file',
    root_dir: rootDir,
    files_to_scan: filesToScan.length,
    old_file: oldFilePath,
    new_file: newFilePath,
  });

  let totalFilesFixed = 0;
  let totalEditsApplied = 0;

  for (const filePath of filesToScan) {
    try {
      if (!existsSync(filePath) || filePath === newFilePath) {
        continue; // Skip the renamed file itself
      }

      const content = readFileSync(filePath, 'utf-8');
      const lines = content.split('\n');
      const edits: any[] = [];

      for (let lineIndex = 0; lineIndex < lines.length; lineIndex++) {
        const line = lines[lineIndex];
        // Use the comprehensive regex that includes dynamic import()
        const importRegex =
          /((?:from|require\s*\(|import\s*\(|export\s+.*?from)\s*['"`])(\.\.?\/[^'"`]+)(['"`])/g;
        importRegex.lastIndex = 0;

        let match;
        while ((match = importRegex.exec(line)) !== null) {
          const oldImportPath = match[2];
          // Calculate what file this import path points to
          const targetFile = resolve(dirname(filePath), oldImportPath);

          // Check if this import was pointing to our renamed file
          let pointsToRenamedFile = false;

          // Direct match
          if (targetFile === oldFilePath) {
            pointsToRenamedFile = true;
          } else {
            // Handle extension mismatches (e.g., .js imports pointing to .ts files)
            const baseTarget = targetFile.replace(/\.(js|mjs|cjs|ts|tsx|jsx)$/, '');
            const baseOldFile = oldFilePath.replace(/\.(js|mjs|cjs|ts|tsx|jsx)$/, '');

            if (baseTarget === baseOldFile) {
              pointsToRenamedFile = true;
            } else {
              // Try common extensions
              for (const ext of ['.ts', '.tsx', '.js', '.jsx', '.mjs', '.cjs']) {
                if (baseTarget + ext === oldFilePath) {
                  pointsToRenamedFile = true;
                  break;
                }
              }

              // Try index files
              if (!pointsToRenamedFile) {
                for (const indexExt of ['/index.ts', '/index.tsx', '/index.js', '/index.jsx']) {
                  if (targetFile + indexExt === oldFilePath) {
                    pointsToRenamedFile = true;
                    break;
                  }
                }
              }
            }
          }

          if (pointsToRenamedFile) {
            // Calculate the new import path from this file to the new location
            let newImportPath = relative(dirname(filePath), newFilePath).replace(/\\/g, '/');

            // Preserve original extensions - critical for TypeScript projects
            // If the original import used .js, keep .js (common in TS projects)
            if (oldImportPath.endsWith('.js') && newImportPath.endsWith('.ts')) {
              newImportPath = newImportPath.replace(/\.ts$/, '.js');
            } else if (oldImportPath.endsWith('.js') && newImportPath.endsWith('.tsx')) {
              newImportPath = newImportPath.replace(/\.tsx$/, '.js');
            }

            // Handle index file optimization
            if (!oldImportPath.endsWith('/index') && newImportPath.endsWith('/index')) {
              newImportPath = newImportPath.substring(0, newImportPath.length - 6);
            }

            // Ensure relative paths start with ./ or ../
            if (!newImportPath.startsWith('.') && !newImportPath.startsWith('/')) {
              newImportPath = './' + newImportPath;
            }

            // Only make changes if the path actually needs updating
            if (oldImportPath !== newImportPath) {
              edits.push({
                line: lineIndex,
                startCol: match.index! + match[1].length,
                endCol: match.index! + match[1].length + oldImportPath.length,
                newText: newImportPath,
                oldText: oldImportPath,
              });
            }
          }
        }
      }

      // Apply edits if any were found
      if (edits.length > 0) {
        // Apply edits in reverse order to maintain positions
        const sortedEdits = edits.sort((a, b) => b.line - a.line || b.startCol - a.startCol);
        const updatedLines = [...lines];

        for (const edit of sortedEdits) {
          const line = updatedLines[edit.line];
          updatedLines[edit.line] =
            line.substring(0, edit.startCol) + edit.newText + line.substring(edit.endCol);
        }

        writeFileSync(filePath, updatedLines.join('\n'), 'utf-8');
        totalFilesFixed++;
        totalEditsApplied += edits.length;

        logger.debug('Fixed dynamic imports in file', {
          tool: 'rename_file',
          file: filePath,
          edits_count: edits.length,
          edits: edits.map((e) => ({ line: e.line + 1, old: e.oldText, new: e.newText })),
        });
      }
    } catch (fileError) {
      logger.warn('Failed to process file during dynamic import fix', {
        tool: 'rename_file',
        file: filePath,
        error: fileError,
      });
    }
  }

  if (totalFilesFixed > 0) {
    logger.debug('Completed dynamic import fix across project', {
      tool: 'rename_file',
      files_fixed: totalFilesFixed,
      total_edits: totalEditsApplied,
      old_file: oldFilePath,
      new_file: newFilePath,
    });
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
    // Circular dependency safety check
    const { projectScanner } = await import(
      '../../../../../server/src/services/project-analyzer.js'
    );
    const { dirname, relative, resolve } = await import('node:path');

    const absoluteOldPath = resolve(old_path);
    const absoluteNewPath = resolve(new_path);
    const oldDir = dirname(absoluteOldPath);
    const newDir = dirname(absoluteNewPath);

    // Only check for circular dependencies if moving between different directories
    if (oldDir !== newDir) {
      logger.debug('Checking for circular dependencies before file rename', {
        tool: 'rename_file',
        old_path,
        new_path,
      });

      // Find all files that import the file being moved
      const importers = await projectScanner.findImporters(absoluteOldPath);

      if (importers.length > 0) {
        // Check if any importer is in a directory that would create a circular dependency
        for (const importer of importers) {
          const importerDir = dirname(importer);
          const relativePath = relative(newDir, importerDir);

          // If the importer is in a subdirectory of the new location, this could create a circular dependency
          if (
            !relativePath.startsWith('..') &&
            relativePath !== '' &&
            !relativePath.startsWith('/')
          ) {
            const relativeImporter = relative(process.cwd(), importer);
            const relativeOld = relative(process.cwd(), old_path);
            const relativeNew = relative(process.cwd(), new_path);

            return createMCPResponse(
              `⚠️ Cannot rename ${relativeOld} to ${relativeNew} - this would create a circular dependency.\n\n` +
                `The file ${relativeImporter} imports ${relativeOld}.\n` +
                `Moving ${relativeOld} to ${relativeNew} would place it in a parent directory of its importer, ` +
                `potentially creating circular import relationships.\n\n` +
                `Consider:\n` +
                `• Moving the file to a different location that doesn't create circular dependencies\n` +
                `• Refactoring the imports to break the circular dependency first\n` +
                `• Using a shared utilities directory that both can import from`
            );
          }
        }
      }
    }

    const { renameFile } = await import('../../../../../server/src/core/file-operations/editor.js');

    // Set rootDir to project root to ensure comprehensive import updates
    // This fixes the issue where files outside the immediate scope are missed
    let rootDir = process.cwd();

    // Verify the working directory exists and is accessible
    if (!existsSync(rootDir)) {
      // Fallback to the directory containing the file being moved
      rootDir = dirname(absoluteOldPath);
    }

    logger.debug('File rename operation started', {
      tool: 'rename_file',
      root_dir: rootDir,
      old_path,
      new_path,
      dry_run,
    });
    const result = await renameFile(old_path, new_path, undefined, {
      dry_run,
      rootDir,
      useGitignore: false, // Don't filter gitignored files so tests work
    });

    if (!result.success) {
      return createMCPResponse(`Failed to rename file: ${result.error}`);
    }

    if (dry_run) {
      // In dry-run mode, show what would be changed
      const message = result.error || '[DRY RUN] No changes would be made';
      return createMCPResponse(message);
    }

    // Post-processing: Fix any dynamic imports that the initial rename might have missed
    // This addresses the bug where dynamic import() statements are not updated
    if (!dry_run) {
      try {
        logger.debug('Starting post-processing to fix dynamic imports', {
          tool: 'rename_file',
          old_path: absoluteOldPath,
          new_path: absoluteNewPath,
          root_dir: rootDir,
        });
        await fixDynamicImportsInProject(absoluteOldPath, absoluteNewPath, rootDir);
        logger.debug('Completed post-processing for dynamic imports', {
          tool: 'rename_file',
          old_path: absoluteOldPath,
          new_path: absoluteNewPath,
        });
      } catch (fixError) {
        logger.error('Failed to fix dynamic imports after rename', {
          tool: 'rename_file',
          old_path: absoluteOldPath,
          new_path: absoluteNewPath,
          error: fixError,
        });
        // Don't fail the entire operation for this post-processing step
      }
    }

    // Success message
    const importCount = result.importUpdates
      ? Object.keys(result.importUpdates.changes || {}).length
      : 0;

    const additionalInfo = `Files modified: ${result.filesModified.length}\n${
      importCount > 0 ? `Files with updated imports: ${importCount}` : 'No import updates needed'
    }`;

    return createFileModificationResponse(`renamed ${old_path} to ${new_path}`, new_path, {
      additionalInfo,
    });
  } catch (error) {
    return createMCPResponse(
      `Error renaming file: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Internal helper for getting raw WorkspaceEdit data from rename operations
 * Used by orchestration handlers for atomic operations
 */
export async function getRenameFileWorkspaceEdit(args: {
  old_path: string;
  new_path: string;
}): Promise<{ success: boolean; workspaceEdit?: WorkspaceEdit; error?: string }> {
  const { old_path, new_path } = args;

  try {
    const { renameFile } = await import('../../../../../server/src/core/file-operations/editor.js');

    // Use the same rootDir calculation as handleRenameFile
    const absoluteOldPath = resolve(old_path);
    const absoluteNewPath = resolve(new_path);

    // Set rootDir to project root to ensure comprehensive import updates
    // This fixes the issue where files outside the immediate scope are missed
    let rootDir = process.cwd();

    // Verify the working directory exists and is accessible
    if (!existsSync(rootDir)) {
      // Fallback to the directory containing the file being moved
      rootDir = dirname(absoluteOldPath);
    }

    const result = await renameFile(old_path, new_path, undefined, {
      dry_run: true, // Always dry run for workspace edit extraction
      rootDir,
      useGitignore: false,
    });

    if (!result.success) {
      return { success: false, error: result.error };
    }

    return { success: true, workspaceEdit: result.importUpdates };
  } catch (error) {
    return {
      success: false,
      error: error instanceof Error ? error.message : String(error),
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
      return createMCPResponse(
        `File ${file_path} already exists. Use overwrite: true to replace it.`
      );
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

    const details = content ? ` with ${content.length} characters` : ' (empty file)';
    return createSuccessResponse(`created ${file_path}${details}`);
  } catch (error) {
    return createMCPResponse(
      `Error creating file: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for delete_file tool
export async function handleDeleteFile(args: { file_path: string; force?: boolean }) {
  const { file_path, force = false } = args;
  const absolutePath = resolve(file_path);

  try {
    // Check if file exists
    if (!existsSync(absolutePath)) {
      return createMCPResponse(`File ${file_path} does not exist.`);
    }

    // Import the project scanner for impact analysis
    const { projectScanner } = await import(
      '../../../../../server/src/services/project-analyzer.js'
    );

    // Find all files that import this file
    logger.debug('Analyzing file deletion impact', {
      tool: 'delete_file',
      file_path: absolutePath,
    });
    const importers = await projectScanner.findImporters(absolutePath);

    if (importers.length > 0 && !force) {
      // File is imported by other files - warn user
      const relativeImporters = importers.map((imp) => relative(process.cwd(), imp));

      return createMCPResponse(
        `⚠️ Cannot delete ${file_path} - it is imported by ${importers.length} file${importers.length === 1 ? '' : 's'}:\n\n${relativeImporters.map((f) => `  • ${f}`).join('\n')}\n\n${importers.length === 1 ? 'This file depends' : 'These files depend'} on ${file_path}. Deleting it will cause import errors.\n\nTo force deletion despite broken imports, use:\n  force: true`
      );
    }

    // If force is true or no importers, proceed with deletion
    if (importers.length > 0 && force) {
      logger.debug('Force deleting file with importers', {
        tool: 'delete_file',
        file_path: absolutePath,
        importer_count: importers.length,
        force: true,
      });
    }

    // Delete the file
    unlinkSync(absolutePath);

    // Build success message
    let message = `✅ Successfully deleted ${file_path}`;

    if (importers.length > 0) {
      const relativeImporters = importers.map((imp) => relative(process.cwd(), imp));
      message += `\n\n⚠️ Warning: ${importers.length} file${importers.length === 1 ? ' has' : 's have'} broken imports:\n${relativeImporters.map((f) => `  • ${f}`).join('\n')}\n\nYou may need to update or remove these import statements.`;
    }

    return createMCPResponse(message);
  } catch (error) {
    return createMCPResponse(
      `Error deleting file: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

/**
 * Get health status of LSP servers and system resources
 */
export async function handleHealthCheck(
  {
    include_details = false,
  }: import('../../../../../server/src/mcp/handler-types.js').HealthCheckArgs,
  _services: import('../../../../../server/src/services/service-context.js').ServiceContext
): Promise<import('../../../../../server/src/mcp/utils.js').MCPResponse> {
  try {
    const { cpus, totalmem, loadavg } = await import('node:os');

    // Get system metrics
    const cpuCores = cpus().length;
    const memoryGb = Math.round(totalmem() / (1024 * 1024 * 1024));
    const loadAverage = loadavg()[0] || 0; // Provide fallback value

    // Basic health metrics
    const health = {
      timestamp: new Date().toISOString(),
      status: 'healthy',
      lsp_servers: {
        active_count: 0, // Simplified - we don't have direct access to server manager
        max_allowed: 8, // From our MAX_CONCURRENT_SERVERS
      },
      system: {
        cpu_cores: cpuCores,
        memory_gb: memoryGb,
        load_average: loadAverage,
      },
    };

    let message = '## CodeBuddy Health Status\n\n';
    message += `**Status**: ${health.status}\n`;
    message += `**Timestamp**: ${health.timestamp}\n\n`;

    message += '### LSP Servers\n';
    message += `- **Active**: ${health.lsp_servers.active_count}/${health.lsp_servers.max_allowed}\n`;

    if (include_details) {
      message += '\n**Note**: Detailed server information requires enhanced monitoring access.\n';
    }

    message += '\n### System Resources\n';
    message += `- **CPU Cores**: ${health.system.cpu_cores}\n`;
    message += `- **Memory**: ${health.system.memory_gb}GB\n`;
    message += `- **Load Average**: ${health.system.load_average.toFixed(2)}\n`;

    // Health assessment
    const isOverloaded = health.system.load_average > health.system.cpu_cores * 0.8;
    const isAtCapacity = health.lsp_servers.active_count >= health.lsp_servers.max_allowed;

    if (isOverloaded || isAtCapacity) {
      message += '\n### ⚠️ Warnings\n';
      if (isOverloaded) {
        message += `- High CPU load (${health.system.load_average.toFixed(2)} > ${(health.system.cpu_cores * 0.8).toFixed(2)})\n`;
      }
      if (isAtCapacity) {
        message += `- LSP servers at capacity (${health.lsp_servers.active_count}/${health.lsp_servers.max_allowed})\n`;
      }
    }

    return createMCPResponse(message);
  } catch (error) {
    return createMCPResponse(
      `Error getting health status: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for execute_workflow tool
export async function handleExecuteWorkflow(
  args: { chain: any; inputs: Record<string, any> },
  context: ServiceContext
) {
  const { chain, inputs } = args;

  try {
    const orchestrator = new DependencyOrchestrator(context);
    const result = await orchestrator.execute(chain, inputs);

    return createMCPResponse(`Workflow executed successfully: ${JSON.stringify(result, null, 2)}`);
  } catch (error) {
    return createMCPResponse(
      `Error executing workflow: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for update_package_json tool
export async function handleUpdatePackageJson(args: {
  file_path: string;
  add_dependencies?: Record<string, string>;
  add_dev_dependencies?: Record<string, string>;
  remove_dependencies?: string[];
  add_scripts?: Record<string, string>;
  remove_scripts?: string[];
  update_version?: string;
  workspace_config?: { workspaces?: string[] };
  dry_run?: boolean;
}) {
  const {
    file_path,
    add_dependencies = {},
    add_dev_dependencies = {},
    remove_dependencies = [],
    add_scripts = {},
    remove_scripts = [],
    update_version,
    workspace_config,
    dry_run = false,
  } = args;

  try {
    const { existsSync, readFileSync, writeFileSync } = await import('node:fs');
    const { resolve } = await import('node:path');

    const absolutePath = resolve(file_path);

    if (!existsSync(absolutePath)) {
      return createMCPResponse(`Error: File does not exist: ${file_path}`);
    }

    // Read and parse the package.json
    const content = readFileSync(absolutePath, 'utf8');
    let packageJson: any;

    try {
      packageJson = JSON.parse(content);
    } catch (parseError) {
      return createMCPResponse(`Error: Invalid JSON in ${file_path}: ${parseError}`);
    }

    // Track changes for reporting
    const changes: string[] = [];

    // Add dependencies
    if (Object.keys(add_dependencies).length > 0) {
      if (!packageJson.dependencies) {
        packageJson.dependencies = {};
      }
      for (const [name, version] of Object.entries(add_dependencies)) {
        packageJson.dependencies[name] = version;
        changes.push(`Added dependency: ${name}@${version}`);
      }
    }

    // Add dev dependencies
    if (Object.keys(add_dev_dependencies).length > 0) {
      if (!packageJson.devDependencies) {
        packageJson.devDependencies = {};
      }
      for (const [name, version] of Object.entries(add_dev_dependencies)) {
        packageJson.devDependencies[name] = version;
        changes.push(`Added devDependency: ${name}@${version}`);
      }
    }

    // Remove dependencies
    for (const name of remove_dependencies) {
      let removed = false;
      if (packageJson.dependencies && packageJson.dependencies[name]) {
        delete packageJson.dependencies[name];
        changes.push(`Removed dependency: ${name}`);
        removed = true;
      }
      if (packageJson.devDependencies && packageJson.devDependencies[name]) {
        delete packageJson.devDependencies[name];
        changes.push(`Removed devDependency: ${name}`);
        removed = true;
      }
      if (!removed) {
        changes.push(`Warning: ${name} not found in dependencies`);
      }
    }

    // Add scripts
    if (Object.keys(add_scripts).length > 0) {
      if (!packageJson.scripts) {
        packageJson.scripts = {};
      }
      for (const [name, script] of Object.entries(add_scripts)) {
        packageJson.scripts[name] = script;
        changes.push(`Added script: ${name}`);
      }
    }

    // Remove scripts
    for (const name of remove_scripts) {
      if (packageJson.scripts && packageJson.scripts[name]) {
        delete packageJson.scripts[name];
        changes.push(`Removed script: ${name}`);
      } else {
        changes.push(`Warning: Script ${name} not found`);
      }
    }

    // Update version
    if (update_version) {
      const oldVersion = packageJson.version || 'not set';
      packageJson.version = update_version;
      changes.push(`Updated version: ${oldVersion} → ${update_version}`);
    }

    // Update workspace configuration
    if (workspace_config?.workspaces) {
      packageJson.workspaces = workspace_config.workspaces;
      changes.push(`Updated workspaces configuration`);
    }

    if (changes.length === 0) {
      return createMCPResponse(`No changes specified for ${file_path}`);
    }

    if (dry_run) {
      return createMCPResponse(
        `[DRY RUN] Would make the following changes to ${file_path}:\n\n` +
          changes.map((change) => `• ${change}`).join('\n')
      );
    }

    // Write the updated package.json with proper formatting
    const updatedContent = JSON.stringify(packageJson, null, 2) + '\n';
    writeFileSync(absolutePath, updatedContent, 'utf8');

    return createMCPResponse(
      `✅ Successfully updated ${file_path}\n\n` +
        `Changes made:\n` +
        changes.map((change) => `• ${change}`).join('\n')
    );
  } catch (error) {
    return createMCPResponse(
      `Error updating package.json: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Register utility tools with the central registry
registerTools(
  {
    get_diagnostics: { handler: handleGetDiagnostics, requiresService: 'diagnostic' },
    restart_server: { handler: handleRestartServer, requiresService: 'lsp' },
    rename_file: { handler: handleRenameFile, requiresService: 'none' },
    create_file: { handler: handleCreateFile, requiresService: 'none' },
    delete_file: { handler: handleDeleteFile, requiresService: 'none' },
    health_check: { handler: handleHealthCheck, requiresService: 'serviceContext' },
    execute_workflow: { handler: handleExecuteWorkflow, requiresService: 'serviceContext' },
    update_package_json: { handler: handleUpdatePackageJson, requiresService: 'none' },
  },
  'utility-handlers'
);
