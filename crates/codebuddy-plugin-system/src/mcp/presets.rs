//! Built-in MCP server presets

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpPreset {
    pub id: String,
    pub name: String,
    pub description: String,
    pub command: Vec<String>,
    pub env: HashMap<String, String>,
    pub auto_start: bool,
}

/// Get all built-in presets
pub fn get_presets() -> HashMap<String, McpPreset> {
    let mut presets = HashMap::new();

    // context7
    presets.insert(
        "context7".to_string(),
        McpPreset {
            id: "context7".to_string(),
            name: "Context7".to_string(),
            description: "Up-to-date documentation for any library".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@upstash/context7-mcp".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // git
    presets.insert(
        "git".to_string(),
        McpPreset {
            id: "git".to_string(),
            name: "Git MCP".to_string(),
            description: "Git operations and history".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-git".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // filesystem
    presets.insert(
        "filesystem".to_string(),
        McpPreset {
            id: "filesystem".to_string(),
            name: "Filesystem MCP".to_string(),
            description: "Enhanced filesystem operations".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-filesystem".to_string(),
                ".".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // github
    presets.insert(
        "github".to_string(),
        McpPreset {
            id: "github".to_string(),
            name: "GitHub MCP".to_string(),
            description: "GitHub API integration for repositories, issues, and PRs".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-github".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // postgres
    presets.insert(
        "postgres".to_string(),
        McpPreset {
            id: "postgres".to_string(),
            name: "PostgreSQL MCP".to_string(),
            description: "PostgreSQL database operations and queries".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-postgres".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // brave-search
    presets.insert(
        "brave-search".to_string(),
        McpPreset {
            id: "brave-search".to_string(),
            name: "Brave Search MCP".to_string(),
            description: "Web search using Brave Search API".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-brave-search".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // puppeteer
    presets.insert(
        "puppeteer".to_string(),
        McpPreset {
            id: "puppeteer".to_string(),
            name: "Puppeteer MCP".to_string(),
            description: "Browser automation and web scraping".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-puppeteer".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // slack
    presets.insert(
        "slack".to_string(),
        McpPreset {
            id: "slack".to_string(),
            name: "Slack MCP".to_string(),
            description: "Slack workspace integration for messages and channels".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-slack".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // memory
    presets.insert(
        "memory".to_string(),
        McpPreset {
            id: "memory".to_string(),
            name: "Memory MCP".to_string(),
            description: "Persistent conversation memory and knowledge graphs".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-memory".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // fetch
    presets.insert(
        "fetch".to_string(),
        McpPreset {
            id: "fetch".to_string(),
            name: "Fetch MCP".to_string(),
            description: "HTTP requests and API calls".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-fetch".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // sqlite
    presets.insert(
        "sqlite".to_string(),
        McpPreset {
            id: "sqlite".to_string(),
            name: "SQLite MCP".to_string(),
            description: "SQLite database operations and queries".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-sqlite".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    // sequential-thinking
    presets.insert(
        "sequential-thinking".to_string(),
        McpPreset {
            id: "sequential-thinking".to_string(),
            name: "Sequential Thinking MCP".to_string(),
            description: "Enhanced reasoning and step-by-step problem solving".to_string(),
            command: vec![
                "npx".to_string(),
                "-y".to_string(),
                "@modelcontextprotocol/server-sequential-thinking".to_string(),
            ],
            env: HashMap::new(),
            auto_start: true,
        },
    );

    presets
}

/// Get a specific preset by ID
pub fn get_preset(id: &str) -> Option<McpPreset> {
    get_presets().get(id).cloned()
}

/// List all preset IDs
pub fn list_preset_ids() -> Vec<String> {
    get_presets().keys().cloned().collect()
}
