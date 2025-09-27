#!/usr/bin/env node

/**
 * FUSE Setup CLI
 * Interactive setup helper for installing FUSE on different platforms
 */

import { execSync } from 'node:child_process';
import readline from 'node:readline';
import { checkFuseAvailability, printFuseStatus } from '../fs/fuse-detector.js';

const rl = readline.createInterface({
  input: process.stdin,
  output: process.stdout,
});

function prompt(question: string): Promise<string> {
  return new Promise((resolve) => {
    rl.question(question, resolve);
  });
}

async function setupLinuxFuse(): Promise<boolean> {
  console.log('\n📦 Setting up FUSE on Linux...\n');

  // Detect package manager
  let packageManager: string;
  let installCommand: string;

  try {
    execSync('which apt-get', { stdio: 'ignore' });
    packageManager = 'apt-get';
    installCommand = 'sudo apt-get install -y fuse fuse-dev';
  } catch {
    try {
      execSync('which yum', { stdio: 'ignore' });
      packageManager = 'yum';
      installCommand = 'sudo yum install -y fuse fuse-devel';
    } catch {
      try {
        execSync('which dnf', { stdio: 'ignore' });
        packageManager = 'dnf';
        installCommand = 'sudo dnf install -y fuse fuse-devel';
      } catch {
        try {
          execSync('which pacman', { stdio: 'ignore' });
          packageManager = 'pacman';
          installCommand = 'sudo pacman -S --noconfirm fuse2 fuse3';
        } catch {
          console.log('❌ Could not detect package manager');
          console.log('Please install FUSE manually:');
          console.log('  Debian/Ubuntu: sudo apt-get install fuse fuse-dev');
          console.log('  RedHat/CentOS: sudo yum install fuse fuse-devel');
          console.log('  Fedora: sudo dnf install fuse fuse-devel');
          console.log('  Arch: sudo pacman -S fuse2 fuse3');
          return false;
        }
      }
    }
  }

  console.log(`Detected package manager: ${packageManager}`);
  console.log(`Installation command: ${installCommand}`);

  const proceed = await prompt('\nProceed with installation? (y/n): ');
  if (proceed.toLowerCase() !== 'y') {
    console.log('Installation cancelled');
    return false;
  }

  try {
    console.log('\nInstalling FUSE packages...');
    execSync(installCommand, { stdio: 'inherit' });

    // Add user to fuse group
    console.log('\nAdding current user to fuse group...');
    try {
      execSync('sudo usermod -aG fuse $USER', { stdio: 'inherit' });
      console.log('✅ User added to fuse group');
      console.log('⚠️  You may need to logout and login again for group changes to take effect');
    } catch {
      console.log('⚠️  Could not add user to fuse group (group may not exist on this system)');
    }

    // Rebuild native modules
    console.log('\nRebuilding native modules...');
    try {
      execSync('npm rebuild @cocalc/fuse-native', { stdio: 'inherit' });
      console.log('✅ Native modules rebuilt successfully');
    } catch {
      console.log('⚠️  Could not rebuild native modules - you may need to reinstall the package');
    }

    return true;
  } catch (error) {
    console.error('❌ Installation failed:', error);
    return false;
  }
}

async function setupMacOSFuse(): Promise<boolean> {
  console.log('\n🍎 Setting up FUSE on macOS...\n');

  // Check for Homebrew
  try {
    execSync('which brew', { stdio: 'ignore' });
  } catch {
    console.log('❌ Homebrew not found');
    console.log('Please install Homebrew first:');
    console.log(
      '  /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
    );
    return false;
  }

  console.log('Installation options:');
  console.log('1. Install macFUSE via Homebrew (recommended)');
  console.log('2. Download macFUSE manually');
  console.log('3. Cancel');

  const choice = await prompt('\nSelect option (1-3): ');

  switch (choice) {
    case '1':
      try {
        console.log('\nInstalling macFUSE via Homebrew...');
        execSync('brew install --cask macfuse', { stdio: 'inherit' });

        console.log('\n⚠️  IMPORTANT: macFUSE requires a kernel extension');
        console.log('You may need to:');
        console.log('1. Open System Preferences > Security & Privacy');
        console.log('2. Click "Allow" next to the macFUSE developer');
        console.log('3. Restart your Mac');

        // Rebuild native modules
        console.log('\nRebuilding native modules...');
        try {
          execSync('npm rebuild @cocalc/fuse-native', { stdio: 'inherit' });
          console.log('✅ Native modules rebuilt successfully');
        } catch {
          console.log(
            '⚠️  Could not rebuild native modules - you may need to reinstall the package'
          );
        }

        return true;
      } catch (error) {
        console.error('❌ Installation failed:', error);
        return false;
      }

    case '2':
      console.log('\nPlease download and install macFUSE manually:');
      console.log('  1. Visit: https://osxfuse.github.io');
      console.log('  2. Download the latest macFUSE package');
      console.log('  3. Install the package');
      console.log('  4. Allow the kernel extension in System Preferences > Security & Privacy');
      console.log('  5. Restart your Mac');
      console.log('  6. Run: npm rebuild @cocalc/fuse-native');
      return false;

    default:
      console.log('Installation cancelled');
      return false;
  }
}

async function main() {
  console.log('🚀 CodeFlow Buddy FUSE Setup\n');

  // Check current status
  const status = checkFuseAvailability();
  printFuseStatus(status);

  if (status.available) {
    console.log('\n✨ FUSE is already properly configured!');
    rl.close();
    process.exit(0);
  }

  // Offer to install based on platform
  console.log('\nWould you like to set up FUSE for your system?');

  const setup = await prompt('Setup FUSE? (y/n): ');
  if (setup.toLowerCase() !== 'y') {
    console.log('Setup cancelled');
    rl.close();
    process.exit(0);
  }

  let success = false;

  switch (process.platform) {
    case 'linux':
      success = await setupLinuxFuse();
      break;

    case 'darwin':
      success = await setupMacOSFuse();
      break;

    case 'win32':
      console.log('\n❌ FUSE is not supported on Windows');
      console.log('Consider using WSL2 (Windows Subsystem for Linux) for FUSE support');
      console.log('Learn more: https://docs.microsoft.com/en-us/windows/wsl/install');
      break;

    default:
      console.log(`\n❌ Platform '${process.platform}' is not supported`);
      break;
  }

  if (success) {
    // Verify installation
    console.log('\n🔍 Verifying FUSE installation...');
    const newStatus = checkFuseAvailability();
    printFuseStatus(newStatus);

    if (newStatus.available) {
      console.log('\n✅ FUSE setup completed successfully!');
    } else {
      console.log('\n⚠️  FUSE setup completed but verification failed');
      console.log('You may need to restart your system or terminal');
    }
  }

  rl.close();
  process.exit(success ? 0 : 1);
}

// Run if called directly
if (require.main === module) {
  main().catch(console.error);
}

export { main as setupFuse };
