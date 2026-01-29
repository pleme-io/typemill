//! Embedded documentation viewer

use include_dir::{include_dir, Dir};
use termimad::MadSkin;

// Embed the docs directory at compile time
static DOCS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../docs");

/// Display documentation
pub fn show_docs(topic: Option<String>, raw: bool, search: Option<String>) {
    if let Some(query) = search {
        search_docs(&query);
        return;
    }

    if let Some(topic_name) = topic {
        show_topic(&topic_name, raw);
    } else {
        list_docs();
    }
}

/// List all available documentation
fn list_docs() {
    println!("\n📚 Mill Documentation\n");
    println!("Available topics:\n");

    let mut topics = Vec::new();
    collect_docs(&DOCS_DIR, "", &mut topics);
    topics.sort();

    for (path, description) in topics {
        println!("  {:30} {}", path, description);
    }

    println!("\n💡 Usage:");
    println!("  mill docs <topic>           # View a specific document");
    println!("  mill docs --search <query>  # Search documentation");
    println!("  mill docs <topic> --raw     # Show raw markdown");
    println!("\nExamples:");
    println!("  mill docs getting-started");
    println!("  mill docs tools/refactor");
    println!("  mill docs --search 'LSP setup'");
}

/// Recursively collect documentation files
fn collect_docs(dir: &Dir, prefix: &str, topics: &mut Vec<(String, String)>) {
    for entry in dir.entries() {
        let name = entry.path().file_name().unwrap().to_str().unwrap();

        if let Some(file) = entry.as_file() {
            if name.ends_with(".md") {
                let file_name = name.trim_end_matches(".md");
                let path = if prefix.is_empty() {
                    file_name.to_string()
                } else {
                    format!("{}/{}", prefix, file_name)
                };

                // Extract first line as description
                let content = file.contents_utf8().unwrap_or("");
                let description = extract_description(content);

                topics.push((path, description));
            }
        } else if let Some(subdir) = entry.as_dir() {
            let new_prefix = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };
            collect_docs(subdir, &new_prefix, topics);
        }
    }
}

/// Extract description from markdown (first heading or first paragraph)
fn extract_description(content: &str) -> String {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("# ") {
            return trimmed.trim_start_matches("# ").to_string();
        } else if !trimmed.is_empty() && !trimmed.starts_with("<!--") && !trimmed.starts_with("---")
        {
            // First non-empty, non-comment line
            let desc = trimmed.chars().take(60).collect::<String>();
            if trimmed.len() > 60 {
                return format!("{}...", desc);
            }
            return desc;
        }
    }
    String::from("(No description)")
}

/// Display a specific documentation topic
fn show_topic(topic: &str, raw: bool) {
    // Try to find the file
    let possible_paths = vec![
        format!("{}.md", topic),
        format!("{}/README.md", topic),
        format!("README.md"), // If just asking for the main readme
    ];

    let mut content = None;
    let mut found_path = String::new();

    for path in possible_paths {
        if let Some(file) = DOCS_DIR.get_file(&path) {
            content = Some(file.contents_utf8().unwrap_or(""));
            found_path = path;
            break;
        }
    }

    if let Some(text) = content {
        if raw {
            // Show raw markdown
            println!("{}", text);
        } else {
            // Render markdown with termimad
            render_markdown(text, &found_path);
        }
    } else {
        eprintln!("❌ Documentation topic '{}' not found.", topic);
        eprintln!("\n💡 Run 'mill docs' to see available topics.");
        std::process::exit(1);
    }
}

/// Render markdown content with nice formatting
fn render_markdown(content: &str, title: &str) {
    // Print title bar
    println!("\n{}", "═".repeat(80));
    println!("📄 {}", title);
    println!("{}\n", "═".repeat(80));

    // Create termimad skin with default colors
    let skin = MadSkin::default();

    // Render the markdown
    skin.print_text(content);

    println!();
}

/// Search documentation for a keyword
fn search_docs(query: &str) {
    println!("\n🔍 Searching for '{}' in documentation...\n", query);

    let query_lower = query.to_lowercase();
    let mut results = Vec::new();

    search_in_dir(&DOCS_DIR, "", &query_lower, &mut results);

    if results.is_empty() {
        println!("No results found for '{}'.", query);
    } else {
        println!("Found {} result(s):\n", results.len());

        for (path, matches) in results {
            println!("  📄 {}", path);
            for (line_num, line) in matches {
                println!("     {}:  {}", line_num, line.trim());
            }
            println!();
        }
    }
}

/// Recursively search documentation
fn search_in_dir(
    dir: &Dir,
    prefix: &str,
    query: &str,
    results: &mut Vec<(String, Vec<(usize, String)>)>,
) {
    for entry in dir.entries() {
        let name = entry.path().file_name().unwrap().to_str().unwrap();

        if let Some(file) = entry.as_file() {
            if name.ends_with(".md") {
                let file_name = name.trim_end_matches(".md");
                let path = if prefix.is_empty() {
                    file_name.to_string()
                } else {
                    format!("{}/{}", prefix, file_name)
                };

                if let Some(content) = file.contents_utf8() {
                    let mut matches = Vec::new();

                    for (line_num, line) in content.lines().enumerate() {
                        if line.to_lowercase().contains(query) {
                            matches.push((line_num + 1, line.to_string()));
                        }
                    }

                    if !matches.is_empty() {
                        results.push((path, matches));
                    }
                }
            }
        } else if let Some(subdir) = entry.as_dir() {
            let new_prefix = if prefix.is_empty() {
                name.to_string()
            } else {
                format!("{}/{}", prefix, name)
            };
            search_in_dir(subdir, &new_prefix, query, results);
        }
    }
}
