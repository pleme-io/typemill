//! Codebuddy playground library
//! Contains Rust code for testing LSP functionality

pub mod processor;
pub mod utils;

pub use processor::DataProcessor;
pub use utils::{process_data, calculate_average};

/// Common data structures
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataItem {
    pub id: u32,
    pub value: f64,
    pub name: String,
    pub timestamp: Option<chrono::DateTime<chrono::Utc>>,
}

impl DataItem {
    pub fn new(id: u32, value: f64, name: String) -> Self {
        Self {
            id,
            value,
            name,
            timestamp: Some(chrono::Utc::now()),
        }
    }

    pub fn is_valid(&self) -> bool {
        self.id > 0 && !self.name.trim().is_empty()
    }
}

/// Processing configuration
#[derive(Debug, Clone)]
pub struct ProcessorConfig {
    pub max_items: usize,
    pub enable_logging: bool,
    pub batch_size: usize,
}

impl Default for ProcessorConfig {
    fn default() -> Self {
        Self {
            max_items: 1000,
            enable_logging: true,
            batch_size: 100,
        }
    }
}

/// Error types for the library
#[derive(Debug, thiserror::Error)]
pub enum ProcessingError {
    #[error("Invalid data: {0}")]
    InvalidData(String),
    #[error("Processing limit exceeded: {0}")]
    LimitExceeded(usize),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, ProcessingError>;