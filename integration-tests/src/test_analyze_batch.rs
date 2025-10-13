use crate::harness::{client::TestClient, workspace::TestWorkspace};
use serde_json::json;

#[tokio::test]
async fn test_analyze_batch_all_categories() {
    let workspace = TestWorkspace::new();
    let file_content = r#"
use std::collections::HashMap; // unused import

fn long_function() {
    println!("1");
    println!("2");
    println!("3");
    println!("4");
    println!("5");
    println!("6");
    println!("7");
    println!("8");
    println!("9");
    println!("10");
    println!("11");
    println!("12");
    println!("13");
    println!("14");
    println!("15");
    println!("16");
    println!("17");
    println!("18");
    println!("19");
    println!("20");
    println!("21");
    println!("22");
    println!("23");
    println!("24");
    println!("25");
    println!("26");
    println!("27");
    println!("28");
    println!("29");
    println!("30");
    println!("31");
    println!("32");
    println!("33");
    println!("34");
    println!("35");
    println!("36");
    println!("37");
    println!("38");
    println!("39");
    println!("40");
    println!("41");
    println!("42");
    println!("43");
    println!("44");
    println!("45");
    println!("46");
    println!("47");
    println!("48");
    println!("49");
    println!("50");
    println!("51");
}

// TODO: Add a test for this
fn untested_function() {
    println!("This function is not tested");
}
"#;
    workspace.create_file("src/main.rs", file_content);

    let mut client = TestClient::new(workspace.path());

    let response = client
        .call_tool(
            "analyze.batch",
            json!({
                "queries": [
                    {
                        "command": "analyze.quality",
                        "kind": "smells",
                        "scope": { "type": "file", "path": "src/main.rs" }
                    },
                    {
                        "command": "analyze.dead_code",
                        "kind": "unused_imports",
                        "scope": { "type": "file", "path": "src/main.rs" }
                    },
                    {
                        "command": "analyze.dependencies",
                        "kind": "imports",
                        "scope": { "type": "file", "path": "src/main.rs" }
                    },
                    {
                        "command": "analyze.structure",
                        "kind": "symbols",
                        "scope": { "type": "file", "path": "src/main.rs" }
                    },
                    {
                        "command": "analyze.documentation",
                        "kind": "todos",
                        "scope": { "type": "file", "path": "src/main.rs" }
                    },
                    {
                        "command": "analyze.tests",
                        "kind": "coverage",
                        "scope": { "type": "file", "path": "src/main.rs" }
                    }
                ]
            }),
        )
        .await
        .expect("analyze.batch call should succeed");

    let result = response
        .get("result")
        .and_then(|r| r.as_object())
        .expect("Response should have a result object");

    let results_array = result.get("results").and_then(|r| r.as_array()).expect("Should have results array");
    assert_eq!(results_array.len(), 6, "Should have a result for each of the 6 queries");

    let quality_result = results_array.iter().find(|r| r["command"] == "analyze.quality").expect("Should have quality result");
    let quality_findings = quality_result["result"]["findings"].as_array().unwrap();
    assert!(!quality_findings.is_empty(), "Should have findings for category 'quality'");
    let first_finding = &quality_findings[0];
    assert_eq!(first_finding["kind"], "long_method");
    assert!(first_finding["metrics"]["sloc"].as_u64().unwrap() > 50);
    assert_eq!(first_finding["severity"], "medium");
}
