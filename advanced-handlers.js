// src/mcp/handlers/advanced-handlers.ts
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
function pathToUri(filePath) {
  return pathToFileURL(filePath).toString();
}
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

// src/mcp/utils.ts
function createMCPResponse(text) {
  return {
    content: [
      {
        type: "text",
        text
      }
    ]
  };
}
function createLimitedSupportResponse(featureName, serverDescription, warningMessage, result) {
  let text = `⚠️ **${featureName}** - Limited support on ${serverDescription}

`;
  text += `**Warning:** ${warningMessage}

`;
  if (result) {
    text += `**Result:**
${result}`;
  } else {
    text += "**Result:** Feature attempted but may not work as expected on this server.";
  }
  return createMCPResponse(text);
}

// src/mcp/handlers/advanced-handlers.ts
async function handleGetCodeActions(fileService, args) {
  const { file_path, range } = args;
  const absolutePath = resolve(file_path);
  try {
    const codeActions = await fileService.getCodeActions(absolutePath, range);
    if (codeActions.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: `No code actions available for ${file_path}${range ? ` at lines ${range.start.line + 1}-${range.end.line + 1}` : ""}.`
          }
        ]
      };
    }
    const actionDescriptions = codeActions.filter((action) => action && (action.title || action.kind)).map((action, index) => {
      if (action.title) {
        return `${index + 1}. ${action.title}${action.kind ? ` (${action.kind})` : ""}`;
      }
      return `${index + 1}. Code action (${action.kind || "unknown"})`;
    });
    return {
      content: [
        {
          type: "text",
          text: `Found ${codeActions.length} code action${codeActions.length === 1 ? "" : "s"} for ${file_path}:

${actionDescriptions.join(`
`)}

Note: These actions show what's available but cannot be applied directly through this tool. Use your editor's code action functionality to apply them.`
        }
      ]
    };
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Error getting code actions: ${error instanceof Error ? error.message : String(error)}`
        }
      ]
    };
  }
}
async function handleFormatDocument(fileService, args) {
  const { file_path, options } = args;
  const absolutePath = resolve(file_path);
  try {
    const lspOptions = options ? {
      tabSize: options.tab_size,
      insertSpaces: options.insert_spaces,
      trimTrailingWhitespace: options.trim_trailing_whitespace,
      insertFinalNewline: options.insert_final_newline,
      trimFinalNewlines: options.trim_final_newlines
    } : undefined;
    const formatEdits = await fileService.formatDocument(absolutePath, lspOptions);
    if (formatEdits.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: `No formatting changes needed for ${file_path}. The file is already properly formatted.`
          }
        ]
      };
    }
    const workspaceEdit = {
      changes: {
        [pathToUri(absolutePath)]: formatEdits
      }
    };
    const editResult = await applyWorkspaceEdit(workspaceEdit);
    if (!editResult.success) {
      return {
        content: [
          {
            type: "text",
            text: `Failed to apply formatting: ${editResult.error}`
          }
        ]
      };
    }
    return {
      content: [
        {
          type: "text",
          text: `✅ Successfully formatted ${file_path} with ${formatEdits.length} change${formatEdits.length === 1 ? "" : "s"}.`
        }
      ]
    };
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Error formatting document: ${error instanceof Error ? error.message : String(error)}`
        }
      ]
    };
  }
}
async function handleSearchWorkspaceSymbols(symbolService, args, lspClient) {
  const { query } = args;
  try {
    const symbols = await symbolService.searchWorkspaceSymbols(query, lspClient.serverManager.activeServers, lspClient.preloadServers.bind(lspClient));
    if (symbols.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: `No symbols found matching "${query}". Try a different search term or ensure the language server is properly configured.`
          }
        ]
      };
    }
    const symbolDescriptions = symbols.slice(0, 50).map((symbol, index) => {
      const location = symbol.location;
      const filePath = uriToPath(location.uri);
      const line = location.range.start.line + 1;
      const character = location.range.start.character + 1;
      const symbolKind = symbol.kind ? String(symbol.kind) : "unknown";
      return `${index + 1}. ${symbol.name} (${symbolKind}) - ${filePath}:${line}:${character}`;
    });
    const resultText = symbols.length > 50 ? `Found ${symbols.length} symbols matching "${query}" (showing first 50):

${symbolDescriptions.join(`
`)}` : `Found ${symbols.length} symbol${symbols.length === 1 ? "" : "s"} matching "${query}":

${symbolDescriptions.join(`
`)}`;
    return {
      content: [
        {
          type: "text",
          text: resultText
        }
      ]
    };
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Error searching workspace symbols: ${error instanceof Error ? error.message : String(error)}`
        }
      ]
    };
  }
}
async function handleGetDocumentSymbols(symbolService, args) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);
  try {
    const symbols = await symbolService.getDocumentSymbols(absolutePath);
    if (symbols.length === 0) {
      return {
        content: [
          {
            type: "text",
            text: `No symbols found in ${file_path}. The file may be empty or the language server may not support this file type.`
          }
        ]
      };
    }
    const isHierarchical = symbolService.isDocumentSymbolArray(symbols);
    let symbolDescriptions;
    if (isHierarchical) {
      const formatDocumentSymbol = (symbol, indent = 0) => {
        const prefix = "  ".repeat(indent);
        const line = symbol.range.start.line + 1;
        const character = symbol.range.start.character + 1;
        const symbolKind = symbolService.symbolKindToString(symbol.kind);
        const result = [`${prefix}${symbol.name} (${symbolKind}) - Line ${line}:${character}`];
        if (symbol.children && symbol.children.length > 0) {
          for (const child of symbol.children) {
            result.push(...formatDocumentSymbol(child, indent + 1));
          }
        }
        return result;
      };
      symbolDescriptions = [];
      for (const symbol of symbols) {
        symbolDescriptions.push(...formatDocumentSymbol(symbol));
      }
    } else {
      symbolDescriptions = symbols.map((symbol, index) => {
        const line = symbol.location.range.start.line + 1;
        const character = symbol.location.range.start.character + 1;
        const symbolKind = symbol.kind ? symbolService.symbolKindToString(symbol.kind) : "unknown";
        return `${index + 1}. ${symbol.name} (${symbolKind}) - Line ${line}:${character}`;
      });
    }
    return {
      content: [
        {
          type: "text",
          text: `Document outline for ${file_path}:

