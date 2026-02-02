#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const binaryName = process.platform === 'win32' ? 'mill.exe' : 'mill';

const PLATFORM_MAP = {
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
};

const platformKey = `${process.platform}-${process.arch}`;
const target = PLATFORM_MAP[platformKey];
const binaryPath = target ? path.join(__dirname, target, binaryName) : null;

if (!binaryPath || !fs.existsSync(binaryPath)) {
  console.error('TypeMill binary not found.');
  console.error('');
  console.error('This can happen if:');
  console.error('  1. Your platform is not supported by this package build');
  console.error('  2. The release was built without your target');
  console.error('');
  console.error('Try rebuilding and republishing the package for your target.');
  console.error('Or build from source: cargo install --git https://github.com/goobits/typemill');
  process.exit(1);
}

// Pass all arguments to the binary
const args = process.argv.slice(2);

const child = spawn(binaryPath, args, {
  stdio: 'inherit',
  env: process.env,
});

child.on('error', (error) => {
  console.error('Failed to start TypeMill:', error.message);
  process.exit(1);
});

child.on('close', (code) => {
  process.exit(code || 0);
});
