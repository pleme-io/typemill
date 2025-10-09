use super::*;
use crate::import_updater::path_resolver::ImportPathResolver;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tempfile::TempDir;

#[test]
fn test_calculate_relative_import() {
    let temp_dir = TempDir::new().unwrap();
    let resolver = ImportPathResolver::new(temp_dir.path());

    let from_file = temp_dir.path().join("src/components/Button.tsx");
    let to_file = temp_dir.path().join("src/utils/helpers.ts");

    let result = resolver
        .calculate_relative_import(&from_file, &to_file)
        .unwrap();
    assert_eq!(result, "../utils/helpers");
}

#[test]
fn test_extract_import_path() {
    let line1 = "import { Component } from './component';";
    assert_eq!(
        file_scanner::extract_import_path(line1),
        Some("./component".to_string())
    );

    let line2 = "const utils = require('../utils/helpers');";
    assert_eq!(
        file_scanner::extract_import_path(line2),
        Some("../utils/helpers".to_string())
    );

    let line3 = "import React from 'react';";
    assert_eq!(
        file_scanner::extract_import_path(line3),
        Some("react".to_string())
    );
}

#[tokio::test]
async fn test_import_cache_usage() {
    use std::fs;
    use std::io::Write;

    let temp_dir = TempDir::new().unwrap();
    let cache = Arc::new(Mutex::new(HashMap::new()));
    let resolver = ImportPathResolver::with_cache(temp_dir.path(), cache.clone());

    // Create test files (consistent naming with imports)
    let file_a = temp_dir.path().join("fileA.ts");
    let fileb = temp_dir.path().join("fileB.ts");
    let file_c = temp_dir.path().join("fileC.ts");

    // fileA imports fileB using './fileB' path
    fs::write(&file_a, "import { foo } from './fileB';\n").unwrap();
    fs::write(&fileb, "export const foo = 1;\n").unwrap();
    fs::write(&file_c, "import { bar } from './other';\n").unwrap();

    // First call - should populate cache
    let project_files = vec![file_a.clone(), fileb.clone(), file_c.clone()];
    let affected = resolver
        .find_affected_files(&fileb, &project_files)
        .await
        .unwrap();
    assert_eq!(affected.len(), 1);
    assert!(affected.contains(&file_a));

    // Check cache stats - should have entries now
    let (total, valid) = resolver.cache_stats();
    assert!(total > 0, "Cache should have entries after first scan");
    assert!(valid > 0, "Cache should have valid entries");

    // Second call - should use cache (file hasn't been modified)
    let affected2 = resolver
        .find_affected_files(&fileb, &project_files)
        .await
        .unwrap();
    assert_eq!(affected2, affected, "Cached results should match");

    // Modify fileA to invalidate cache
    std::thread::sleep(std::time::Duration::from_millis(10));
    let mut file = fs::OpenOptions::new().append(true).open(&file_a).unwrap();
    file.write_all(b"// comment\n").unwrap();
    drop(file);

    // Third call - cache should be invalidated for fileA
    let affected3 = resolver
        .find_affected_files(&fileb, &project_files)
        .await
        .unwrap();
    assert_eq!(affected3.len(), 1);
    assert!(
        affected3.contains(&file_a),
        "Should still detect fileA after modification"
    );
}