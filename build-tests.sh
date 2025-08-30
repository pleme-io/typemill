#!/bin/bash
# Build script to compile TypeScript files for unit test imports

echo "Building TypeScript files for unit tests..."

# Create a test-build directory for test outputs
mkdir -p test-build

# Find all TypeScript files and compile them individually with bun
# Use --format esm to prevent bundling, output to test-build directory
find src -name "*.ts" -exec bun build --format esm --target node --outdir test-build {} \;

echo "Unit test build complete. Tests can now import .js files from test-build/."