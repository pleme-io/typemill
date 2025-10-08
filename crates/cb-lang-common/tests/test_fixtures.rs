//! Shared test fixtures for import helper testing
//!
//! This module provides common test data and utilities for testing import
//! manipulation across different language plugins. The fixtures represent
//! realistic import patterns from various programming languages.
//!
//! # Usage
//!
//! ```rust
//! use test_fixtures::fixtures::*;
//!
//! let content = SWIFT_IMPORTS;
//! // Test Swift import manipulation
//! ```
//!
//! # Available Fixtures
//!
//! - **SWIFT_IMPORTS** - Swift import statements
//! - **RUST_IMPORTS** - Rust use statements
//! - **PYTHON_IMPORTS** - Python import statements with docstrings
//! - **GO_IMPORTS** - Go import block
//! - **TYPESCRIPT_IMPORTS** - TypeScript ES6 imports
//! - **JAVA_IMPORTS** - Java import statements
//!
//! # Test Utilities
//!
//! - `normalize_line_endings()` - Convert all line endings to LF
//! - `count_lines()` - Count lines accounting for empty content
//! - `extract_import_lines()` - Extract only import statements
//! - `generate_import_file()` - Generate synthetic import files

/// Common import patterns across languages
pub mod fixtures {
    /// Swift import statements (iOS/macOS development)
    pub const SWIFT_IMPORTS: &str = r#"import Foundation
import UIKit
import SwiftUI
import Combine

class MyViewController: UIViewController {
    func viewDidLoad() {
        super.viewDidLoad()
    }
}
"#;

    /// Rust use statements (with grouped imports)
    pub const RUST_IMPORTS: &str = r#"use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, Read, Write};
use crate::module::submodule;
use super::parent_module;

fn main() {
    println!("Hello, world!");
}
"#;

    /// Python imports (with various import styles)
    pub const PYTHON_IMPORTS: &str = r#"#!/usr/bin/env python3
"""Module for demonstrating import patterns."""

import os
import sys
from typing import List, Dict, Optional
from collections import defaultdict
import numpy as np
from pathlib import Path

def main():
    """Main entry point."""
    pass

if __name__ == '__main__':
    main()
"#;

    /// Go imports (with grouped imports)
    pub const GO_IMPORTS: &str = r#"package main

import (
    "fmt"
    "os"
    "strings"

    "github.com/user/repo/pkg"
    "github.com/other/lib"
)

func main() {
    fmt.Println("Hello, World!")
}
"#;

    /// TypeScript ES6 imports
    pub const TYPESCRIPT_IMPORTS: &str = r#"import { Component } from '@angular/core';
import { HttpClient } from '@angular/common/http';
import * as Utils from './utils';
import type { User } from './types';
import './styles.css';

export class AppComponent {
    constructor(private http: HttpClient) {}
}
"#;

    /// Java import statements
    pub const JAVA_IMPORTS: &str = r#"package com.example.myapp;

import java.util.List;
import java.util.ArrayList;
import java.util.HashMap;
import java.io.File;
import java.io.IOException;

import com.example.utils.StringUtils;
import com.example.models.User;

public class Main {
    public static void main(String[] args) {
        System.out.println("Hello, World!");
    }
}
"#;

    /// Complex nested imports (TypeScript)
    pub const COMPLEX_IMPORTS: &str = r#"// Core framework imports
import React, { useState, useEffect, useMemo } from 'react';
import { BrowserRouter, Route, Switch } from 'react-router-dom';

// UI library imports
import { Button, TextField, Select } from '@mui/material';
import { makeStyles } from '@mui/styles';

// Internal imports
import { api } from '@/services/api';
import { useAuth } from '@/hooks/useAuth';
import { UserProfile } from '@/components/UserProfile';
import type { ApiResponse, User, Settings } from '@/types';

// Utilities and helpers
import { formatDate, parseDate } from '@/utils/date';
import { logger } from '@/utils/logger';

// Styles
import './App.css';
import styles from './App.module.css';

export const App: React.FC = () => {
    return <div>App</div>;
};
"#;

    /// No imports (code only)
    pub const NO_IMPORTS: &str = r#"class Foo {
    func bar() {
        print("Hello")
    }
}
"#;

    /// Empty file
    pub const EMPTY_FILE: &str = "";

    /// Only imports (no code)
    pub const ONLY_IMPORTS: &str = r#"import A
import B
import C
import D
"#;

    /// Imports with comments
    pub const IMPORTS_WITH_COMMENTS: &str = r#"// Required for networking
import Foundation

// UI framework
import UIKit

/*
 * SwiftUI is used for modern UI
 */
import SwiftUI

class App {}
"#;

    /// Mixed line endings (CRLF and LF)
    pub const MIXED_LINE_ENDINGS: &str = "import A\r\nimport B\nimport C\r\nclass Foo {}";
}

