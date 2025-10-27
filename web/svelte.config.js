import adapter from '@sveltejs/adapter-auto';
import { vitePreprocess } from '@sveltejs/vite-plugin-svelte';
import { mdsvex } from 'mdsvex';
import rehypeSlug from 'rehype-slug';
import rehypeAutolinkHeadings from 'rehype-autolink-headings';
import remarkGfm from 'remark-gfm';

/** @type {import('@sveltejs/kit').Config} */
const config = {
	// File extensions to process
	extensions: ['.svelte', '.md', '.svx'],

	// Configure preprocessors
	preprocess: [
		vitePreprocess(),
		mdsvex({
			extensions: ['.md', '.svx'],
			remarkPlugins: [remarkGfm],
			rehypePlugins: [
				rehypeSlug,
				[rehypeAutolinkHeadings, { behavior: 'wrap' }]
			],
			highlight: {
				highlighter: async (code, lang = 'text') => {
					const { codeToHtml } = await import('shiki');
					const html = await codeToHtml(code, {
						lang,
						theme: 'github-dark'
					});
					return `{@html \`${html}\` }`;
				}
			}
		})
	],

	kit: {
		adapter: adapter()
	}
};

export default config;
