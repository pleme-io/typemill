//! Rename planning service for Unified Refactoring API
//!
//! Provides rename planning for:
//! - Symbol renaming (via LSP)
//! - File renaming (via MoveService)
//! - Directory renaming (via MoveService)

pub(crate) mod directory_rename;
pub(crate) mod file_rename;
mod plan_converter;
pub(crate) mod symbol_rename;
mod utils;

use crate::handlers::common::lsp_uri_from_uri_str;
use crate::handlers::tools::extensions::get_concrete_app_state;
use lsp_types::{Position, WorkspaceEdit};
use mill_foundation::errors::{MillError as ServerError, MillResult as ServerResult};
use mill_foundation::planning::{PlanSummary, PlanWarning, RenamePlan};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use tracing::{debug, info};

/// Planning service for rename operations
pub struct RenameService;

impl RenameService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RenameService {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Reserved for future options support
pub(crate) struct RenamePlanParams {
    /// Single target (existing API)
    #[serde(default)]
    target: Option<RenameTarget>,
    /// Multiple targets for batch operations (new API)
    #[serde(default)]
    targets: Option<Vec<RenameTarget>>,
    /// New name for single target (ignored when targets is set)
    #[serde(default)]
    new_name: Option<String>,
    #[serde(default)]
    options: RenameOptions,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct RenameTarget {
    pub kind: String, // "symbol" | "file" | "directory"
    pub path: String,
    /// New name for this target (required for batch mode, optional for single mode)
    #[serde(default)]
    pub new_name: Option<String>,
    #[serde(default)]
    pub selector: Option<SymbolSelector>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct SymbolSelector {
    pub position: Position,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)] // Reserved for future configuration
pub struct RenameOptions {
    /// Preview mode - don't actually apply changes (default: true for safety)
    #[serde(default = "crate::default_true")]
    pub dry_run: bool,

    #[serde(default)]
    pub strict: Option<bool>,
    #[serde(default)]
    pub validate_scope: Option<bool>,
    #[serde(default)]
    pub update_imports: Option<bool>,

    /// Scope configuration for what to update
    #[serde(default)]
    pub scope: Option<String>, // "code-only" | "all" | "custom"

    /// Custom scope configuration (when scope="custom")
    #[serde(default)]
    pub custom_scope: Option<mill_foundation::core::rename_scope::RenameScope>,

