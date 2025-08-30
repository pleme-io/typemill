// src/mcp/handlers/hierarchy-handlers.ts
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

// src/mcp/handlers/hierarchy-handlers.ts
async function handlePrepareCallHierarchy(hierarchyService, args) {
  const { file_path, line, character } = args;
  const absolutePath = resolve(file_path);
  try {
    const items = await hierarchyService.prepareCallHierarchy(absolutePath, {
      line: line - 1,
      character
    });
    if (items.length === 0) {
      return createMCPResponse(`No call hierarchy available for position ${line}:${character} in ${file_path}`);
    }
    const itemDescriptions = items.map((item, index) => {
      const kindName = getSymbolKindName(item.kind);
      const range = `${item.range.start.line + 1}:${item.range.start.character} - ${item.range.end.line + 1}:${item.range.end.character}`;
      const detail = item.detail ? ` - ${item.detail}` : "";
      return `${index + 1}. **${item.name}** (${kindName}) at ${range}${detail}
   URI: ${item.uri}`;
    });
    return createMCPResponse(`## Call Hierarchy Items for ${file_path}:${line}:${character}

Found ${items.length} item${items.length === 1 ? "" : "s"}:

${itemDescriptions.join(`

`)}

*Use these items with get_call_hierarchy_incoming_calls or get_call_hierarchy_outgoing_calls.*`);
  } catch (error) {
    return createMCPResponse(`Error preparing call hierarchy: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetCallHierarchyIncomingCalls(hierarchyService, args) {
  const { item } = args;
  try {
    const incomingCalls = await hierarchyService.getCallHierarchyIncomingCalls(item);
    if (incomingCalls.length === 0) {
      return createMCPResponse(`No incoming calls found for ${item.name}`);
    }
    const callDescriptions = incomingCalls.map((call, index) => {
      const fromKind = getSymbolKindName(call.from.kind);
      const fromRange = `${call.from.range.start.line + 1}:${call.from.range.start.character} - ${call.from.range.end.line + 1}:${call.from.range.end.character}`;
      const fromDetail = call.from.detail ? ` - ${call.from.detail}` : "";
      const ranges = call.fromRanges.map((range) => `${range.start.line + 1}:${range.start.character} - ${range.end.line + 1}:${range.end.character}`).join(", ");
      return `${index + 1}. From **${call.from.name}** (${fromKind}) at ${fromRange}${fromDetail}
   Call sites: ${ranges}
   URI: ${call.from.uri}`;
    });
    return createMCPResponse(`## Incoming Calls to ${item.name}

Found ${incomingCalls.length} incoming call${incomingCalls.length === 1 ? "" : "s"}:

${callDescriptions.join(`

`)}`);
  } catch (error) {
    return createMCPResponse(`Error getting incoming calls: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetCallHierarchyOutgoingCalls(hierarchyService, args) {
  const { item } = args;
  try {
    const outgoingCalls = await hierarchyService.getCallHierarchyOutgoingCalls(item);
    if (outgoingCalls.length === 0) {
      return createMCPResponse(`No outgoing calls found from ${item.name}`);
    }
    const callDescriptions = outgoingCalls.map((call, index) => {
      const toKind = getSymbolKindName(call.to.kind);
      const toRange = `${call.to.range.start.line + 1}:${call.to.range.start.character} - ${call.to.range.end.line + 1}:${call.to.range.end.character}`;
      const toDetail = call.to.detail ? ` - ${call.to.detail}` : "";
      const ranges = call.fromRanges.map((range) => `${range.start.line + 1}:${range.start.character} - ${range.end.line + 1}:${range.end.character}`).join(", ");
      return `${index + 1}. To **${call.to.name}** (${toKind}) at ${toRange}${toDetail}
   Call sites: ${ranges}
   URI: ${call.to.uri}`;
    });
    return createMCPResponse(`## Outgoing Calls from ${item.name}

Found ${outgoingCalls.length} outgoing call${outgoingCalls.length === 1 ? "" : "s"}:

${callDescriptions.join(`

`)}`);
  } catch (error) {
    return createMCPResponse(`Error getting outgoing calls: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handlePrepareTypeHierarchy(hierarchyService, args) {
  const { file_path, line, character } = args;
  const absolutePath = resolve(file_path);
  try {
    const items = await hierarchyService.prepareTypeHierarchy(absolutePath, {
      line: line - 1,
      character
    });
    if (items.length === 0) {
      return createMCPResponse(`No type hierarchy available for position ${line}:${character} in ${file_path}`);
    }
    const itemDescriptions = items.map((item, index) => {
      const kindName = getSymbolKindName(item.kind);
      const range = `${item.range.start.line + 1}:${item.range.start.character} - ${item.range.end.line + 1}:${item.range.end.character}`;
      const detail = item.detail ? ` - ${item.detail}` : "";
      return `${index + 1}. **${item.name}** (${kindName}) at ${range}${detail}
   URI: ${item.uri}`;
    });
    return createMCPResponse(`## Type Hierarchy Items for ${file_path}:${line}:${character}

Found ${items.length} item${items.length === 1 ? "" : "s"}:

${itemDescriptions.join(`

`)}

*Use these items with get_type_hierarchy_supertypes or get_type_hierarchy_subtypes.*`);
  } catch (error) {
    return createMCPResponse(`Error preparing type hierarchy: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetTypeHierarchySupertypes(hierarchyService, args) {
  const { item } = args;
  try {
    const supertypes = await hierarchyService.getTypeHierarchySupertypes(item);
    if (supertypes.length === 0) {
      return createMCPResponse(`No supertypes found for ${item.name}`);
    }
    const supertypeDescriptions = supertypes.map((supertype, index) => {
      const kindName = getSymbolKindName(supertype.kind);
      const range = `${supertype.range.start.line + 1}:${supertype.range.start.character} - ${supertype.range.end.line + 1}:${supertype.range.end.character}`;
      const detail = supertype.detail ? ` - ${supertype.detail}` : "";
      return `${index + 1}. **${supertype.name}** (${kindName}) at ${range}${detail}
   URI: ${supertype.uri}`;
    });
    return createMCPResponse(`## Supertypes of ${item.name}

Found ${supertypes.length} supertype${supertypes.length === 1 ? "" : "s"}:

${supertypeDescriptions.join(`

`)}`);
  } catch (error) {
    return createMCPResponse(`Error getting supertypes: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetTypeHierarchySubtypes(hierarchyService, args) {
  const { item } = args;
  try {
    const subtypes = await hierarchyService.getTypeHierarchySubtypes(item);
    if (subtypes.length === 0) {
      return createMCPResponse(`No subtypes found for ${item.name}`);
    }
    const subtypeDescriptions = subtypes.map((subtype, index) => {
      const kindName = getSymbolKindName(subtype.kind);
      const range = `${subtype.range.start.line + 1}:${subtype.range.start.character} - ${subtype.range.end.line + 1}:${subtype.range.end.character}`;
      const detail = subtype.detail ? ` - ${subtype.detail}` : "";
      return `${index + 1}. **${subtype.name}** (${kindName}) at ${range}${detail}
   URI: ${subtype.uri}`;
    });
    return createMCPResponse(`## Subtypes of ${item.name}

Found ${subtypes.length} subtype${subtypes.length === 1 ? "" : "s"}:

${subtypeDescriptions.join(`

`)}`);
  } catch (error) {
    return createMCPResponse(`Error getting subtypes: ${error instanceof Error ? error.message : String(error)}`);
  }
}
async function handleGetSelectionRange(hierarchyService, args) {
  const { file_path, positions } = args;
  const absolutePath = resolve(file_path);
  try {
    const lspPositions = positions.map((pos) => ({
      line: pos.line - 1,
      character: pos.character
    }));
    const selectionRanges = await hierarchyService.getSelectionRange(absolutePath, lspPositions);
    if (selectionRanges.length === 0) {
      return createMCPResponse(`No selection ranges available for the given positions in ${file_path}`);
    }
    const rangeDescriptions = selectionRanges.map((selectionRange, index) => {
      const originalPos = positions[index];
      if (!originalPos) {
        return `Position ${index + 1}: No original position available`;
      }
      const ranges = [];
      let current = selectionRange;
      let level = 0;
      while (current && level < 10) {
        const range = `${current.range.start.line + 1}:${current.range.start.character} - ${current.range.end.line + 1}:${current.range.end.character}`;
        ranges.push(`   Level ${level}: ${range}`);
        current = current.parent;
        level++;
      }
      return `Position ${index + 1} (${originalPos.line}:${originalPos.character}):
${ranges.join(`
`)}`;
    });
    return createMCPResponse(`## Selection Ranges for ${file_path}

${rangeDescriptions.join(`

`)}

*Selection ranges show expandable text selections from specific positions outward.*`);
  } catch (error) {
    return createMCPResponse(`Error getting selection ranges: ${error instanceof Error ? error.message : String(error)}`);
  }
}
function getSymbolKindName(kind) {
  const kindMap = {
    1: "File",
    2: "Module",
    3: "Namespace",
    4: "Package",
    5: "Class",
    6: "Method",
    7: "Property",
    8: "Field",
    9: "Constructor",
    10: "Enum",
    11: "Interface",
    12: "Function",
    13: "Variable",
    14: "Constant",
    15: "String",
    16: "Number",
    17: "Boolean",
    18: "Array",
    19: "Object",
    20: "Key",
    21: "Null",
    22: "EnumMember",
    23: "Struct",
    24: "Event",
    25: "Operator",
    26: "TypeParameter"
  };
  return kindMap[kind] || `Unknown(${kind})`;
}
export {
  handlePrepareTypeHierarchy,
  handlePrepareCallHierarchy,
  handleGetTypeHierarchySupertypes,
  handleGetTypeHierarchySubtypes,
  handleGetSelectionRange,
  handleGetCallHierarchyOutgoingCalls,
  handleGetCallHierarchyIncomingCalls
};
