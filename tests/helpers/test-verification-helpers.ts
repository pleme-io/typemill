import { expect } from 'bun:test';
import { existsSync, readFileSync } from 'node:fs';

/**
 * Helper utilities for thorough test verification of multi-file operations
 */

/**
 * Verify a file contains exact content
 */
export function verifyFileContainsExact(filePath: string, expectedContent: string): void {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }
  const actualContent = readFileSync(filePath, 'utf-8');
  expect(actualContent).toBe(expectedContent);
}

/**
 * Verify a file contains all expected strings
 */
export function verifyFileContainsAll(
  filePath: string,
  expectedStrings: string[],
  options: { trimLines?: boolean; caseSensitive?: boolean } = {}
): void {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }

  const actualContent = readFileSync(filePath, 'utf-8');
  const content = options.caseSensitive === false ? actualContent.toLowerCase() : actualContent;

  const missing: string[] = [];
  for (const expected of expectedStrings) {
    const searchStr = options.caseSensitive === false ? expected.toLowerCase() : expected;
    if (!content.includes(searchStr)) {
      missing.push(expected);
    }
  }

  if (missing.length > 0) {
    console.error(`File ${filePath} is missing expected content:`);
    console.error('Missing strings:', missing);
    console.error('Actual content preview (first 500 chars):');
    console.error(actualContent.substring(0, 500));
    throw new Error(`File ${filePath} is missing ${missing.length} expected string(s)`);
  }
}

/**
 * Verify a file does not contain any of the specified strings
 */
export function verifyFileDoesNotContain(
  filePath: string,
  unexpectedStrings: string[],
  options: { caseSensitive?: boolean } = {}
): void {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }

  const actualContent = readFileSync(filePath, 'utf-8');
  const content = options.caseSensitive === false ? actualContent.toLowerCase() : actualContent;

  const found: string[] = [];
  for (const unexpected of unexpectedStrings) {
    const searchStr = options.caseSensitive === false ? unexpected.toLowerCase() : unexpected;
    if (content.includes(searchStr)) {
      found.push(unexpected);
    }
  }

  if (found.length > 0) {
    console.error(`File ${filePath} contains unexpected content:`);
    console.error('Found strings:', found);
    throw new Error(`File ${filePath} contains ${found.length} unexpected string(s)`);
  }
}

/**
 * Capture the state of multiple files
 */
export function captureFileStates(filePaths: string[]): Map<string, string | null> {
  const states = new Map<string, string | null>();

  for (const filePath of filePaths) {
    if (existsSync(filePath)) {
      states.set(filePath, readFileSync(filePath, 'utf-8'));
    } else {
      states.set(filePath, null);
    }
  }

  return states;
}

/**
 * Verify specific changes between before and after states
 */
export function verifyFileChanges(
  filePath: string,
  beforeState: string | null,
  afterState: string | null,
  expectedChanges: {
    added?: string[];
    removed?: string[];
    unchanged?: string[];
  }
): void {
  // Check file existence changes
  if (beforeState === null && afterState === null) {
    throw new Error(`File ${filePath} did not exist before or after`);
  }

  if (beforeState === null && afterState !== null) {
    console.log(`✅ File ${filePath} was created`);
  }

  if (beforeState !== null && afterState === null) {
    console.log(`✅ File ${filePath} was deleted`);
    return;
  }

  if (beforeState !== null && afterState !== null) {
    // Verify added content
    if (expectedChanges.added) {
      for (const added of expectedChanges.added) {
        if (!afterState.includes(added)) {
          throw new Error(`Expected content not added to ${filePath}: "${added}"`);
        }
        if (beforeState.includes(added)) {
          throw new Error(`Content already existed in ${filePath}: "${added}"`);
        }
      }
    }

    // Verify removed content
    if (expectedChanges.removed) {
      for (const removed of expectedChanges.removed) {
        if (afterState.includes(removed)) {
          throw new Error(`Expected content not removed from ${filePath}: "${removed}"`);
        }
        if (!beforeState.includes(removed)) {
          throw new Error(`Content to remove didn't exist in ${filePath}: "${removed}"`);
        }
      }
    }

    // Verify unchanged content
    if (expectedChanges.unchanged) {
      for (const unchanged of expectedChanges.unchanged) {
        if (!beforeState.includes(unchanged)) {
          throw new Error(`Unchanged content not in original ${filePath}: "${unchanged}"`);
        }
        if (!afterState.includes(unchanged)) {
          throw new Error(`Unchanged content missing from modified ${filePath}: "${unchanged}"`);
        }
      }
    }
  }
}

/**
 * Verify an import statement was changed correctly
 */
export function verifyImportStatement(
  filePath: string,
  oldImport: string | RegExp,
  newImport: string
): void {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }

  const content = readFileSync(filePath, 'utf-8');

  // Check old import is gone
  if (typeof oldImport === 'string') {
    if (content.includes(oldImport)) {
      throw new Error(`Old import still present in ${filePath}: "${oldImport}"`);
    }
  } else {
    if (oldImport.test(content)) {
      throw new Error(`Old import pattern still matches in ${filePath}: ${oldImport}`);
    }
  }

  // Check new import is present
  if (!content.includes(newImport)) {
    console.error(`Expected import not found in ${filePath}`);
    console.error(`Looking for: "${newImport}"`);
    console.error('File content preview:');
    console.error(content.split('\n').slice(0, 20).join('\n'));
    throw new Error(`New import not present in ${filePath}: "${newImport}"`);
  }
}

