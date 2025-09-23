/**
 * Main utilities barrel export
 */

// File utilities
export * from './file/index.js';

// Platform utilities  
export * from './platform/index.js';

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