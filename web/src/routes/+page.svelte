<div class="hero">
	<h1>🤖 TypeMill</h1>
	<p>Pure Rust MCP server bridging Language Server Protocol (LSP) to AI coding assistants</p>
	<div class="btn-group">
		<a href="#quick-start" class="btn">Quick Start</a>
		<a href="/docs" class="btn btn-secondary">Documentation</a>
		<a href="https://github.com/goobits/typemill" class="btn btn-secondary" target="_blank" rel="noopener">GitHub</a>
	</div>
</div>

<div class="features">
	<div class="feature">
		<h3>🎯 Safe Refactoring</h3>
		<p>Unified dryRun API with preview mode by default. Explicit opt-in required for execution with automatic rollback on failure.</p>
	</div>
	<div class="feature">
		<h3>🔍 LSP Integration</h3>
		<p>Native language server support for precise code intelligence across TypeScript, Rust, and Python projects.</p>
	</div>
	<div class="feature">
		<h3>⚡ Rust Performance</h3>
		<p>Zero-cost abstractions, memory safety, and async I/O for maximum performance and reliability.</p>
	</div>
	<div class="feature">
		<h3>🔄 Comprehensive Updates</h3>
		<p>Automatic import updates and cross-file reference tracking for seamless refactoring.</p>
	</div>
	<div class="feature">
		<h3>🐳 Production Ready</h3>
		<p>WebSocket server, JWT authentication, multi-tenant isolation, and Docker support.</p>
	</div>
	<div class="feature">
		<h3>🛠️ 28 MCP Tools</h3>
		<p>Navigation, refactoring, analysis, workspace operations, and batch processing tools.</p>
	</div>
</div>

<section id="quick-start">
	<h2>Quick Start</h2>
	<p>Get TypeMill running in 2 minutes:</p>
	<pre><code># Install (recommended method)
curl -fsSL https://raw.githubusercontent.com/goobits/mill/main/install.sh | bash

# Alternative: Build from source
cargo install mill --locked

# Auto-detect languages and configure
mill setup

# Start the server
mill start

# Verify it's running
mill status</code></pre>
</section>

<section>
	<h2>Connect Your AI Assistant</h2>
	<p>Add TypeMill to your MCP configuration (e.g., Claude Desktop):</p>
	<pre><code>{`{
  "mcpServers": {
    "mill": {
      "command": "mill",
      "args": ["start"]
    }
  }
}`}</code></pre>
</section>

<section>
	<h2>Example Commands</h2>
	<p>Ask your AI assistant to:</p>
	<pre><code>"Find the definition of main in src/main.rs"
"Show me all references to the Config type"
"Rename the function oldName to newName"
"Analyze code quality in src/app.rs and show suggestions"</code></pre>
</section>

<section>
	<h2>Available Tools</h2>
	<table>
		<thead>
			<tr>
				<th>Category</th>
				<th>Tools</th>
				<th>Description</th>
			</tr>
		</thead>
		<tbody>
			<tr>
				<td><span class="badge">Inspect &amp; Search</span></td>
				<td>2 tools</td>
				<td>inspect_code, search_code</td>
			</tr>
			<tr>
				<td><span class="badge">Change</span></td>
				<td>4 tools</td>
				<td>rename_all, relocate, prune, refactor</td>
			</tr>
			<tr>
				<td><span class="badge">Workspace</span></td>
				<td>1 tool</td>
				<td>workspace (actions: create_package, extract_dependencies, update_members, find_replace, verify_project)</td>
			</tr>
			<tr>
				<td><span class="badge-success">System</span></td>
				<td>1 tool</td>
				<td>Health checks and diagnostics</td>
			</tr>
		</tbody>
	</table>
	<p style="text-align: center; margin-top: 2rem;">
		<a href="/tools" class="btn">Explore All Tools</a>
	</p>
</section>

<section>
	<h2>Language Support</h2>
	<table>
		<thead>
			<tr>
				<th>Language</th>
				<th>Extensions</th>
				<th>LSP Server</th>
				<th>Refactoring</th>
			</tr>
		</thead>
		<tbody>
			<tr>
				<td>TypeScript/JavaScript</td>
				<td>ts, tsx, js, jsx</td>
				<td>typescript-language-server</td>
				<td><span class="badge-success">Full ✅</span></td>
			</tr>
			<tr>
				<td>Rust</td>
				<td>rs</td>
				<td>rust-analyzer</td>
				<td><span class="badge-success">Full ✅</span></td>
			</tr>
			<tr>
				<td>Python</td>
				<td>py</td>
				<td>pylsp</td>
				<td><span class="badge-success">Full ✅</span></td>
			</tr>
		</tbody>
	</table>
</section>
