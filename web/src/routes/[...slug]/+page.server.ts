import { error } from '@sveltejs/kit';
import { readMarkdownFile, renderMarkdown } from '$lib/server/markdown';
import type { PageServerLoad } from './$types';

export const load: PageServerLoad = async ({ params }) => {
	const { slug } = params;

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
	} else if (slug === 'DEVELOPMENT') {
		filePath = 'docs/DEVELOPMENT.md';
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
