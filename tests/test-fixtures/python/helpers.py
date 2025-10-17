"""
Helper functions for Python playground
"""

from datetime import datetime
from typing import Any
from math_utils import DataItem


def log_info(message: str) -> None:
    """Log an info message with timestamp"""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{timestamp}] INFO: {message}")


def log_error(message: str) -> None:
    """Log an error message with timestamp"""
    timestamp = datetime.now().strftime("%Y-%m-%d %H:%M:%S")
    print(f"[{timestamp}] ERROR: {message}")


def format_result(item: DataItem) -> str:
    """Format a data item for display"""
    timestamp_str = item.timestamp.strftime("%H:%M:%S") if item.timestamp else "N/A"
    return f"ID: {item.id}, Name: {item.name}, Value: {item.value:.2f}, Time: {timestamp_str}"


def safe_divide(a: float, b: float) -> float:
    """Safely divide two numbers"""
    if b == 0:
        log_error("Division by zero attempted")
        return 0.0
    return a / b


def validate_input(value: Any) -> bool:
    """Validate input data"""
    if value is None:
        log_error("None value provided")
        return False

    if isinstance(value, str) and len(value.strip()) == 0:
        log_error("Empty string provided")
        return False

    return True