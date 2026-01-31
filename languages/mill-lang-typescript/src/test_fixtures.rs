//! TypeScript test fixtures for integration testing
//!
//! This module defines TypeScript-equivalent code samples for cross-language
//! testing. Each scenario represents a specific test case (simple function,
//! complex nested logic, generics, async/await, JSX, decorators, etc.)
//! with expected complexity metrics.

use mill_plugin_api::test_fixtures::*;

/// Get all TypeScript test fixtures
pub fn typescript_test_fixtures() -> LanguageTestFixtures {
    LanguageTestFixtures {
        complexity_scenarios: vec![
            // Scenario 1: Simple function (CC=1, Cognitive=0)
            ComplexityFixture {
                scenario_name: "simple_function",
                source_code: "function simple(x: number): number {\n    return x + 1;\n}\n",
                file_name: "simple.ts",
                expected_cyclomatic_min: 1,
                expected_cyclomatic_max: 1,
                expected_cognitive_min: 0,
                expected_cognitive_max: 1,
                expected_nesting_depth_min: 0,
            },

            // Scenario 2: Moderate complexity (CC=3)
            ComplexityFixture {
                scenario_name: "moderate_complexity",
                source_code: "function moderate(x: number): number {\n    if (x > 0) {\n        return x * 2;\n    } else if (x < 0) {\n        return x * -1;\n    } else {\n        return 0;\n    }\n}\n",
                file_name: "moderate.ts",
                expected_cyclomatic_min: 3,
                expected_cyclomatic_max: 4,
                expected_cognitive_min: 2,
                expected_cognitive_max: 5,
                expected_nesting_depth_min: 1,
            },

            // Scenario 3: High nested complexity (CC=7+)
            ComplexityFixture {
                scenario_name: "high_nested_complexity",
                source_code: "function complexNested(a: number, b: number, c: number): number {\n    if (a > 0) {\n        if (b > 0) {\n            if (c > 0) {\n                return a + b + c;\n            } else {\n                return a + b;\n            }\n        } else if (c > 0) {\n            return a + c;\n        } else {\n            return a;\n        }\n    } else if (b > 0) {\n        if (c > 0) {\n            return b + c;\n        } else {\n            return b;\n        }\n    } else {\n        return c ? c : 0;\n    }\n}\n",
                file_name: "complex.ts",
                expected_cyclomatic_min: 7,
                expected_cyclomatic_max: 10,
                expected_cognitive_min: 10,
                expected_cognitive_max: 20,
                expected_nesting_depth_min: 3,
            },

            // Scenario 4: Flat with early returns
            ComplexityFixture {
                scenario_name: "flat_early_returns",
                source_code: "function flatGuards(a: unknown, b: unknown, c: unknown): boolean {\n    if (!a) {\n        return false;\n    }\n    if (!b) {\n        return false;\n    }\n    if (!c) {\n        return false;\n    }\n    return true;\n}\n",
                file_name: "flat.ts",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 5,
                expected_cognitive_min: 3,
                expected_cognitive_max: 8,
                expected_nesting_depth_min: 1,
            },

            // Scenario 5: Generics with type constraints (TypeScript-specific)
            ComplexityFixture {
                scenario_name: "generics_complexity",
                source_code: r#"function processItems<T extends { id: number }>(
    items: T[],
    predicate: (item: T) => boolean
): T[] {
    const result: T[] = [];
    for (const item of items) {
        if (predicate(item)) {
            if (item.id > 0) {
                result.push(item);
            }
        }
    }
    return result;
}
"#,
                file_name: "generics.ts",
                expected_cyclomatic_min: 3,
                expected_cyclomatic_max: 4,
                expected_cognitive_min: 4,
                expected_cognitive_max: 10,
                expected_nesting_depth_min: 2,
            },

            // Scenario 6: Async/await with error handling (TypeScript-specific)
            ComplexityFixture {
                scenario_name: "async_await_complexity",
                source_code: r#"async function fetchWithRetry(
    url: string,
    maxRetries: number
): Promise<Response> {
    let lastError: Error | null = null;
    for (let attempt = 0; attempt < maxRetries; attempt++) {
        try {
            const response = await fetch(url);
            if (response.ok) {
                return response;
            } else if (response.status >= 500) {
                lastError = new Error(`Server error: ${response.status}`);
            } else {
                throw new Error(`Client error: ${response.status}`);
            }
        } catch (error) {
            lastError = error as Error;
            if (attempt === maxRetries - 1) {
                throw lastError;
            }
        }
    }
    throw lastError ?? new Error('Unknown error');
}
"#,
                file_name: "async.ts",
                expected_cyclomatic_min: 6,
                expected_cyclomatic_max: 9,
                expected_cognitive_min: 8,
                expected_cognitive_max: 18,
                expected_nesting_depth_min: 3,
            },

            // Scenario 7: JSX component with conditional rendering (TypeScript-specific)
            ComplexityFixture {
                scenario_name: "jsx_conditional_rendering",
                source_code: r#"interface Props {
    user: { name: string; role: string } | null;
    isLoading: boolean;
    error?: string;
}

function UserCard({ user, isLoading, error }: Props): JSX.Element {
    if (isLoading) {
        return <div className="loading">Loading...</div>;
    }
    if (error) {
        return <div className="error">{error}</div>;
    }
    if (!user) {
        return <div className="empty">No user found</div>;
    }
    return (
        <div className="card">
            <h2>{user.name}</h2>
            {user.role === 'admin' ? (
                <span className="badge admin">Admin</span>
            ) : user.role === 'moderator' ? (
                <span className="badge mod">Moderator</span>
            ) : (
                <span className="badge">User</span>
            )}
        </div>
    );
}
"#,
                file_name: "component.tsx",
                expected_cyclomatic_min: 5,
                expected_cyclomatic_max: 8,
                expected_cognitive_min: 6,
                expected_cognitive_max: 15,
                expected_nesting_depth_min: 2,
            },

            // Scenario 8: Decorator pattern (TypeScript-specific)
            ComplexityFixture {
                scenario_name: "decorator_pattern",
                source_code: r#"function log(target: any, propertyKey: string, descriptor: PropertyDescriptor) {
    const original = descriptor.value;
    descriptor.value = function(...args: any[]) {
        console.log(`Calling ${propertyKey}`);
        const result = original.apply(this, args);
        if (result instanceof Promise) {
            return result.then((r: any) => {
                console.log(`${propertyKey} resolved`);
                return r;
            }).catch((e: any) => {
                console.log(`${propertyKey} rejected`);
                throw e;
            });
        }
        console.log(`${propertyKey} returned`);
        return result;
    };
    return descriptor;
}

