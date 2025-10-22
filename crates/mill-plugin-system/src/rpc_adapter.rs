//! An adapter that makes an out-of-process RPC plugin conform to the
//! `LanguagePlugin` trait.

use crate::process_manager::PluginProcess;
use async_trait::async_trait;
use mill_plugin_api::{ LanguageMetadata , LanguagePlugin , ManifestData , ParsedSource , PluginCapabilities , PluginError , PluginResult , };
use serde_json::Value;
use std::path::Path;
use std::sync::Arc;

/// An adapter that presents an external plugin process as a `LanguagePlugin`.
pub struct RpcAdapterPlugin {
    process: Arc<PluginProcess>,
    metadata: LanguageMetadata,
}

impl RpcAdapterPlugin {
    /// Creates a new RPC adapter for the given process and metadata.
    pub fn new(process: Arc<PluginProcess>, metadata: LanguageMetadata) -> Self {
        Self { process, metadata }
    }

    /// A helper function to make RPC calls and handle response conversion.
    async fn call_rpc<T: serde::de::DeserializeOwned>(
        &self,
        method: &str,
        params: Value,
    ) -> PluginResult<T> {
        let response_value = self
            .process
            .call(method, params)
            .await
            .map_err(|e| PluginError::internal(format!("RPC call failed: {}", e)))?;

        serde_json::from_value(response_value).map_err(|e| {
            PluginError::internal(format!("Failed to deserialize RPC response: {}", e))
        })
    }
}

#[async_trait]
impl LanguagePlugin for RpcAdapterPlugin {
    fn metadata(&self) -> &LanguageMetadata {
        &self.metadata
    }

    fn capabilities(&self) -> PluginCapabilities {
        // For now, assume external plugins support everything.
        // A more advanced implementation would fetch capabilities from the plugin.
        PluginCapabilities::none().with_imports().with_workspace()
    }

    async fn parse(&self, source: &str) -> PluginResult<ParsedSource> {
        self.call_rpc("parse", serde_json::to_value(source)?).await
    }

    async fn analyze_manifest(&self, path: &Path) -> PluginResult<ManifestData> {
        self.call_rpc("analyze_manifest", serde_json::to_value(path)?)
            .await
    }

    // Since this is an adapter, it cannot be downcast to a concrete type.
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    // Optional traits like ImportSupport and WorkspaceSupport would also need
    // to be implemented on the adapter, making their own RPC calls. For now,
    // we will leave them as None.
}