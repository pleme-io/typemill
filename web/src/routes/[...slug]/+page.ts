import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types';

interface MarkdownModule {
	default: any;
	metadata?: {
		title?: string;
		description?: string;
		[key: string]: any;
	};
}

// Import all markdown files from the docs directory
const markdownFiles = import.meta.glob('/workspace/docs/**/*.md', { eager: false });
const rootMarkdownFiles = import.meta.glob('/workspace/*.md', { eager: false });

// Combine both glob patterns
const allMarkdownFiles = { ...markdownFiles, ...rootMarkdownFiles };

export const load: PageLoad = async ({ params }) => {
	const { slug } = params;

	// Handle special routes
	let filePath: string;

	if (!slug || slug === '') {
		// Home route -> /workspace/README.md
		filePath = '/workspace/README.md';
	} else if (slug === 'docs') {
		// /docs -> /workspace/docs/README.md
		filePath = '/workspace/docs/README.md';
	} else if (slug === 'contributing') {
		// /contributing -> /workspace/contributing.md
		filePath = '/workspace/contributing.md';
	} else {
		// Try to find the markdown file in docs directory
		// Convert URL slug to file path (e.g., "tools/navigation" -> "/workspace/docs/tools/navigation.md")
		filePath = `/workspace/docs/${slug}.md`;
	}

	// Check if file exists in our glob
	if (!allMarkdownFiles[filePath]) {
		// Also check for README.md in the directory
		const readmePath = `/workspace/docs/${slug}/README.md`;
		if (allMarkdownFiles[readmePath]) {
			filePath = readmePath;
		} else {
			throw error(404, `Page not found: ${slug}`);
		}
	}

	try {
		// Dynamically import the markdown file
		const module = await allMarkdownFiles[filePath]() as MarkdownModule;

		return {
			component: module.default,
			metadata: module.metadata || {},
			slug: slug || 'home',
			filePath
		};
	} catch (e) {
		console.error('Error loading markdown:', e);
		throw error(500, 'Failed to load page content');
	}
};
