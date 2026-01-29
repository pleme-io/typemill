//! The Planner service for converting Intents into executable Workflows.

use mill_foundation::core::model::workflow::{Intent, Step, Workflow, WorkflowMetadata};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Workflow template loaded from configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WorkflowTemplate {
    name: String,
    metadata: WorkflowMetadata,
    steps: Vec<Step>,
    required_params: Vec<String>,
}

/// Root structure for workflows.json configuration file
#[derive(Debug, Deserialize)]
struct WorkflowsConfig {
    workflows: HashMap<String, WorkflowTemplate>,
}

/// Defines the contract for a service that can plan workflows.
pub trait Planner: Send + Sync {
    fn plan_for_intent(&self, intent: &Intent) -> Result<Workflow, String>;
}

/// The default implementation of the Planner service.
pub struct DefaultPlanner {
    /// Map of intent names to their workflow templates
    recipes: HashMap<String, WorkflowTemplate>,
}

impl DefaultPlanner {
    pub fn new() -> Arc<Self> {
        Self::from_config_path(PathBuf::from(".typemill/workflows.json"))
    }

    /// Create a planner by loading workflows from a configuration file
    pub fn from_config_path(config_path: PathBuf) -> Arc<Self> {
        info!(config_path = %config_path.display(), "Loading workflow recipes from configuration");

        // Try to read the workflows configuration file
        let recipes = match fs::read_to_string(&config_path) {
            Ok(content) => match serde_json::from_str::<WorkflowsConfig>(&content) {
                Ok(config) => {
                    info!(
                        recipe_count = config.workflows.len(),
                        "Successfully loaded workflow recipes"
                    );
                    config.workflows
                }
                Err(e) => {
                    warn!(
                        error = %e,
                        config_path = %config_path.display(),
                        "Failed to parse workflows.json - using empty recipe set"
                    );
                    HashMap::new()
                }
            },
            Err(e) => {
                warn!(
                    error = %e,
                    config_path = %config_path.display(),
                    "Failed to read workflows.json - using empty recipe set"
                );
                HashMap::new()
            }
        };

        Arc::new(Self { recipes })
    }

    /// Replace placeholders in a string with values from intent parameters
    fn replace_placeholders(text: &str, params: &Value) -> String {
        let mut result = text.to_string();

        if let Value::Object(map) = params {
            for (key, value) in map {
                let placeholder = format!("{{{}}}", key);
                let replacement = match value {
                    Value::String(s) => s.clone(),
                    Value::Number(n) => n.to_string(),
                    Value::Bool(b) => b.to_string(),
                    _ => continue,
                };
                result = result.replace(&placeholder, &replacement);
            }
        }

        result
    }

    /// Recursively replace placeholders in a JSON value
    fn replace_placeholders_in_value(value: &Value, params: &Value) -> Value {
        match value {
            Value::String(s) => {
                // Don't replace $steps placeholders, only {param} placeholders
                if s.starts_with("$steps.") {
                    value.clone()
                } else if s.starts_with("{") && s.ends_with("}") && s.len() > 2 {
                    // This is a pure placeholder like "{file_path}" - try to get the actual value
                    let param_name = &s[1..s.len() - 1];
                    if let Value::Object(map) = params {
                        if let Some(param_value) = map.get(param_name) {
                            // Return the actual value (could be string, number, etc.)
                            return param_value.clone();
                        }
                    }
                    // If not found, fall back to string replacement
                    Value::String(Self::replace_placeholders(s, params))
                } else {
                    Value::String(Self::replace_placeholders(s, params))
                }
            }
            Value::Object(map) => {
                let mut new_map = serde_json::Map::new();
                for (key, val) in map {
                    let new_key = Self::replace_placeholders(key, params);
                    new_map.insert(new_key, Self::replace_placeholders_in_value(val, params));
                }
                Value::Object(new_map)
            }
            Value::Array(arr) => Value::Array(
                arr.iter()
                    .map(|v| Self::replace_placeholders_in_value(v, params))
                    .collect(),
            ),
            _ => value.clone(),
        }
    }
}

