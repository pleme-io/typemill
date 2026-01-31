import { json, error } from '@sveltejs/kit';
import { readFile } from 'fs/promises';
import { join, isAbsolute } from 'path';
import type { RequestHandler } from './$types';

// Parse frontmatter from markdown content
function parseFrontmatter(content: string): { metadata: Record<string, any>; content: string } {
	const frontmatterRegex = /^---\s*\n([\s\S]*?)\n---\s*\n([\s\S]*)$/;
	const match = content.match(frontmatterRegex);

	if (!match) {
		return { metadata: {}, content };
	}

	const [, frontmatterStr, mainContent] = match;
	const metadata: Record<string, any> = {};

	// Parse YAML-like frontmatter
	frontmatterStr.split('\n').forEach(line => {
		const colonIndex = line.indexOf(':');
		if (colonIndex > 0) {
			const key = line.slice(0, colonIndex).trim();
			const value = line.slice(colonIndex + 1).trim();
			metadata[key] = value.replace(/^["']|["']$/g, ''); // Remove quotes
		}
	});

	return { metadata, content: mainContent };
}

export const GET: RequestHandler = async ({ url }) => {
	const path = url.searchParams.get('path');

	if (!path) {
		throw error(400, 'Path parameter is required');
	}

	// Security: Prevent directory traversal
	// Check for parent directory references and absolute paths (including Windows drive paths)
	if (path.includes('..') || isAbsolute(path) || path.startsWith('/')) {
		throw error(400, 'Invalid path');
	}

	try {
		// Read markdown file from workspace root
		const filePath = join('/workspace', path);
		const content = await readFile(filePath, 'utf-8');

		// Parse frontmatter
		const { metadata, content: markdownContent } = parseFrontmatter(content);

		return json({
			content: markdownContent,
			metadata,
			path
		});
	} catch (e: any) {
		if (e.code === 'ENOENT') {
			throw error(404, `File not found: ${path}`);
		}
		console.error('Error reading markdown file:', e);
		throw error(500, 'Failed to read file');
	}
};
