package com.codebuddy.example.utils;

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
