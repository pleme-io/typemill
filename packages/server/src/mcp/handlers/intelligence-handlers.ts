// MCP handlers for LLM agent intelligence features
import { resolve } from 'node:path';
import type { IntelligenceService } from '../../services/intelligence/intelligence-service.js';
import {
  formatHumanPosition,
  formatHumanRange,
  toHumanPosition,
  toLSPPosition,
} from '../../../../@codeflow/core/src/utils/index.js';
import { registerTools } from '../tool-registry.js';
import {
  createContextualErrorResponse,
  createLimitedSupportResponse,
  createListResponse,
  createMCPResponse,
  createNoResultsResponse,
} from '../utils.js';

// Handler for get_hover tool
export async function handleGetHover(
  intelligenceService: IntelligenceService,
  args: { file_path: string; line: number; character: number }
) {
  const { file_path, line, character } = args;
  const absolutePath = resolve(file_path);

  try {
    const humanPos = { line, character };
    const lspPos = toLSPPosition(humanPos);
    const hover = await intelligenceService.getHover(absolutePath, lspPos);

    if (!hover) {
      return createNoResultsResponse(
        'hover information',
        `position ${line}:${character} in ${file_path}`
      );
    }

    let content = '';

    // Handle different content formats
    if (typeof hover.contents === 'string') {
      content = hover.contents;
    } else if (Array.isArray(hover.contents)) {
      content = hover.contents
        .map((item) => {
          if (typeof item === 'string') return item;
          if (typeof item === 'object' && item && 'language' in item && 'value' in item) {
            const markedString = item as { language: string; value: string };
            return `\`\`\`${markedString.language}\n${markedString.value}\n\`\`\``;
          }
          if (typeof item === 'object' && item && 'value' in item) {
            return (item as { value: string }).value;
          }
          return String(item);
        })
        .join('\n\n');
    } else if (hover.contents && typeof hover.contents === 'object') {
      if ('value' in hover.contents) {
        content = hover.contents.value;
      }
    }

    const rangeInfo = hover.range
      ? ` (range: ${formatHumanRange({ start: toHumanPosition(hover.range.start), end: toHumanPosition(hover.range.end) }, 'short')})`
      : '';

    return createMCPResponse(
      `## Hover Information for ${file_path}:${line}:${character}${rangeInfo}\n\n${content}`
    );
  } catch (error) {
    return createContextualErrorResponse(error, {
      operation: 'get hover information',
      filePath: file_path,
      suggestions: [
        'Ensure the file is supported by the language server',
        'Check that the position is valid',
        'Try a different position in the file',
      ],
    });
  }
}

// Handler for get_completions tool
export async function handleGetCompletions(
  intelligenceService: IntelligenceService,
  args: { file_path: string; line: number; character: number; trigger_character?: string }
) {
  const { file_path, line, character, trigger_character } = args;
  const absolutePath = resolve(file_path);

  try {
    const humanPos = { line, character };
    const lspPos = toLSPPosition(humanPos);
    const completions = await intelligenceService.getCompletions(
      absolutePath,
      lspPos,
      trigger_character
    );

    if (completions.length === 0) {
      return createNoResultsResponse(
        'completions',
        `position ${line}:${character} in ${file_path}`
      );
    }

    // Sort completions by sort text or label
    const sortedCompletions = completions
      .sort((a, b) => (a.sortText || a.label).localeCompare(b.sortText || b.label))
      .slice(0, 50); // Limit to top 50 completions

    const completionItems = sortedCompletions.map((item, index) => {
      const kindName = getCompletionKindName(item.kind);
      const detail = item.detail ? ` - ${item.detail}` : '';
      const deprecated = item.deprecated || item.tags?.includes(1) ? ' [DEPRECATED]' : '';
      const insertText =
        item.insertText && item.insertText !== item.label ? ` (inserts: "${item.insertText}")` : '';

      return `${index + 1}. **${item.label}** (${kindName})${detail}${deprecated}${insertText}`;
    });

    const triggerInfo = trigger_character ? ` (triggered by '${trigger_character}')` : '';
    const title = `Code Completions for ${file_path}:${line}:${character}${triggerInfo}`;

    return createListResponse(title, completionItems, {
      singular: 'completion',
      plural: 'completions',
      showTotal: true,
    });
  } catch (error) {
    return createContextualErrorResponse(error, {
      operation: 'get code completions',
      filePath: file_path,
      suggestions: [
        'Ensure the file is supported by the language server',
        'Check that the position is valid for completions',
        'Try a different position or trigger character',
      ],
    });
  }
}

