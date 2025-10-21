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
        .find_affected_files(&fileb, &project_files, &vec![])
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
        .find_affected_files(&fileb, &project_files, &vec![])
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
        .find_affected_files(&fileb, &project_files, &vec![])
        .await
        .unwrap();
    assert_eq!(affected3.len(), 1);
    assert!(
        affected3.contains(&file_a),
        "Should still detect fileA after modification"
    );
}

/// Test for the bug fix: cache should store ALL imports, not just the one being checked
#[tokio::test]
async fn test_cache_stores_all_imports_directory_rename() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let cache = Arc::new(Mutex::new(HashMap::new()));
    let resolver = ImportPathResolver::with_cache(temp_dir.path(), cache.clone());

    // Create directory with multiple files
    fs::create_dir_all(temp_dir.path().join("core")).unwrap();
    let api_file = temp_dir.path().join("core/api.ts");
    let utils_file = temp_dir.path().join("core/utils.ts");
    let app_file = temp_dir.path().join("app.ts");

    // app.ts imports BOTH api.ts and utils.ts
    fs::write(
        &app_file,
        "import { api } from './core/api';\nimport { util } from './core/utils';\n",
    )
    .unwrap();
    fs::write(&api_file, "export const api = 1;\n").unwrap();
    fs::write(&utils_file, "export const util = 2;\n").unwrap();

    let project_files = vec![app_file.clone(), api_file.clone(), utils_file.clone()];

    // First check: Does app.ts import api.ts?
    let affected1 = resolver
        .find_affected_files(&api_file, &project_files, &vec![])
        .await
        .unwrap();
    assert_eq!(affected1.len(), 1);
    assert!(affected1.contains(&app_file), "app.ts should import api.ts");

    // CRITICAL: Cache should now contain ALL imports from app.ts, not just api.ts
    // Second check: Does app.ts import utils.ts?
    let affected2 = resolver
        .find_affected_files(&utils_file, &project_files, &vec![])
        .await
        .unwrap();

    // This is the bug we fixed: before, cache had only [api.ts], so this would return empty
    assert_eq!(
        affected2.len(),
        1,
        "app.ts should import utils.ts (cache should contain ALL imports)"
    );
    assert!(
        affected2.contains(&app_file),
        "Cache hit should find utils.ts in cached import list"
    );
}

#[tokio::test]
async fn test_resolve_import_to_file() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let resolver = ImportPathResolver::new(temp_dir.path());

    // Create test files
    let utils_file = temp_dir.path().join("utils.ts");
    let app_file = temp_dir.path().join("app.ts");
    fs::write(&utils_file, "export const foo = 1;\n").unwrap();
    fs::write(&app_file, "import { foo } from './utils';\n").unwrap();

    let project_files = vec![utils_file.clone(), app_file.clone()];

    // Test resolving relative import
    let resolved = resolver.resolve_import_to_file("./utils", &app_file, &project_files);
    assert!(resolved.is_some(), "Should resolve ./utils to utils.ts");
    assert_eq!(
        resolved.unwrap().canonicalize().unwrap(),
        utils_file.canonicalize().unwrap()
    );

    // Test that node_modules imports are skipped
    let external = resolver.resolve_import_to_file("react", &app_file, &project_files);
    assert!(
        external.is_none(),
        "Should not resolve external package imports"
    );
}

#[tokio::test]
async fn test_get_all_imported_files() {
    use std::fs;

    let temp_dir = TempDir::new().unwrap();
    let resolver = ImportPathResolver::new(temp_dir.path());

    // Create multiple files
    fs::create_dir_all(temp_dir.path().join("utils")).unwrap();
    let helpers_file = temp_dir.path().join("utils/helpers.ts");
    let types_file = temp_dir.path().join("utils/types.ts");
    let app_file = temp_dir.path().join("app.ts");

    fs::write(&helpers_file, "export const help = 1;\n").unwrap();
    fs::write(&types_file, "export type Foo = string;\n").unwrap();

    // app.ts imports both helpers and types
    let content =
        "import { help } from './utils/helpers';\nimport type { Foo } from './utils/types';\n";
    fs::write(&app_file, content).unwrap();

    let project_files = vec![helpers_file.clone(), types_file.clone(), app_file.clone()];

    // Get all imported files from app.ts
    let imports = resolver.get_all_imported_files(content, &app_file, &vec![], &project_files);

    assert_eq!(imports.len(), 2, "Should find both imports from app.ts");

    let canonical_imports: Vec<_> = imports.iter().map(|p| p.canonicalize().unwrap()).collect();

    assert!(
        canonical_imports.contains(&helpers_file.canonicalize().unwrap()),
        "Should include helpers.ts"
    );
    assert!(
        canonical_imports.contains(&types_file.canonicalize().unwrap()),
        "Should include types.ts"
    );
}
