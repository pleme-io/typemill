import { readFile } from 'fs/promises';
import { join, isAbsolute } from 'path';
import { marked } from 'marked';
import hljs from 'highlight.js';

// Configure marked once
const renderer = {
	code({ text, lang }: { text: string; lang?: string }) {
		const validLang = !!(lang && hljs.getLanguage(lang));
		const highlighted = validLang
			? hljs.highlight(text, { language: lang }).value
			: hljs.highlight(text, { language: 'plaintext' }).value;

		return `<pre><code class="hljs${validLang ? ` language-${lang}` : ''}">${highlighted}</code></pre>`;
	}
};

marked.use({
	gfm: true,
	breaks: false,
	renderer
});

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

export async function readMarkdownFile(relativePath: string): Promise<{ content: string; metadata: Record<string, any> }> {
	// Security: Prevent directory traversal and arbitrary file read
	// Check for parent directory references and absolute paths (including Windows drive paths)
	if (relativePath.includes('..') || isAbsolute(relativePath) || relativePath.startsWith('/')) {
		throw new Error('Invalid path');
	}

	// Security: Only allow reading markdown files
	if (!relativePath.toLowerCase().endsWith('.md')) {
		throw new Error('Invalid path');
	}

	const root = process.env.WORKSPACE_ROOT || process.cwd();
	const filePath = join(root, relativePath);

	try {
		const fileContent = await readFile(filePath, 'utf-8');
		return parseFrontmatter(fileContent);
	} catch (e: any) {
		if (e.code === 'ENOENT') {
			throw new Error('File not found');
		}
		throw e;
	}
}

export async function renderMarkdown(markdown: string): Promise<string> {
	const result = marked.parse(markdown);
	if (result instanceof Promise) {
		return await result;
	}
	return result;
}
