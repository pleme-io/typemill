#!/usr/bin/env node

const https = require('https');
const fs = require('fs');
const path = require('path');
const { execSync } = require('child_process');
const zlib = require('zlib');

const VERSION = require('../package.json').version;
const REPO = 'goobits/typemill';

// Map Node.js platform/arch to Rust target triples
const PLATFORM_MAP = {
  'darwin-x64': 'x86_64-apple-darwin',
  'darwin-arm64': 'aarch64-apple-darwin',
  'linux-x64': 'x86_64-unknown-linux-gnu',
  'linux-arm64': 'aarch64-unknown-linux-gnu',
  'win32-x64': 'x86_64-pc-windows-msvc',
  'win32-arm64': 'aarch64-pc-windows-msvc',
};

function getPlatformKey() {
  return `${process.platform}-${process.arch}`;
}

function getBinaryName() {
  return process.platform === 'win32' ? 'mill.exe' : 'mill';
}

function getDownloadUrl(target) {
  const ext = process.platform === 'win32' ? '.zip' : '.tar.gz';
  return `https://github.com/${REPO}/releases/download/v${VERSION}/typemill-v${VERSION}-${target}${ext}`;
}

function downloadFile(url) {
  return new Promise((resolve, reject) => {
    const makeRequest = (url, redirectCount = 0) => {
      if (redirectCount > 5) {
        reject(new Error('Too many redirects'));
        return;
      }

      const protocol = url.startsWith('https') ? https : require('http');
      protocol.get(url, (response) => {
        if (response.statusCode >= 300 && response.statusCode < 400 && response.headers.location) {
          makeRequest(response.headers.location, redirectCount + 1);
          return;
        }

        if (response.statusCode !== 200) {
          reject(new Error(`Failed to download: ${response.statusCode}`));
          return;
        }

        const chunks = [];
        response.on('data', (chunk) => chunks.push(chunk));
        response.on('end', () => resolve(Buffer.concat(chunks)));
        response.on('error', reject);
      }).on('error', reject);
    };

    makeRequest(url);
  });
}

async function extractTarGz(buffer, destDir) {
  const tar = require('tar');
  const tmpFile = path.join(destDir, 'tmp.tar.gz');

  fs.writeFileSync(tmpFile, buffer);

  await tar.x({
    file: tmpFile,
    cwd: destDir,
  });

  fs.unlinkSync(tmpFile);
}

async function extractZip(buffer, destDir) {
  const AdmZip = require('adm-zip');
  const zip = new AdmZip(buffer);
  zip.extractAllTo(destDir, true);
}

async function main() {
  const platformKey = getPlatformKey();
  const target = PLATFORM_MAP[platformKey];

  if (!target) {
    console.error(`Unsupported platform: ${platformKey}`);
    console.error('Supported platforms:', Object.keys(PLATFORM_MAP).join(', '));
    process.exit(1);
  }

  const binDir = path.join(__dirname, '..', 'bin');
  const binaryPath = path.join(binDir, getBinaryName());

  // Skip if binary already exists (for development)
  if (fs.existsSync(binaryPath)) {
    console.log('TypeMill binary already exists, skipping download.');
    return;
  }

  console.log(`Downloading TypeMill v${VERSION} for ${target}...`);

  const url = getDownloadUrl(target);

  try {
    const buffer = await downloadFile(url);

    if (!fs.existsSync(binDir)) {
      fs.mkdirSync(binDir, { recursive: true });
    }

    if (process.platform === 'win32') {
      await extractZip(buffer, binDir);
    } else {
      await extractTarGz(buffer, binDir);
    }

    // Make binary executable on Unix
    if (process.platform !== 'win32') {
      fs.chmodSync(binaryPath, 0o755);
    }

    console.log('TypeMill installed successfully!');
    console.log(`Run 'npx typemill --help' to get started.`);
  } catch (error) {
    // If download fails, provide helpful message
    if (error.message.includes('404') || error.message.includes('Failed to download')) {
      console.warn('\n');
      console.warn('Pre-built binary not available for this platform.');
      console.warn('You can build from source with: cargo install --git https://github.com/goobits/typemill');
      console.warn('\n');
      console.warn('Or check releases at: https://github.com/goobits/typemill/releases');
      console.warn('\n');
      // Don't fail the install - allow manual setup
      process.exit(0);
    }
    throw error;
  }
}

main().catch((error) => {
  console.error('Failed to install TypeMill:', error.message);
  process.exit(1);
});
