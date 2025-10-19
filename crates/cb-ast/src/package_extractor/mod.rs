//! Logic for the extract_module_to_package refactoring tool.
//!
//! This module provides language-agnostic package extraction capabilities
//! for extracting modules into separate packages.

pub mod edits;
pub mod manifest;
pub mod planner;
pub mod workspace;

#[cfg(test)]
mod tests;

use crate::error::AstResult;
use codebuddy_foundation::protocol::EditPlan;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ExtractModuleToPackageParams {
    pub source_package: String,
    pub module_path: String,
    pub target_package_path: String,
    pub target_package_name: String,
    pub update_imports: Option<bool>,
    pub create_manifest: Option<bool>,
    pub dry_run: Option<bool>,
    pub is_workspace_member: Option<bool>,
}

/// Main entry point for extracting a module to a package
///
/// This function orchestrates the extraction process by:
/// 1. Detecting the source package language
/// 2. Selecting the appropriate plugin from the registry
/// 3. Generating an EditPlan for the refactoring
///
/// # Arguments
///
/// * `params` - Extraction parameters
/// * `plugin_registry` - Registry of language plugins
pub async fn plan_extract_module_to_package_with_registry(
    params: ExtractModuleToPackageParams,
    plugin_registry: &cb_plugin_api::PluginRegistry,
) -> AstResult<EditPlan> {
    planner::plan_extract_module_to_package(params, plugin_registry).await
}