use super::TestWorkspace;

/// Create a TypeScript project with common test files.
pub fn create_typescript_project() -> TestWorkspace {
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("test-project");

    // Create main.ts - exports and uses functions
    let main_content = r#"
import { utils } from './utils.js';
import { processor } from './processor.js';

export class TestMain {
    private value: number = 42;

    public process(input: string): string {
        return processor.transform(utils.format(input));
    }

    public getValue(): number {
        return this.value;
    }
}

export const mainInstance = new TestMain();
"#;
    workspace.create_file("src/main.ts", main_content);

    // Create utils.ts - utility functions
    let utils_content = r#"
export const utils = {
    format(input: string): string {
        return input.trim().toLowerCase();
    },

    validate(input: string): boolean {
        return input.length > 0;
    }
};

export function helperFunction(data: any): string {
    return JSON.stringify(data);
}
"#;
    workspace.create_file("src/utils.ts", utils_content);

    // Create processor.ts - data processing
    let processor_content = r#"
export const processor = {
    transform(input: string): string {
        return `processed_${input}`;
    }
};

// This function is never used - should be detected as dead code
export function unusedFunction(param: string): void {
    console.log("This is never called", param);
}

export class UnusedClass {
    private data: string;

    constructor(data: string) {
        this.data = data;
    }
}
"#;
    workspace.create_file("src/processor.ts", processor_content);

    workspace
}

/// Create a JavaScript ES module project.
pub fn create_javascript_project() -> TestWorkspace {
    let workspace = TestWorkspace::new();

    // Create package.json for ES modules
    workspace.create_file(
        "package.json",
        r#"{
  "name": "js-test-project",
  "version": "1.0.0",
  "type": "module"
}"#,
    );

    // Create main.js
    workspace.create_file(
        "main.js",
        r#"import { add, subtract } from './math.js';
import { formatMessage } from './utils.js';

export function calculate(a, b, operation) {
    if (operation === 'add') {
        return add(a, b);
    } else if (operation === 'subtract') {
        return subtract(a, b);
    }
    return 0;
}

export function displayResult(result) {
    return formatMessage(`Result: ${result}`);
}
"#,
    );

    // Create math.js
    workspace.create_file(
        "math.js",
        r#"export function add(a, b) {
    return a + b;
}

export function subtract(a, b) {
    return a - b;
}

export function multiply(a, b) {
    return a * b;
}

// Unused function for dead code detection
export function divide(a, b) {
    return b !== 0 ? a / b : null;
}
"#,
    );

    // Create utils.js
    workspace.create_file(
        "utils.js",
        r#"export function formatMessage(msg) {
    return `[INFO] ${msg}`;
}

export function logError(error) {
    console.error(`[ERROR] ${error}`);
}
"#,
    );

    workspace
}

/// Create a mixed TypeScript/JavaScript project.
pub fn create_mixed_project() -> TestWorkspace {
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("mixed-project");

    // Create TypeScript file
    workspace.create_file(
        "src/types.ts",
        r#"export interface User {
    id: number;
    name: string;
    email: string;
}

export interface Product {
    id: number;
    title: string;
    price: number;
}

export type OrderStatus = 'pending' | 'processing' | 'completed' | 'cancelled';
"#,
    );

    // Create JavaScript file that imports TypeScript
    workspace.create_file(
        "src/service.js",
        r#"import { User, Product } from './types';

export class DataService {
    constructor() {
        this.users = [];
        this.products = [];
    }

    addUser(user) {
        this.users.push(user);
    }

    addProduct(product) {
        this.products.push(product);
    }

    getUser(id) {
        return this.users.find(u => u.id === id);
    }
}
"#,
    );

    // Create TypeScript file that imports JavaScript
    workspace.create_file(
        "src/controller.ts",
        r#"import { DataService } from './service.js';
import { User, Product } from './types';

export class Controller {
    private service: DataService;

    constructor() {
        this.service = new DataService();
    }

    createUser(name: string, email: string): User {
        const user: User = {
            id: Date.now(),
            name,
            email
        };
        this.service.addUser(user);
        return user;
    }
}
"#,
    );

    workspace
}