/**
 * Verify exact line content at a specific line number
 */
export function verifyLineContent(
  filePath: string,
  lineNumber: number,
  expectedContent: string,
  options: { trim?: boolean } = { trim: true }
): void {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }

  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');

  if (lineNumber < 1 || lineNumber > lines.length) {
    throw new Error(
      `Line ${lineNumber} out of range in ${filePath} (file has ${lines.length} lines)`
    );
  }

  const actualLine = options.trim ? lines[lineNumber - 1].trim() : lines[lineNumber - 1];
  const expected = options.trim ? expectedContent.trim() : expectedContent;

  if (actualLine !== expected) {
    console.error(`Line ${lineNumber} mismatch in ${filePath}`);
    console.error(`Expected: "${expected}"`);
    console.error(`Actual:   "${actualLine}"`);
    throw new Error(`Line ${lineNumber} content mismatch in ${filePath}`);
  }
}

/**
 * Count occurrences of a pattern in a file
 */
export function countOccurrences(filePath: string, pattern: string | RegExp): number {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }

  const content = readFileSync(filePath, 'utf-8');

  if (typeof pattern === 'string') {
    let count = 0;
    let index = 0;
    while ((index = content.indexOf(pattern, index)) !== -1) {
      count++;
      index += pattern.length;
    }
    return count;
  }
  const matches = content.match(new RegExp(pattern, 'g'));
  return matches ? matches.length : 0;
}

/**
 * Verify file was modified (by checking timestamps or content)
 */
export function verifyFileWasModified(filePath: string, originalContent: string | null): boolean {
  if (!existsSync(filePath)) {
    return originalContent !== null; // File was deleted
  }

  const currentContent = readFileSync(filePath, 'utf-8');
  return currentContent !== originalContent;
}

/**
 * Get specific lines from a file
 */
export function getFileLines(filePath: string, startLine: number, endLine?: number): string[] {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }

  const content = readFileSync(filePath, 'utf-8');
  const lines = content.split('\n');

  const start = Math.max(0, startLine - 1);
  const end = endLine ? Math.min(lines.length, endLine) : startLine;

  return lines.slice(start, end);
}

/**
 * Verify that a file compiles (by checking for TypeScript errors)
 * Note: This is a simplified check - full compilation would require TSC
 */
export function verifyNoObviousSyntaxErrors(filePath: string): void {
  if (!existsSync(filePath)) {
    throw new Error(`File does not exist: ${filePath}`);
  }

  const content = readFileSync(filePath, 'utf-8');

  // Check for balanced braces
  let braceCount = 0;
  for (const char of content) {
    if (char === '{') braceCount++;
    if (char === '}') braceCount--;
  }
  if (braceCount !== 0) {
    throw new Error(`Unbalanced braces in ${filePath}`);
  }

  // Check for balanced parentheses
  let parenCount = 0;
  for (const char of content) {
    if (char === '(') parenCount++;
    if (char === ')') parenCount--;
  }
  if (parenCount !== 0) {
    throw new Error(`Unbalanced parentheses in ${filePath}`);
  }

  // Check for balanced brackets
  let bracketCount = 0;
  for (const char of content) {
    if (char === '[') bracketCount++;
    if (char === ']') bracketCount--;
  }
  if (bracketCount !== 0) {
    throw new Error(`Unbalanced brackets in ${filePath}`);
  }
}

/**
 * Compare two files for equality
 */
export function filesAreEqual(filePath1: string, filePath2: string): boolean {
  if (!existsSync(filePath1) || !existsSync(filePath2)) {
    return false;
  }

  const content1 = readFileSync(filePath1, 'utf-8');
  const content2 = readFileSync(filePath2, 'utf-8');

  return content1 === content2;
}

/**
 * Detailed file diff for debugging
 */
export function showFileDiff(filePath: string, before: string, after: string): void {
  console.log(`\n📝 File changes in ${filePath}:`);

  const beforeLines = before.split('\n');
  const afterLines = after.split('\n');

  const maxLines = Math.max(beforeLines.length, afterLines.length);

  for (let i = 0; i < maxLines; i++) {
    const beforeLine = beforeLines[i] || '';
    const afterLine = afterLines[i] || '';

    if (beforeLine !== afterLine) {
      if (beforeLine && !afterLine) {
        console.log(`  Line ${i + 1}: DELETED: "${beforeLine}"`);
      } else if (!beforeLine && afterLine) {
        console.log(`  Line ${i + 1}: ADDED: "${afterLine}"`);
      } else {
        console.log(`  Line ${i + 1}: CHANGED:`);
        console.log(`    FROM: "${beforeLine}"`);
        console.log(`    TO:   "${afterLine}"`);
      }
    }
  }
}
