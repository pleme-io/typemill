import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';

export const load: PageLoad = async ({ params, fetch }) => {
	const { slug } = params;

	// Determine the file path based on the slug
	let filePath: string;

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
	}

	try {
		// Fetch the markdown content from our API route
		const response = await fetch(`/api/markdown?path=${encodeURIComponent(filePath)}`);

		if (!response.ok) {
			// Try with README.md appended
			const readmeResponse = await fetch(`/api/markdown?path=${encodeURIComponent(`docs/${slug}/README.md`)}`);
			if (!readmeResponse.ok) {
				throw error(404, `Page not found: ${slug}`);
			}
			const data = await readmeResponse.json();
			return {
				content: data.content,
				metadata: data.metadata || {},
				slug: slug || 'home',
				filePath: `docs/${slug}/README.md`
			};
		}

		const data = await response.json();
		return {
			content: data.content,
			metadata: data.metadata || {},
			slug: slug || 'home',
			filePath
		};
	} catch (e) {
		console.error('Error loading markdown:', e);
		throw error(500, 'Failed to load page content');
	}
};
