/**
 * Main utilities barrel export
 */

// File utilities
export * from '../../../../server/src/utils/file/index.js';
export {
  normalizePath,
  pathToUrl,
  readFileContent,
  resolvePath,
  urlToPath,
  writeFileContent,
} from '../../../../server/src/utils/file/index.js';
// Performance utilities
export * from '../../../../server/src/utils/performance.js';
export {
  globalPerformanceTracker,
  measurePerformance,
  withPerformanceMeasurement,
} from '../../../../server/src/utils/performance.js';
// Platform utilities
export * from '../../../../server/src/utils/platform/index.js';

// Re-export commonly used utilities for convenience
export {
  getLSPServerPaths,
  isProcessRunning,
  terminateProcess,
} from '../../../../server/src/utils/platform/index.js';
// Position utilities
export * from '../../../../server/src/utils/position.js';

export {
  formatFileLocation,
  formatHumanPosition,
  parsePositionString,
  toHumanPosition,
  toLSPPosition,
} from '../../../../server/src/utils/position.js';
// Validation utilities
export * from '../../../../server/src/utils/validation.js';
export {
  assertFileExists,
  assertNonEmptyString,
  assertValidFilePath,
  assertValidLSPPosition,
  assertValidSymbolName,
  ValidationError,
} from '../../../../server/src/utils/validation.js';
