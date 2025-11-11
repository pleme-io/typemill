import adapter from '@sveltejs/adapter-auto';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { mdsvex } from 'mdsvex';
import rehypeSlug from 'rehype-slug';
import rehypeAutolinkHeadings from 'rehype-autolink-headings';
import remarkGfm from 'remark-gfm';
import {
	calloutsPlugin,
	mermaidPlugin,
	linksPlugin,
	codeHighlightPlugin
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
		adapter: adapter()
	}
};

export default config;
