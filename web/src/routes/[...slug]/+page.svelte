<script lang="ts">
	import type { PageData } from './$types';
	import { marked } from 'marked';
	import { onMount } from 'svelte';
	import hljs from 'highlight.js/lib/core';
	import javascript from 'highlight.js/lib/languages/javascript';
	import typescript from 'highlight.js/lib/languages/typescript';
	import rust from 'highlight.js/lib/languages/rust';
	import python from 'highlight.js/lib/languages/python';
	import bash from 'highlight.js/lib/languages/bash';
	import json from 'highlight.js/lib/languages/json';
	import yaml from 'highlight.js/lib/languages/yaml';
	import 'highlight.js/styles/github-dark.css';

	export let data: PageData;

	let htmlContent = '';

	// Register languages
	hljs.registerLanguage('javascript', javascript);
	hljs.registerLanguage('typescript', typescript);
	hljs.registerLanguage('rust', rust);
	hljs.registerLanguage('python', python);
	hljs.registerLanguage('bash', bash);
	hljs.registerLanguage('json', json);
	hljs.registerLanguage('yaml', yaml);

	// Extract title from metadata or generate from slug
	$: title = data.metadata?.title || formatTitle(data.slug);
	$: description = data.metadata?.description || '';

	function formatTitle(slug: string): string {
		if (slug === 'home') return 'TypeMill';
		return slug
			.split('/')
			.map(part => part.split('-').map(word =>
				word.charAt(0).toUpperCase() + word.slice(1)
			).join(' '))
			.join(' / ');
	}

	// Generate breadcrumbs from slug
	$: breadcrumbs = generateBreadcrumbs(data.slug);

	function generateBreadcrumbs(slug: string) {
		if (slug === 'home') return [{ label: 'Home', href: '/' }];

		const parts = slug.split('/');
		const crumbs = [{ label: 'Home', href: '/' }];

		let path = '';
		for (const part of parts) {
			path += (path ? '/' : '') + part;
			crumbs.push({
				label: part.split('-').map(w => w.charAt(0).toUpperCase() + w.slice(1)).join(' '),
				href: `/${path}`
			});
		}

		return crumbs;
	}

	// Convert markdown to HTML with syntax highlighting
	$: {
		marked.setOptions({
			gfm: true,
			breaks: false,
		});
		const parsed = marked.parse(data.content);
		if (parsed instanceof Promise) {
			parsed.then((res) => (htmlContent = res));
		} else {
			htmlContent = parsed;
		}
	}

	// Apply syntax highlighting after mount
	onMount(() => {
		document.querySelectorAll('pre code').forEach((block) => {
			hljs.highlightElement(block as HTMLElement);
		});
	});
</script>

