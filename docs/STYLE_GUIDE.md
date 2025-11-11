# TypeMill Documentation Style Guide

> Last updated: 2025-11-11
> Status: Active - All documentation must follow these guidelines

This guide ensures consistency, clarity, and professionalism across all TypeMill documentation. Inspired by Apple's minimal aesthetics and Medium's approachable clarity.

---

## Core Principles

1. **Honest & Factual** - No marketing fluff, just accurate information
2. **Concise & Dense** - Maximum information, minimum words
3. **Easy to Grok** - Accessible to beginners, detailed for power users
4. **Well-Organized** - Logical structure, clear hierarchy
5. **Confident Minimal** - Active voice, present tense, direct statements

---

## Voice & Tone

### Product vs CLI Naming

**TypeMill** (capital T, capital M)
- Product name in prose
- Used when referring to the project, system, or capabilities
- Examples: "TypeMill provides 29 tools", "TypeMill architecture"

**mill** (lowercase, in backticks)
- CLI command in code examples
- Used when showing actual commands
- Examples: "`mill setup`", "`mill tool rename`"

**Never**: Mill, typemill, type-mill, Mill/mill mixed randomly

### Active Voice

✅ **Good:**
- "TypeMill scans your project"
- "The handler processes the request"
- "Run `mill setup` to configure"

❌ **Avoid:**
- "Your project is scanned by TypeMill"
- "The request is processed by the handler"
- "`mill setup` can be run to configure"

### Present Tense

✅ **Good:**
- "TypeMill provides 29 tools"
- "The server listens on port 3000"
- "Analysis tools detect code smells"

❌ **Avoid:**
- "TypeMill will provide 29 tools"
- "The server will listen on port 3000"
- "Analysis tools can detect code smells"

### Direct & Confident

✅ **Good:**
- "Set `dryRun: false` to execute changes"
- "The default port is 3000"
- "Three analysis tools are available"

❌ **Avoid:**
- "You should probably set `dryRun: false` if you want to execute changes"
- "The default port is typically 3000"
- "There are three analysis tools you can use"

---

## Emoji Usage

### Headers: No Emojis
Headers should be clean and scannable.

✅ **Good:**
```markdown
## Quick Start
### Installation Options
```

❌ **Avoid:**
```markdown
## 🚀 Quick Start
### 📦 Installation Options
```

**Exception**: High-level H2 section markers in navigation docs (README.md, docs/README.md) may use 1-2 emojis for wayfinding:
- `## 🚀 Start Here` (acceptable in navigation hubs)
- `## 📚 Documentation by Role` (acceptable in navigation hubs)

### Body: Semantic Emojis Only

Use emojis that add meaning, not decoration.

✅ **Allowed:**
- ⚠️ Warnings and cautions
- ✅ Checklist items and completed steps
- ❌ Incorrect examples or anti-patterns
- 💡 Tips and helpful notes
- ℹ️ Informational callouts

✅ **Examples:**
```markdown
⚠️ **Warning:** Never commit secrets to config files

**Checklist:**
- ✅ Install LSP servers
- ✅ Configure .typemill/config.json
- ❌ Don't skip the setup step

💡 **Tip:** Use `mill doctor` to diagnose issues
```

❌ **Avoid decorative emojis:**
- 🚀 🎯 📋 🛠️ 🔍 🏗️ 📖 🆘 ✨ ⚡ 🐳 🔄 (no semantic meaning)

---

## Terminology Standards

### Tools vs Commands vs Handlers

**Tool**: Public MCP tool that users/AI call
- "The `rename` tool supports files and directories"
- "`analyze.quality` is an analysis tool"

**Command**: CLI subcommand
- "The `mill setup` command detects languages"
- "Run the `mill start` command"

**Handler**: Internal implementation (rare in user docs)
- "The `RenameHandler` processes rename requests"
- Use only in architecture/development docs

### Tool Counts

**In user-facing docs**: Use "29 public MCP tools"
- README.md (project overview)
- docs/tools/README.md (tool catalog)

**Elsewhere**: Avoid specific numbers, use qualitative descriptions
- "comprehensive tools for..."
- "extensive analysis capabilities"
- Link to the catalog instead of repeating counts

**Rationale**: Tool counts change. Keeping them in 2 places prevents drift.

### Configuration Terms

- **LSP server**: Not "language server" or "LSP"
- **Environment variable**: Not "env var" on first use
- **Configuration file**: Not "config file" in formal docs (casual contexts OK)
- **Dry run**: Two words, not "dryRun" or "dry-run" in prose

---

## Document Structure

### Heading Hierarchy

Use logical heading levels without skipping:

✅ **Good:**
```markdown
# Document Title (H1)
## Major Section (H2)
### Subsection (H3)
#### Detail (H4)
```

❌ **Avoid:**
```markdown
# Document Title (H1)
### Subsection (H3) ← skipped H2
```

**H1**: One per document (document title)
**H2**: Major sections
**H3**: Subsections within H2
**H4**: Details within H3 (rarely needed)

### Tables of Contents

Add TOC to any document longer than 100 lines or with 5+ H2 sections.

**Generate automatically** using doctoc:
```markdown
<!-- toc -->
<!-- tocstop -->
```

### Code Examples

Use descriptive language tags:
- ` ```bash ` for shell commands
- ` ```json ` for JSON examples
- ` ```rust ` for Rust code
- ` ```typescript ` for TypeScript code

Always include expected output or result when helpful.

### Links

**Internal links**: Use relative paths
- `[configuration](user-guide/configuration.md)` (from docs/)
- `[../tools/README.md](../tools/README.md)` (from docs/user-guide/)

