<script lang="ts">
	import '../app.css';
	import { generateNavigation } from '$lib/navigation';
	import { MermaidHydrator, CodeCopyHydrator } from '@goobits/docs-engine/components';

	let { children } = $props();

	let navOpen = $state(false);
	const navigation = generateNavigation();

	function toggleNav() {
		navOpen = !navOpen;
	}
</script>

<!-- Hydrators for interactive documentation features -->
<MermaidHydrator />
<CodeCopyHydrator />

<svelte:head>
	<title>TypeMill - Rust MCP Server for AI Coding Assistants</title>
	<meta name="description" content="Pure Rust MCP server bridging Language Server Protocol (LSP) to AI coding assistants with comprehensive tools for navigation, refactoring, and analysis" />
</svelte:head>

<div class="app">
	<header>
		<div class="header-content">
			<div class="logo">
				<a href="/">🤖 <span>TypeMill</span></a>
			</div>
			<button class="nav-toggle" onclick={toggleNav} aria-label="Toggle navigation">
				{navOpen ? '✕' : '☰'}
			</button>
			<nav class:open={navOpen}>
				<a href="/">Home</a>
				<a href="/docs">Docs</a>
				<a href="/tools">Tools</a>
				<a href="/architecture">Architecture</a>
				<a href="/contributing">Contributing</a>
				<a href="https://github.com/goobits/typemill" target="_blank" rel="noopener">GitHub</a>
			</nav>
		</div>
	</header>

	<main>
		{@render children?.()}
	</main>

	<footer>
		<div class="footer-content">
			<p>© 2025 TypeMill. Open source under the MIT/Apache-2.0 License.</p>
			<div class="footer-links">
				<a href="https://github.com/goobits/typemill/issues" target="_blank" rel="noopener">Issues</a>
				<a href="https://github.com/goobits/typemill/discussions" target="_blank" rel="noopener">Discussions</a>
			</div>
		</div>
	</footer>
</div>