class Service {
    @log
    async process(data: string): Promise<string> {
        if (!data) {
            throw new Error('No data');
        }
        return data.toUpperCase();
    }
}
"#,
                file_name: "decorator.ts",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 7,
                expected_cognitive_min: 5,
                expected_cognitive_max: 12,
                expected_nesting_depth_min: 2,
            },

            // Scenario 9: Switch statement with fallthrough (TypeScript-specific)
            ComplexityFixture {
                scenario_name: "switch_complexity",
                source_code: r#"function handleAction(action: { type: string; payload?: any }): string {
    switch (action.type) {
        case 'INIT':
            return 'Initializing';
        case 'LOAD':
        case 'RELOAD':
            if (action.payload?.force) {
                return 'Force loading';
            }
            return 'Loading';
        case 'SAVE':
            if (!action.payload) {
                throw new Error('Payload required');
            }
            return 'Saving';
        case 'DELETE':
            return action.payload?.soft ? 'Soft deleting' : 'Hard deleting';
        default:
            return 'Unknown action';
    }
}
"#,
                file_name: "switch.ts",
                expected_cyclomatic_min: 7,
                expected_cyclomatic_max: 10,
                expected_cognitive_min: 6,
                expected_cognitive_max: 14,
                expected_nesting_depth_min: 2,
            },

            // Scenario 10: Type guards and narrowing (TypeScript-specific)
            ComplexityFixture {
                scenario_name: "type_guards",
                source_code: r#"interface Cat { meow(): void; }
interface Dog { bark(): void; }
type Pet = Cat | Dog;

function isCat(pet: Pet): pet is Cat {
    return 'meow' in pet;
}

function handlePet(pet: Pet | null | undefined): string {
    if (pet === null) {
        return 'No pet (null)';
    }
    if (pet === undefined) {
        return 'No pet (undefined)';
    }
    if (isCat(pet)) {
        pet.meow();
        return 'Cat handled';
    } else {
        pet.bark();
        return 'Dog handled';
    }
}
"#,
                file_name: "typeguards.ts",
                expected_cyclomatic_min: 4,
                expected_cyclomatic_max: 6,
                expected_cognitive_min: 4,
                expected_cognitive_max: 10,
                expected_nesting_depth_min: 1,
            },
        ],

        refactoring_scenarios: vec![
            // Extract variable
            RefactoringFixture {
                scenario_name: "extract_simple_expression",
                source_code: "function calculate(): number {\n    const result = 10 + 20;\n    return result;\n}\n",
                file_name: "extract_var.ts",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "sum".to_string(),
                    start_line: 1,
                    start_char: 20,
                    end_line: 1,
                    end_char: 27,
                },
            },

            // Extract function from multiline code
            RefactoringFixture {
                scenario_name: "extract_multiline_function",
                source_code: "function main(): void {\n    const x = 1;\n    const y = 2;\n    const result = x + y;\n    console.log(result);\n}\n",
                file_name: "extract_func.ts",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "addNumbers".to_string(),
                    start_line: 1,
                    start_char: 4,
                    end_line: 3,
                    end_char: 24,
                },
            },

            // Inline variable
            RefactoringFixture {
                scenario_name: "inline_simple_variable",
                source_code: "function process(): number {\n    const multiplier = 2;\n    const result = 10 * multiplier;\n    return result;\n}\n",
                file_name: "inline_var.ts",
                operation: RefactoringOperation::InlineVariable {
                    line: 1,
                    character: 10,
                },
            },

            // Extract async function
            RefactoringFixture {
                scenario_name: "extract_async_function",
                source_code: r#"async function fetchData(): Promise<void> {
    const response = await fetch('/api/data');
    const json = await response.json();
    console.log(json);
}
"#,
                file_name: "extract_async.ts",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "parseResponse".to_string(),
                    start_line: 1,
                    start_char: 4,
                    end_line: 2,
                    end_char: 38,
                },
            },

            // Extract variable from JSX expression
            RefactoringFixture {
                scenario_name: "extract_jsx_expression",
                source_code: r#"function Greeting({ name }: { name: string }): JSX.Element {
    return <h1>Hello, {name.toUpperCase()}!</h1>;
}
"#,
                file_name: "extract_jsx.tsx",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "formattedName".to_string(),
                    start_line: 1,
                    start_char: 23,
                    end_line: 1,
                    end_char: 41,
                },
            },

            // Extract function from generic code
            RefactoringFixture {
                scenario_name: "extract_generic_function",
                source_code: r#"function transform<T>(items: T[]): T[] {
    const filtered = items.filter(item => item !== null);
    const mapped = filtered.map(item => item);
    return mapped;
}
"#,
                file_name: "extract_generic.ts",
                operation: RefactoringOperation::ExtractFunction {
                    new_name: "filterAndMap".to_string(),
                    start_line: 1,
                    start_char: 4,
                    end_line: 2,
                    end_char: 47,
                },
            },

            // Inline variable in arrow function
            RefactoringFixture {
                scenario_name: "inline_arrow_variable",
                source_code: "const calculate = (x: number): number => {\n    const doubled = x * 2;\n    return doubled + 1;\n};\n",
                file_name: "inline_arrow.ts",
                operation: RefactoringOperation::InlineVariable {
                    line: 1,
                    character: 10,
                },
            },

            // Extract variable from ternary expression
            RefactoringFixture {
                scenario_name: "extract_ternary",
                source_code: "function getValue(condition: boolean): string {\n    return condition ? 'yes' : 'no';\n}\n",
                file_name: "extract_ternary.ts",
                operation: RefactoringOperation::ExtractVariable {
                    variable_name: "result".to_string(),
                    start_line: 1,
                    start_char: 11,
                    end_line: 1,
                    end_char: 35,
                },
            },
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fixtures_not_empty() {
        let fixtures = typescript_test_fixtures();
        assert!(
            !fixtures.complexity_scenarios.is_empty(),
            "Should have complexity scenarios"
        );
        assert!(
            !fixtures.refactoring_scenarios.is_empty(),
            "Should have refactoring scenarios"
        );
    }

    #[test]
    fn test_complexity_scenarios_valid() {
        let fixtures = typescript_test_fixtures();
        for scenario in &fixtures.complexity_scenarios {
            assert!(
                !scenario.scenario_name.is_empty(),
                "Scenario name should not be empty"
            );
            assert!(
                !scenario.source_code.is_empty(),
                "Source code should not be empty for {}",
                scenario.scenario_name
            );
            assert!(
                scenario.file_name.ends_with(".ts") || scenario.file_name.ends_with(".tsx"),
                "File should have .ts or .tsx extension for {}",
                scenario.scenario_name
            );
            assert!(
                scenario.expected_cyclomatic_min <= scenario.expected_cyclomatic_max,
                "Cyclomatic min should be <= max for {}",
                scenario.scenario_name
            );
            assert!(
                scenario.expected_cognitive_min <= scenario.expected_cognitive_max,
                "Cognitive min should be <= max for {}",
                scenario.scenario_name
            );
        }
    }

    #[test]
    fn test_refactoring_scenarios_valid() {
        let fixtures = typescript_test_fixtures();
        for scenario in &fixtures.refactoring_scenarios {
            assert!(
                !scenario.scenario_name.is_empty(),
                "Scenario name should not be empty"
            );
            assert!(
                !scenario.source_code.is_empty(),
                "Source code should not be empty for {}",
                scenario.scenario_name
            );
            assert!(
                scenario.file_name.ends_with(".ts") || scenario.file_name.ends_with(".tsx"),
                "File should have .ts or .tsx extension for {}",
                scenario.scenario_name
            );
        }
    }

    #[test]
    fn test_typescript_specific_scenarios() {
        let fixtures = typescript_test_fixtures();
        let scenario_names: Vec<&str> = fixtures
            .complexity_scenarios
            .iter()
            .map(|s| s.scenario_name)
            .collect();

        // Check TypeScript-specific scenarios exist
        assert!(
            scenario_names.contains(&"generics_complexity"),
            "Should have generics scenario"
        );
        assert!(
            scenario_names.contains(&"async_await_complexity"),
            "Should have async/await scenario"
        );
        assert!(
            scenario_names.contains(&"jsx_conditional_rendering"),
            "Should have JSX scenario"
        );
        assert!(
            scenario_names.contains(&"decorator_pattern"),
            "Should have decorator scenario"
        );
        assert!(
            scenario_names.contains(&"type_guards"),
            "Should have type guards scenario"
        );
    }

    #[test]
    fn test_refactoring_operations() {
        let fixtures = typescript_test_fixtures();

        let has_extract_function = fixtures
            .refactoring_scenarios
            .iter()
            .any(|s| matches!(s.operation, RefactoringOperation::ExtractFunction { .. }));
        let has_extract_variable = fixtures
            .refactoring_scenarios
            .iter()
            .any(|s| matches!(s.operation, RefactoringOperation::ExtractVariable { .. }));
        let has_inline_variable = fixtures
            .refactoring_scenarios
            .iter()
            .any(|s| matches!(s.operation, RefactoringOperation::InlineVariable { .. }));

        assert!(has_extract_function, "Should have extract function scenarios");
        assert!(has_extract_variable, "Should have extract variable scenarios");
        assert!(has_inline_variable, "Should have inline variable scenarios");
    }
}
