//! Data processor module for Rust playground

use crate::{DataItem, ProcessorConfig, ProcessingError, Result};
use std::collections::HashMap;

/// Main data processor struct
pub struct DataProcessor {
    config: ProcessorConfig,
    items: Vec<DataItem>,
    stats: ProcessingStats,
}

#[derive(Debug, Default)]
pub struct ProcessingStats {
    pub total_processed: usize,
    pub errors: usize,
    pub average_value: f64,
}

impl DataProcessor {
    /// Create a new data processor with default configuration
    pub fn new() -> Self {
        Self::with_config(ProcessorConfig::default())
    }

    /// Create a new data processor with custom configuration
    pub fn with_config(config: ProcessorConfig) -> Self {
        Self {
            config,
            items: Vec::new(),
            stats: ProcessingStats::default(),
        }
    }

    /// Process a batch of data items
    pub fn process_batch(&mut self, items: Vec<DataItem>) -> Result<Vec<DataItem>> {
        if items.len() > self.config.max_items {
            return Err(ProcessingError::LimitExceeded(items.len()));
        }

        let mut processed = Vec::new();
        let mut errors = 0;

        for item in items {
            match self.process_single_item(item) {
                Ok(processed_item) => {
                    processed.push(processed_item);
                    self.stats.total_processed += 1;
                }
                Err(_) => {
                    errors += 1;
                    self.stats.errors += 1;
                }
            }
        }

        if self.config.enable_logging {
            println!("Processed {} items, {} errors", processed.len(), errors);
        }

        self.update_average(&processed);
        Ok(processed)
    }

    /// Process a single data item
    fn process_single_item(&self, mut item: DataItem) -> Result<DataItem> {
        if !item.is_valid() {
            return Err(ProcessingError::InvalidData(format!(
                "Invalid item with id: {}",
                item.id
            )));
        }

        // Transform the item
        item.value *= 2.0;
        item.name = item.name.to_uppercase();
        item.timestamp = Some(chrono::Utc::now());

        Ok(item)
    }

    /// Update running average
    fn update_average(&mut self, items: &[DataItem]) {
        if items.is_empty() {
            return;
        }

        let sum: f64 = items.iter().map(|item| item.value).sum();
        self.stats.average_value = sum / items.len() as f64;
    }

    /// Get processing statistics
    pub fn get_stats(&self) -> &ProcessingStats {
        &self.stats
    }

    /// Get all processed items
    pub fn get_items(&self) -> &[DataItem] {
        &self.items
    }

    /// Clear all items and reset stats
    pub fn clear(&mut self) {
        self.items.clear();
        self.stats = ProcessingStats::default();
    }

    /// Find items by value range
    pub fn find_items_in_range(&self, min_value: f64, max_value: f64) -> Vec<&DataItem> {
        self.items
            .iter()
            .filter(|item| item.value >= min_value && item.value <= max_value)
            .collect()
    }

    /// Group items by value ranges
    pub fn group_by_value_ranges(&self, range_size: f64) -> HashMap<u32, Vec<&DataItem>> {
        let mut groups = HashMap::new();

        for item in &self.items {
            let range_key = (item.value / range_size) as u32;
            groups.entry(range_key).or_insert_with(Vec::new).push(item);
        }

        groups
    }
}

impl Default for DataProcessor {
    fn default() -> Self {
        Self::new()
    }
}