package com.codebuddy.example;

import com.codebuddy.example.utils.Helper;
import com.codebuddy.example.utils.StringProcessor;
import com.codebuddy.example.data.DataItem;
import com.codebuddy.example.data.DataProcessor;

import java.util.List;
import java.util.ArrayList;

/**
 * Main class demonstrating Java AST functionality
 */
public class Main {

    public static void main(String[] args) {
        System.out.println("Starting Java playground example");

        // Use Helper utility
        Helper.logInfo("Application started");

        // Create data processor
        DataProcessor processor = new DataProcessor();

        // Create sample data items
        List<DataItem> items = new ArrayList<>();
        items.add(new DataItem(1, "First Item", 10.5));
        items.add(new DataItem(2, "Second Item", 20.3));
        items.add(new DataItem(3, "Third Item", 15.7));

        // Process data
        List<DataItem> processed = processor.processItems(items);

        // Display results
        for (DataItem item : processed) {
            String formatted = StringProcessor.format(item.getName());
            Helper.logInfo("Processed: " + formatted + " - Value: " + item.getValue());
        }

        // Calculate statistics
        double average = processor.calculateAverage(processed);
        Helper.logInfo("Average value: " + average);

        // Use qualified static method call
        Helper.printSeparator();
    }
}
