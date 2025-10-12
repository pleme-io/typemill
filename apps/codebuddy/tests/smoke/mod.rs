//! Protocol Smoke Tests
//!
//! These tests validate that protocol layers (MCP, LSP) work correctly.
//! They test connectivity, handshakes, routing, and basic error handling.
//!
//! ## Purpose
//!
//! Smoke tests validate protocol connectivity ONLY. They do NOT test:
//! - Business logic (that's what unit tests do)
//! - Feature implementations (that's what integration tests do)
//! - Edge cases (that's what E2E tests do)
//!
//! ## What They Test
//!
//! **MCP Protocol:**
//! - Server initialization via STDIO
//! - JSON-RPC 2.0 message format
//! - Tool routing through MCP
//! - Response serialization/deserialization
//! - Error handling
//!
//! **LSP Protocol:**
//! - LSP server initialization
//! - LSP message format
//! - Request routing by file extension
//! - Multiple language server support
//! - Connection reuse
//!
//! ## Running Smoke Tests
//!
//! All smoke tests are marked `#[ignore]` because they require external services:
//!
//! ```bash
//! # Run MCP smoke tests (requires running MCP server)
//! cargo nextest run --workspace --ignored smoke::mcp
//!
//! # Run LSP smoke tests (requires LSP servers installed)
//! cargo nextest run --workspace --ignored --features lsp-tests smoke::lsp
//!
//! # Run all smoke tests
//! cargo nextest run --workspace --ignored --features lsp-tests
//! ```
//!
//! ## Requirements
//!
//! **MCP tests:**
//! - Running MCP server (typically started via `codebuddy start`)
//!
//! **LSP tests:**
//! - TypeScript: `npm install -g typescript-language-server`
//! - Rust: `rustup component add rust-analyzer`

mod lsp;
mod mcp;
