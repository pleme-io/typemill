/**
 * Input validation utilities for MCP tool handlers
 * Centralizes common validation logic and provides consistent error messages
 */

import { promises as fs } from 'fs';
import path from 'path';
import {
  type LSPPosition,
  type HumanPosition,
  isValidLSPPosition,
  isValidHumanPosition,
  ensureLSPPosition,
  ensureHumanPosition,
} from './position.js';

/**
 * Validation result interface
 */
export interface ValidationResult {
  valid: boolean;
  error?: string;
}

/**
 * Validation error class for consistent error handling
 */
export class ValidationError extends Error {
  constructor(message: string, public readonly field: string) {
    super(message);
    this.name = 'ValidationError';
  }
}

/**
 * Assert that a value is a non-empty string
 */
export function assertNonEmptyString(value: unknown, fieldName: string): asserts value is string {
  if (typeof value !== 'string') {
    throw new ValidationError(`${fieldName} must be a string`, fieldName);
  }
  if (value.trim().length === 0) {
    throw new ValidationError(`${fieldName} cannot be empty`, fieldName);
  }
}

/**
 * Assert that a value is a valid number
 */
export function assertValidNumber(
  value: unknown,
  fieldName: string,
  options: { min?: number; max?: number; integer?: boolean } = {}
): asserts value is number {
  if (typeof value !== 'number' || isNaN(value)) {
    throw new ValidationError(`${fieldName} must be a valid number`, fieldName);
  }

  if (options.integer && !Number.isInteger(value)) {
    throw new ValidationError(`${fieldName} must be an integer`, fieldName);
  }

  if (options.min !== undefined && value < options.min) {
    throw new ValidationError(`${fieldName} must be at least ${options.min}`, fieldName);
  }

  if (options.max !== undefined && value > options.max) {
    throw new ValidationError(`${fieldName} must be at most ${options.max}`, fieldName);
  }
}

/**
 * Validate file path
 */
export function validateFilePath(filePath: unknown): ValidationResult {
  try {
    assertNonEmptyString(filePath, 'file_path');

    // Check for valid file extensions
    const supportedExtensions = ['.ts', '.tsx', '.js', '.jsx', '.py', '.go', '.rs', '.java', '.cpp', '.c'];
    const ext = path.extname(filePath).toLowerCase();

    if (!supportedExtensions.includes(ext)) {
      return {
        valid: false,
        error: `Unsupported file extension '${ext}'. Supported: ${supportedExtensions.join(', ')}`,
      };
    }

    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof ValidationError ? error.message : 'Invalid file path',
    };
  }
}

/**
 * Assert that a file path is valid and supported
 */
export function assertValidFilePath(filePath: unknown): asserts filePath is string {
  const validation = validateFilePath(filePath);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, 'file_path');
  }
}

/**
 * Assert that a file exists
 */
export async function assertFileExists(filePath: string): Promise<void> {
  try {
    const stat = await fs.stat(filePath);
    if (!stat.isFile()) {
      throw new ValidationError(`Path exists but is not a file: ${filePath}`, 'file_path');
    }
  } catch (error) {
    if ((error as NodeJS.ErrnoException).code === 'ENOENT') {
      throw new ValidationError(`File not found: ${filePath}`, 'file_path');
    }
    throw new ValidationError(`Cannot access file: ${filePath}`, 'file_path');
  }
}

/**
 * Validate symbol name
 */
export function validateSymbolName(symbolName: unknown): ValidationResult {
  try {
    assertNonEmptyString(symbolName, 'symbol_name');

    // Basic identifier validation (allows most programming language identifiers)
    const identifierRegex = /^[a-zA-Z_$][a-zA-Z0-9_$]*$/;
    if (!identifierRegex.test(symbolName)) {
      return {
        valid: false,
        error: 'Symbol name must be a valid identifier (letters, numbers, underscore, dollar sign)',
      };
    }

    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof ValidationError ? error.message : 'Invalid symbol name',
    };
  }
}

/**
 * Assert that a symbol name is valid
 */
export function assertValidSymbolName(symbolName: unknown): asserts symbolName is string {
  const validation = validateSymbolName(symbolName);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, 'symbol_name');
  }
}

/**
 * Validate LSP position
 */
export function validateLSPPosition(position: unknown): ValidationResult {
  try {
    const pos = ensureLSPPosition(position);
    if (!isValidLSPPosition(pos)) {
      return {
        valid: false,
        error: 'Position line and character must be non-negative numbers',
      };
    }
    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : 'Invalid position',
    };
  }
}

/**
 * Assert that a position is valid LSP position
 */
export function assertValidLSPPosition(position: unknown): asserts position is LSPPosition {
  const validation = validateLSPPosition(position);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, 'position');
  }
}

/**
 * Validate human position
 */
