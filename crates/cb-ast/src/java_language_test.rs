//! Unit tests for Java language adapter and AST functionality

#[cfg(test)]
mod tests {
    use crate::language::{JavaAdapter, LanguageAdapter, ReferenceKind, ScanScope};

    #[test]
    fn test_java_find_import_declarations_top_level() {
        let source = r#"
package com.example.test;

import com.example.utils.Helper;
import com.example.utils.StringProcessor;
import com.example.data.DataItem;
import java.util.List;

public class TestClass {
    public static void main(String[] args) {
        Helper.doSomething();
    }
}
"#;

        let adapter = JavaAdapter;
        let result = adapter.find_module_references(source, "Helper", ScanScope::TopLevelOnly);

        assert!(result.is_ok(), "Should successfully parse Java source");
        let refs = result.unwrap();

        // Should find the import declaration
        assert!(
            refs.iter()
                .any(|r| r.kind == ReferenceKind::Declaration && r.text.contains("Helper")),
            "Should find Helper import declaration"
        );

        // TopLevelOnly should NOT find qualified path usage
        assert!(
            !refs.iter().any(|r| r.kind == ReferenceKind::QualifiedPath),
            "TopLevelOnly scope should not find qualified paths"
        );
    }

    #[test]
    fn test_java_find_qualified_paths() {
        let source = r#"
package com.example.test;

import com.example.utils.Helper;

public class TestClass {
    public void testMethod() {
        Helper.logInfo("Starting");
        Helper.logError("Error occurred");

        String msg = "Helper module usage";
    }
}
"#;

        let adapter = JavaAdapter;
        let result = adapter.find_module_references(source, "Helper", ScanScope::QualifiedPaths);

        assert!(result.is_ok(), "Should successfully parse Java source");
        let refs = result.unwrap();

        // Should find both import and qualified method calls
        assert!(
            refs.iter().any(|r| r.kind == ReferenceKind::Declaration),
            "Should find import declaration"
        );

        let qualified_refs: Vec<_> = refs
            .iter()
            .filter(|r| r.kind == ReferenceKind::QualifiedPath)
            .collect();

        assert!(
            qualified_refs.len() >= 2,
            "Should find at least 2 qualified method calls (Helper.logInfo, Helper.logError), found: {}",
            qualified_refs.len()
        );
    }

    #[test]
    fn test_java_find_package_imports() {
        let source = r#"
package com.example.test;

import com.example.utils.Helper;
import com.example.utils.StringProcessor;
import com.other.utils.Helper as OtherHelper;

public class TestClass {
}
"#;

        let adapter = JavaAdapter;
        let result = adapter.find_module_references(source, "utils", ScanScope::TopLevelOnly);

        assert!(result.is_ok(), "Should successfully parse Java source");
        let refs = result.unwrap();

        // Should find imports containing "utils" package
        assert!(
            refs.len() >= 2,
            "Should find at least 2 imports with 'utils' in path, found: {}",
            refs.len()
        );

        // Verify import paths contain package name
        for ref_item in &refs {
            assert!(
                ref_item.text.contains("utils"),
                "Reference text should contain 'utils': {}",
                ref_item.text
            );
        }
    }

    #[test]
    fn test_java_find_string_literals_all_scope() {
        let source = r#"
package com.example.test;

public class TestClass {
    public void testMethod() {
        String msg = "Using Helper module for logging";
        String path = "com.example.utils.Helper";
        System.out.println("Helper is useful");
    }
}
"#;

        let adapter = JavaAdapter;
        let result = adapter.find_module_references(source, "Helper", ScanScope::All);

        assert!(result.is_ok(), "Should successfully parse Java source");
        let refs = result.unwrap();

        // Should find string literals containing "Helper"
        let string_refs: Vec<_> = refs
            .iter()
            .filter(|r| r.kind == ReferenceKind::StringLiteral)
            .collect();

        assert!(
            string_refs.len() >= 2,
            "Should find at least 2 string literals containing 'Helper', found: {}",
            string_refs.len()
        );
    }