**External links**: Use full URLs
- `[GitHub](https://github.com/goobits/typemill)`

**Cross-references**: Link to related docs when helpful
- "See [refactoring tools](tools/refactoring.md) for examples"

---

## Content Guidelines

### Documentation Types

**User Guides**: Conversational, example-driven, second-person
- "You can configure TypeMill by..."
- "Run `mill setup` to get started"

**API Reference**: Terse, technical, third-person
- "Parameters: `file_path` (string, required)"
- "Returns: EditPlan object"

**Architecture Docs**: Technical but accessible, explain concepts
- "The plugin system uses trait-based dispatch"
- Use analogies when they clarify complex topics

**Troubleshooting**: Problem-focused, solution-oriented
- Start with symptoms, provide specific fixes
- Include common error messages

### Examples & Code Blocks

**Show, don't just tell:**

✅ **Good:**
```markdown
Rename a file from old.rs to new.rs:
` ` `bash
mill tool rename --target file:src/old.rs --new-name src/new.rs
` ` `
```

❌ **Insufficient:**
```markdown
Use the rename tool to rename files.
```

**Include both request and response** for tool examples:
```markdown
**Request:**
` ` `json
{
  "name": "rename",
  "arguments": { ... }
}
` ` `

**Response:**
` ` `json
{
  "success": true,
  ...
}
` ` `
```

### Warnings & Notes

Use consistent formatting:

```markdown
⚠️ **Warning:** Critical information that prevents errors

💡 **Tip:** Helpful suggestion or best practice

ℹ️ **Note:** Additional context or clarification
```

---

## File Organization

### Naming Conventions

**User-facing docs**: Descriptive, lowercase with hyphens
- `getting-started.md` (not `GettingStarted.md` or `getting_started.md`)
- `troubleshooting.md` (not `trouble-shooting.md`)

**Architecture docs**: Descriptive, specific
- `system-design.md` (not `overview.md` - too generic)
- `handler-contracts.md` (not `api.md` - too vague)

**Avoid generic names**: overview, guide, reference (unless truly comprehensive)

### Directory Structure

```
docs/
├── README.md                  (navigation hub)
├── STYLE_GUIDE.md            (this file)
├── user-guide/               (beginner-friendly tutorials)
│   ├── getting-started.md
│   ├── configuration.md
│   └── troubleshooting.md
├── tools/                    (tool reference docs)
│   ├── README.md            (catalog)
│   ├── refactoring.md
│   └── analysis.md
├── architecture/             (system design)
│   ├── overview.md
│   └── api_contracts.md
├── development/              (contributor docs)
│   ├── overview.md
│   └── testing.md
└── operations/               (deployment)
    ├── docker_deployment.md
    └── cache_configuration.md
```

**Placement rules:**
- User-facing content → `user-guide/`
- Tool documentation → `tools/`
- Deep technical → `architecture/`
- Contributing → `development/`
- Deployment/ops → `operations/`

---

## Quality Checklist

Before committing documentation:

- [ ] No decorative emojis in headers
- [ ] Active voice where possible
- [ ] Present tense for current functionality
- [ ] "TypeMill" for product, "`mill`" for commands
- [ ] Tool counts only in README.md and tools/README.md
- [ ] Heading hierarchy logical (no skipped levels)
- [ ] Code examples include language tags
- [ ] Internal links use relative paths
- [ ] No marketing language or hyperbole
- [ ] Examples tested and working
- [ ] Consistent terminology (tool/command/handler)

---

## Exceptions

### When to Diverge

This guide covers 95% of documentation. Exceptions allowed when:

1. **Maintaining backward compatibility**: Existing external links, file names
2. **Technical accuracy**: Passive voice may be clearer in some technical contexts
3. **Established conventions**: Industry-standard terms, even if they violate guidelines
4. **Brand identity**: Title emoji (🤖 TypeMill) is part of brand

**Rule**: Document the exception and rationale inline or in commit message.

---

## Enforcement

### Human Review

All PRs touching documentation must:
1. Follow this guide
2. Pass markdown linting (markdownlint)
3. Pass prose linting (vale) for voice/tone
4. Have working internal links

### Automated Checks (Future)

CI will eventually enforce:
- Tool count consistency
- Heading hierarchy
- Link validity
- Prose quality (passive voice, jargon)

---

## Examples

### Before & After

**Before** (violates multiple guidelines):
```markdown
## 🚀 Using Mill

Mill provides a lot of great tools that you can use. There are 36 tools in total. You should probably start by running the setup command which will help you get things configured.

The rename tool can be used to rename files.
```

**After** (follows guidelines):
```markdown
## Using TypeMill

TypeMill provides comprehensive tools for code intelligence. See [tools/README.md](tools/README.md) for the complete catalog.

**Start here:**
1. Run `mill setup` to detect languages and configure LSP servers
2. Use `mill tool rename --target file:old.rs --new-name new.rs` to rename files

The rename tool supports files, directories, and code symbols.
```

**Changes:**
- ❌ Removed decorative emoji (🚀)
- ✅ Changed "Mill" → "TypeMill"
- ✅ Removed vague tool count, linked to catalog
- ✅ Changed passive "can be used" → active "supports"
- ✅ Added concrete example with `mill` command
- ✅ Removed "should probably" uncertainty

---

## Feedback & Updates

This guide evolves with the project. Suggest improvements via:
- GitHub issues (label: `documentation`)
- PRs with `docs/STYLE_GUIDE.md` changes
- Discussion in contributor channels

**Review schedule**: Quarterly (or after major releases)