${symbolDescriptions.join(`
`)}`
        }
      ]
    };
  } catch (error) {
    return {
      content: [
        {
          type: "text",
          text: `Error getting document symbols: ${error instanceof Error ? error.message : String(error)}`
        }
      ]
    };
  }
}
async function handleGetFoldingRanges(fileService, args, lspClient) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);
  try {
    const foldingRanges = await fileService.getFoldingRanges(absolutePath);
    if (foldingRanges.length === 0) {
      return createMCPResponse(`No folding ranges found in ${file_path}. The file may not have collapsible code blocks.`);
    }
    const rangeDescriptions = foldingRanges.map((range, index) => {
      const startLine = range.startLine + 1;
      const endLine = range.endLine + 1;
      const kind = range.kind || "code";
      const characterInfo = range.startCharacter !== undefined && range.endCharacter !== undefined ? ` (chars ${range.startCharacter}-${range.endCharacter})` : "";
      return `${index + 1}. **${kind}** block: Lines ${startLine}-${endLine}${characterInfo}${range.collapsedText ? ` ("${range.collapsedText}")` : ""}`;
    });
    const kindCount = foldingRanges.reduce((acc, range) => {
      const kind = range.kind || "code";
      acc[kind] = (acc[kind] || 0) + 1;
      return acc;
    }, {});
    const kindSummary = Object.entries(kindCount).map(([kind, count]) => `${count} ${kind}`).join(", ");
    const response = `## Folding Ranges for ${file_path}

**Found ${foldingRanges.length} foldable regions:** ${kindSummary}

${rangeDescriptions.join(`
`)}

