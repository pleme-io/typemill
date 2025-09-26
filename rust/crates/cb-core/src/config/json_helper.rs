use serde_json::{Map, Value};

/// Convert snake_case keys to camelCase recursively
pub fn to_camel_case_keys(value: Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut new_map = Map::new();
            for (key, val) in map {
                let camel_key = to_camel_case(&key);
                new_map.insert(camel_key, to_camel_case_keys(val));
            }
            Value::Object(new_map)
        }
        Value::Array(arr) => {
            Value::Array(arr.into_iter().map(to_camel_case_keys).collect())
        }
        other => other,
    }
}

/// Convert a snake_case string to camelCase
fn to_camel_case(s: &str) -> String {
    let mut result = String::new();
    let mut capitalize_next = false;

    for ch in s.chars() {
        if ch == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            result.push(ch.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            result.push(ch);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_to_camel_case() {
        assert_eq!(to_camel_case("snake_case"), "snakeCase");
        assert_eq!(to_camel_case("timeout_ms"), "timeoutMs");
        assert_eq!(to_camel_case("max_clients"), "maxClients");
        assert_eq!(to_camel_case("already_camel"), "alreadyCamel");
    }

    #[test]
    fn test_to_camel_case_keys() {
        let input = json!({
            "max_clients": 10,
            "timeout_ms": 5000,
            "nested_object": {
                "inner_field": "value"
            }
        });

        let expected = json!({
            "maxClients": 10,
            "timeoutMs": 5000,
            "nestedObject": {
                "innerField": "value"
            }
        });

        assert_eq!(to_camel_case_keys(input), expected);
    }
}