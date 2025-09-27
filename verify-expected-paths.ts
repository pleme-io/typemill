#!/usr/bin/env bun

import { relative } from 'path';

// After move
const newFile = '/tmp/test/packages/@scope/features/lsp/src/client.ts';

// Target files
const pathUtilsFile = '/tmp/test/packages/server/src/core/file-operations/path-utils.js';
const loggerFile = '/tmp/test/packages/server/src/core/diagnostics/logger.js';
const configFile = '/tmp/test/packages/shared/config.js';

console.log('After moving to:', newFile);
console.log('\nCalculating relative paths to target files:\n');

const pathUtilsRelative = relative('/tmp/test/packages/@scope/features/lsp/src', pathUtilsFile);
console.log('Path utils:');
console.log('  Target:', pathUtilsFile);
console.log('  Relative:', pathUtilsRelative);

const loggerRelative = relative('/tmp/test/packages/@scope/features/lsp/src', loggerFile);
console.log('\nLogger:');
console.log('  Target:', loggerFile);
console.log('  Relative:', loggerRelative);

const configRelative = relative('/tmp/test/packages/@scope/features/lsp/src', configFile);
console.log('\nConfig:');
console.log('  Target:', configFile);
console.log('  Relative:', configRelative);
