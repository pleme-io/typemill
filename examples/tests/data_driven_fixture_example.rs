// Example: Data-Driven Test Fixture
// Location: crates/cb-test-support/src/harness/test_fixtures.rs
// Purpose: Language-specific test data for data-driven testing

#[derive(Debug, Clone)]
pub struct GoToDefinitionTestCase {
    pub language_id: &'static str,
    pub files: &'static [(&'static str, &'static str)],
    pub trigger_point: (&'static str, u32, u32),
    pub expected_location: (&'static str, u32, u32),
}

pub const GO_TO_DEFINITION_TESTS: &[GoToDefinitionTestCase] = &[
    // TypeScript Case
    GoToDefinitionTestCase {
        language_id: "ts",
        files: &[
            ("main.ts", "import { util } from './util';\nutil();"),
            ("util.ts", "export function util() {}"),
        ],
        trigger_point: ("main.ts", 0, 9),
        expected_location: ("util.ts", 0, 17),
    },
    // Python Case
    GoToDefinitionTestCase {
        language_id: "py",
        files: &[
            ("main.py", "from helper import func\nfunc()"),
            ("helper.py", "def func():\n    return 42"),
        ],
        trigger_point: ("main.py", 0, 19),
        expected_location: ("helper.py", 0, 4),
    },
    // Go Case
    GoToDefinitionTestCase {
        language_id: "go",
        files: &[
            ("main.go", "package main\n\nimport \"./helper\"\n\nfunc main() {\n    helper.DoWork()\n}"),
            ("helper/helper.go", "package helper\n\nfunc DoWork() {}"),
        ],
        trigger_point: ("main.go", 5, 11),
        expected_location: ("helper/helper.go", 2, 5),
    },
];