// Handler for get_inlay_hints tool
export async function handleGetInlayHints(
  intelligenceService: IntelligenceService,
  args: {
    file_path: string;
    start_line: number;
    start_character: number;
    end_line: number;
    end_character: number;
  }
) {
  const { file_path, start_line, start_character, end_line, end_character } = args;
  const absolutePath = resolve(file_path);

  try {
    const hints = await intelligenceService.getInlayHints(absolutePath, {
      start: toLSPPosition({ line: start_line, character: start_character }),
      end: toLSPPosition({ line: end_line, character: end_character }),
    });

    if (hints.length === 0) {
      return createNoResultsResponse(
        'inlay hints',
        `range ${start_line}:${start_character} - ${end_line}:${end_character} in ${file_path}`
      );
    }

    const hintItems = hints.map((hint, index) => {
      const position = formatHumanPosition(toHumanPosition(hint.position), 'short');
      const label = Array.isArray(hint.label)
        ? hint.label.map((part) => part.value).join('')
        : hint.label;
      const kindName = hint.kind === 1 ? 'Type' : hint.kind === 2 ? 'Parameter' : 'Other';
      const tooltip = hint.tooltip
        ? ` (tooltip: ${typeof hint.tooltip === 'string' ? hint.tooltip : hint.tooltip.value})`
        : '';

      return `${index + 1}. **${label}** at ${position} (${kindName})${tooltip}`;
    });

    const title = `Inlay Hints for ${file_path} (${start_line}:${start_character} - ${end_line}:${end_character})`;
    return createListResponse(title, hintItems, {
      singular: 'hint',
      plural: 'hints',
      showTotal: true,
    });
  } catch (error) {
    return createContextualErrorResponse(error, {
      operation: 'get inlay hints',
      filePath: file_path,
      suggestions: [
        'Ensure the language server supports inlay hints',
        'Check that the range is valid',
        'Try a smaller range',
      ],
    });
  }
}

// Handler for get_semantic_tokens tool
export async function handleGetSemanticTokens(
  intelligenceService: IntelligenceService,
  args: { file_path: string }
) {
  const { file_path } = args;
  const absolutePath = resolve(file_path);

  try {
    const tokens = await intelligenceService.getSemanticTokens(absolutePath);

    if (!tokens || !tokens.data || tokens.data.length === 0) {
      return createNoResultsResponse('semantic tokens', file_path);
    }

    // Semantic tokens are encoded as a flat array of integers
    // Each token is 5 integers: deltaLine, deltaChar, length, tokenType, tokenModifiers
    const tokenCount = tokens.data.length / 5;
    const resultId = tokens.resultId ? ` (result ID: ${tokens.resultId})` : '';

    // Decode the first few tokens as examples
    const exampleTokens = [];
    let currentLine = 0;
    let currentChar = 0;

    for (let i = 0; i < Math.min(10, tokenCount); i++) {
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

      const humanPos = toHumanPosition({ line: currentLine, character: currentChar });
      exampleTokens.push(
        `  Token ${i + 1}: Line ${humanPos.line}, Col ${humanPos.character}, Length ${length}, Type ${tokenType}, Modifiers ${tokenModifiers}`
      );
    }

    return createMCPResponse(
      `## Semantic Tokens for ${file_path}${resultId}\n\nFound ${tokenCount} semantic tokens.\n\nFirst ${Math.min(10, tokenCount)} tokens:\n${exampleTokens.join('\n')}\n\n*Note: Semantic tokens provide detailed syntax and semantic information for enhanced code understanding and highlighting.*`
    );
  } catch (error) {
    return createContextualErrorResponse(error, {
      operation: 'get semantic tokens',
      filePath: file_path,
      suggestions: [
        'Ensure the language server supports semantic tokens',
        'Check that the file is not too large',
        'Try with a smaller file first',
      ],
    });
  }
}

// Helper function to get completion kind name
function getCompletionKindName(kind?: number): string {
  const kindMap: Record<number, string> = {
    1: 'Text',
    2: 'Method',
    3: 'Function',
    4: 'Constructor',
    5: 'Field',
    6: 'Variable',
    7: 'Class',
    8: 'Interface',
    9: 'Module',
    10: 'Property',
    11: 'Unit',
    12: 'Value',
    13: 'Enum',
    14: 'Keyword',
    15: 'Snippet',
    16: 'Color',
    17: 'File',
    18: 'Reference',
    19: 'Folder',
    20: 'EnumMember',
    21: 'Constant',
    22: 'Struct',
    23: 'Event',
    24: 'Operator',
    25: 'TypeParameter',
  };
  return kind !== undefined ? kindMap[kind] || `Unknown(${kind})` : 'Unknown';
}

