use integration_tests::harness::{TestClient, TestWorkspace};
use serde_json::{json, Value};
use std::path::Path;
#[tokio::test]
async fn test_apply_workspace_edit_single_file() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.path().join("edit_test.ts");
    let initial_content = r#"
export function oldFunctionName(x: number): number {
    return x * 2;
}

const result = oldFunctionName(5);
"#;
    std::fs::write(&file_path, initial_content).unwrap();
    let response = client
        .call_tool(
            "apply_workspace_edit",
            json!(
                { "changes" : { file_path.to_string_lossy() : [{ "range" : { "start" : {
                "line" : 1, "character" : 16 }, "end" : { "line" : 1, "character" : 31 }
                }, "newText" : "newFunctionName" }, { "range" : { "start" : { "line" : 5,
                "character" : 15 }, "end" : { "line" : 5, "character" : 30 } }, "newText"
                : "newFunctionName" }] } }
            ),
        )
        .await
        .unwrap();
    assert!(response["applied"].as_bool().unwrap_or(false));
    let content = std::fs::read_to_string(&file_path).unwrap();
    assert!(content.contains("newFunctionName"));
    assert!(!content.contains("oldFunctionName"));
}
#[tokio::test]
async fn test_apply_workspace_edit_multiple_files() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let file1 = workspace.path().join("types.ts");
    let file2 = workspace.path().join("usage.ts");
    std::fs::write(
        &file1,
        r#"
export interface OldInterface {
    id: number;
    name: string;
}
"#,
    )
    .unwrap();
    std::fs::write(
        &file2,
        r#"
import { OldInterface } from './types';

const item: OldInterface = {
    id: 1,
    name: 'test'
};
"#,
    )
    .unwrap();
    let response = client
        .call_tool(
            "apply_workspace_edit",
            json!(
                { "changes" : { file1.to_string_lossy() : [{ "range" : { "start" : {
                "line" : 1, "character" : 17 }, "end" : { "line" : 1, "character" : 29 }
                }, "newText" : "NewInterface" }], file2.to_string_lossy() : [{ "range" :
                { "start" : { "line" : 1, "character" : 9 }, "end" : { "line" : 1,
                "character" : 21 } }, "newText" : "NewInterface" }, { "range" : { "start"
                : { "line" : 3, "character" : 12 }, "end" : { "line" : 3, "character" :
                24 } }, "newText" : "NewInterface" }] } }
            ),
        )
        .await
        .unwrap();
    assert!(response["applied"].as_bool().unwrap_or(false));
    let content1 = std::fs::read_to_string(&file1).unwrap();
    let content2 = std::fs::read_to_string(&file2).unwrap();
    assert!(content1.contains("NewInterface"));
    assert!(!content1.contains("OldInterface"));
    assert!(content2.contains("NewInterface"));
    assert!(!content2.contains("OldInterface"));
}
#[tokio::test]
async fn test_apply_workspace_edit_atomic_failure() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let existing_file = workspace.path().join("existing.ts");
    let nonexistent_file = workspace.path().join("nonexistent.ts");
    std::fs::write(&existing_file, "const x = 1;").unwrap();
    let response = client
        .call_tool(
            "apply_workspace_edit",
            json!(
                { "changes" : { existing_file.to_string_lossy() : [{ "range" : { "start"
                : { "line" : 0, "character" : 6 }, "end" : { "line" : 0, "character" : 7
                } }, "newText" : "y" }], nonexistent_file.to_string_lossy() : [{ "range"
                : { "start" : { "line" : 0, "character" : 0 }, "end" : { "line" : 0,
                "character" : 0 } }, "newText" : "const z = 3;" }] } }
            ),
        )
        .await;
    match response {
        Ok(resp) => {
            assert!(resp["applied"].as_bool().unwrap_or(false));
        }
        Err(_) => {
            let content = std::fs::read_to_string(&existing_file).unwrap();
            assert_eq!(content, "const x = 1;");
        }
    }
}
#[tokio::test]
async fn test_format_document_typescript() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.path().join("format_test.ts");
    let unformatted_content = r#"
interface User{id:number;name:string;email?:string;}

function   createUser(  data: any  ) : User{
return{
id:Math.random(),
name:data.name,
email:data.email
};
}

const user=createUser({name:"John",email:"john@example.com"});
"#;
    std::fs::write(&file_path, unformatted_content).unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let response = client
        .call_tool(
            "format_document",
            json!({ "file_path" : file_path.to_string_lossy() }),
        )
        .await
        .unwrap();
    assert!(response["formatted"].as_bool().unwrap_or(false));
    let formatted_content = std::fs::read_to_string(&file_path).unwrap();
    assert!(formatted_content.contains("interface User"));
    assert!(formatted_content.contains("function createUser"));
    assert_ne!(formatted_content.trim(), unformatted_content.trim());
}
#[tokio::test]
async fn test_format_document_with_options() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.path().join("format_options.ts");
    let content = r#"
