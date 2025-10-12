use crate::harness::{TestClient, TestWorkspace};
use serde_json::json;
use std::path::PathBuf;

use super::ProjectFixtures;

impl ProjectFixtures {
    /// Create a performance test project with configurable complexity
    pub async fn create_performance_project(
        workspace: &TestWorkspace,
        client: &mut TestClient,
        complexity_level: usize,
    ) -> Result<Vec<PathBuf>, Box<dyn std::error::Error>> {
        let mut created_files = Vec::new();

        let base_count = complexity_level * 10;

        // Create many interface files
        for i in 0..base_count {
            let file_path = workspace.path().join(format!("perf_interface_{}.ts", i));
            let content = format!(
                r#"
export interface PerfInterface{i} {{
    id{i}: number;
    data{i}: string;
    nested{i}: {{
        value{i}: boolean;
        array{i}: number[];
        map{i}: Record<string, any>;
    }};
}}

export type Union{i} = 'type{i}A' | 'type{i}B' | 'type{i}C';

export interface Extended{i} extends PerfInterface{i} {{
    additional{i}: Union{i};
    computed{i}: () => string;
}}
"#,
                i = i
            );

            client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            created_files.push(file_path);
        }

        // Create implementation files that use the interfaces
        for i in 0..base_count / 2 {
            let file_path = workspace.path().join(format!("perf_impl_{}.ts", i));
            let imports = (0..5).map(|j| {
                let idx = (i * 5 + j) % base_count;
                format!("import {{ PerfInterface{idx}, Extended{idx}, Union{idx} }} from './perf_interface_{idx}';", idx = idx)
            }).collect::<Vec<_>>().join("\n");

            let content = format!(
                r#"
{imports}

export class PerfClass{i} {{
    private data: Map<number, any> = new Map();

    processData(items: any[]): any[] {{
        return items.map((item, index) => ({{
            ...item,
            processed: true,
            index,
            timestamp: Date.now()
        }}));
    }}

    async asyncOperation(): Promise<any[]> {{
        await new Promise(resolve => setTimeout(resolve, 1));
        return this.processData([]);
    }}

    complexComputation(input: number): number {{
        let result = input;
        for (let j = 0; j < 1000; j++) {{
            result = Math.sin(result) * Math.cos(result);
        }}
        return result;
    }}
}}
"#,
                imports = imports,
                i = i
            );

            client
                .call_tool(
                    "create_file",
                    json!({
                        "file_path": file_path.to_string_lossy(),
                        "content": content
                    }),
                )
                .await?;

            created_files.push(file_path);
        }

        Ok(created_files)
    }
}
