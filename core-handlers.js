// src/mcp/handlers/core-handlers.ts
import { resolve } from "node:path";

// src/file-editor.ts
import {
  existsSync,
  lstatSync,
  readFileSync,
  realpathSync,
  renameSync,
  statSync,
  unlinkSync,
  writeFileSync
} from "node:fs";

// src/path-utils.ts
import { fileURLToPath, pathToFileURL } from "node:url";
function uriToPath(uri) {
  return fileURLToPath(uri);
}

// src/file-editor.ts
async function applyWorkspaceEdit(workspaceEdit, options = {}) {
  const {
    validateBeforeApply = true,
    createBackupFiles = validateBeforeApply,
    lspClient
  } = options;
  const backups = [];
  const filesModified = [];
  const backupFilePaths = [];
  if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
    return {
      success: true,
      filesModified: [],
      backupFiles: []
    };
  }
  try {
    for (const [uri, edits] of Object.entries(workspaceEdit.changes)) {
      const filePath = uriToPath(uri);
      if (!existsSync(filePath)) {
        throw new Error(`File does not exist: ${filePath}`);
      }
      const stats = lstatSync(filePath);
      if (stats.isSymbolicLink()) {
        try {
          const realPath = realpathSync(filePath);
          const targetStats = statSync(realPath);
          if (!targetStats.isFile()) {
            throw new Error(`Symlink target is not a file: ${realPath}`);
          }
        } catch (error) {
          throw new Error(`Cannot resolve symlink ${filePath}: ${error}`);
        }
      } else if (!stats.isFile()) {
        throw new Error(`Not a file: ${filePath}`);
      }
      try {
        readFileSync(filePath, "utf-8");
      } catch (error) {
        throw new Error(`Cannot read file: ${filePath} - ${error}`);
      }
    }
    for (const [uri, edits] of Object.entries(workspaceEdit.changes)) {
      const originalPath = uriToPath(uri);
      let targetPath = originalPath;
      const originalStats = lstatSync(originalPath);
      if (originalStats.isSymbolicLink()) {
        targetPath = realpathSync(originalPath);
        process.stderr.write(`[DEBUG] Editing symlink target: ${targetPath} (via ${originalPath})
`);
      }
      const originalContent = readFileSync(targetPath, "utf-8");
      const backup = {
        originalPath,
        targetPath,
        originalContent
      };
      backups.push(backup);
      if (createBackupFiles) {
        const backupPath = `${originalPath}.bak`;
        writeFileSync(backupPath, originalContent, "utf-8");
        backupFilePaths.push(backupPath);
      }
      const modifiedContent = applyEditsToContent(originalContent, edits, validateBeforeApply);
      const tempPath = `${targetPath}.tmp-${process.pid}-${Date.now()}-${Math.random().toString(36).slice(2)}`;
      writeFileSync(tempPath, modifiedContent, "utf-8");
      try {
        renameSync(tempPath, targetPath);
      } catch (error) {
        try {
          if (existsSync(tempPath)) {
            unlinkSync(tempPath);
          }
        } catch {}
        throw error;
      }
      filesModified.push(originalPath);
      if (lspClient) {
        await lspClient.syncFileContent(originalPath);
      }
    }
    return {
      success: true,
      filesModified,
      backupFiles: backupFilePaths
    };
  } catch (error) {
    for (const backup of backups) {
      try {
        writeFileSync(backup.targetPath, backup.originalContent, "utf-8");
      } catch (rollbackError) {
        console.error(`Failed to rollback ${backup.targetPath}:`, rollbackError);
      }
    }
    return {
      success: false,
      filesModified: [],
      backupFiles: backupFilePaths,
      error: error instanceof Error ? error.message : String(error)
    };
  }
}
function applyEditsToContent(content, edits, validate) {
  const lineEnding = content.includes(`\r
`) ? `\r
` : `
`;
  const lines = content.split(/\r?\n/);
  const sortedEdits = [...edits].sort((a, b) => {
    if (a.range.start.line !== b.range.start.line) {
      return b.range.start.line - a.range.start.line;
    }
    return b.range.start.character - a.range.start.character;
  });
  for (const edit of sortedEdits) {
    const { start, end } = edit.range;
    if (validate) {
      if (start.line < 0 || start.line >= lines.length) {
        throw new Error(`Invalid start line ${start.line} (file has ${lines.length} lines)`);
      }
      if (end.line < 0 || end.line >= lines.length) {
        throw new Error(`Invalid end line ${end.line} (file has ${lines.length} lines)`);
      }
      if (start.line > end.line || start.line === end.line && start.character > end.character) {
        throw new Error(`Invalid range: start (${start.line}:${start.character}) is after end (${end.line}:${end.character})`);
      }
      const startLine = lines[start.line];
      if (startLine !== undefined) {
        if (start.character < 0 || start.character > startLine.length) {
          throw new Error(`Invalid start character ${start.character} on line ${start.line} (line has ${startLine.length} characters)`);
        }
      }
      const endLine = lines[end.line];
      if (endLine !== undefined) {
        if (end.character < 0 || end.character > endLine.length) {
          throw new Error(`Invalid end character ${end.character} on line ${end.line} (line has ${endLine.length} characters)`);
        }
      }
    }
    if (start.line === end.line) {
      const line = lines[start.line];
      if (line !== undefined) {
        lines[start.line] = line.substring(0, start.character) + edit.newText + line.substring(end.character);
      }
    } else {
      const startLine = lines[start.line];
      const endLine = lines[end.line];
      if (startLine !== undefined && endLine !== undefined) {
        const newLine = startLine.substring(0, start.character) + edit.newText + endLine.substring(end.character);
        lines.splice(start.line, end.line - start.line + 1, newLine);
      }
    }
  }
  return lines.join(lineEnding);
}