export function validateHumanPosition(position: unknown): ValidationResult {
  try {
    const pos = ensureHumanPosition(position);
    if (!isValidHumanPosition(pos)) {
      return {
        valid: false,
        error: 'Position line and character must be positive numbers',
      };
    }
    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof Error ? error.message : 'Invalid position',
    };
  }
}

/**
 * Assert that a position is valid human position
 */
export function assertValidHumanPosition(position: unknown): asserts position is HumanPosition {
  const validation = validateHumanPosition(position);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, 'position');
  }
}

/**
 * Validate that line and character are provided as separate arguments
 */
export function validateLineAndCharacter(line: unknown, character: unknown): ValidationResult {
  try {
    assertValidNumber(line, 'line', { min: 1, integer: true });
    assertValidNumber(character, 'character', { min: 0, integer: true });
    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof ValidationError ? error.message : 'Invalid line or character',
    };
  }
}

/**
 * Assert that line and character are valid
 */
export function assertValidLineAndCharacter(line: unknown, character: unknown): asserts line is number {
  const validation = validateLineAndCharacter(line, character);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, 'line_character');
  }
}

/**
 * Validate workspace path
 */
export function validateWorkspacePath(workspacePath: unknown): ValidationResult {
  if (workspacePath === undefined || workspacePath === null) {
    return { valid: true }; // Optional parameter
  }

  try {
    assertNonEmptyString(workspacePath, 'workspace_path');

    if (!path.isAbsolute(workspacePath)) {
      return {
        valid: false,
        error: 'Workspace path must be absolute',
      };
    }

    return { valid: true };
  } catch (error) {
    return {
      valid: false,
      error: error instanceof ValidationError ? error.message : 'Invalid workspace path',
    };
  }
}

/**
 * Assert that workspace path is valid
 */
export function assertValidWorkspacePath(workspacePath: unknown): asserts workspacePath is string | undefined {
  const validation = validateWorkspacePath(workspacePath);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, 'workspace_path');
  }
}

/**
 * Validate boolean value
 */
export function validateBoolean(value: unknown, fieldName: string): ValidationResult {
  if (typeof value === 'boolean') {
    return { valid: true };
  }

  if (value === 'true' || value === 'false') {
    return { valid: true };
  }

  if (value === undefined || value === null) {
    return { valid: true }; // Optional boolean
  }

  return {
    valid: false,
    error: `${fieldName} must be a boolean value`,
  };
}

/**
 * Assert that a value is a valid boolean
 */
export function assertValidBoolean(value: unknown, fieldName: string): asserts value is boolean | undefined {
  const validation = validateBoolean(value, fieldName);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, fieldName);
  }
}

/**
 * Validate array of strings
 */
export function validateStringArray(value: unknown, fieldName: string): ValidationResult {
  if (!Array.isArray(value)) {
    return {
      valid: false,
      error: `${fieldName} must be an array`,
    };
  }

  for (let i = 0; i < value.length; i++) {
    if (typeof value[i] !== 'string') {
      return {
        valid: false,
        error: `${fieldName}[${i}] must be a string`,
      };
    }
  }

  return { valid: true };
}

/**
 * Assert that a value is a valid string array
 */
export function assertValidStringArray(value: unknown, fieldName: string): asserts value is string[] {
  const validation = validateStringArray(value, fieldName);
  if (!validation.valid) {
    throw new ValidationError(validation.error!, fieldName);
  }
}

/**
 * Create comprehensive validation function for common MCP tool arguments
 */
export function validateMCPToolArgs(args: Record<string, unknown>): {
  valid: boolean;
  errors: Array<{ field: string; message: string }>;
} {
  const errors: Array<{ field: string; message: string }> = [];

  // Validate file_path if present
  if ('file_path' in args) {
    const fileValidation = validateFilePath(args.file_path);
    if (!fileValidation.valid) {
      errors.push({ field: 'file_path', message: fileValidation.error! });
    }
  }

  // Validate symbol_name if present
  if ('symbol_name' in args) {
    const symbolValidation = validateSymbolName(args.symbol_name);
    if (!symbolValidation.valid) {
      errors.push({ field: 'symbol_name', message: symbolValidation.error! });
    }
  }

  // Validate line and character if present
  if ('line' in args && 'character' in args) {
    const lineCharValidation = validateLineAndCharacter(args.line, args.character);
    if (!lineCharValidation.valid) {
      errors.push({ field: 'line_character', message: lineCharValidation.error! });
    }
  }

  // Validate workspace_path if present
  if ('workspace_path' in args) {
    const workspaceValidation = validateWorkspacePath(args.workspace_path);
    if (!workspaceValidation.valid) {
      errors.push({ field: 'workspace_path', message: workspaceValidation.error! });
    }
  }

  return {
    valid: errors.length === 0,
    errors,
  };
}