impl Planner for DefaultPlanner {
    /// Generates a workflow for a given intent by looking up the recipe template and substituting parameters.
    fn plan_for_intent(&self, intent: &Intent) -> Result<Workflow, String> {
        debug!(intent_name = %intent.name, "Planning workflow for intent");

        // Look up the workflow template
        let template = self
            .recipes
            .get(&intent.name)
            .ok_or_else(|| format!("No workflow planner found for intent '{}'", intent.name))?;

        // Check that all required parameters are present
        for required_param in &template.required_params {
            if intent.params.get(required_param).is_none() {
                return Err(format!("Missing required parameter '{}'", required_param));
            }
        }

        // Clone the template and replace placeholders
        let workflow_name = Self::replace_placeholders(&template.name, &intent.params);

        let steps: Vec<Step> = template
            .steps
            .iter()
            .map(|step_template| {
                // Convert template params to JSON value for processing
                let template_params_value = serde_json::to_value(&step_template.params)
                    .expect("Failed to serialize step params");

                // Replace placeholders in step params
                let params =
                    Self::replace_placeholders_in_value(&template_params_value, &intent.params);

                // Replace placeholders in description
                let description =
                    Self::replace_placeholders(&step_template.description, &intent.params);

                Step {
                    tool: step_template.tool.clone(),
                    params,
                    description,
                    requires_confirmation: step_template.requires_confirmation,
                }
            })
            .collect();

        Ok(Workflow {
            name: workflow_name,
            metadata: template.metadata.clone(),
            steps,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn create_planner() -> Arc<dyn Planner> {
        // For tests, we'll use a test config with inline data
        // In production, DefaultPlanner::new() loads from .typemill/workflows.json
        let test_config = r#"
        {
            "workflows": {
                "refactor.renameSymbol": {
                    "name": "Rename '{old_name}' to '{newName}'",
                    "metadata": {
                        "complexity": 2
                    },
                    "steps": [
                        {
                            "tool": "rename_all",
                            "params": {
                                "target": {
                                    "kind": "symbol",
                                    "path": "{file_path}",
                                    "selector": {
                                        "symbol_name": "{old_name}"
                                    }
                                },
                                "newName": "{newName}",
                                "options": {
                                    "dryRun": false,
                                    "validateChecksums": true
                                }
                            },
                            "description": "Rename '{old_name}' → '{newName}' across the project",
                            "requires_confirmation": true
                        }
                    ],
                    "required_params": ["filePath", "old_name", "newName"]
                },
                "refactor.extractFunction": {
                    "name": "Extract function '{function_name}'",
                    "metadata": {
                        "complexity": 1
                    },
                    "steps": [
                        {
                            "tool": "refactor",
                            "params": {
                                "action": "extract",
                                "kind": "function",
                                "source": {
                                    "filePath": "{file_path}",
                                    "range": {
                                        "start": {"line": "{start_line}", "character": 0},
                                        "end": {"line": "{end_line}", "character": 0}
                                    },
                                    "name": "{function_name}"
                                },
                                "options": {
                                    "dryRun": false,
                                    "validateChecksums": true
                                }
                            },
                            "description": "Extract function '{function_name}' and apply changes",
                            "requires_confirmation": true
                        }
                    ],
                    "required_params": ["filePath", "start_line", "end_line", "function_name"]
                },
                "docs.generateDocstring": {
                    "name": "Generate documentation for '{symbol_name}'",
                    "metadata": {
                        "complexity": 3
                    },
                    "steps": [
                        {
                            "tool": "search_code",
                            "params": {
                                "query": "{symbol_name}",
                                "filePath": "{filePath}"
                            },
                            "description": "Find the location of symbol '{symbol_name}'",
                            "requires_confirmation": null
                        },
                        {
                            "tool": "inspect_code",
                            "params": {
                                "filePath": "{filePath}",
                                "line": "$steps.0.symbols.0.range.start.line",
                                "character": "$steps.0.symbols.0.range.start.character",
                                "include": ["typeInfo"]
                            },
                            "description": "Get signature information for '{symbol_name}'",
                            "requires_confirmation": null
                        },
                        {
                            "tool": "apply_workspace_edit",
                            "params": {
                                "changes": {
                                    "{filePath}": [
                                        {
                                            "range": {
                                                "start": {
                                                    "line": "$steps.0.symbols.0.range.start.line",
                                                    "character": 0
                                                },
                                                "end": {
                                                    "line": "$steps.0.symbols.0.range.start.line",
                                                    "character": 0
                                                }
                                            },
                                            "newText": "/** TODO: Add documentation based on: $steps.1.contents */\\n"
                                        }
                                    ]
                                }
                            },
                            "description": "Insert placeholder docstring above symbol",
                            "requires_confirmation": null
                        }
                    ],
                    "required_params": ["filePath", "symbol_name"]
                }
            }
        }
        "#;

        let config: WorkflowsConfig = serde_json::from_str(test_config).unwrap();
        Arc::new(DefaultPlanner {
            recipes: config.workflows,
        })
    }

    // All recipes are loaded from workflows.json configuration
    // See .typemill/workflows.json for recipe definitions

    #[test]
    fn test_plan_rename_symbol() {
        let planner = create_planner();
        let intent = Intent {
            name: "refactor.renameSymbol".to_string(),
            params: json!({
                "filePath": "src/test.ts",
                "old_name": "oldFunc",
                "newName": "newFunc"
            }),
        };

        let result = planner.plan_for_intent(&intent);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "Rename 'oldFunc' to 'newFunc'");
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.metadata.complexity, 2);

        // Check rename step (unified API with dryRun: false)
        assert_eq!(workflow.steps[0].tool, "rename_all");
        assert!(workflow.steps[0].params.get("target").is_some());
        assert_eq!(
            workflow.steps[0]
                .params
                .get("newName")
                .unwrap()
                .as_str()
                .unwrap(),
            "newFunc"
        );
        assert_eq!(workflow.steps[0].requires_confirmation, Some(true));
    }

    #[test]
    fn test_plan_extract_function() {
        let planner = create_planner();
        let intent = Intent {
            name: "refactor.extractFunction".to_string(),
            params: json!({
                "filePath": "src/main.ts",
                "start_line": 10,
                "end_line": 20,
                "function_name": "extractedFunc"
            }),
        };

        let result = planner.plan_for_intent(&intent);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "Extract function 'extractedFunc'");
        assert_eq!(workflow.steps.len(), 1);
        assert_eq!(workflow.metadata.complexity, 1);

        // Check extract step (unified API with dryRun: false)
        let step = &workflow.steps[0];
        assert_eq!(step.tool, "refactor");
        assert_eq!(
            step.params.get("action").unwrap().as_str().unwrap(),
            "extract"
        );
        assert!(step.params.get("kind").is_some());
        assert!(step.params.get("source").is_some());
        assert_eq!(step.requires_confirmation, Some(true));
    }

