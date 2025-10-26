#[cfg(test)]
mod test_debug {
    use crate::harness::{TestClient, TestWorkspace};
    use serde_json::json;

    #[tokio::test]
    async fn test_debug_error_message() {
        let workspace = TestWorkspace::new();
        workspace.create_file("test.ts", "export function simple() { return 1; }");
        let mut client = TestClient::new(workspace.path());
        let test_file = workspace.absolute_path("test.ts");

        let response = client
            .call_tool(
                "analyze.quality",
                json!({
                    "kind": "performance",
                    "scope": {
                        "type": "file",
                        "path": test_file.to_string_lossy()
                    }
                }),
            )
            .await;

        match response {
            Err(e) => {
                eprintln!("ERROR (Debug format): {:?}", e);
                eprintln!("ERROR (Display format): {}", e);
                panic!("Intentional panic to show error");
            }
            Ok(value) => {
                eprintln!("OK value: {:?}", value);
                panic!("Unexpected success");
            }
        }
    }
}
