// src/mcp/handlers/intelligence-handlers.ts
import { resolve } from "node:path";

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

// src/mcp/handlers/intelligence-handlers.ts
async function handleGetHover(intelligenceService, args) {
  console.error("[DEBUG handleGetHover] Called with args:", args);
  const { file_path, line, character } = args;
  const absolutePath = resolve(file_path);
  console.error("[DEBUG handleGetHover] Resolved path:", absolutePath);
  try {
    console.error("[DEBUG handleGetHover] Calling intelligenceService.getHover");
    const hover = await intelligenceService.getHover(absolutePath, {
      line: line - 1,
      character
    });
    console.error("[DEBUG handleGetHover] Got hover result:", hover);
    if (!hover) {
      return createMCPResponse(`No hover information available for position ${line}:${character} in ${file_path}`);
    }
    let content = "";
    if (typeof hover.contents === "string") {
      content = hover.contents;
    } else if (Array.isArray(hover.contents)) {
      content = hover.contents.map((item) => {
        if (typeof item === "string")
          return item;
        if (typeof item === "object" && item && "language" in item && "value" in item) {
          const markedString = item;
          return `\`\`\`${markedString.language}
${markedString.value}
\`\`\``;
        }
        if (typeof item === "object" && item && "value" in item) {
          return item.value;
        }
        return String(item);
      }).join(`

`);
    } else if (hover.contents && typeof hover.contents === "object") {
      if ("value" in hover.contents) {
        content = hover.contents.value;
      }
    }
    const rangeInfo = hover.range ? ` (range: ${hover.range.start.line + 1}:${hover.range.start.character} - ${hover.range.end.line + 1}:${hover.range.end.character})` : "";
    return createMCPResponse(`## Hover Information for ${file_path}:${line}:${character}${rangeInfo}

${content}`);
  } catch (error) {
    return createMCPResponse(`Error getting hover information: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetCompletions(intelligenceService, args) {
  const { file_path, line, character, trigger_character } = args;
  const absolutePath = resolve(file_path);
  try {
    const completions = await intelligenceService.getCompletions(absolutePath, {
      line: line - 1,
      character
    }, trigger_character);
    if (completions.length === 0) {
      return createMCPResponse(`No completions available for position ${line}:${character} in ${file_path}`);
    }
    const sortedCompletions = completions.sort((a, b) => (a.sortText || a.label).localeCompare(b.sortText || b.label)).slice(0, 50);
    const completionItems = sortedCompletions.map((item, index) => {
      const kindName = getCompletionKindName(item.kind);
      const detail = item.detail ? ` - ${item.detail}` : "";
      const deprecated = item.deprecated || item.tags?.includes(1) ? " [DEPRECATED]" : "";
      const insertText = item.insertText && item.insertText !== item.label ? ` (inserts: "${item.insertText}")` : "";
      return `${index + 1}. **${item.label}** (${kindName})${detail}${deprecated}${insertText}`;
    });
    const triggerInfo = trigger_character ? ` (triggered by '${trigger_character}')` : "";
    return createMCPResponse(`## Code Completions for ${file_path}:${line}:${character}${triggerInfo}

Found ${completions.length} completion${completions.length === 1 ? "" : "s"}:

${completionItems.join(`
`)}`);
  } catch (error) {
    return createMCPResponse(`Error getting completions: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetInlayHints(intelligenceService, args) {
  const { file_path, start_line, start_character, end_line, end_character } = args;
  const absolutePath = resolve(file_path);
  try {
    const hints = await intelligenceService.getInlayHints(absolutePath, {
      start: {
        line: start_line - 1,
        character: start_character
      },
      end: {
        line: end_line - 1,
        character: end_character
      }
    });
    if (hints.length === 0) {
      return createMCPResponse(`No inlay hints available for range ${start_line}:${start_character} - ${end_line}:${end_character} in ${file_path}`);
    }
    const hintItems = hints.map((hint, index) => {
      const position = `${hint.position.line + 1}:${hint.position.character}`;
      const label = Array.isArray(hint.label) ? hint.label.map((part) => part.value).join("") : hint.label;
      const kindName = hint.kind === 1 ? "Type" : hint.kind === 2 ? "Parameter" : "Other";
      const tooltip = hint.tooltip ? ` (tooltip: ${typeof hint.tooltip === "string" ? hint.tooltip : hint.tooltip.value})` : "";
      return `${index + 1}. **${label}** at ${position} (${kindName})${tooltip}`;
    });
    return createMCPResponse(`## Inlay Hints for ${file_path} (${start_line}:${start_character} - ${end_line}:${end_character})

Found ${hints.length} hint${hints.length === 1 ? "" : "s"}:

${hintItems.join(`
`)}`);
  } catch (error) {
    return createMCPResponse(`Error getting inlay hints: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetSemanticTokens(intelligenceService, args) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);
  try {
    const tokens = await intelligenceService.getSemanticTokens(absolutePath);
    if (!tokens || !tokens.data || tokens.data.length === 0) {
      return createMCPResponse(`No semantic tokens available for ${file_path}`);
    }
    const tokenCount = tokens.data.length / 5;
    const resultId = tokens.resultId ? ` (result ID: ${tokens.resultId})` : "";
    const exampleTokens = [];
    let currentLine = 0;
    let currentChar = 0;
    for (let i = 0;i < Math.min(10, tokenCount); i++) {
      const offset = i * 5;
      const deltaLine = tokens.data[offset] || 0;
      const deltaChar = tokens.data[offset + 1] || 0;
      const length = tokens.data[offset + 2] || 0;
      const tokenType = tokens.data[offset + 3];
      const tokenModifiers = tokens.data[offset + 4];
      currentLine += deltaLine;
      if (deltaLine === 0) {
        currentChar += deltaChar;
      } else {
        currentChar = deltaChar;
      }
      exampleTokens.push(`  Token ${i + 1}: Line ${currentLine + 1}, Col ${currentChar + 1}, Length ${length}, Type ${tokenType}, Modifiers ${tokenModifiers}`);
    }
    return createMCPResponse(`## Semantic Tokens for ${file_path}${resultId}

Found ${tokenCount} semantic tokens.

First ${Math.min(10, tokenCount)} tokens:
${exampleTokens.join(`
`)}

*Note: Semantic tokens provide detailed syntax and semantic information for enhanced code understanding and highlighting.*`);
  } catch (error) {
    return createMCPResponse(`Error getting semantic tokens: ${error instanceof Error ? error.message : String(error)}`);
  }
}
function getCompletionKindName(kind) {
  const kindMap = {
    1: "Text",
    2: "Method",
    3: "Function",
    4: "Constructor",
    5: "Field",
    6: "Variable",
    7: "Class",
    8: "Interface",
    9: "Module",
    10: "Property",
    11: "Unit",
    12: "Value",
    13: "Enum",
    14: "Keyword",
    15: "Snippet",
    16: "Color",
    17: "File",
    18: "Reference",
    19: "Folder",
    20: "EnumMember",
    21: "Constant",
    22: "Struct",
    23: "Event",
    24: "Operator",
    25: "TypeParameter"
  };
  return kind !== undefined ? kindMap[kind] || `Unknown(${kind})` : "Unknown";
}
async function handleGetSignatureHelp(intelligenceService, args) {
  const { file_path, line, character, trigger_character } = args;
  const absolutePath = resolve(file_path);
  try {
    const signatureHelp = await intelligenceService.getSignatureHelp(absolutePath, {
      line: line - 1,
      character
    }, trigger_character);
    if (!signatureHelp || !signatureHelp.signatures || signatureHelp.signatures.length === 0) {
      return createMCPResponse(`No signature help available for position ${line}:${character} in ${file_path}`);
    }
    const signatures = signatureHelp.signatures;
    const activeSignature = signatureHelp.activeSignature ?? 0;
    const activeParameter = signatureHelp.activeParameter;
    let response = `## Function Signature Help for ${file_path}:${line}:${character}

`;
    if (signatures.length > 1) {
      response += `**${signatures.length} signatures available** (showing active signature):

`;
    }
    const signature = signatures[activeSignature] || signatures[0];
    if (!signature) {
      return createMCPResponse(`No valid signature available for position ${line}:${character} in ${file_path}`);
    }
    response += `**${signature.label}**

`;
    if (signature.documentation) {
      let doc = signature.documentation;
      if (typeof doc === "object" && doc.value) {
        doc = doc.value;
      }
      response += `${doc}

`;
    }
    if (signature.parameters && signature.parameters.length > 0) {
      response += `**Parameters:**
`;
      signature.parameters.forEach((param, index) => {
        const isActive = activeParameter !== undefined && index === activeParameter;
        const marker = isActive ? "\uD83D\uDC49 " : "   ";
        const emphasis = isActive ? "**" : "";
        let paramLabel = "";
        if (typeof param.label === "string") {
          paramLabel = param.label;
        } else if (Array.isArray(param.label)) {
          const [start, end] = param.label;
          paramLabel = signature.label.substring(start, end);
        }
        response += `${marker}${emphasis}${paramLabel}${emphasis}`;
        if (param.documentation) {
          let paramDoc = param.documentation;
          if (typeof paramDoc === "object" && paramDoc.value) {
            paramDoc = paramDoc.value;
          }
          response += ` - ${paramDoc}`;
        }
        response += `
`;
      });
    }
    if (signatures.length > 1) {
      response += `
**Other signatures:**
`;
      signatures.forEach((sig, index) => {
        if (index !== activeSignature) {
          response += `• ${sig.label}
`;
        }
      });
    }
    response += `
*Signature help shows function parameters and documentation for the function being called.*`;
    return createMCPResponse(response);
  } catch (error) {
    if (error instanceof Error && error.message.includes("not supported")) {
      return createLimitedSupportResponse("Signature Help", "Current Language Server", "Server may not fully support signature help or the position is not inside a function call", "Check server configuration for signature help support");
    }
    return createMCPResponse(`Error getting signature help: ${error instanceof Error ? error.message : String(error)}`);
  }
}
export {
  handleGetSignatureHelp,
  handleGetSemanticTokens,
  handleGetInlayHints,
  handleGetHover,
  handleGetCompletions
};
