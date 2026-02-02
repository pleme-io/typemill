import { error } from '@sveltejs/kit';
import { readMarkdownFile, renderMarkdown, getProjectRoot } from '$lib/server/markdown';
import { readdir } from 'fs/promises';
import { join } from 'path';
import type { PageServerLoad, EntryGenerator } from './$types';

// Automatically discover all doc routes by scanning the docs directory
export const entries: EntryGenerator = async () => {
	const root = await getProjectRoot();
	const routes: Array<{ slug: string }> = [];

	// Add top-level routes that exist
	routes.push({ slug: 'docs' });
	routes.push({ slug: 'contributing' });

	// Recursively scan docs directory
	async function scanDir(dir: string, prefix: string = '') {
		try {
			const dirEntries = await readdir(join(root, 'docs', dir), { withFileTypes: true });

			// Check if directory has a README.md
			const hasReadme = dirEntries.some(e => e.isFile() && e.name.toLowerCase() === 'readme.md');

			for (const entry of dirEntries) {
				if (entry.isDirectory()) {
					const slug = prefix ? `${prefix}/${entry.name}` : entry.name;
					// Recursively scan subdirectory (which will add routes if it has README)
					await scanDir(join(dir, entry.name), slug);
				} else if (entry.isFile() && entry.name.endsWith('.md')) {
					const baseName = entry.name.replace(/\.md$/, '');
					// Skip README files - only add as directory route if README exists
					if (baseName.toLowerCase() === 'readme') {
						// Add directory route since it has a README
						if (prefix) {
							routes.push({ slug: prefix });
						}
						continue;
					}
					const slug = prefix ? `${prefix}/${baseName}` : baseName;
					routes.push({ slug });
				}
			}
		} catch {
			// Directory doesn't exist or isn't readable
		}
	}

	// Scan the docs directory
	await scanDir('');

	return routes;
};

export const load: PageServerLoad = async ({ params }) => {
	// Strip .md extension from slug if present (handles markdown links)
	let slug = params.slug;
	if (slug?.endsWith('.md')) {
		slug = slug.slice(0, -3);
	}

	// Determine the file path based on the slug
	let filePath: string;
	let potentialReadmePath: string | null = null;

	if (!slug || slug === '') {
		// Home route
		filePath = 'README.md';
	} else if (slug === 'docs') {
		filePath = 'docs/README.md';
	} else if (slug === 'contributing') {
		filePath = 'contributing.md';
	} else {
		// Try docs directory first
		filePath = `docs/${slug}.md`;
		potentialReadmePath = `docs/${slug}/README.md`;
	}

	try {
		let rawData;

		try {
			rawData = await readMarkdownFile(filePath);
		} catch (e: any) {
			if (e.message === 'File not found' && potentialReadmePath) {
				// Try with README.md appended
				try {
					rawData = await readMarkdownFile(potentialReadmePath);
					filePath = potentialReadmePath;
				} catch (innerE: any) {
					// If specifically the second attempt failed, throw 404
					if (innerE.message === 'File not found') {
						throw error(404, `Page not found: ${slug}`);
					}
					throw innerE;
				}
			} else if (e.message === 'File not found') {
				throw error(404, `Page not found: ${slug}`);
			} else {
				throw e;
			}
		}

		const htmlContent = await renderMarkdown(rawData.content);

		return {
			htmlContent,
			metadata: rawData.metadata || {},
			slug: slug || 'home',
			filePath
		};
	} catch (e: any) {
		// If it's already an HTTP error (from error()), rethrow it
		if (e.status) {
			throw e;
		}
		console.error('Error loading markdown:', e);
		throw error(500, 'Failed to load page content');
	}
};
