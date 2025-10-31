use crate::CPlugin;
use mill_plugin_api::LanguagePlugin;

#[test]
fn test_lsp_installer() {
    let plugin = CPlugin::default();
    let installer = plugin.lsp_installer().unwrap();

    assert_eq!(installer.lsp_name(), "clangd");

    // This test will pass if clangd is installed, and fail if it is not.
    // This is acceptable for now, as it verifies that the check is working.
    installer.check_installed().unwrap();
}