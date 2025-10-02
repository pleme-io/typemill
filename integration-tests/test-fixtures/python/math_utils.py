"""
Math utilities for playground testing
Contains Python code for testing LSP functionality
"""

from typing import List, Optional, Dict, Any
from dataclasses import dataclass
from datetime import datetime


@dataclass
class DataItem:
    """Data item for processing"""
    id: int
    value: float
    name: str
    timestamp: Optional[datetime] = None


class DataProcessor:
    """Main data processor class for testing"""

    def __init__(self, max_items: int = 1000):
        self.max_items = max_items
        self.items: List[DataItem] = []
        self.processed_count = 0

    def process_data(self, items: List[DataItem]) -> List[DataItem]:
        """Process a list of data items"""
        processed = []
        for item in items:
            if self.is_valid_item(item):
                processed_item = self.transform_item(item)
                processed.append(processed_item)
                self.processed_count += 1

        return processed

    def is_valid_item(self, item: DataItem) -> bool:
        """Check if an item is valid for processing"""
        return (
            item.id > 0 and
            item.value is not None and
            len(item.name.strip()) > 0
        )

    def transform_item(self, item: DataItem) -> DataItem:
        """Transform an individual item"""
        return DataItem(
            id=item.id,
            value=item.value * 2,  # Simple transformation
            name=item.name.upper(),
            timestamp=datetime.now()
        )

    def get_stats(self) -> Dict[str, Any]:
        """Get processing statistics"""
        return {
            'total_items': len(self.items),
            'processed_count': self.processed_count,
            'max_items': self.max_items
        }


def calculate_average(values: List[float]) -> float:
    """Calculate average of a list of values"""
    if not values:
        return 0.0
    return sum(values) / len(values)


def find_outliers(values: List[float], threshold: float = 2.0) -> List[float]:
    """Find outlier values based on standard deviation"""
    if len(values) < 2:
        return []

    mean = calculate_average(values)
    variance = sum((x - mean) ** 2 for x in values) / len(values)
    std_dev = variance ** 0.5

    outliers = []
    for value in values:
        if abs(value - mean) > threshold * std_dev:
            outliers.append(value)

    return outliers


# Constants for testing
DEFAULT_THRESHOLD = 1.5
MAX_PROCESSING_SIZE = 10000

# Test data
SAMPLE_DATA = [
    DataItem(1, 10.5, "First Item"),
    DataItem(2, 20.3, "Second Item"),
    DataItem(3, 15.7, "Third Item"),
]