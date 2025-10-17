package com.codebuddy.example.utils;

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
