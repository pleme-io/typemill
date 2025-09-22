//! Main entry point for Rust playground

use codebuddy_playground::{DataProcessor, DataItem, ProcessorConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Starting Rust playground example");

    // Create a data processor with custom config
    let config = ProcessorConfig {
        max_items: 50,
        enable_logging: true,
        batch_size: 10,
    };
    let mut processor = DataProcessor::with_config(config);

    // Create sample data
    let sample_data = vec![
        DataItem::new(1, 10.5, "First Item".to_string()),
        DataItem::new(2, 20.3, "Second Item".to_string()),
        DataItem::new(3, 15.7, "Third Item".to_string()),
        DataItem::new(4, 8.9, "Fourth Item".to_string()),
        DataItem::new(5, 25.1, "Fifth Item".to_string()),
    ];

    // Process the data
    match processor.process_batch(sample_data) {
        Ok(processed_items) => {
            println!("Successfully processed {} items", processed_items.len());

            for item in &processed_items {
                println!("  - {}: {} (value: {:.2})", item.id, item.name, item.value);
            }

            // Print statistics
            let stats = processor.get_stats();
            println!("Processing statistics:");
            println!("  Total processed: {}", stats.total_processed);
            println!("  Errors: {}", stats.errors);
            println!("  Average value: {:.2}", stats.average_value);
        }
        Err(e) => {
            eprintln!("Error processing data: {}", e);
            return Err(e.into());
        }
    }

    // Demonstrate utility functions
    let utils_data = codebuddy_playground::utils::create_sample_data();
    let average = codebuddy_playground::utils::calculate_average(&utils_data);
    println!("Utility average: {:.2}", average);

    let processed_utils = codebuddy_playground::utils::process_data(utils_data)?;
    println!("Processed {} items with utilities", processed_utils.len());

    Ok(())
}