    /// Consolidate source package into target (for directory renames only)
    /// When true, merges Cargo.toml dependencies and updates all imports.
    /// When None, auto-detects based on path patterns (moving crate into another crate's src/).
    #[serde(default)]
    pub consolidate: Option<bool>,
}

// Manual Default implementation to ensure dry_run defaults to true for safety.
// This is critical because #[serde(default = "default_true")] only applies
// when deserializing the field itself, NOT when the entire struct is defaulted
// via #[serde(default)] on the parent struct's field.
impl Default for RenameOptions {
    fn default() -> Self {
        Self {
            dry_run: true, // Safe default - preview mode
            strict: None,
            validate_scope: None,
            update_imports: None,
            scope: None,
            custom_scope: None,
            consolidate: None,
        }
    }
}

impl RenameOptions {
    /// Build RenameScope from options
    /// Resolves update_all flag if present in custom_scope
    pub fn to_rename_scope(&self) -> mill_foundation::core::rename_scope::RenameScope {
        let scope = match self.scope.as_deref() {
            // New names (preferred)
            Some("code") => mill_foundation::core::rename_scope::RenameScope::code(),
            Some("standard") | None => mill_foundation::core::rename_scope::RenameScope::standard(),
            Some("comments") => mill_foundation::core::rename_scope::RenameScope::comments(),
            Some("everything") => mill_foundation::core::rename_scope::RenameScope::everything(),

            // Deprecated aliases (still work)
            Some("code-only") => mill_foundation::core::rename_scope::RenameScope::code(),
            Some("all") => mill_foundation::core::rename_scope::RenameScope::standard(),

            Some("custom") => self
                .custom_scope
                .clone()
                .unwrap_or_else(mill_foundation::core::rename_scope::RenameScope::standard),
            // BUG FIX: This was returning `None`, causing file discovery to fail for .md, .yml.
            // By returning a standard scope, we ensure all files are discovered by default.
            _ => mill_foundation::core::rename_scope::RenameScope::standard(),
        };

        // Resolve update_all flag if present
        scope.resolve_update_all()
    }
}

impl RenameService {
    /// Deduplicate document changes by merging text edits for the same file
    ///
    /// When multiple targets in a batch rename modify the same file (e.g., root Cargo.toml),
    /// we need to merge their edits rather than having the last one win.
    /// Edits are sorted in reverse order to ensure safety when applying them.
    fn dedupe_document_changes(
        changes: Vec<lsp_types::DocumentChangeOperation>,
    ) -> Vec<lsp_types::DocumentChangeOperation> {
        use lsp_types::{
            AnnotatedTextEdit, DocumentChangeOperation, OneOf,
            OptionalVersionedTextDocumentIdentifier, ResourceOp, TextDocumentEdit, TextEdit, Uri,
        };
        use std::collections::HashMap;

        let mut result = Vec::new();
        // Buffer for collecting edits by URI
        // Note: Uri has interior mutability but is effectively immutable in LSP protocol
        #[allow(clippy::mutable_key_type)]
        let mut edits_buffer: HashMap<Uri, Vec<OneOf<TextEdit, AnnotatedTextEdit>>> =
            HashMap::new();

        // Helper to get range from either TextEdit or AnnotatedTextEdit
        let get_range = |edit: &OneOf<TextEdit, AnnotatedTextEdit>| -> lsp_types::Range {
            match edit {
                OneOf::Left(e) => e.range,
                OneOf::Right(e) => e.text_edit.range,
            }
        };

        // Helper to flush edits for a specific URI
        let flush_uri = |uri: &Uri,
                         buffer: &mut HashMap<Uri, Vec<OneOf<TextEdit, AnnotatedTextEdit>>>,
                         out: &mut Vec<DocumentChangeOperation>| {
            if let Some(mut edits) = buffer.remove(uri) {
                if !edits.is_empty() {
                    // Sort edits in reverse order (bottom to top) to prevent position shifts
                    edits.sort_by(|a, b| {
                        let range_a = get_range(a);
                        let range_b = get_range(b);
                        match range_b.start.line.cmp(&range_a.start.line) {
                            std::cmp::Ordering::Equal => {
                                range_b.start.character.cmp(&range_a.start.character)
                            }
                            other => other,
                        }
                    });

                    out.push(DocumentChangeOperation::Edit(TextDocumentEdit {
                        text_document: OptionalVersionedTextDocumentIdentifier {
                            uri: uri.clone(),
                            version: None,
                        },
                        edits,
                    }));
                }
            }
        };

        for change in changes {
            match change {
                DocumentChangeOperation::Edit(text_doc_edit) => {
                    let uri = text_doc_edit.text_document.uri;
                    edits_buffer
                        .entry(uri)
                        .or_default()
                        .extend(text_doc_edit.edits);
                }
                DocumentChangeOperation::Op(op) => {
                    // Identify affected URIs that act as barriers
                    let affected_uris = match &op {
                        ResourceOp::Create(c) => vec![&c.uri],
                        ResourceOp::Delete(d) => vec![&d.uri],
                        ResourceOp::Rename(r) => vec![&r.old_uri, &r.new_uri],
                    };

                    // Flush pending edits for these URIs BEFORE the operation
                    // This ensures edits happen before rename/delete (if applicable)
                    // or edits happen before create (wait, no, edits before create is impossible usually)
                    //
                    // Logic check:
                    // - Rename A->B:
                    //   - Flush A: Edits to A must happen before rename. Correct.
                    //   - Flush B: Edits to B (if any pending) must happen before... wait.
                    //     If we have pending edits for B, and we are renaming A->B.
                    //     This implies B existed before (or we are overwriting).
                    //     If B existed, edits apply to old B. So before overwrite is correct.
                    //
                    // - Create A:
                    //   - Flush A: Edits to A before create?
                    //     If A didn't exist, we shouldn't have edits for it.
                    //     If we did (e.g. from previous steps), they are likely invalid or apply to a previous A.
                    //     Flushing here preserves order.
                    //
                    // - Delete A:
                    //   - Flush A: Edits to A before delete. Correct.
                    for uri in affected_uris {
                        flush_uri(uri, &mut edits_buffer, &mut result);
                    }

                    // Add the operation itself
                    result.push(DocumentChangeOperation::Op(op));
                }
            }
        }

        // Flush all remaining edits in buffer
        // Note: Hash map iteration order is random, but these files are independent of remaining Ops
        // and independent of each other (otherwise they would have been flushed by an Op).
        // To be deterministic for tests, we can sort by URI.
        let mut remaining_uris: Vec<_> = edits_buffer.keys().cloned().collect();
        remaining_uris.sort_by(|a, b| a.as_str().cmp(b.as_str()));

        for uri in remaining_uris {
            flush_uri(&uri, &mut edits_buffer, &mut result);
        }

        result
    }