/// Test helper utilities
pub mod utils {
    /// Normalize all line endings to LF (Unix-style)
    ///
    /// # Examples
    ///
    /// ```
    /// use test_fixtures::utils::normalize_line_endings;
    ///
    /// let crlf = "line1\r\nline2\r\n";
    /// let lf = normalize_line_endings(crlf);
    /// assert_eq!(lf, "line1\nline2\n");
    /// ```
    pub fn normalize_line_endings(s: &str) -> String {
        s.replace("\r\n", "\n")
    }

    /// Count lines in a string, handling empty content correctly
    ///
    /// # Examples
    ///
    /// ```
    /// use test_fixtures::utils::count_lines;
    ///
    /// assert_eq!(count_lines(""), 0);
    /// assert_eq!(count_lines("line1"), 1);
    /// assert_eq!(count_lines("line1\nline2"), 2);
    /// ```
    pub fn count_lines(s: &str) -> usize {
        if s.is_empty() {
            0
        } else {
            s.lines().count()
        }
    }

    /// Extract lines matching a predicate
    ///
    /// # Examples
    ///
    /// ```
    /// use test_fixtures::utils::extract_matching_lines;
    ///
    /// let content = "import A\ncode\nimport B";
    /// let imports = extract_matching_lines(content, |line| line.starts_with("import"));
    /// assert_eq!(imports, vec!["import A", "import B"]);
    /// ```
    pub fn extract_matching_lines<F>(content: &str, predicate: F) -> Vec<String>
    where
        F: Fn(&str) -> bool,
    {
        content
            .lines()
            .filter(|line| predicate(line))
            .map(|s| s.to_string())
            .collect()
    }

    /// Extract only import lines (language-agnostic heuristic)
    ///
    /// Looks for common import keywords: import, use, from, include, require
    pub fn extract_import_lines(content: &str) -> Vec<String> {
        extract_matching_lines(content, |line| {
            let trimmed = line.trim();
            trimmed.starts_with("import ")
                || trimmed.starts_with("use ")
                || trimmed.starts_with("from ")
                || trimmed.starts_with("#include")
                || trimmed.starts_with("require ")
                || trimmed.starts_with("�e ")
        })
    }

    /// Generate a synthetic file with specified number of imports and code lines
    ///
    /// # Examples
    ///
    /// ```
    /// use test_fixtures::utils::generate_import_file;
    ///
    /// let content = generate_import_file(5, 10, "import", "code");
    /// assert!(content.contains("import"));
    /// assert!(content.contains("code"));
    /// ```
    pub fn generate_import_file(
        import_count: usize,
        code_count: usize,
        import_prefix: &str,
        code_prefix: &str,
    ) -> String {
        let mut lines = Vec::new();

        // Add imports
        for i in 0..import_count {
            lines.push(format!("{} module_{}", import_prefix, i));
        }

        // Add blank line separator
        if import_count > 0 && code_count > 0 {
            lines.push(String::new());
        }

        // Add code
        for i in 0..code_count {
            lines.push(format!("{} line_{}", code_prefix, i));
        }

        lines.join("\n")
    }

    /// Count occurrences of a substring in content
    pub fn count_occurrences(content: &str, pattern: &str) -> usize {
        content.matches(pattern).count()
    }

    /// Split content into header (imports) and body (code)
    ///
    /// Returns (imports, body) where imports includes all leading import-like lines
    pub fn split_imports_and_code(content: &str) -> (String, String) {
        let lines: Vec<&str> = content.lines().collect();
        let mut import_end = 0;

        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();
            if !trimmed.is_empty()
                && !trimmed.starts_with("import")
                && !trimmed.starts_with("use")
                && !trimmed.starts_with("from")
                && !trimmed.starts_with("#include")
                && !trimmed.starts_with("//")
                && !trimmed.starts_with("/*")
                && !trimmed.starts_with("*")
            {
                import_end = i;
                break;
            }
        }

        if import_end == 0 {
            return (String::new(), content.to_string());
        }

        let imports = lines[..import_end].join("\n");
        let body = lines[import_end..].join("\n");

        (imports, body)
    }

    /// Deduplicate consecutive lines
    pub fn deduplicate_lines(content: &str) -> String {
        let mut result = Vec::new();
        let mut prev: Option<&str> = None;

        for line in content.lines() {
            if Some(line) != prev {
                result.push(line);
                prev = Some(line);
            }
        }

        result.join("\n")
    }

    /// Check if content has CRLF line endings
    pub fn has_crlf(content: &str) -> bool {
        content.contains("\r\n")
    }

    /// Convert content to CRLF line endings
    pub fn to_crlf(content: &str) -> String {
        normalize_line_endings(content).replace("\n", "\r\n")
    }
}

// ============================================================================
// Integration Tests Using Fixtures
// ============================================================================

