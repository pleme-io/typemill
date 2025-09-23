/**
 * Main utilities barrel export
 */

// File utilities
export * from './file/index.js';

// Platform utilities
export * from './platform/index.js';

// Position utilities
export * from './position.js';

// Validation utilities
export * from './validation.js';

// Performance utilities
export * from './performance.js';

// Re-export commonly used utilities for convenience
export {
  isProcessRunning,
  terminateProcess,
  getLSPServerPaths
} from './platform/index.js';

export {
  readFileContent,
  writeFileContent,
  resolvePath,
  normalizePath,
  urlToPath,
  pathToUrl
} from './file/index.js';

export {
  toHumanPosition,
  toLSPPosition,
  formatHumanPosition,
  formatFileLocation,
  parsePositionString
} from './position.js';

export {
  assertNonEmptyString,
  assertValidFilePath,
  assertFileExists,
  assertValidSymbolName,
  assertValidLSPPosition,
  ValidationError
} from './validation.js';

export {
  measurePerformance,
  withPerformanceMeasurement,
  globalPerformanceTracker
} from './performance.js';