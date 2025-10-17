package com.codebuddy.example.data;

import com.codebuddy.example.utils.Helper;
import java.util.List;
import java.util.ArrayList;
import java.util.stream.Collectors;

/**
 * Processes collections of DataItem objects
 */
public class DataProcessor {
    private int processedCount = 0;

    /**
     * Process a list of data items
     */
    public List<DataItem> processItems(List<DataItem> items) {
        Helper.logInfo("Processing " + items.size() + " items");

        List<DataItem> processed = items.stream()
            .map(this::processItem)
            .collect(Collectors.toList());

        processedCount += processed.size();
        return processed;
    }

    /**
     * Process a single item
     */
    private DataItem processItem(DataItem item) {
        // Apply some transformation
        item.setValue(item.getValue() * 1.1);
        return item;
    }

    /**
     * Calculate average value from items
     */
    public double calculateAverage(List<DataItem> items) {
        if (items == null || items.isEmpty()) {
            return 0.0;
        }

        double sum = items.stream()
            .mapToDouble(DataItem::getValue)
            .sum();

        return sum / items.size();
    }

    /**
     * Get total processed count
     */
    public int getProcessedCount() {
        return processedCount;
    }

    /**
     * Reset processor state
     */
    public void reset() {
        processedCount = 0;
        Helper.logInfo("Processor reset");
    }
}