#[cfg(test)]
mod tests {
    use super::fixtures::*;
    use super::utils::*;
    use cb_lang_common::import_helpers::*;

    #[test]
    fn test_fixtures_are_valid() {
        // Verify all fixtures are non-panicking to parse
        assert!(count_lines(SWIFT_IMPORTS) > 0);
        assert!(count_lines(RUST_IMPORTS) > 0);
        assert!(count_lines(PYTHON_IMPORTS) > 0);
        assert!(count_lines(GO_IMPORTS) > 0);
        assert!(count_lines(TYPESCRIPT_IMPORTS) > 0);
        assert!(count_lines(JAVA_IMPORTS) > 0);
    }

    #[test]
    fn test_extract_import_lines_swift() {
        let imports = extract_import_lines(SWIFT_IMPORTS);
        assert_eq!(imports.len(), 4);
        assert!(imports[0].contains("Foundation"));
        assert!(imports[3].contains("Combine"));
    }

    #[test]
    fn test_extract_import_lines_rust() {
        let imports = extract_import_lines(RUST_IMPORTS);
        assert_eq!(imports.len(), 5);
        assert!(imports[0].contains("HashMap"));
    }

    #[test]
    fn test_extract_import_lines_python() {
        let imports = extract_import_lines(PYTHON_IMPORTS);
        assert_eq!(imports.len(), 6); // import os, sys, from typing, from collections, import numpy, from pathlib
        assert!(imports[0].contains("import os"));
        assert!(imports[2].contains("from typing"));
    }

    #[test]
    fn test_normalize_line_endings() {
        let crlf = "line1\r\nline2\r\nline3";
        let normalized = normalize_line_endings(crlf);
        assert_eq!(normalized, "line1\nline2\nline3");
        assert!(!normalized.contains("\r\n"));
    }

    #[test]
    fn test_count_lines_edge_cases() {
        assert_eq!(count_lines(""), 0);
        assert_eq!(count_lines("single"), 1);
        assert_eq!(count_lines("line1\nline2"), 2);
        assert_eq!(count_lines("line1\nline2\n"), 2); // trailing newline
    }

    #[test]
    fn test_generate_import_file() {
        let content = generate_import_file(3, 5, "import", "code");
        assert_eq!(count_lines(&content), 9); // 3 imports + 1 blank + 5 code
        assert!(content.contains("import module_0"));
        assert!(content.contains("code line_0"));
    }

    #[test]
    fn test_split_imports_and_code_swift() {
        let (imports, code) = split_imports_and_code(SWIFT_IMPORTS);
        assert!(imports.contains("import Foundation"));
        assert!(code.contains("class MyViewController"));
    }

    // Integration tests combining fixtures with import_helpers

    #[test]
    fn test_find_last_import_swift() {
        let idx = find_last_matching_line(SWIFT_IMPORTS, |line| line.trim().starts_with("import"));
        assert!(idx.is_some());
        let line = SWIFT_IMPORTS.lines().nth(idx.unwrap()).unwrap();
        assert!(line.contains("Combine"));
    }

    #[test]
    fn test_insert_import_after_last_swift() {
        let idx = find_last_matching_line(SWIFT_IMPORTS, |line| line.trim().starts_with("import"))
            .unwrap();

        let result = insert_line_at(SWIFT_IMPORTS, idx + 1, "import NewModule");
        assert!(result.contains("import NewModule"));
        assert!(result.contains("import Combine"));
    }

    #[test]
    fn test_remove_all_imports_rust() {
        let (result, count) =
            remove_lines_matching(RUST_IMPORTS, |line| line.trim().starts_with("use"));

        assert_eq!(count, 5);
        assert!(!result.contains("use std::"));
        assert!(result.contains("fn main()"));
    }

    #[test]
    fn test_replace_module_name_typescript() {
        let (result, count) = replace_in_lines(TYPESCRIPT_IMPORTS, "@angular", "@custom");
        assert_eq!(count, 2);
        assert!(result.contains("@custom/core"));
        assert!(result.contains("@custom/common"));
        assert!(!result.contains("@angular"));
    }

    #[test]
    fn test_complex_imports_manipulation() {
        // Find last import
        let idx =
            find_last_matching_line(COMPLEX_IMPORTS, |line| line.trim().starts_with("import"))
                .unwrap();

        // Insert new import
        let with_new = insert_line_at(COMPLEX_IMPORTS, idx + 1, "import { NEW } from '@/lib';");

        // Replace import source
        let (replaced, _) = replace_in_lines(&with_new, "@/services", "@/api");

        // Remove style imports
        let (final_result, _) = remove_lines_matching(&replaced, |line| {
            line.contains(".css") || line.contains(".module.css")
        });

        assert!(final_result.contains("import { NEW }"));
        assert!(final_result.contains("@/api"));
        assert!(!final_result.contains(".css"));
    }
}
