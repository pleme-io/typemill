// test_data/complex.ts

// Unused import for dead_code analysis
import { B } from './circular';

// High complexity function for quality analysis
export function highComplexityFunction(a: number, b: number, c: number): number {
    if (a > b) {
        if (b > c) {
            if (a > c) {
                return a;
            } else {
                return c;
            }
        } else {
            return b;
        }
    } else {
        if (b < c) {
            if (a < c) {
                return c;
            } else {
                return a;
            }
        } else {
            return b;
        }
    }
}

// Undocumented function for documentation analysis
export function undocumentedFunction() {
    console.log("This function is not documented.");
}

// Function with a potential test coverage issue for tests analysis
export function untestedFunction(value: number) {
    if (value > 0) {
        return true;
    }
    return false;
}
