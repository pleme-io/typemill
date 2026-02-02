import adapter from '@sveltejs/adapter-cloudflare';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { mdsvex } from 'mdsvex';
import rehypeSlug from 'rehype-slug';
import rehypeAutolinkHeadings from 'rehype-autolink-headings';
import remarkGfm from 'remark-gfm';
import {
	calloutsPlugin,
	mermaidPlugin,
	linksPlugin,
	codeHighlightPlugin,
	tabsPlugin,
	filetreePlugin
} from '@goobits/docs-engine/plugins';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// File extensions to process
	extensions: ['.svelte', '.md', '.svx'],

	// Configure preprocessors
	preprocess: [
		vitePreprocess(),
		mdsvex({
			extensions: ['.md', '.svx'],
			remarkPlugins: [
				remarkGfm,
				calloutsPlugin(), // NOTE, TIP, WARNING, etc.
				mermaidPlugin(), // Architecture diagrams
				tabsPlugin(), // Tabbed code examples
				filetreePlugin(), // Interactive file trees
				linksPlugin({
					// Automatic .md link handling
					topLevelFiles: ['README', 'CLAUDE', 'LICENSE', 'contributing']
				}),
				codeHighlightPlugin({
					// Enhanced code blocks with copy buttons
					theme: 'github-dark',
					showLineNumbers: false,
					showCopyButton: true
				})
			],
			rehypePlugins: [rehypeSlug, [rehypeAutolinkHeadings, { behavior: 'wrap' }]]
		})
	],

	kit: {
		adapter: adapter(),
		prerender: {
			// Don't crawl links - only render routes from entries()
			crawl: false,
			// Handle errors gracefully
			handleHttpError: ({ path, message }) => {
				// Ignore 404s for .md links (they're handled by the load function)
				if (path.endsWith('.md')) {
					return;
				}
				// Log other errors but don't fail the build
				console.warn(`Prerender warning: ${path} - ${message}`);
			}
		}
	}
};

export default config;
