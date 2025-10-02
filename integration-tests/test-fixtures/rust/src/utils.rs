//! Utility functions for Rust playground

use crate::{DataItem, Result, ProcessingError};

/// Process a vector of data items with filtering
pub fn process_data(items: Vec<DataItem>) -> Result<Vec<DataItem>> {
    let mut processed = Vec::new();

    for item in items {
        if item.is_valid() {
            let mut processed_item = item.clone();
            processed_item.value *= 1.5;
            processed_item.name = format!("Processed_{}", processed_item.name);
            processed.push(processed_item);
        }
    }

    Ok(processed)
}

/// Calculate average value from a vector of items
pub fn calculate_average(items: &[DataItem]) -> f64 {
    if items.is_empty() {
        return 0.0;
    }

    let sum: f64 = items.iter().map(|item| item.value).sum();
    sum / items.len() as f64
}

/// Find outliers in data based on standard deviation
pub fn find_outliers(items: &[DataItem], threshold: f64) -> Vec<&DataItem> {
    if items.len() < 2 {
        return Vec::new();
    }

    let mean = calculate_average(items);
    let variance = items
        .iter()
        .map(|item| (item.value - mean).powi(2))
        .sum::<f64>() / items.len() as f64;
    let std_dev = variance.sqrt();

    items
        .iter()
        .filter(|item| (item.value - mean).abs() > threshold * std_dev)
        .collect()
}

/// Sort items by value
pub fn sort_by_value(items: &mut [DataItem]) {
    items.sort_by(|a, b| a.value.partial_cmp(&b.value).unwrap_or(std::cmp::Ordering::Equal));
}

/// Sort items by name
pub fn sort_by_name(items: &mut [DataItem]) {
    items.sort_by(|a, b| a.name.cmp(&b.name));
}

/// Filter items by minimum value
pub fn filter_by_min_value(items: Vec<DataItem>, min_value: f64) -> Vec<DataItem> {
    items
        .into_iter()
        .filter(|item| item.value >= min_value)
        .collect()
}

/// Validate a batch of items
pub fn validate_batch(items: &[DataItem]) -> Result<()> {
    for item in items {
        if !item.is_valid() {
            return Err(ProcessingError::InvalidData(format!(
                "Invalid item with id: {}",
                item.id
            )));
        }
    }
    Ok(())
}

/// Create sample data for testing
pub fn create_sample_data() -> Vec<DataItem> {
    vec![
        DataItem::new(1, 10.5, "First".to_string()),
        DataItem::new(2, 20.3, "Second".to_string()),
        DataItem::new(3, 15.7, "Third".to_string()),
        DataItem::new(4, 8.9, "Fourth".to_string()),
        DataItem::new(5, 25.1, "Fifth".to_string()),
    ]
}

/// Format item for display
pub fn format_item(item: &DataItem) -> String {
    format!(
        "ID: {}, Name: {}, Value: {:.2}",
        item.id, item.name, item.value
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_average() {
        let items = create_sample_data();
        let avg = calculate_average(&items);
        assert!(avg > 0.0);
    }

    #[test]
    fn test_process_data() {
        let items = create_sample_data();
        let result = process_data(items);
        assert!(result.is_ok());
    }
}