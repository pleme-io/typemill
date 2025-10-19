//! Dry-run result wrapping utilities

use codebuddy_core::dry_run::DryRunnable;
use codebuddy_foundation::protocol::ApiResult as ServerResult;
use serde_json::{json, Value};

/// Wrap an operation result with dry-run status if applicable
pub fn wrap_dry_run_result(result: DryRunnable<Value>) -> ServerResult<Value> {
    if result.dry_run {
        // Merge status into the result object instead of nesting
        if let Value::Object(mut obj) = result.result {
            obj.insert("status".to_string(), json!("preview"));
            Ok(Value::Object(obj))
        } else {
            // Fallback for non-object results
            Ok(json!({
                "status": "preview",
                "result": result.result
            }))
        }
    } else {
        Ok(result.result)
    }
}