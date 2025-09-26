//! LSP (Language Server Protocol) message types and structures

use serde::{Deserialize, Serialize};

/// LSP request message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspRequest {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// LSP response message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspResponse {
    pub jsonrpc: String,
    pub id: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<LspError>,
}

/// LSP notification message
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspNotification {
    pub jsonrpc: String,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// LSP error object
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// LSP position in a document
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspPosition {
    /// Line position (0-based)
    pub line: u32,
    /// Character position (0-based, UTF-16 code units)
    pub character: u32,
}

/// LSP range in a document
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspRange {
    /// The range's start position
    pub start: LspPosition,
    /// The range's end position
    pub end: LspPosition,
}

/// LSP location reference
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspLocation {
    /// The location's URI
    pub uri: String,
    /// The location's range
    pub range: LspRange,
}

/// LSP text document identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspTextDocumentIdentifier {
    /// The text document's URI
    pub uri: String,
}

/// LSP versioned text document identifier
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspVersionedTextDocumentIdentifier {
    /// The text document's URI
    pub uri: String,
    /// The version number of this document
    pub version: Option<i32>,
}

/// LSP text document position params
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspTextDocumentPositionParams {
    /// The text document
    pub text_document: LspTextDocumentIdentifier,
    /// The position inside the text document
    pub position: LspPosition,
}

/// LSP definition params
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspDefinitionParams {
    #[serde(flatten)]
    pub text_document_position_params: LspTextDocumentPositionParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress_params: Option<LspWorkDoneProgressParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_result_params: Option<LspPartialResultParams>,
}

/// LSP references params
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspReferencesParams {
    #[serde(flatten)]
    pub text_document_position_params: LspTextDocumentPositionParams,
    pub context: LspReferenceContext,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress_params: Option<LspWorkDoneProgressParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_result_params: Option<LspPartialResultParams>,
}

/// LSP reference context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspReferenceContext {
    /// Include the declaration of the current symbol
    pub include_declaration: bool,
}

/// LSP hover params
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspHoverParams {
    #[serde(flatten)]
    pub text_document_position_params: LspTextDocumentPositionParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress_params: Option<LspWorkDoneProgressParams>,
}

/// LSP hover result
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspHover {
    /// The hover's content
    pub contents: LspMarkupContent,
    /// An optional range
    #[serde(skip_serializing_if = "Option::is_none")]
    pub range: Option<LspRange>,
}

/// LSP markup content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspMarkupContent {
    /// The type of the markup
    pub kind: String, // "plaintext" or "markdown"
    /// The content itself
    pub value: String,
}

/// LSP completion params
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspCompletionParams {
    #[serde(flatten)]
    pub text_document_position_params: LspTextDocumentPositionParams,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<LspCompletionContext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_progress_params: Option<LspWorkDoneProgressParams>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_result_params: Option<LspPartialResultParams>,
}

/// LSP completion context
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspCompletionContext {
    /// How the completion was triggered
    pub trigger_kind: u32,
    /// The trigger character (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trigger_character: Option<String>,
}

/// LSP completion item
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspCompletionItem {
    /// The label of this completion item
    pub label: String,
    /// The kind of this completion item
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<u32>,
    /// A human-readable string with additional information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    /// A human-readable string that represents a doc-comment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub documentation: Option<LspDocumentation>,
    /// Indicates if this item is deprecated
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    /// Select this item when showing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preselect: Option<bool>,
    /// A string that should be used when filtering
    #[serde(skip_serializing_if = "Option::is_none")]
    pub filter_text: Option<String>,
    /// A string that should be inserted into a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text: Option<String>,
    /// The format of the insert text
    #[serde(skip_serializing_if = "Option::is_none")]
    pub insert_text_format: Option<u32>,
    /// An edit which is applied to a document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text_edit: Option<LspTextEdit>,
    /// An optional array of additional text edits
    #[serde(skip_serializing_if = "Option::is_none")]
    pub additional_text_edits: Option<Vec<LspTextEdit>>,
    /// An optional array of commit characters
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commit_characters: Option<Vec<String>>,
    /// An optional command that is executed after inserting
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<LspCommand>,
    /// Additional data preserved between a completion request and resolve
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// LSP documentation content
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged)]
#[non_exhaustive]
pub enum LspDocumentation {
    String(String),
    MarkupContent(LspMarkupContent),
}

/// LSP text edit
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspTextEdit {
    /// The range of the text document to be manipulated
    pub range: LspRange,
    /// The string to be inserted
    pub new_text: String,
}

/// LSP command
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspCommand {
    /// Title of the command
    pub title: String,
    /// The identifier of the actual command handler
    pub command: String,
    /// Arguments that the command handler should be invoked with
    #[serde(skip_serializing_if = "Option::is_none")]
    pub arguments: Option<Vec<serde_json::Value>>,
}

/// LSP work done progress params
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspWorkDoneProgressParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub work_done_token: Option<serde_json::Value>,
}

/// LSP partial result params
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspPartialResultParams {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub partial_result_token: Option<serde_json::Value>,
}

/// LSP diagnostic
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnostic {
    /// The range at which the message applies
    pub range: LspRange,
    /// The diagnostic's severity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<u32>,
    /// The diagnostic's code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<serde_json::Value>,
    /// An optional property to describe the error code
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code_description: Option<LspCodeDescription>,
    /// A human-readable string describing the source
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    /// The diagnostic's message
    pub message: String,
    /// Additional metadata about the diagnostic
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tags: Option<Vec<u32>>,
    /// An array of related diagnostic information
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_information: Option<Vec<LspDiagnosticRelatedInformation>>,
    /// A data entry field that is preserved
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// LSP code description
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspCodeDescription {
    /// An URI to open with more information about the diagnostic error
    pub href: String,
}

/// LSP diagnostic related information
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LspDiagnosticRelatedInformation {
    /// The location of this related diagnostic information
    pub location: LspLocation,
    /// The message of this related diagnostic information
    pub message: String,
}

impl LspRequest {
    /// Create a new LSP request
    pub fn new(id: impl Into<serde_json::Value>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: None,
        }
    }

    /// Create a new LSP request with parameters
    pub fn with_params(
        id: impl Into<serde_json::Value>,
        method: impl Into<String>,
        params: serde_json::Value,
    ) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            method: method.into(),
            params: Some(params),
        }
    }
}

impl LspResponse {
    /// Create a new success response
    pub fn success(id: impl Into<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: Some(result),
            error: None,
        }
    }

    /// Create a new error response
    pub fn error(id: impl Into<serde_json::Value>, error: LspError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: id.into(),
            result: None,
            error: Some(error),
        }
    }
}

impl LspNotification {
    /// Create a new notification
    pub fn new(method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: None,
        }
    }

    /// Create a new notification with parameters
    pub fn with_params(method: impl Into<String>, params: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params: Some(params),
        }
    }
}