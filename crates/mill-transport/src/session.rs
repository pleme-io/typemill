//! Session information for transport layer

#[derive(Debug, Clone, Default)]
pub struct SessionInfo {
    /// The ID of the user making the request, for multi-tenancy.
    pub user_id: Option<String>,
}