<svelte:head>
	<title>{title} | TypeMill</title>
	{#if description}
		<meta name="description" content={description} />
	{/if}
</svelte:head>

<div class="doc-page">
	<!-- Breadcrumbs -->
	<nav class="breadcrumbs">
		{#each breadcrumbs as crumb, i (crumb.href)}
			{#if i > 0}<span class="separator">/</span>{/if}
			{#if i === breadcrumbs.length - 1}
				<span class="current">{crumb.label}</span>
			{:else}
				<a href={crumb.href}>{crumb.label}</a>
			{/if}
		{/each}
	</nav>

	<!-- Markdown Content -->
	<article class="markdown-content">
		{@html htmlContent}
	</article>
</div>

<style>
	.doc-page {
		max-width: 900px;
		margin: 0 auto;
		padding: 2rem 1rem;
	}

	.breadcrumbs {
		font-size: 0.875rem;
		color: #6b7280;
		margin-bottom: 2rem;
		padding-bottom: 1rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.breadcrumbs a {
		color: #3b82f6;
		text-decoration: none;
	}

	.breadcrumbs a:hover {
		text-decoration: underline;
	}

	.breadcrumbs .separator {
		margin: 0 0.5rem;
		color: #d1d5db;
	}

	.breadcrumbs .current {
		color: #111827;
		font-weight: 500;
	}

	/* Markdown Content Styles */
	.markdown-content {
		line-height: 1.7;
		color: #1f2937;
	}

	.markdown-content :global(h1) {
		font-size: 2.25rem;
		font-weight: 700;
		margin-top: 0;
		margin-bottom: 1.5rem;
		line-height: 1.2;
		color: #111827;
	}

	.markdown-content :global(h2) {
		font-size: 1.875rem;
		font-weight: 600;
		margin-top: 3rem;
		margin-bottom: 1rem;
		line-height: 1.3;
		color: #111827;
		border-bottom: 2px solid #e5e7eb;
		padding-bottom: 0.5rem;
	}

	.markdown-content :global(h3) {
		font-size: 1.5rem;
		font-weight: 600;
		margin-top: 2rem;
		margin-bottom: 0.75rem;
		line-height: 1.4;
		color: #374151;
	}

	.markdown-content :global(h4) {
		font-size: 1.25rem;
		font-weight: 600;
		margin-top: 1.5rem;
		margin-bottom: 0.5rem;
		color: #4b5563;
	}

	.markdown-content :global(h5),
	.markdown-content :global(h6) {
		font-size: 1.125rem;
		font-weight: 600;
		margin-top: 1.5rem;
		margin-bottom: 0.5rem;
		color: #6b7280;
	}

	.markdown-content :global(p) {
		margin-bottom: 1.25rem;
	}

	.markdown-content :global(a) {
		color: #2563eb;
		text-decoration: none;
		border-bottom: 1px solid transparent;
		transition: border-color 0.2s;
	}

	.markdown-content :global(a:hover) {
		border-bottom-color: #2563eb;
	}

	.markdown-content :global(ul),
	.markdown-content :global(ol) {
		margin-bottom: 1.25rem;
		padding-left: 1.5rem;
	}

	.markdown-content :global(li) {
		margin-bottom: 0.5rem;
	}

	.markdown-content :global(li > ul),
	.markdown-content :global(li > ol) {
		margin-top: 0.5rem;
		margin-bottom: 0.5rem;
	}

	.markdown-content :global(code) {
		background-color: #f3f4f6;
		padding: 0.2rem 0.4rem;
		border-radius: 0.25rem;
		font-size: 0.875em;
		font-family: 'Consolas', 'Monaco', 'Courier New', monospace;
		color: #dc2626;
	}

	.markdown-content :global(pre) {
		background-color: #1f2937;
		padding: 1.25rem;
		border-radius: 0.5rem;
		overflow-x: auto;
		margin-bottom: 1.5rem;
		border: 1px solid #374151;
	}

	.markdown-content :global(pre code) {
		background-color: transparent;
		padding: 0;
		color: inherit;
		font-size: 0.875rem;
		line-height: 1.6;
	}

	.markdown-content :global(blockquote) {
		border-left: 4px solid #3b82f6;
		padding-left: 1rem;
		margin-left: 0;
		margin-right: 0;
		margin-bottom: 1.25rem;
		color: #4b5563;
		font-style: italic;
		background-color: #f9fafb;
		padding: 1rem 1rem 1rem 1.5rem;
		border-radius: 0.25rem;
	}

	.markdown-content :global(blockquote p) {
		margin-bottom: 0.5rem;
	}

	.markdown-content :global(blockquote p:last-child) {
		margin-bottom: 0;
	}

	.markdown-content :global(table) {
		width: 100%;
		border-collapse: collapse;
		margin-bottom: 1.5rem;
		font-size: 0.9375rem;
	}

	.markdown-content :global(thead) {
		background-color: #f9fafb;
	}

	.markdown-content :global(th) {
		padding: 0.75rem 1rem;
		text-align: left;
		font-weight: 600;
		border-bottom: 2px solid #e5e7eb;
		color: #111827;
	}

	.markdown-content :global(td) {
		padding: 0.75rem 1rem;
		border-bottom: 1px solid #e5e7eb;
	}

	.markdown-content :global(tr:last-child td) {
		border-bottom: none;
	}

	.markdown-content :global(tr:hover) {
		background-color: #f9fafb;
	}

	.markdown-content :global(hr) {
		border: none;
		border-top: 2px solid #e5e7eb;
		margin: 2rem 0;
	}

	.markdown-content :global(img) {
		max-width: 100%;
		height: auto;
		border-radius: 0.5rem;
		margin: 1.5rem 0;
	}

	.markdown-content :global(strong) {
		font-weight: 600;
		color: #111827;
	}

	.markdown-content :global(em) {
		font-style: italic;
	}

	/* Task lists */
	.markdown-content :global(input[type="checkbox"]) {
		margin-right: 0.5rem;
	}

	/* Responsive */
	@media (max-width: 768px) {
		.doc-page {
			padding: 1rem 0.75rem;
		}

		.markdown-content :global(h1) {
			font-size: 1.875rem;
		}

		.markdown-content :global(h2) {
			font-size: 1.5rem;
		}

		.markdown-content :global(h3) {
			font-size: 1.25rem;
		}

		.markdown-content :global(pre) {
			padding: 1rem;
			margin-left: -0.75rem;
			margin-right: -0.75rem;
			border-radius: 0;
		}
	}
</style>
