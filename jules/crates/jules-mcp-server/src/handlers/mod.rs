//! MCP tool handlers

pub mod activities;
pub mod plans;
pub mod sessions;
pub mod sources;

// Re-export handler structs for easy registration in the server
pub use activities::{ListActivities, SendMessage};
pub use plans::ApprovePlan;
pub use sessions::{CreateSession, DeleteSession, GetSession, ListSessions};
pub use sources::{GetSource, ListSources};