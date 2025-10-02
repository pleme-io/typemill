"""
Main Python module for playground testing
"""

from math_utils import DataProcessor, DataItem, calculate_average, SAMPLE_DATA
from helpers import log_info, format_result


def main():
    """Main function for testing"""
    log_info("Starting Python playground example")

    # Create processor
    processor = DataProcessor(max_items=100)

    # Process sample data
    log_info(f"Processing {len(SAMPLE_DATA)} items")
    processed_items = processor.process_data(SAMPLE_DATA)

    # Display results
    for item in processed_items:
        result = format_result(item)
        print(f"Processed: {result}")

    # Calculate statistics
    values = [item.value for item in processed_items]
    avg_value = calculate_average(values)
    log_info(f"Average value: {avg_value:.2f}")

    # Get processor stats
    stats = processor.get_stats()
    print(f"Processor stats: {stats}")


if __name__ == "__main__":
    main()