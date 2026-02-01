import { json, error } from '@sveltejs/kit';
import { readMarkdownFile } from '$lib/server/markdown';
import type { RequestHandler } from './$types';

export const GET: RequestHandler = async ({ url }) => {
	const path = url.searchParams.get('path');

	if (!path) {
		throw error(400, 'Path parameter is required');
	}

	try {
		const { content: markdownContent, metadata } = await readMarkdownFile(path);

		return json({
			content: markdownContent,
			metadata,
			path
		});
	} catch (e: any) {
		if (e.message === 'File not found') {
			throw error(404, `File not found: ${path}`);
		}
		if (e.message === 'Invalid path') {
			throw error(400, 'Invalid path');
		}
		console.error('Error reading markdown file:', e);
		throw error(500, 'Failed to read file');
	}
};