// Handler for get_signature_help tool
export async function handleGetSignatureHelp(
  intelligenceService: IntelligenceService,
  args: { file_path: string; line: number; character: number; trigger_character?: string }
) {
  const { file_path, line, character, trigger_character } = args;
  const absolutePath = resolve(file_path);

  try {
    const humanPos = { line, character };
    const lspPos = toLSPPosition(humanPos);
    const signatureHelp = await intelligenceService.getSignatureHelp(
      absolutePath,
      lspPos,
      trigger_character
    );

    if (!signatureHelp || !signatureHelp.signatures || signatureHelp.signatures.length === 0) {
      return createNoResultsResponse(
        'signature help',
        `position ${line}:${character} in ${file_path}`
      );
    }

    const signatures = signatureHelp.signatures;
    const activeSignature = signatureHelp.activeSignature ?? 0;
    const activeParameter = signatureHelp.activeParameter;

    let response = `## Function Signature Help for ${file_path}:${line}:${character}\n\n`;

    if (signatures.length > 1) {
      response += `**${signatures.length} signatures available** (showing active signature):\n\n`;
    }

    // Show the active signature prominently
    const signature = signatures[activeSignature] || signatures[0];
    if (!signature) {
      return createNoResultsResponse(
        'valid signature',
        `position ${line}:${character} in ${file_path}`
      );
    }

    response += `**${signature.label}**\n\n`;

    if (signature.documentation) {
      let doc = signature.documentation;
      if (typeof doc === 'object' && doc.value) {
        doc = doc.value;
      }
      response += `${doc}\n\n`;
    }

    // Show parameters with active parameter highlighted
    if (signature.parameters && signature.parameters.length > 0) {
      response += '**Parameters:**\n';
      signature.parameters.forEach((param, index) => {
        const isActive = activeParameter !== undefined && index === activeParameter;
        const marker = isActive ? '👉 ' : '   ';
        const emphasis = isActive ? '**' : '';

        let paramLabel = '';
        if (typeof param.label === 'string') {
          paramLabel = param.label;
        } else if (Array.isArray(param.label)) {
          // Extract parameter name from label range
          const [start, end] = param.label;
          paramLabel = signature.label.substring(start, end);
        }

        response += `${marker}${emphasis}${paramLabel}${emphasis}`;

        if (param.documentation) {
          let paramDoc = param.documentation;
          if (typeof paramDoc === 'object' && paramDoc.value) {
            paramDoc = paramDoc.value;
          }
          response += ` - ${paramDoc}`;
        }
        response += '\n';
      });
    }

    // Show other signatures if available
    if (signatures.length > 1) {
      response += '\n**Other signatures:**\n';
      signatures.forEach((sig, index) => {
        if (index !== activeSignature) {
          response += `• ${sig.label}\n`;
        }
      });
    }

    response +=
      '\n*Signature help shows function parameters and documentation for the function being called.*';

    return createMCPResponse(response);
  } catch (error) {
    // Check if it's a capability-related error
    if (error instanceof Error && error.message.includes('not supported')) {
      return createLimitedSupportResponse(
        'Signature Help',
        'Current Language Server',
        'Server may not fully support signature help or the position is not inside a function call',
        'Check server configuration for signature help support'
      );
    }

    return createContextualErrorResponse(error, {
      operation: 'get signature help',
      filePath: file_path,
      suggestions: [
        'Ensure you are inside a function call',
        'Check that the language server supports signature help',
        'Try a different position in the function call',
      ],
    });
  }
}

// Register intelligence tools with the central registry
registerTools(
  {
    get_hover: { handler: handleGetHover, requiresService: 'intelligence' },
    get_completions: { handler: handleGetCompletions, requiresService: 'intelligence' },
    get_inlay_hints: { handler: handleGetInlayHints, requiresService: 'intelligence' },
    get_semantic_tokens: { handler: handleGetSemanticTokens, requiresService: 'intelligence' },
    get_signature_help: { handler: handleGetSignatureHelp, requiresService: 'intelligence' },
  },
  'intelligence-handlers'
);
