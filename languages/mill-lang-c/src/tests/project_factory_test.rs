use crate::CPlugin;
use mill_plugin_api::{CreatePackageConfig, LanguagePlugin, PackageType, ProjectFactory, Template};
use tempfile::tempdir;

#[test]
fn test_project_factory() {
    let plugin = CPlugin::default();
    let project_factory = plugin.project_factory().unwrap();

    let dir = tempdir().unwrap();
    let project_path = dir.path().join("my-c-project");

    let config = CreatePackageConfig {
        package_path: project_path.to_str().unwrap().to_string(),
        package_type: PackageType::Binary,
        template: Template::Minimal,
        add_to_workspace: false,
        workspace_root: dir.path().to_str().unwrap().to_string(),
    };

    let result = project_factory.create_package(&config).unwrap();

    assert_eq!(result.package_info.name, "my-c-project");
    assert_eq!(result.created_files.len(), 2);

    let main_c_path = project_path.join("src/main.c");
    assert!(main_c_path.exists());

    let makefile_path = project_path.join("Makefile");
    assert!(makefile_path.exists());
}