    /// Analyze WorkspaceEdit to calculate checksums and summary
    pub(crate) async fn analyze_workspace_edit(
        &self,
        edit: &WorkspaceEdit,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<(HashMap<String, String>, PlanSummary, Vec<PlanWarning>)> {
        let mut file_checksums = HashMap::new();
        let mut affected_files: HashSet<std::path::PathBuf> = HashSet::new();

        // Extract file paths from WorkspaceEdit
        if let Some(ref changes) = edit.changes {
            for uri in changes.keys() {
                let path = std::path::PathBuf::from(
                    urlencoding::decode(uri.path().as_str())
                        .map_err(|_| ServerError::internal("Invalid URI path".to_string()))?
                        .into_owned(),
                );
                affected_files.insert(path);
            }
        }

        if let Some(ref document_changes) = edit.document_changes {
            match document_changes {
                lsp_types::DocumentChanges::Edits(edits) => {
                    for edit in edits {
                        let path = std::path::PathBuf::from(
                            urlencoding::decode(edit.text_document.uri.path().as_str())
                                .map_err(|_| ServerError::internal("Invalid URI path".to_string()))?
                                .into_owned(),
                        );
                        affected_files.insert(path);
                    }
                }
                lsp_types::DocumentChanges::Operations(ops) => {
                    for op in ops {
                        match op {
                            lsp_types::DocumentChangeOperation::Edit(edit) => {
                                let path = std::path::PathBuf::from(
                                    urlencoding::decode(edit.text_document.uri.path().as_str())
                                        .map_err(|_| {
                                            ServerError::internal("Invalid URI path".to_string())
                                        })?
                                        .into_owned(),
                                );
                                affected_files.insert(path);
                            }
                            lsp_types::DocumentChangeOperation::Op(resource_op) => {
                                match resource_op {
                                    lsp_types::ResourceOp::Create(create) => {
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(create.uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                    }
                                    lsp_types::ResourceOp::Rename(rename) => {
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(rename.old_uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(rename.new_uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                    }
                                    lsp_types::ResourceOp::Delete(delete) => {
                                        let path = std::path::PathBuf::from(
                                            urlencoding::decode(delete.uri.path().as_str())
                                                .map_err(|_| {
                                                    ServerError::internal(
                                                        "Invalid URI path".to_string(),
                                                    )
                                                })?
                                                .into_owned(),
                                        );
                                        affected_files.insert(path);
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        // Calculate checksums for all affected files
        for file_path in &affected_files {
            if tokio::fs::try_exists(file_path).await.unwrap_or(false) {
                if let Ok(content) = context.app_state.file_service.read_file(file_path).await {
                    file_checksums.insert(
                        file_path.to_string_lossy().to_string(),
                        utils::calculate_checksum(&content),
                    );
                }
            }
        }

        let summary = PlanSummary {
            affected_files: affected_files.len(),
            created_files: 0,
            deleted_files: 0,
        };

        let warnings = Vec::new();

        Ok((file_checksums, summary, warnings))
    }

    /// Plan batch rename for multiple targets
    pub(crate) async fn plan_batch_rename(
        &self,
        targets: &[RenameTarget],
        options: &RenameOptions,
        context: &mill_handler_api::ToolHandlerContext,
    ) -> ServerResult<RenamePlan> {
        use mill_foundation::planning::{PlanMetadata, RenamePlan};

        debug!(targets_count = targets.len(), "Planning batch rename");

        // Validate all targets have new_name
        for (idx, target) in targets.iter().enumerate() {
            if target.new_name.is_none() {
                return Err(ServerError::invalid_request(format!(
                    "Target {} (path: {}) missing new_name field (required for batch mode)",
                    idx, target.path
                )));
            }
        }

        // Detect naming conflicts (same new_name from different sources)
        let mut new_names_map: HashMap<String, Vec<String>> = HashMap::new();
        for target in targets {
            let new_name = target.new_name.as_ref().ok_or_else(|| {
                ServerError::invalid_request(format!(
                    "Missing 'newName' for target '{}'",
                    target.path
                ))
            })?;
            new_names_map
                .entry(new_name.clone())
                .or_default()
                .push(target.path.clone());
        }

        let mut warnings = Vec::new();
        for (new_name, sources) in &new_names_map {
            if sources.len() > 1 {
                warnings.push(PlanWarning {
                    code: "BATCH_RENAME_CONFLICT".to_string(),
                    message: format!(
                        "Multiple targets rename to '{}': {}",
                        new_name,
                        sources.join(", ")
                    ),
                    candidates: Some(sources.clone()),
                });
            }
        }

        // Fail if there are conflicts
        if !warnings.is_empty() {
            return Err(ServerError::invalid_request(format!(
                "Batch rename has naming conflicts: {}",
                warnings
                    .iter()
                    .map(|w| w.message.clone())
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }

        // TWO-PHASE BATCH: Collect dir moves, normalize to absolute paths
        let project_root = &context.app_state.project_root;
        let dir_moves: Vec<(std::path::PathBuf, std::path::PathBuf)> = targets
            .iter()
            .filter(|t| t.kind == "directory")
            .filter_map(|t| {
                // new_name already validated above, but defensive check
                let new_name = t.new_name.as_ref()?;
                let old_path = std::path::PathBuf::from(&t.path);
                let new_path = std::path::PathBuf::from(new_name);

                // Normalize to absolute paths for workspace planning
                let abs_old = if old_path.is_absolute() {
                    old_path
                } else {
                    project_root.join(old_path)
                };
                let abs_new = if new_path.is_absolute() {
                    new_path
                } else {
                    project_root.join(new_path)
                };

                Some((abs_old, abs_new))
            })
            .collect();

        let mut all_document_changes = Vec::new();
        let mut all_file_checksums = HashMap::new();
        let mut total_affected_files = HashSet::new();

        // PHASE 1: Plan batch workspace manifest updates (e.g., Cargo.toml workspace.members)
        // This generates a single atomic update for all moves, preventing conflicting edits
        if !dir_moves.is_empty() {
            debug!(
                dir_moves_count = dir_moves.len(),
                "Planning batch workspace manifest updates"
            );

            // Get concrete AppState to access move_service()
            let concrete_state = get_concrete_app_state(&context.app_state)?;

            match concrete_state
                .move_service()
                .plan_batch_workspace_updates(&dir_moves)
                .await
            {
                Ok(updates) if !updates.is_empty() => {
                    debug!(
                        updates_count = updates.len(),
                        "Batch workspace updates planned, converting to LSP edits"
                    );

                    // Convert (path, old_content, new_content) to LSP edits
                    for (manifest_path, old_content, new_content) in updates {
                        let uri = url::Url::from_file_path(&manifest_path).map_err(|_| {
                            ServerError::internal(format!(
                                "Invalid manifest path: {}",
                                manifest_path.display()
                            ))
                        })?;

                        // Full-file replacement edit
                        let edit = lsp_types::TextEdit {
                            range: lsp_types::Range {
                                start: lsp_types::Position {
                                    line: 0,
                                    character: 0,
                                },
                                end: lsp_types::Position {
                                    line: old_content.lines().count() as u32,
                                    character: 0,
                                },
                            },
                            new_text: new_content,
                        };

                        all_document_changes.push(lsp_types::DocumentChangeOperation::Edit(
                            lsp_types::TextDocumentEdit {
                                text_document: lsp_types::OptionalVersionedTextDocumentIdentifier {
                                    uri: lsp_uri_from_uri_str(uri.as_str())?,
                                    version: None,
                                },
                                edits: vec![lsp_types::OneOf::Left(edit)],
                            },
                        ));
                    }
                }
                Ok(_) => {
                    debug!("No batch workspace updates needed");
                }
                Err(e) => {
                    debug!(error = %e, "Failed to plan batch workspace updates");
                }
            }
        }

        // PHASE 2: Plan individual target renames (may include duplicate workspace edits)

        for target in targets {
            let new_name = target.new_name.as_ref().ok_or_else(|| {
                ServerError::invalid_request(format!(
                    "Missing 'newName' for target '{}'",
                    target.path
                ))
            })?;

            debug!(
                kind = %target.kind,
                path = %target.path,
                new_name = %new_name,
                "Planning individual rename in batch"
            );

            let plan = match target.kind.as_str() {
                "symbol" => {
                    self.plan_symbol_rename(target, new_name, options, context)
                        .await?
                }
                "file" => {
                    self.plan_file_rename(target, new_name, options, context)
                        .await?
                }
                "directory" => {
                    self.plan_directory_rename(target, new_name, options, context)
                        .await?
                }
                kind => {
                    return Err(ServerError::invalid_request(format!(
                        "Unsupported rename kind in batch: {}. Must be one of: symbol, file, directory",
                        kind
                    )));
                }
            };

            // Debug: log plan details
            let plan_doc_changes_count = plan
                .edits
                .document_changes
                .as_ref()
                .map(|dc| match dc {
                    lsp_types::DocumentChanges::Operations(ops) => ops.len(),
                    lsp_types::DocumentChanges::Edits(edits) => edits.len(),
                })
                .unwrap_or(0);
            debug!(
                target_path = %target.path,
                new_name = %new_name,
                document_changes_count = plan_doc_changes_count,
                affected_files = plan.summary.affected_files,
                "Individual plan generated in batch"
            );

            // Collect document changes from this plan (file renames + text edits)
            if let Some(ref doc_changes) = plan.edits.document_changes {
                match doc_changes {
                    lsp_types::DocumentChanges::Operations(ops) => {
                        all_document_changes.extend(ops.clone());
                    }
                    lsp_types::DocumentChanges::Edits(edits) => {
                        // Convert edits to operations
                        for edit in edits {
                            all_document_changes
                                .push(lsp_types::DocumentChangeOperation::Edit(edit.clone()));
                        }
                    }
                }
            }

            // Merge file checksums
            all_file_checksums.extend(plan.file_checksums);

            // Track affected files from the actual edits (not just target path)
            // This ensures cross-file symbol renames count all affected files
            if let Some(ref changes) = plan.edits.changes {
                for uri in changes.keys() {
                    if let Some(path) = url::Url::parse(uri.as_str())
                        .ok()
                        .and_then(|u| u.to_file_path().ok())
                    {
                        total_affected_files.insert(path);
                    }
                }
            }
            if let Some(ref doc_changes) = plan.edits.document_changes {
                // Helper to convert lsp_types::Uri to file path
                let uri_to_path = |uri: &lsp_types::Uri| -> Option<std::path::PathBuf> {
                    url::Url::parse(uri.as_str())
                        .ok()
                        .and_then(|u| u.to_file_path().ok())
                };

                match doc_changes {
                    lsp_types::DocumentChanges::Operations(ops) => {
                        for op in ops {
                            match op {
                                lsp_types::DocumentChangeOperation::Edit(edit) => {
                                    if let Some(path) = uri_to_path(&edit.text_document.uri) {
                                        total_affected_files.insert(path);
                                    }
                                }
                                lsp_types::DocumentChangeOperation::Op(resource_op) => {
                                    match resource_op {
                                        lsp_types::ResourceOp::Create(c) => {
                                            if let Some(path) = uri_to_path(&c.uri) {
                                                total_affected_files.insert(path);
                                            }
                                        }
                                        lsp_types::ResourceOp::Rename(r) => {
                                            if let Some(path) = uri_to_path(&r.old_uri) {
                                                total_affected_files.insert(path);
                                            }
                                            if let Some(path) = uri_to_path(&r.new_uri) {
                                                total_affected_files.insert(path);
                                            }
                                        }
                                        lsp_types::ResourceOp::Delete(d) => {
                                            if let Some(path) = uri_to_path(&d.uri) {
                                                total_affected_files.insert(path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    lsp_types::DocumentChanges::Edits(edits) => {
                        for edit in edits {
                            if let Some(path) = uri_to_path(&edit.text_document.uri) {
                                total_affected_files.insert(path);
                            }
                        }
                    }
                }
            }
        }

        // Filter duplicate full-file edits (keep first=batch version)
        let mut seen_full_file_edits: HashSet<String> = HashSet::new();
        let filtered_changes: Vec<_> = all_document_changes
            .into_iter()
            .filter(|change| {
                if let lsp_types::DocumentChangeOperation::Edit(edit) = change {
                    let is_full_file = edit.edits.iter().any(|e| {
                        let text_edit = match e {
                            lsp_types::OneOf::Left(te) => te,
                            lsp_types::OneOf::Right(ae) => &ae.text_edit,
                        };
                        text_edit.range.start.line == 0 && text_edit.range.start.character == 0
                    });
                    if is_full_file {
                        let uri = edit.text_document.uri.to_string();
                        if seen_full_file_edits.contains(&uri) {
                            return false;
                        } else {
                            seen_full_file_edits.insert(uri);
                            return true;
                        }
                    }
                }
                true
            })
            .collect();

        // Deduplicate and merge text edits for the same file
        // This prevents "last edit wins" when multiple targets modify the same config file
        let deduped_document_changes = Self::dedupe_document_changes(filtered_changes);

        // Build merged WorkspaceEdit with documentChanges
        let merged_workspace_edit = WorkspaceEdit {
            changes: None,
            document_changes: Some(lsp_types::DocumentChanges::Operations(
                deduped_document_changes,
            )),
            change_annotations: None,
        };

        // Build summary
        let summary = PlanSummary {
            affected_files: total_affected_files.len(),
            created_files: 0,
            deleted_files: 0,
        };

        // Build metadata
        let metadata = PlanMetadata {
            plan_version: "1.0".to_string(),
            kind: "batch_rename".to_string(),
            language: "multi".to_string(),
            estimated_impact: utils::estimate_impact(total_affected_files.len()),
            created_at: chrono::Utc::now().to_rfc3339(),
        };

        info!(
            targets_count = targets.len(),
            affected_files = total_affected_files.len(),
            "Batch rename plan completed"
        );

        Ok(RenamePlan {
            edits: merged_workspace_edit,
            summary,
            warnings,
            metadata,
            file_checksums: all_file_checksums,
            is_consolidation: false,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lsp_types::{
        DocumentChangeOperation, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range,
        ResourceOp, TextDocumentEdit, TextEdit,
    };
    #[test]
    fn test_dedupe_document_changes_merges_edits() {
        let uri: lsp_types::Uri = "file:///test/file.rs".parse().unwrap();

        // Edit 1: line 10
        let edit1 = TextEdit {
            range: Range {
                start: Position {
                    line: 10,
                    character: 0,
                },
                end: Position {
                    line: 10,
                    character: 5,
                },
            },
            new_text: "new1".to_string(),
        };

        // Edit 2: line 5
        let edit2 = TextEdit {
            range: Range {
                start: Position {
                    line: 5,
                    character: 0,
                },
                end: Position {
                    line: 5,
                    character: 5,
                },
            },
            new_text: "new2".to_string(),
        };

        let change1 = DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(edit1.clone())],
        });

        let change2 = DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(edit2.clone())],
        });

        let changes = vec![change1, change2];
        let deduped = RenameService::dedupe_document_changes(changes);

        assert_eq!(deduped.len(), 1);
        match &deduped[0] {
            DocumentChangeOperation::Edit(edit) => {
                assert_eq!(edit.text_document.uri, uri);
                assert_eq!(edit.edits.len(), 2);

                // Should be sorted in reverse order (line 10 then line 5)
                let e1 = match &edit.edits[0] {
                    OneOf::Left(e) => e,
                    _ => panic!("Expected TextEdit"),
                };
                let e2 = match &edit.edits[1] {
                    OneOf::Left(e) => e,
                    _ => panic!("Expected TextEdit"),
                };

                assert_eq!(e1.range.start.line, 10);
                assert_eq!(e2.range.start.line, 5);
            }
            _ => panic!("Expected Edit operation"),
        }
    }

    #[test]
    fn test_dedupe_document_changes_preserves_other_ops() {
        let uri: lsp_types::Uri = "file:///test/file.rs".parse().unwrap();

        // Use non-empty edit so it's not dropped by optimization
        let edit = DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
                new_text: "foo".to_string(),
            })],
        });

        let create = DocumentChangeOperation::Op(ResourceOp::Create(lsp_types::CreateFile {
            uri: "file:///test/new.rs".parse().unwrap(),
            options: None,
            annotation_id: None,
        }));

        let changes = vec![create.clone(), edit];
        let deduped = RenameService::dedupe_document_changes(changes);

        assert_eq!(deduped.len(), 2);
        // Order: Preserved (Create then Edit) as they are independent files
        match &deduped[0] {
            DocumentChangeOperation::Op(ResourceOp::Create(_)) => {}
            op => panic!("Expected Create first, but got {:?}", op),
        }
        match &deduped[1] {
            DocumentChangeOperation::Edit(_) => {}
            op => panic!("Expected Edit second, but got {:?}", op),
        }
    }

    #[test]
    fn test_dedupe_document_changes_preserves_order_relative_to_rename() {
        let uri_a: lsp_types::Uri = "file:///test/file_a.rs".parse().unwrap();
        let uri_b: lsp_types::Uri = "file:///test/file_b.rs".parse().unwrap();

        // Original sequence: Rename A->B, then Edit B
        // This simulates: rename file, then update content in the new file (e.g. package name)
        let rename = DocumentChangeOperation::Op(ResourceOp::Rename(lsp_types::RenameFile {
            old_uri: uri_a.clone(),
            new_uri: uri_b.clone(),
            options: None,
            annotation_id: None,
        }));

        let edit_b = DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri_b.clone(),
                version: None,
            },
            edits: vec![OneOf::Left(TextEdit {
                range: Range {
                    start: Position {
                        line: 0,
                        character: 0,
                    },
                    end: Position {
                        line: 0,
                        character: 5,
                    },
                },
                new_text: "updated".to_string(),
            })],
        });

        let changes = vec![rename.clone(), edit_b.clone()];
        let deduped = RenameService::dedupe_document_changes(changes);

        // Expect: Rename A->B, THEN Edit B
        // Current implementation puts edits first, so it would result in: Edit B, Rename A->B
        // which is invalid because we can't edit B before it exists.

        assert_eq!(deduped.len(), 2);

        match &deduped[0] {
            DocumentChangeOperation::Op(ResourceOp::Rename(_)) => {}
            op => panic!("Expected Rename first, but got {:?}", op),
        }

        match &deduped[1] {
            DocumentChangeOperation::Edit(_) => {}
            op => panic!("Expected Edit second, but got {:?}", op),
        }
    }

    #[test]
    fn test_dedupe_document_changes_preserves_annotations() {
        use lsp_types::AnnotatedTextEdit;

        let uri: lsp_types::Uri = "file:///test/file.rs".parse().unwrap();

        let edit = DocumentChangeOperation::Edit(TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier {
                uri: uri.clone(),
                version: None,
            },
            edits: vec![OneOf::Right(AnnotatedTextEdit {
                text_edit: TextEdit {
                    range: Range {
                        start: Position {
                            line: 0,
                            character: 0,
                        },
                        end: Position {
                            line: 0,
                            character: 5,
                        },
                    },
                    new_text: "annotated".to_string(),
                },
                annotation_id: "test-annotation".to_string(),
            })],
        });

        let changes = vec![edit];
        let deduped = RenameService::dedupe_document_changes(changes);

        assert_eq!(deduped.len(), 1);
        match &deduped[0] {
            DocumentChangeOperation::Edit(e) => {
                assert_eq!(e.edits.len(), 1);
                match &e.edits[0] {
                    OneOf::Right(ae) => {
                        assert_eq!(ae.annotation_id, "test-annotation");
                        assert_eq!(ae.text_edit.new_text, "annotated");
                    }
                    _ => panic!("Expected AnnotatedTextEdit"),
                }
            }
            _ => panic!("Expected Edit"),
        }
    }
}
