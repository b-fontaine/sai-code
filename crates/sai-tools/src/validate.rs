//! Input validation utility for deserializing tool inputs from JSON.

use sai_core::error::ToolError;
use serde::de::DeserializeOwned;

/// Deserialize a `serde_json::Value` into a typed input struct.
///
/// Returns a `ToolError::InvalidInput` with a descriptive message on failure.
pub fn parse_input<T: DeserializeOwned>(input: serde_json::Value) -> Result<T, ToolError> {
    serde_json::from_value(input).map_err(|e| ToolError::InvalidInput(e.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Debug, Deserialize, PartialEq)]
    struct TestInput {
        name: String,
        count: u32,
    }

    #[test]
    fn valid_input_parses() {
        let json = serde_json::json!({"name": "foo", "count": 42});
        let result: TestInput = parse_input(json).unwrap();
        assert_eq!(
            result,
            TestInput {
                name: "foo".into(),
                count: 42
            }
        );
    }

    #[test]
    fn missing_field_returns_error() {
        let json = serde_json::json!({"name": "foo"});
        let result = parse_input::<TestInput>(json);
        assert!(result.is_err());
        match result.unwrap_err() {
            ToolError::InvalidInput(msg) => assert!(msg.contains("count")),
            other => panic!("expected InvalidInput, got {other:?}"),
        }
    }

    #[test]
    fn wrong_type_returns_error() {
        let json = serde_json::json!({"name": "foo", "count": "not a number"});
        let result = parse_input::<TestInput>(json);
        assert!(result.is_err());
    }
}