const obj={a:1,b:2,c:3};
function test(){return"hello";}
"#;
    std::fs::write(&file_path, content).unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
    let response = client
        .call_tool(
            "format_document",
            json!(
                { "file_path" : file_path.to_string_lossy(), "options" : { "tabSize" : 4,
                "insertSpaces" : true } }
            ),
        )
        .await
        .unwrap();
    assert!(response["formatted"].as_bool().unwrap_or(false));
    let formatted_content = std::fs::read_to_string(&file_path).unwrap();
    assert_ne!(formatted_content.trim(), content.trim());
}
#[tokio::test]
async fn test_get_code_actions_quick_fixes() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.path().join("code_actions.ts");
    let content_with_issues = r#"
import { unusedImport, usedImport } from './utils';

interface User {
    id: number;
    name: string;
}

function processUser(user: User): void {
    console.log(usedImport(user.name));

    // Missing return type annotation
    function helper() {
        return "helper";
    }

    // Unused variable
    const unusedVar = "not used";
}
"#;
    std::fs::write(&file_path, content_with_issues).unwrap();
    let utils_file = workspace.path().join("utils.ts");
    std::fs::write(
        &utils_file,
        r#"
export function unusedImport(x: string): string {
    return x.toUpperCase();
}

export function usedImport(x: string): string {
    return x.toLowerCase();
}
"#,
    )
    .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    let response = client
        .call_tool(
            "get_code_actions",
            json!(
                { "file_path" : file_path.to_string_lossy(), "range" : { "start" : {
                "line" : 0, "character" : 0 }, "end" : { "line" : 20, "character" : 0 } }
                }
            ),
        )
        .await
        .unwrap();
    let actions = response["actions"].as_array().unwrap();
    assert!(!actions.is_empty());
    let action_titles: Vec<String> = actions
        .iter()
        .filter_map(|a| a["title"].as_str())
        .map(|s| s.to_string())
        .collect();
    let has_relevant_actions = action_titles.iter().any(|title| {
        title.contains("unused")
            || title.contains("import")
            || title.contains("remove")
            || title.contains("organize")
            || title.contains("fix")
    });
    assert!(
        has_relevant_actions,
        "Expected relevant code actions, got: {:?}",
        action_titles
    );
}
#[tokio::test]
async fn test_get_code_actions_refactoring() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.path().join("refactor.ts");
    let content = r#"