/// Create a Java project with Maven structure and realistic code examples.
pub fn create_java_project() -> TestWorkspace {
    let workspace = TestWorkspace::new();
    workspace.setup_java_project("java-test-project");

    // Create Main.java
    let main_content = r#"package com.codebuddy.example;

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
"#;
    workspace.create_file(
        "src/main/java/com/codebuddy/example/Main.java",
        main_content,
    );

    // Create Helper.java
    let helper_content = r#"package com.codebuddy.example.utils;

import java.time.LocalDateTime;
import java.time.format.DateTimeFormatter;

/**
 * Helper utility class with static methods
 */
public class Helper {

    private static final DateTimeFormatter FORMATTER =
        DateTimeFormatter.ofPattern("yyyy-MM-dd HH:mm:ss");

    /**
     * Log an informational message with timestamp
     */
    public static void logInfo(String message) {
        String timestamp = LocalDateTime.now().format(FORMATTER);
        System.out.println("[INFO] " + timestamp + " - " + message);
    }

    /**
     * Log an error message
     */
    public static void logError(String message) {
        System.err.println("[ERROR] " + message);
    }

    /**
     * Print a separator line
     */
    public static void printSeparator() {
        System.out.println("=".repeat(50));
    }

    /**
     * Unused method for dead code detection
     */
    public static void unusedMethod(String param) {
        System.out.println("This method is never called: " + param);
    }
}
"#;
    workspace.create_file(
        "src/main/java/com/codebuddy/example/utils/Helper.java",
        helper_content,
    );

    // Create StringProcessor.java
    let processor_content = r#"package com.codebuddy.example.utils;

/**
 * String processing utilities
 */
public class StringProcessor {

    /**
     * Format a string by trimming and converting to title case
     */
    public static String format(String input) {
        if (input == null || input.trim().isEmpty()) {
            return "";
        }

        String trimmed = input.trim().toLowerCase();
        return Character.toUpperCase(trimmed.charAt(0)) + trimmed.substring(1);
    }

    /**
     * Validate that a string is not empty
     */
    public static boolean validate(String input) {
        return input != null && !input.trim().isEmpty();
    }

    /**
     * Truncate string to maximum length
     */
    public static String truncate(String input, int maxLength) {
        if (input == null || input.length() <= maxLength) {
            return input;
        }
        return input.substring(0, maxLength) + "...";
    }
}
"#;
    workspace.create_file(
        "src/main/java/com/codebuddy/example/utils/StringProcessor.java",
        processor_content,
    );

    // Create DataItem.java
    let item_content = r#"package com.codebuddy.example.data;

/**
 * Represents a data item with id, name, and numeric value
 */
public class DataItem {
    private int id;
    private String name;
    private double value;

    public DataItem(int id, String name, double value) {
        this.id = id;
        this.name = name;
        this.value = value;
    }

    public int getId() {
        return id;
    }

    public void setId(int id) {
        this.id = id;
    }

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }

    public double getValue() {
        return value;
    }

    public void setValue(double value) {
        this.value = value;
    }

    @Override
    public String toString() {
        return "DataItem{id=" + id + ", name='" + name + "', value=" + value + "}";
    }
}
"#;
    workspace.create_file(
        "src/main/java/com/codebuddy/example/data/DataItem.java",
        item_content,
    );

    // Create DataProcessor.java
    let data_processor_content = r#"package com.codebuddy.example.data;

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
"#;
    workspace.create_file(
        "src/main/java/com/codebuddy/example/data/DataProcessor.java",
        data_processor_content,
    );

    workspace
}

/// Create a project with circular dependencies for testing.
pub fn create_circular_dependency_project() -> TestWorkspace {
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("circular-deps");

    // Create moduleA.ts that imports moduleB
    workspace.create_file(
        "src/moduleA.ts",
        r#"import { functionB } from './moduleB';

export function functionA() {
    return 'A: ' + functionB();
}

export function helperA() {
    return 'Helper A';
}
"#,
    );

    // Create moduleB.ts that imports moduleA (circular)
    workspace.create_file(
        "src/moduleB.ts",
        r#"import { helperA } from './moduleA';

export function functionB() {
    return 'B: ' + helperA();
}
"#,
    );

    workspace
}

/// Create a project for testing rename operations.
pub fn create_rename_test_project() -> TestWorkspace {
    let workspace = TestWorkspace::new();
    workspace.setup_typescript_project("rename-test");

    // File that exports a symbol
    workspace.create_file(
        "src/exporter.ts",
        r#"export function oldFunctionName(value: string): string {
    return value.toUpperCase();
}

export class OldClassName {
    private data: string;

    constructor(data: string) {
        this.data = data;
    }

    getData(): string {
        return this.data;
    }
}

export const OLD_CONSTANT = 'constant_value';
"#,
    );

    // File that imports and uses the symbols
    workspace.create_file(
        "src/consumer.ts",
        r#"import { oldFunctionName, OldClassName, OLD_CONSTANT } from './exporter';

export function useImports() {
    const result = oldFunctionName('test');
    const instance = new OldClassName('data');
    const data = instance.getData();

    console.log(result, data, OLD_CONSTANT);

    // Multiple references to test thorough renaming
    const anotherCall = oldFunctionName('another');
    const anotherInstance = new OldClassName('more');
}
"#,
    );

    // Another consumer for cross-file testing
    workspace.create_file(
        "src/another-consumer.ts",
        r#"import { oldFunctionName as renamed, OldClassName } from './exporter';

export class ConsumerClass {
    private helper = new OldClassName('helper');

    process(input: string): string {
        return renamed(input);
    }

    getHelper(): OldClassName {
        return this.helper;
    }
}
"#,
    );

    workspace
}
