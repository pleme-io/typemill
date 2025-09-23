/**
 * Position utilities for converting between LSP (0-indexed) and human-readable (1-indexed) positions
 * This eliminates the constant +1/-1 position conversion bugs throughout the codebase
 */

/**
 * LSP Position (0-indexed line and character)
 */
export interface LSPPosition {
  line: number;
  character: number;
}

/**
 * Human-readable position (1-indexed line and character)
 */
export interface HumanPosition {
  line: number;
  character: number;
}

/**
 * LSP Range with start and end positions
 */
export interface LSPRange {
  start: LSPPosition;
  end: LSPPosition;
}

/**
 * Human-readable range with start and end positions
 */
export interface HumanRange {
  start: HumanPosition;
  end: HumanPosition;
}

/**
 * Convert LSP position (0-indexed) to human-readable position (1-indexed)
 */
export function toHumanPosition(lspPos: LSPPosition): HumanPosition {
  return {
    line: lspPos.line + 1,
    character: lspPos.character + 1,
  };
}

/**
 * Convert human-readable position (1-indexed) to LSP position (0-indexed)
 */
export function toLSPPosition(humanPos: HumanPosition): LSPPosition {
  return {
    line: Math.max(0, humanPos.line - 1),
    character: Math.max(0, humanPos.character - 1),
  };
}

/**
 * Convert LSP range to human-readable range
 */
export function toHumanRange(lspRange: LSPRange): HumanRange {
  return {
    start: toHumanPosition(lspRange.start),
    end: toHumanPosition(lspRange.end),
  };
}

/**
 * Convert human-readable range to LSP range
 */
export function toLSPRange(humanRange: HumanRange): LSPRange {
  return {
    start: toLSPPosition(humanRange.start),
    end: toLSPPosition(humanRange.end),
  };
}

/**
 * Format human position as a readable string
 * @param pos Human-readable position
 * @param format Format style
 * @returns Formatted string like "Line 15, Col 23" or "15:23"
 */
export function formatHumanPosition(
  pos: HumanPosition,
  format: 'long' | 'short' = 'long'
): string {
  if (format === 'short') {
    return `${pos.line}:${pos.character}`;
  }
  return `Line ${pos.line}, Col ${pos.character}`;
}

/**
 * Format human range as a readable string
 * @param range Human-readable range
 * @param format Format style
 * @returns Formatted string like "Line 15, Col 23 - Line 20, Col 30" or "15:23-20:30"
 */
export function formatHumanRange(
  range: HumanRange,
  format: 'long' | 'short' = 'long'
): string {
  const startStr = formatHumanPosition(range.start, format);
  const endStr = formatHumanPosition(range.end, format);

  if (format === 'short') {
    return `${startStr}-${endStr}`;
  }
  return `${startStr} to ${endStr}`;
}

/**
 * Format file location with human-readable position
 * @param filePath File path
 * @param pos Human-readable position
 * @param format Format style
 * @returns Formatted string like "src/file.ts:15:23" or "src/file.ts at Line 15, Col 23"
 */
export function formatFileLocation(
  filePath: string,
  pos: HumanPosition,
  format: 'short' | 'long' = 'short'
): string {
  if (format === 'short') {
    return `${filePath}:${pos.line}:${pos.character}`;
  }
  return `${filePath} at ${formatHumanPosition(pos, 'long')}`;
}

/**
 * Parse position string in format "line:character" to human position
 * @param input String like "15:23"
 * @returns Human position or null if invalid
 */
export function parsePositionString(input: string): HumanPosition | null {
  const parts = input.split(':');
  if (parts.length !== 2) return null;

  const line = parseInt(parts[0]!, 10);
  const character = parseInt(parts[1]!, 10);

  if (isNaN(line) || isNaN(character) || line < 1 || character < 1) {
    return null;
  }

  return { line, character };
}

/**
 * Validate that a position is valid (non-negative)
 */
export function isValidLSPPosition(pos: LSPPosition): boolean {
  return pos.line >= 0 && pos.character >= 0;
}

/**
 * Validate that a human position is valid (positive)
 */
export function isValidHumanPosition(pos: HumanPosition): boolean {
  return pos.line >= 1 && pos.character >= 1;
}

/**
 * Helper to convert unknown position object to LSP position safely
 */
export function ensureLSPPosition(pos: any): LSPPosition {
  if (typeof pos !== 'object' || pos === null) {
    throw new Error('Position must be an object');
  }

  const line = typeof pos.line === 'number' ? pos.line : parseInt(pos.line, 10);
  const character = typeof pos.character === 'number' ? pos.character : parseInt(pos.character, 10);

  if (isNaN(line) || isNaN(character)) {
    throw new Error('Position line and character must be numbers');
  }

  const lspPos = { line: Math.max(0, line), character: Math.max(0, character) };

  if (!isValidLSPPosition(lspPos)) {
    throw new Error('Invalid LSP position: line and character must be non-negative');
  }

  return lspPos;
}

/**
 * Helper to convert unknown position object to human position safely
 */
export function ensureHumanPosition(pos: any): HumanPosition {
  if (typeof pos !== 'object' || pos === null) {
    throw new Error('Position must be an object');
  }

  const line = typeof pos.line === 'number' ? pos.line : parseInt(pos.line, 10);
  const character = typeof pos.character === 'number' ? pos.character : parseInt(pos.character, 10);

  if (isNaN(line) || isNaN(character)) {
    throw new Error('Position line and character must be numbers');
  }

  const humanPos = { line: Math.max(1, line), character: Math.max(1, character) };

  if (!isValidHumanPosition(humanPos)) {
    throw new Error('Invalid human position: line and character must be positive');
  }

  return humanPos;
}