*Folding ranges show logical code blocks that can be collapsed for better code navigation and understanding.*`;
    return createMCPResponse(response);
  } catch (error) {
    if (error instanceof Error && error.message.includes("not supported")) {
      const serverInfo = "Current Language Server";
      return createLimitedSupportResponse("Folding Ranges", "Current Language Server", "Server may not fully support folding ranges or the file has no collapsible regions", `Server capabilities: ${serverInfo}`);
    }
    return createMCPResponse(`Error getting folding ranges: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetDocumentLinks(fileService, args, lspClient) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);
  try {
    const documentLinks = await fileService.getDocumentLinks(absolutePath);
    if (documentLinks.length === 0) {
      return createMCPResponse(`No document links found in ${file_path}. The file may not contain URLs, imports, or other linkable references.`);
    }
    const linkDescriptions = documentLinks.map((link, index) => {
      const startLine = link.range.start.line + 1;
      const startChar = link.range.start.character + 1;
      const endLine = link.range.end.line + 1;
      const endChar = link.range.end.character + 1;
      let description = `${index + 1}. **Link** at Line ${startLine}:${startChar}`;
      if (startLine !== endLine || startChar !== endChar) {
        description += ` to ${endLine}:${endChar}`;
      }
      if (link.target) {
        description += `
   Target: ${link.target}`;
      }
      if (link.tooltip) {
        description += `
   Info: ${link.tooltip}`;
      }
      return description;
    });
    const categories = {
      urls: documentLinks.filter((link) => link.target?.startsWith("http")),
      files: documentLinks.filter((link) => link.target?.startsWith("file:")),
      packages: documentLinks.filter((link) => link.target?.includes("pkg.go.dev") || link.target?.includes("docs.rs") || link.target?.includes("npmjs.com")),
      other: documentLinks.filter((link) => link.target && !link.target.startsWith("http") && !link.target.startsWith("file:"))
    };
    let categorySummary = "";
    if (categories.urls.length > 0)
      categorySummary += `${categories.urls.length} URLs, `;
    if (categories.files.length > 0)
      categorySummary += `${categories.files.length} files, `;
    if (categories.packages.length > 0)
      categorySummary += `${categories.packages.length} packages, `;
    if (categories.other.length > 0)
      categorySummary += `${categories.other.length} other links, `;
    categorySummary = categorySummary.replace(/, $/, "");
    const response = `## Document Links for ${file_path}

**Found ${documentLinks.length} links:** ${categorySummary}

${linkDescriptions.join(`

`)}

*Document links help navigate between related files, external documentation, and web resources. Different language servers provide different types of links.*`;
    return createMCPResponse(response);
  } catch (error) {
    if (error instanceof Error && error.message.includes("not supported")) {
      const serverInfo = "Current Language Server";
      return createLimitedSupportResponse("Document Links", "Current Language Server", "Server may not fully support document links or the file contains no linkable content", `Server capabilities: ${serverInfo}`);
    }
    return createMCPResponse(`Error getting document links: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleApplyWorkspaceEdit(fileService, args) {
  const { changes, validate_before_apply = true } = args;
  try {
    const workspaceEdit = {
      changes: {}
    };
    for (const [filePath, edits] of Object.entries(changes)) {
      const uri = filePath.startsWith("file://") ? filePath : pathToUri(resolve(filePath));
      const textEdits = edits.map((edit) => ({
        range: edit.range,
        newText: edit.newText
      }));
      if (!workspaceEdit.changes) {
        workspaceEdit.changes = {};
      }
      workspaceEdit.changes[uri] = textEdits;
    }
    if (!workspaceEdit.changes || Object.keys(workspaceEdit.changes).length === 0) {
      return createMCPResponse("No changes provided. Please specify at least one file with edits to apply.");
    }
    const fileCount = Object.keys(workspaceEdit.changes).length;
    const editCount = Object.values(workspaceEdit.changes).reduce((sum, edits) => sum + edits.length, 0);
    const serverSupportsWorkspaceEdit = true;
    const serverDescription = "File-based workspace edit";
    const result = await fileService.applyWorkspaceEdit({
      changes: workspaceEdit.changes
    });
    if (!result.applied) {
      return createMCPResponse(`❌ **Workspace edit failed**

**Error:** ${result.failureReason || "Unknown error"}

**Files targeted:** ${fileCount}
**Total edits:** ${editCount}

*No changes were applied due to the error. All files remain unchanged.*`);
    }
    let response = `✅ **Workspace edit applied successfully**

`;
    const modifiedFiles = Object.keys(workspaceEdit.changes);
    response += `**Files modified:** ${modifiedFiles.length}
`;
    response += `**Total edits applied:** ${editCount}

`;
    if (modifiedFiles.length > 0) {
      response += `**Modified files:**
`;
      for (const file of modifiedFiles) {
        const filePath = file.startsWith("file://") ? uriToPath(file) : file;
        response += `• ${filePath}
`;
      }
    }
    if (!serverSupportsWorkspaceEdit) {
      response += `
⚠️ **Note:** ${serverDescription} doesn't fully support workspace edits, but changes were applied successfully using CCLSP's built-in editor.`;
    }
    response += `

*All changes were applied atomically. If any edit had failed, all changes would have been rolled back.*`;
    return createMCPResponse(response);
  } catch (error) {
    return createMCPResponse(`Error applying workspace edit: ${error instanceof Error ? error.message : String(error)}`);
  }
}
export {
  handleSearchWorkspaceSymbols,
  handleGetFoldingRanges,
  handleGetDocumentSymbols,
  handleGetDocumentLinks,
  handleGetCodeActions,
  handleFormatDocument,
  handleApplyWorkspaceEdit
};
