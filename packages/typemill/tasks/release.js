#!/usr/bin/env node

/**
 * Release script for @goobits/typemill npm package
 *
 * Usage: node tasks/release.js [patch|minor|major]
 *
 * This script:
 * 1. Bumps the version in package.json
 * 2. Syncs version with root Cargo.toml (for Rust binary)
 * 3. Commits and tags the release
 * 4. Publishes to npm (binary downloaded via postinstall)
 */

const fs = require('fs')
const path = require('path')
const { spawnSync } = require('child_process')

const allowedBumps = new Set(['patch', 'minor', 'major', 'premajor', 'preminor', 'prepatch', 'prerelease'])
const bump = process.argv[2] || 'patch'
const targetList = process.env.TYPEMILL_TARGETS || 'aarch64-apple-darwin,aarch64-unknown-linux-gnu'
const targets = targetList
	.split(',')
	.map((entry) => entry.trim())
	.filter(Boolean)

const packageDir = path.join(__dirname, '..')
const repoRoot = path.join(packageDir, '..', '..')
const packageJsonPath = path.join(packageDir, 'package.json')
const cargoTomlPath = path.join(repoRoot, 'Cargo.toml')
const binaryName = process.platform === 'win32' ? 'mill.exe' : 'mill'

if (!allowedBumps.has(bump)) {
	console.error(`Unknown bump type: ${bump}`)
	console.error('Allowed: patch, minor, major, premajor, preminor, prepatch, prerelease')
	process.exit(1)
}

const run = (cmd, args, options = {}) => {
	console.log(`$ ${cmd} ${args.join(' ')}`)
	const result = spawnSync(cmd, args, { stdio: 'inherit', cwd: repoRoot, ...options })
	if (result.error) {
		throw result.error
	}
	if (typeof result.status === 'number' && result.status !== 0) {
		process.exit(result.status)
	}
	return result
}

const runCapture = (cmd, args, options = {}) => {
	const result = spawnSync(cmd, args, { encoding: 'utf8', stdio: 'pipe', cwd: repoRoot, ...options })
	if (result.error) {
		throw result.error
	}
	return result.stdout || ''
}

const readPackageJson = () => JSON.parse(fs.readFileSync(packageJsonPath, 'utf8'))
const writePackageJson = (data) => fs.writeFileSync(packageJsonPath, JSON.stringify(data, null, 2) + '\n')

const readCargoVersion = () => {
	const content = fs.readFileSync(cargoTomlPath, 'utf8')
	const match = /^version\s*=\s*"([^"]+)"/m.exec(content)
	return match ? match[1] : null
}

const writeCargoVersion = (version) => {
	let content = fs.readFileSync(cargoTomlPath, 'utf8')
	content = content.replace(/^(version\s*=\s*)"[^"]+"/m, `$1"${version}"`)
	fs.writeFileSync(cargoTomlPath, content)
}

const buildAndStageBinary = (target) => {
	run('cargo', ['build', '--release', '--target', target])
	const sourceBinary = path.join(repoRoot, 'target', target, 'release', binaryName)
	const destDir = path.join(packageDir, 'bin', target)
	const destBinary = path.join(destDir, binaryName)

	if (!fs.existsSync(sourceBinary)) {
		console.error(`❌ Missing binary for target ${target}: ${sourceBinary}`)
		process.exit(1)
	}

	fs.mkdirSync(destDir, { recursive: true })
	fs.copyFileSync(sourceBinary, destBinary)

	if (process.platform !== 'win32') {
		fs.chmodSync(destBinary, 0o755)
	}
}

const repoDirty = () => {
	const output = runCapture('git', ['status', '--porcelain'])
	return output.trim().length > 0
}

const parseVersion = (version) => {
	const match = /^(\d+)\.(\d+)\.(\d+)(-.+)?$/.exec(version)
	if (!match) return null
	return {
		major: Number.parseInt(match[1], 10),
		minor: Number.parseInt(match[2], 10),
		patch: Number.parseInt(match[3], 10),
		prerelease: match[4] || ''
	}
}

const bumpVersion = (base, bumpType) => {
	const parsed = parseVersion(base)
	if (!parsed) return null
	const next = { ...parsed, prerelease: '' }

	switch (bumpType) {
		case 'major':
			next.major += 1
			next.minor = 0
			next.patch = 0
			break
		case 'minor':
			next.minor += 1
			next.patch = 0
			break
		case 'patch':
			next.patch += 1
			break
		default:
			return null
	}

	return `${next.major}.${next.minor}.${next.patch}`
}

const getPublishedVersion = (packageName) => {
	const result = spawnSync('npm', ['view', packageName, 'version', '--json'], { encoding: 'utf8' })
	if (result.error || result.status !== 0) {
		return null
	}
	const raw = (result.stdout || '').trim()
	if (!raw) return null
	try {
		const parsed = JSON.parse(raw)
		return typeof parsed === 'string' ? parsed : null
	} catch {
		return raw
	}
}

const main = () => {
	console.log('\n📦 TypeMill Release Script\n')

	// Check for uncommitted changes
	if (repoDirty()) {
		console.error('❌ Working tree is dirty. Commit or stash changes before releasing.')
		process.exit(1)
	}

	if (targets.length === 0) {
		console.error('❌ No targets specified. Set TYPEMILL_TARGETS env var to a comma-separated list.')
		process.exit(1)
	}

	console.log(`Building binaries for: ${targets.join(', ')}`)
	targets.forEach(buildAndStageBinary)
	console.log('✅ Built and staged binaries')

	// Read current versions
	const pkg = readPackageJson()
	const cargoVersion = readCargoVersion()
	const publishedVersion = getPublishedVersion(pkg.name)

	console.log(`Current package.json version: ${pkg.version}`)
	console.log(`Current Cargo.toml version: ${cargoVersion}`)
	console.log(`Published npm version: ${publishedVersion || 'none'}`)

	// Calculate next version
	const baseVersion = cargoVersion || pkg.version
	const nextVersion = bumpVersion(baseVersion, bump)

	if (!nextVersion) {
		console.error(`❌ Could not calculate next version from ${baseVersion} with bump type ${bump}`)
		process.exit(1)
	}

	console.log(`\n🚀 Bumping to version: ${nextVersion}\n`)

	// Update package.json
	pkg.version = nextVersion
	writePackageJson(pkg)
	console.log('✅ Updated package.json')

	// Update Cargo.toml
	writeCargoVersion(nextVersion)
	console.log('✅ Updated Cargo.toml')

	// Git commit
	run('git', ['add', packageJsonPath, cargoTomlPath])
	run('git', ['commit', '-m', `chore: release v${nextVersion}`])
	console.log('✅ Committed version bump')

	// Git tag
	run('git', ['tag', '-a', `v${nextVersion}`, '-m', `Release v${nextVersion}`])
	console.log('✅ Created git tag')

	// Publish to npm (local)
	run('npm', ['publish', '--access', 'public'], { cwd: packageDir })
	console.log('✅ Published to npm')

	// Push (optional, for source history)
	run('git', ['push'])
	run('git', ['push', '--tags'])
	console.log('✅ Pushed to remote')

	console.log(`
🎉 Release v${nextVersion} initiated!

Published to npm from this machine.
`)
}

try {
	main()
} catch (error) {
	console.error('❌ Release failed:', error.message)
	process.exit(1)
}