    #[test]
    fn test_plan_generate_docstring() {
        let planner = create_planner();
        let intent = Intent {
            name: "docs.generateDocstring".to_string(),
            params: json!({
                "filePath": "src/utils.ts",
                "symbol_name": "myFunction"
            }),
        };

        let result = planner.plan_for_intent(&intent);
        assert!(result.is_ok());

        let workflow = result.unwrap();
        assert_eq!(workflow.name, "Generate documentation for 'myFunction'");
        assert_eq!(workflow.steps.len(), 3);
        assert_eq!(workflow.metadata.complexity, 3);

        // Check step 1: search_code
        assert_eq!(workflow.steps[0].tool, "search_code");
        assert_eq!(
            workflow.steps[0]
                .params
                .get("query")
                .unwrap()
                .as_str()
                .unwrap(),
            "myFunction"
        );

        // Check step 2: inspect_code
        assert_eq!(workflow.steps[1].tool, "inspect_code");
        assert_eq!(
            workflow.steps[1]
                .params
                .get("line")
                .unwrap()
                .as_str()
                .unwrap(),
            "$steps.0.symbols.0.range.start.line"
        );

        // Check step 3: apply_workspace_edit
        assert_eq!(workflow.steps[2].tool, "apply_workspace_edit");
        assert!(workflow.steps[2].params.get("changes").is_some());
    }

    #[test]
    fn test_plan_missing_parameters() {
        let planner = create_planner();
        let intent = Intent {
            name: "refactor.renameSymbol".to_string(),
            params: json!({
                "filePath": "src/test.ts",
                // Missing old_name and new_name
            }),
        };

        let result = planner.plan_for_intent(&intent);
        assert!(result.is_err());
        let error_msg = result.unwrap_err();
        assert!(
            error_msg.contains("Missing required parameter 'old_name'")
                || error_msg.contains("Missing 'old_name'")
        );
    }

    #[test]
    fn test_plan_unknown_intent() {
        let planner = create_planner();
        let intent = Intent {
            name: "unknown.intent".to_string(),
            params: json!({}),
        };

        let result = planner.plan_for_intent(&intent);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("No workflow planner found"));
    }
}
