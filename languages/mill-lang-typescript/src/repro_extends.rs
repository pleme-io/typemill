#[cfg(test)]
mod tests {
    use crate::tsconfig::TsConfig;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use tempfile::TempDir;

    fn create_file(path: &Path, content: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, content).unwrap();
    }

    #[test]
    fn test_extends_support() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        // .svelte-kit/tsconfig.json
        let base_config = r#"{
            "compilerOptions": {
                "baseUrl": ".",
                "paths": {
                    "$lib/*": ["src/lib/*"]
                }
            }
        }"#;
        create_file(&project_root.join(".svelte-kit/tsconfig.json"), base_config);

        // tsconfig.json extends .svelte-kit/tsconfig.json
        let app_config = r#"{
            "extends": "./.svelte-kit/tsconfig.json",
            "compilerOptions": {
                "baseUrl": "."
            }
        }"#;
        create_file(&project_root.join("tsconfig.json"), app_config);

        // This uses the current TsConfig which doesn't support extends
        // So we expect it to fail or not have the paths
        let config = TsConfig::from_file(&project_root.join("tsconfig.json")).unwrap();

        // Currently, it should only have baseUrl, but NO paths (as they are in base)
        if let Some(opts) = config.compiler_options {
            if let Some(paths) = opts.paths {
                // If it supported extends, this would be true.
                // Since it doesn't, this assertion might fail if I asserted it has paths.
                // But I want to demonstrate it DOES NOT have paths yet.
                assert!(paths.get("$lib/*").is_none(), "Should not have paths yet");
            } else {
                println!("No paths as expected (extends not supported yet)");
            }
        }
    }
}
