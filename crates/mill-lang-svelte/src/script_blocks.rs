//! Utilities for locating <script> blocks in Svelte files.

#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct ScriptBlock {
    pub start: usize,
    pub end: usize,
    pub content_start: usize,
    pub content_end: usize,
}

pub fn find_script_blocks(source: &str) -> Vec<ScriptBlock> {
    let mut blocks = Vec::new();
    let mut cursor = 0;

    while let Some(script_pos) = source[cursor..].find("<script") {
        let abs_script_pos = cursor + script_pos;

        // Find end of opening tag
        let open_tag_end = match source[abs_script_pos..].find('>') {
            Some(rel) => abs_script_pos + rel,
            None => break,
        };

        // Find closing tag
        let close_tag = "</script>";
        let close_pos = match source[open_tag_end + 1..].find(close_tag) {
            Some(rel) => open_tag_end + 1 + rel,
            None => break,
        };

        blocks.push(ScriptBlock {
            start: abs_script_pos,
            end: close_pos + close_tag.len(),
            content_start: open_tag_end + 1,
            content_end: close_pos,
        });

        cursor = close_pos + close_tag.len();
    }

    blocks
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_script_blocks() {
        let source = r#"<script>const a = 1;</script>\n<div/>"#;
        let blocks = find_script_blocks(source);
        assert_eq!(blocks.len(), 1);
        let block = &blocks[0];
        assert_eq!(
            &source[block.content_start..block.content_end],
            "const a = 1;"
        );
    }

    #[test]
    fn handles_multiple_blocks() {
        let source = r#"<script>one</script>\n<script context=\"module\">two</script>"#;
        let blocks = find_script_blocks(source);
        assert_eq!(blocks.len(), 2);
        assert_eq!(
            &source[blocks[0].content_start..blocks[0].content_end],
            "one"
        );
        assert_eq!(
            &source[blocks[1].content_start..blocks[1].content_end],
            "two"
        );
    }
}