// src/mcp/handlers/core-handlers.ts
async function handleFindDefinition(symbolService, args) {
  const { file_path, symbol_name, symbol_kind } = args;
  const absolutePath = resolve(file_path);
  const symbolMatches = await symbolService.findSymbolMatches(absolutePath, symbol_name, symbol_kind);
  const warning = undefined;
  process.stderr.write(`[DEBUG find_definition] Found ${symbolMatches.length} symbol matches for "${symbol_name}"
`);
  if (symbolMatches.length === 0) {
    return {
      content: [
        {
          type: "text",
          text: `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ""} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`
        }
      ]
    };
  }
  const results = [];
  for (const match of symbolMatches) {
    process.stderr.write(`[DEBUG find_definition] Processing match: ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${match.position.line}:${match.position.character}
`);
    try {
      const locations = await symbolService.findDefinition(absolutePath, match.position);
      process.stderr.write(`[DEBUG find_definition] findDefinition returned ${locations.length} locations
`);
      if (locations.length > 0) {
        const locationResults = locations.map((loc) => {
          const filePath = uriToPath(loc.uri);
          const { start, end } = loc.range;
          return `${filePath}:${start.line + 1}:${start.character + 1}`;
        }).join(`
`);
        results.push(`Results for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}:
${locationResults}`);
      } else {
        results.push(`No definition found for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}`);
      }
    } catch (error) {
      results.push(`Error finding definition for ${match.name}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  if (results.length === 0) {
    const responseText2 = warning ? warning : "No definitions found for the specified symbol.";
    return {
      content: [
        {
          type: "text",
          text: responseText2
        }
      ]
    };
  }
  const responseText = warning ? `${warning}

${results.join(`

`)}` : results.join(`

`);
  return {
    content: [
      {
        type: "text",
        text: responseText
      }
    ]
  };
}
async function handleFindReferences(symbolService, args) {
  const { file_path, symbol_name, symbol_kind, include_declaration = true } = args;
  const absolutePath = resolve(file_path);
  const symbolMatches = await symbolService.findSymbolMatches(absolutePath, symbol_name, symbol_kind);
  const warning = undefined;
  process.stderr.write(`[DEBUG find_references] Found ${symbolMatches.length} symbol matches for "${symbol_name}"
`);
  if (symbolMatches.length === 0) {
    return {
      content: [
        {
          type: "text",
          text: `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ""} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`
        }
      ]
    };
  }
  const results = [];
  for (const match of symbolMatches) {
    process.stderr.write(`[DEBUG find_references] Processing match: ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${match.position.line}:${match.position.character}
`);
    try {
      const locations = await symbolService.findReferences(absolutePath, match.position, include_declaration);
      process.stderr.write(`[DEBUG find_references] findReferences returned ${locations.length} locations
`);
      if (locations.length > 0) {
        const locationResults = locations.map((loc) => {
          const filePath = uriToPath(loc.uri);
          const { start, end } = loc.range;
          return `${filePath}:${start.line + 1}:${start.character + 1}`;
        }).join(`
`);
        results.push(`References for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}:
${locationResults}`);
      } else {
        results.push(`No references found for ${match.name} (${symbolService.symbolKindToString(match.kind)}) at ${file_path}:${match.position.line + 1}:${match.position.character + 1}`);
      }
    } catch (error) {
      results.push(`Error finding references for ${match.name}: ${error instanceof Error ? error.message : String(error)}`);
    }
  }
  if (results.length === 0) {
    const responseText2 = warning ? `${warning}

No references found for the specified symbol.` : "No references found for the specified symbol.";
    return {
      content: [
        {
          type: "text",
          text: responseText2
        }
      ]
    };
  }
  const responseText = warning ? `${warning}

${results.join(`

`)}` : results.join(`

`);
  return {
    content: [
      {
        type: "text",
        text: responseText
      }
    ]
  };
}
async function handleRenameSymbol(symbolService, args) {
  const { file_path, symbol_name, symbol_kind, new_name, dry_run = false } = args;
  const absolutePath = resolve(file_path);
  const symbolMatches = await symbolService.findSymbolMatches(absolutePath, symbol_name, symbol_kind);
  const warning = undefined;
  if (symbolMatches.length === 0) {
    const responseText = warning ? `${warning}

No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ""} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.` : `No symbols found with name "${symbol_name}"${symbol_kind ? ` and kind "${symbol_kind}"` : ""} in ${file_path}. Please verify the symbol name and ensure the language server is properly configured.`;
    return {
      content: [
        {
          type: "text",
          text: responseText
        }
      ]
    };
  }
  if (symbolMatches.length > 1) {
    const matchDescriptions = symbolMatches.map((match2, index) => `${index + 1}. ${match2.name} (${symbolService.symbolKindToString(match2.kind)}) at line ${match2.position.line + 1}, character ${match2.position.character + 1}`).join(`
`);
    const responseText = warning ? `${warning}

Multiple symbols found with name "${symbol_name}". Please use rename_symbol_strict to specify which one to rename:

${matchDescriptions}` : `Multiple symbols found with name "${symbol_name}". Please use rename_symbol_strict to specify which one to rename:

${matchDescriptions}`;
    return {
      content: [
        {
          type: "text",
          text: responseText
        }
      ]
    };
  }
  const match = symbolMatches[0];
  if (!match) {
    throw new Error("Symbol match is undefined");
  }
  try {
    const workspaceEdit = await symbolService.renameSymbol(absolutePath, match.position, new_name, dry_run);
    if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
      return {
        content: [
          {
            type: "text",
            text: `No changes needed for renaming "${symbol_name}" to "${new_name}".`
          }
        ]
      };
    }
    const changedFileCount = Object.keys(workspaceEdit.changes).length;
    if (dry_run) {
      return {
        content: [
          {
            type: "text",
            text: `[DRY RUN] Would rename "${symbol_name}" to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? "" : "s"}`
          }
        ]
      };
    }
    const editResult = await applyWorkspaceEdit(workspaceEdit, {
      validateBeforeApply: true
    });
    if (!editResult.success) {
      return {
        content: [
          {
            type: "text",
            text: `Failed to rename symbol: ${editResult.error}`
          }
        ]
      };
    }
    return {
      content: [
        {
          type: "text",
          text: `✅ Successfully renamed "${symbol_name}" to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? "" : "s"}`
        }
      ]
    };
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Error renaming symbol: ${error instanceof Error ? error.message : String(error)}`
        }
      ]
    };
  }
}
async function handleRenameSymbolStrict(symbolService, args) {
  const { file_path, line, character, new_name, dry_run = false } = args;
  const absolutePath = resolve(file_path);
  const position = { line: line - 1, character: character - 1 };
  try {
    const workspaceEdit = await symbolService.renameSymbol(absolutePath, position, new_name, dry_run);
    if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
      return {
        content: [
          {
            type: "text",
            text: `No changes needed for renaming symbol at ${file_path}:${line}:${character} to "${new_name}".`
          }
        ]
      };
    }
    const changedFileCount = Object.keys(workspaceEdit.changes).length;
    if (dry_run) {
      return {
        content: [
          {
            type: "text",
            text: `[DRY RUN] Would rename symbol at ${file_path}:${line}:${character} to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? "" : "s"}`
          }
        ]
      };
    }
    const editResult = await applyWorkspaceEdit(workspaceEdit, {
      validateBeforeApply: true
    });
    if (!editResult.success) {
      return {
        content: [
          {
            type: "text",
            text: `Failed to rename symbol: ${editResult.error}`
          }
        ]
      };
    }
    return {
      content: [
        {
          type: "text",
          text: `✅ Successfully renamed symbol at ${file_path}:${line}:${character} to "${new_name}" across ${changedFileCount} file${changedFileCount === 1 ? "" : "s"}`
        }
      ]
    };
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Error renaming symbol: ${error instanceof Error ? error.message : String(error)}`
        }
      ]
    };
  }
}
export {
  handleRenameSymbolStrict,
  handleRenameSymbol,
  handleFindReferences,
  handleFindDefinition
};
