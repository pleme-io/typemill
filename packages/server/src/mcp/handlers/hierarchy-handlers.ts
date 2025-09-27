// MCP handlers for hierarchy and navigation features
import { resolve } from 'node:path';
import {
  formatHumanRange,
  toHumanPosition,
  toLSPPosition,
} from '../../../../@codeflow/core/src/utils/index.js';
import type { HierarchyService } from '../../services/intelligence/hierarchy-service.js';
import type { CallHierarchyItem, TypeHierarchyItem } from '../../types.js';
import { registerTools } from '../tool-registry.js';
import { createMCPResponse } from '../utils.js';

// Handler for prepare_call_hierarchy tool
export async function handlePrepareCallHierarchy(
  hierarchyService: HierarchyService,
  args: { file_path: string; line: number; character: number }
) {
  const { file_path, line, character } = args;
  const absolutePath = resolve(file_path);

  try {
    const humanPos = { line, character };
    const lspPos = toLSPPosition(humanPos);
    const items = await hierarchyService.prepareCallHierarchy(absolutePath, lspPos);

    if (items.length === 0) {
      return createMCPResponse(
        `No call hierarchy available for position ${line}:${character} in ${file_path}`
      );
    }

    const itemDescriptions = items.map((item, index) => {
      const kindName = getSymbolKindName(item.kind);
      const humanRange = formatHumanRange(
        { start: toHumanPosition(item.range.start), end: toHumanPosition(item.range.end) },
        'short'
      );
      const detail = item.detail ? ` - ${item.detail}` : '';

      return `${index + 1}. **${item.name}** (${kindName}) at ${humanRange}${detail}\n   URI: ${item.uri}`;
    });

    return createMCPResponse(
      `## Call Hierarchy Items for ${file_path}:${line}:${character}\n\nFound ${items.length} item${items.length === 1 ? '' : 's'}:\n\n${itemDescriptions.join('\n\n')}\n\n*Use these items with get_call_hierarchy_incoming_calls or get_call_hierarchy_outgoing_calls.*`
    );
  } catch (error) {
    return createMCPResponse(
      `Error preparing call hierarchy: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for get_call_hierarchy_incoming_calls tool
export async function handleGetCallHierarchyIncomingCalls(
  hierarchyService: HierarchyService,
  args: { item?: CallHierarchyItem; file_path?: string; line?: number; character?: number }
) {
  let item: CallHierarchyItem;

  // Support both API formats: direct item or file_path/line/character
  if (args.item) {
    item = args.item;
  } else if (args.file_path && args.line !== undefined && args.character !== undefined) {
    // First prepare call hierarchy to get the item
    const absolutePath = resolve(args.file_path);
    try {
      const humanPos = { line: args.line, character: args.character };
      const lspPos = toLSPPosition(humanPos);
      const items = await hierarchyService.prepareCallHierarchy(absolutePath, lspPos);

      if (items.length === 0 || !items[0]) {
        return createMCPResponse(
          `No call hierarchy item found at position ${args.line}:${args.character} in ${args.file_path}`
        );
      }

      item = items[0]; // Use the first item
    } catch (error) {
      return createMCPResponse(
        `Error preparing call hierarchy: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  } else {
    return createMCPResponse(
      'Invalid arguments: provide either "item" or "file_path", "line", and "character"'
    );
  }

  try {
    const incomingCalls = await hierarchyService.getCallHierarchyIncomingCalls(item);

    if (incomingCalls.length === 0) {
      return createMCPResponse(`No incoming calls found for ${item.name}`);
    }

    const callDescriptions = incomingCalls.map((call, index) => {
      const fromKind = getSymbolKindName(call.from.kind);
      const fromRange = formatHumanRange(
        {
          start: toHumanPosition(call.from.range.start),
          end: toHumanPosition(call.from.range.end),
        },
        'short'
      );
      const fromDetail = call.from.detail ? ` - ${call.from.detail}` : '';

      const ranges = call.fromRanges
        .map((range) =>
          formatHumanRange(
            { start: toHumanPosition(range.start), end: toHumanPosition(range.end) },
            'short'
          )
        )
        .join(', ');

      return `${index + 1}. From **${call.from.name}** (${fromKind}) at ${fromRange}${fromDetail}\n   Call sites: ${ranges}\n   URI: ${call.from.uri}`;
    });

    return createMCPResponse(
      `## Incoming Calls to ${item.name}\n\nFound ${incomingCalls.length} incoming call${incomingCalls.length === 1 ? '' : 's'}:\n\n${callDescriptions.join('\n\n')}`
    );
  } catch (error) {
    return createMCPResponse(
      `Error getting incoming calls: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Handler for get_call_hierarchy_outgoing_calls tool
export async function handleGetCallHierarchyOutgoingCalls(
  hierarchyService: HierarchyService,
  args: { item?: CallHierarchyItem; file_path?: string; line?: number; character?: number }
) {
  let item: CallHierarchyItem;

  // Support both API formats: direct item or file_path/line/character
  if (args.item) {
    item = args.item;
  } else if (args.file_path && args.line !== undefined && args.character !== undefined) {
    // First prepare call hierarchy to get the item
    const absolutePath = resolve(args.file_path);
    try {
      const humanPos = { line: args.line, character: args.character };
      const lspPos = toLSPPosition(humanPos);
      const items = await hierarchyService.prepareCallHierarchy(absolutePath, lspPos);

      if (items.length === 0 || !items[0]) {
        return createMCPResponse(
          `No call hierarchy item found at position ${args.line}:${args.character} in ${args.file_path}`
        );
      }

      item = items[0]; // Use the first item
    } catch (error) {
      return createMCPResponse(
        `Error preparing call hierarchy: ${error instanceof Error ? error.message : String(error)}`
      );
    }
  } else {
    return createMCPResponse(
      'Invalid arguments: provide either "item" or "file_path", "line", and "character"'
    );
  }

  try {
    const outgoingCalls = await hierarchyService.getCallHierarchyOutgoingCalls(item);

    if (outgoingCalls.length === 0) {
      return createMCPResponse(`No outgoing calls found from ${item.name}`);
    }

    const callDescriptions = outgoingCalls.map((call, index) => {
      const toKind = getSymbolKindName(call.to.kind);
      const toRange = formatHumanRange(
        { start: toHumanPosition(call.to.range.start), end: toHumanPosition(call.to.range.end) },
        'short'
      );
      const toDetail = call.to.detail ? ` - ${call.to.detail}` : '';

      const ranges = call.fromRanges
        .map((range) =>
          formatHumanRange(
            { start: toHumanPosition(range.start), end: toHumanPosition(range.end) },
            'short'
          )
        )
        .join(', ');

      return `${index + 1}. To **${call.to.name}** (${toKind}) at ${toRange}${toDetail}\n   Call sites: ${ranges}\n   URI: ${call.to.uri}`;
    });

    return createMCPResponse(
      `## Outgoing Calls from ${item.name}\n\nFound ${outgoingCalls.length} outgoing call${outgoingCalls.length === 1 ? '' : 's'}:\n\n${callDescriptions.join('\n\n')}`
    );
  } catch (error) {
    return createMCPResponse(
      `Error getting outgoing calls: ${error instanceof Error ? error.message : String(error)}`
    );
  }
}

// Helper function to get symbol kind name
function getSymbolKindName(kind: number): string {
  const kindMap: Record<number, string> = {
    1: 'File',
    2: 'Module',
    3: 'Namespace',
    4: 'Package',
    5: 'Class',
    6: 'Method',
    7: 'Property',
    8: 'Field',
    9: 'Constructor',
    10: 'Enum',
    11: 'Interface',
    12: 'Function',
    13: 'Variable',
    14: 'Constant',
    15: 'String',
    16: 'Number',
    17: 'Boolean',
    18: 'Array',
    19: 'Object',
    20: 'Key',
    21: 'Null',
    22: 'EnumMember',
    23: 'Struct',
    24: 'Event',
    25: 'Operator',
    26: 'TypeParameter',
  };
  return kindMap[kind] || `Unknown(${kind})`;
}

// Register hierarchy tools with the central registry
registerTools(
  {
    prepare_call_hierarchy: { handler: handlePrepareCallHierarchy, requiresService: 'hierarchy' },
    get_call_hierarchy_incoming_calls: {
      handler: handleGetCallHierarchyIncomingCalls,
      requiresService: 'hierarchy',
    },
    get_call_hierarchy_outgoing_calls: {
      handler: handleGetCallHierarchyOutgoingCalls,
      requiresService: 'hierarchy',
    },
  },
  'hierarchy-handlers'
);