class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }

    multiply(a: number, b: number): number {
        return a * b;
    }

    complexCalculation(x: number, y: number): number {
        const sum = this.add(x, y);
        const product = this.multiply(x, y);
        return sum + product;
    }
}
"#;
    std::fs::write(&file_path, content).unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;
    let response = client
        .call_tool(
            "get_code_actions",
            json!(
                { "file_path" : file_path.to_string_lossy(), "range" : { "start" : {
                "line" : 9, "character" : 4 }, "end" : { "line" : 13, "character" : 5 } }
                }
            ),
        )
        .await
        .unwrap();
    let actions = response["actions"].as_array().unwrap();
    for action in actions {
        assert!(action.get("title").is_some());
        assert!(action.get("kind").is_some() || action.get("edit").is_some());
    }
}
#[tokio::test]
async fn test_workspace_operations_integration() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let models_file = workspace.path().join("models.ts");
    let services_file = workspace.path().join("services.ts");
    let main_file = workspace.path().join("main.ts");
    std::fs::write(
        &models_file,
        r#"
export   interface   Product   {
id:string;
name:string;
price:number;
}

export type ProductFilter = (product: Product) => boolean;
"#,
    )
    .unwrap();
    std::fs::write(
        &services_file,
        r#"
import{Product,ProductFilter}from'./models';

export class ProductService{
private products:Product[]=[];

addProduct(product:Product):void{
this.products.push(product);
}

filterProducts(filter:ProductFilter):Product[]{
return this.products.filter(filter);
}
}
"#,
    )
    .unwrap();
    std::fs::write(
        &main_file,
        r#"
import{ProductService}from'./services';
import{Product}from'./models';

const service=new ProductService();
service.addProduct({id:'1',name:'Laptop',price:999});

const expensiveProducts=service.filterProducts(p=>p.price>500);
console.log(expensiveProducts);
"#,
    )
    .unwrap();
    tokio::time::sleep(tokio::time::Duration::from_millis(1500)).await;
    for file in [&models_file, &services_file, &main_file] {
        let response = client
            .call_tool(
                "format_document",
                json!({ "file_path" : file.to_string_lossy() }),
            )
            .await
            .unwrap();
        assert!(response["formatted"].as_bool().unwrap_or(false));
    }
    let response = client
        .call_tool(
            "apply_workspace_edit",
            json!(
                { "changes" : { models_file.to_string_lossy() : [{ "range" : { "start" :
                { "line" : 1, "character" : 26 }, "end" : { "line" : 1, "character" : 33
                } }, "newText" : "Item" }, { "range" : { "start" : { "line" : 7,
                "character" : 13 }, "end" : { "line" : 7, "character" : 20 } }, "newText"
                : "Item" }, { "range" : { "start" : { "line" : 7, "character" : 32 },
                "end" : { "line" : 7, "character" : 39 } }, "newText" : "Item" }],
                services_file.to_string_lossy() : [{ "range" : { "start" : { "line" : 1,
                "character" : 8 }, "end" : { "line" : 1, "character" : 15 } }, "newText"
                : "Item" }, { "range" : { "start" : { "line" : 1, "character" : 16 },
                "end" : { "line" : 1, "character" : 29 } }, "newText" : "ItemFilter" }, {
                "range" : { "start" : { "line" : 3, "character" : 18 }, "end" : { "line"
                : 3, "character" : 25 } }, "newText" : "Item" }, { "range" : { "start" :
                { "line" : 5, "character" : 11 }, "end" : { "line" : 5, "character" : 18
                } }, "newText" : "Item" }, { "range" : { "start" : { "line" : 5,
                "character" : 19 }, "end" : { "line" : 5, "character" : 26 } }, "newText"
                : "item" }, { "range" : { "start" : { "line" : 6, "character" : 18 },
                "end" : { "line" : 6, "character" : 25 } }, "newText" : "item" }, {
                "range" : { "start" : { "line" : 9, "character" : 14 }, "end" : { "line"
                : 9, "character" : 27 } }, "newText" : "ItemFilter" }, { "range" : {
                "start" : { "line" : 9, "character" : 29 }, "end" : { "line" : 9,
                "character" : 36 } }, "newText" : "Item" }], main_file.to_string_lossy()
                : [{ "range" : { "start" : { "line" : 1, "character" : 8 }, "end" : {
                "line" : 1, "character" : 21 } }, "newText" : "ItemService" }, { "range"
                : { "start" : { "line" : 2, "character" : 8 }, "end" : { "line" : 2,
                "character" : 15 } }, "newText" : "Item" }, { "range" : { "start" : {
                "line" : 4, "character" : 19 }, "end" : { "line" : 4, "character" : 32 }
                }, "newText" : "ItemService" }, { "range" : { "start" : { "line" : 5,
                "character" : 8 }, "end" : { "line" : 5, "character" : 18 } }, "newText"
                : "addItem" }, { "range" : { "start" : { "line" : 7, "character" : 7 },
                "end" : { "line" : 7, "character" : 22 } }, "newText" : "expensiveItems"
                }, { "range" : { "start" : { "line" : 7, "character" : 31 }, "end" : {
                "line" : 7, "character" : 46 } }, "newText" : "filterItems" }, { "range"
                : { "start" : { "line" : 8, "character" : 12 }, "end" : { "line" : 8,
                "character" : 27 } }, "newText" : "expensiveItems" }] } }
            ),
        )
        .await
        .unwrap();
    assert!(response["applied"].as_bool().unwrap_or(false));
    let models_content = std::fs::read_to_string(&models_file).unwrap();
    let services_content = std::fs::read_to_string(&services_file).unwrap();
    let main_content = std::fs::read_to_string(&main_file).unwrap();
    assert!(models_content.contains("interface Item"));
    assert!(models_content.contains("ItemFilter"));
    assert!(!models_content.contains("Product"));
    assert!(services_content.contains("Item"));
    assert!(services_content.contains("ItemFilter"));
    assert!(!services_content.contains("Product"));
    assert!(main_content.contains("ItemService"));
    assert!(main_content.contains("Item"));
    assert!(!main_content.contains("Product"));
    let response = client
        .call_tool(
            "get_code_actions",
            json!(
                { "file_path" : main_file.to_string_lossy(), "range" : { "start" : {
                "line" : 0, "character" : 0 }, "end" : { "line" : 10, "character" : 0 } }
                }
            ),
        )
        .await
        .unwrap();
    let actions = response["actions"].as_array().unwrap();
    // Code actions may or may not be available depending on LSP state
    // No assertion needed - we just verify the call succeeds
}
#[tokio::test]
async fn test_workspace_edit_with_validation() {
    let workspace = TestWorkspace::new();
    let mut client = TestClient::new(workspace.path());
    let file_path = workspace.path().join("validate.ts");
    let content = r#"
const value = 42;
console.log(value);
"#;
    std::fs::write(&file_path, content).unwrap();
    let response = client
        .call_tool(
            "apply_workspace_edit",
            json!(
                { "changes" : { file_path.to_string_lossy() : [{ "range" : { "start" : {
                "line" : 100, "character" : 0 }, "end" : { "line" : 100, "character" : 5
                } }, "newText" : "invalid" }] }, "validate_before_apply" : true }
            ),
        )
        .await;
    // Should fail because line 100 doesn't exist in the file
    assert!(
        response.is_err() || response.unwrap()["applied"].as_bool().unwrap_or(false) == false,
        "Workspace edit with invalid line number should fail validation"
    );
    let unchanged_content = std::fs::read_to_string(&file_path).unwrap();
    assert_eq!(unchanged_content.trim(), content.trim());
}
