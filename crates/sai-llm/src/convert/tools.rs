//! Tool call extraction and normalization.

use genai::chat::{StreamEnd, ToolCall as GenaiToolCall};
use sai_core::domain::tool_call::ToolCall;

/// Extract and normalize tool calls from a genai `StreamEnd` event.
///
/// Ensures:
/// - Every tool call has a non-empty `id` (generates UUID if missing).
/// - Arguments are always a parsed `serde_json::Value` object.
pub(crate) fn extract_tool_calls(end: &StreamEnd) -> Vec<ToolCall> {
    match end.captured_tool_calls() {
        Some(calls) => calls.into_iter().map(normalize_tool_call).collect(),
        None => Vec::new(),
    }
}

/// Normalize a single genai `ToolCall` into sai-core format.
fn normalize_tool_call(tc: &GenaiToolCall) -> ToolCall {
    let id = if tc.call_id.is_empty() {
        uuid::Uuid::new_v4().to_string()
    } else {
        tc.call_id.clone()
    };

    // genai already parses arguments into serde_json::Value.
    // If for some reason the value is a string containing JSON,
    // try to parse it further.
    let input = normalize_arguments(&tc.fn_arguments);

    ToolCall {
        id,
        name: tc.fn_name.clone(),
        input,
    }
}

/// Normalize tool call arguments.
///
/// If the value is a JSON string that contains a JSON object,
/// parse it. Otherwise, return as-is.
fn normalize_arguments(value: &serde_json::Value) -> serde_json::Value {
    if let serde_json::Value::String(s) = value {
        // Try to parse string as JSON
        match serde_json::from_str(s) {
            Ok(parsed @ (serde_json::Value::Object(_) | serde_json::Value::Array(_))) => parsed,
            _ => {
                // Not valid JSON or not an object — wrap as-is
                value.clone()
            }
        }
    } else {
        value.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_genai_tool_call(id: &str, name: &str, args: serde_json::Value) -> GenaiToolCall {
        GenaiToolCall {
            call_id: id.to_string(),
            fn_name: name.to_string(),
            fn_arguments: args,
            thought_signatures: None,
        }
    }

    #[test]
    fn normalizes_with_existing_id() {
        let tc = make_genai_tool_call("call_1", "read_file", serde_json::json!({"path": "a.txt"}));
        let result = normalize_tool_call(&tc);
        assert_eq!(result.id, "call_1");
        assert_eq!(result.name, "read_file");
        assert_eq!(result.input["path"], "a.txt");
    }

    #[test]
    fn generates_synthetic_id_when_empty() {
        let tc = make_genai_tool_call("", "read_file", serde_json::json!({}));
        let result = normalize_tool_call(&tc);
        assert!(!result.id.is_empty());
        // Should be a valid UUID
        assert!(uuid::Uuid::parse_str(&result.id).is_ok());
    }

    #[test]
    fn parses_json_string_arguments() {
        let tc = make_genai_tool_call(
            "c1",
            "tool",
            serde_json::Value::String(r#"{"key": "value"}"#.to_string()),
        );
        let result = normalize_tool_call(&tc);
        assert!(result.input.is_object());
        assert_eq!(result.input["key"], "value");
    }

    #[test]
    fn preserves_non_json_string_arguments() {
        let tc = make_genai_tool_call(
            "c1",
            "tool",
            serde_json::Value::String("not json".to_string()),
        );
        let result = normalize_tool_call(&tc);
        assert!(result.input.is_string());
        assert_eq!(result.input.as_str().unwrap(), "not json");
    }

    #[test]
    fn preserves_object_arguments() {
        let args = serde_json::json!({"path": "/tmp/test", "line": 42});
        let tc = make_genai_tool_call("c1", "edit", args.clone());
        let result = normalize_tool_call(&tc);
        assert_eq!(result.input, args);
    }
}