    #[test]
    fn test_java_static_method_calls() {
        let source = r#"
package com.example.test;

import com.example.utils.Helper;
import com.example.utils.StringProcessor;

public class TestClass {
    public void process(String input) {
        Helper.logInfo("Processing started");

        String formatted = StringProcessor.format(input);
        boolean valid = StringProcessor.validate(formatted);

        if (valid) {
            Helper.logInfo("Valid input");
        } else {
            Helper.logError("Invalid input");
        }
    }
}
"#;

        let adapter = JavaAdapter;

        // Test Helper references
        let helper_result =
            adapter.find_module_references(source, "Helper", ScanScope::QualifiedPaths);
        assert!(helper_result.is_ok());
        let helper_refs = helper_result.unwrap();

        let helper_qualified: Vec<_> = helper_refs
            .iter()
            .filter(|r| r.kind == ReferenceKind::QualifiedPath)
            .collect();

        assert!(
            helper_qualified.len() >= 3,
            "Should find at least 3 Helper qualified calls, found: {}",
            helper_qualified.len()
        );

        // Test StringProcessor references
        let processor_result =
            adapter.find_module_references(source, "StringProcessor", ScanScope::QualifiedPaths);
        assert!(processor_result.is_ok());
        let processor_refs = processor_result.unwrap();

        let processor_qualified: Vec<_> = processor_refs
            .iter()
            .filter(|r| r.kind == ReferenceKind::QualifiedPath)
            .collect();

        assert!(
            processor_qualified.len() >= 2,
            "Should find at least 2 StringProcessor qualified calls, found: {}",
            processor_qualified.len()
        );
    }

    #[test]
    fn test_java_no_false_positives() {
        let source = r#"
package com.example.test;

import com.example.utils.Logger;

public class TestClass {
    public void test() {
        Logger.log("Starting Helper process");
        String helperMessage = "Helper message";
    }
}
"#;

        let adapter = JavaAdapter;
        let result = adapter.find_module_references(source, "Helper", ScanScope::QualifiedPaths);

        assert!(result.is_ok());
        let refs = result.unwrap();

        // Should NOT find import or qualified calls (only string content)
        let qualified_refs: Vec<_> = refs
            .iter()
            .filter(|r| r.kind == ReferenceKind::QualifiedPath)
            .collect();

        assert_eq!(
            qualified_refs.len(),
            0,
            "Should not find qualified paths when module is not used"
        );

        let declaration_refs: Vec<_> = refs
            .iter()
            .filter(|r| r.kind == ReferenceKind::Declaration)
            .collect();

        assert_eq!(
            declaration_refs.len(),
            0,
            "Should not find import declarations when module is not imported"
        );
    }

    #[test]
    fn test_java_fully_qualified_imports() {
        let source = r#"
package com.example.test;

import com.codebuddy.example.utils.Helper;
import com.codebuddy.example.data.DataProcessor;
import com.codebuddy.example.data.DataItem;

public class Main {
    public void run() {
        DataProcessor processor = new DataProcessor();
        DataItem item = new DataItem(1, "Test", 10.0);
    }
}
"#;

        let adapter = JavaAdapter;

        // Test finding by simple class name
        let helper_result =
            adapter.find_module_references(source, "Helper", ScanScope::TopLevelOnly);
        assert!(helper_result.is_ok());
        let helper_refs = helper_result.unwrap();
        assert!(helper_refs.len() > 0, "Should find Helper import");

        // Test finding by package name segment
        let utils_result = adapter.find_module_references(source, "utils", ScanScope::TopLevelOnly);
        assert!(utils_result.is_ok());
        let utils_refs = utils_result.unwrap();
        assert!(utils_refs.len() > 0, "Should find utils package import");

        // Test finding by full package path segment
        let data_result = adapter.find_module_references(source, "data", ScanScope::TopLevelOnly);
        assert!(data_result.is_ok());
        let data_refs = data_result.unwrap();
        assert!(
            data_refs.len() >= 2,
            "Should find both data package imports"
        );
    }
}
