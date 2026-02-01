#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');

const binaryName = process.platform === 'win32' ? 'mill.exe' : 'mill';
const binaryPath = path.join(__dirname, binaryName);

if (!fs.existsSync(binaryPath)) {
  console.error('TypeMill binary not found.');
  console.error('');
  console.error('This can happen if:');
  console.error('  1. The postinstall script failed to download the binary');
  console.error('  2. Your platform is not supported');
  console.error('');
  console.error('Try reinstalling: npm install -g typemill');
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
