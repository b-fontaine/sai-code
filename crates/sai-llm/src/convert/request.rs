//! Convert sai-core request types to genai request types.

use genai::chat::{
    ChatMessage as GenaiMessage, ChatRequest as GenaiChatRequest, Tool as GenaiTool, ToolResponse,
};
use sai_core::domain::message::{ContentBlock, Message};
use sai_core::ports::llm::ChatRequest;

/// Convert a `sai_core::ChatRequest` into a `genai::ChatRequest`.
pub(crate) fn to_genai_request(request: &ChatRequest) -> GenaiChatRequest {
    let messages: Vec<GenaiMessage> = request.messages.iter().flat_map(convert_message).collect();

    let mut genai_req = GenaiChatRequest::new(messages);

    if let Some(ref system) = request.system_prompt {
        if !system.is_empty() {
            genai_req = genai_req.with_system(system.clone());
        }
    }

    if !request.tool_definitions.is_empty() {
        let tools: Vec<GenaiTool> = request
            .tool_definitions
            .iter()
            .filter_map(convert_tool_definition)
            .collect();
        if !tools.is_empty() {
            genai_req = genai_req.with_tools(tools);
        }
    }

    genai_req
}

/// Convert a single sai-core `Message` to genai `ChatMessage`(s).
///
/// Most messages produce one genai message, but some may produce
/// multiple (e.g., assistant with tool-use blocks followed by tool
/// results).
fn convert_message(msg: &Message) -> Vec<GenaiMessage> {
    match msg {
        Message::User { content } => {
            vec![GenaiMessage::user(content.clone())]
        }
        Message::Assistant { content, .. } => {
            // Extract text content from content blocks
            let text: String = content
                .iter()
                .filter_map(|block| match block {
                    ContentBlock::Text { text } => Some(text.as_str()),
                    ContentBlock::ToolUse { .. } => None,
                })
                .collect::<Vec<_>>()
                .join("");

            if text.is_empty() {
                // If no text, still need an assistant message for tool calls
                vec![GenaiMessage::assistant("")]
            } else {
                vec![GenaiMessage::assistant(text)]
            }
        }
        Message::ToolResult {
            call_id, content, ..
        } => {
            let response = ToolResponse::new(call_id.clone(), content.clone());
            vec![response.into()]
        }
    }
}

/// Convert a tool definition JSON value to a genai `Tool`.
fn convert_tool_definition(def: &serde_json::Value) -> Option<GenaiTool> {
    let name = def.get("name")?.as_str()?;
    let mut tool = GenaiTool::new(name);

    if let Some(desc) = def.get("description").and_then(|v| v.as_str()) {
        tool = tool.with_description(desc);
    }

    if let Some(schema) = def.get("input_schema").or_else(|| def.get("parameters")) {
        tool = tool.with_schema(schema.clone());
    }

    Some(tool)
}

#[cfg(test)]
mod tests {
    use super::*;
    use sai_core::domain::message::StopReason;
    use sai_core::domain::tool_call::ToolResultStatus;

    #[test]
    fn user_message_converts() {
        let request = ChatRequest::new(vec![Message::user("Hello")]);
        let genai_req = to_genai_request(&request);
        assert_eq!(genai_req.messages.len(), 1);
    }

    #[test]
    fn assistant_message_converts() {
        let request = ChatRequest::new(vec![
            Message::user("Hi"),
            Message::Assistant {
                content: vec![ContentBlock::Text {
                    text: "Hello!".into(),
                }],
                stop_reason: StopReason::EndTurn,
            },
        ]);
        let genai_req = to_genai_request(&request);
        assert_eq!(genai_req.messages.len(), 2);
    }

    #[test]
    fn tool_result_converts() {
        let request = ChatRequest::new(vec![
            Message::user("Read file"),
            Message::ToolResult {
                call_id: "call_1".into(),
                status: ToolResultStatus::Success,
                content: "file contents".into(),
            },
        ]);
        let genai_req = to_genai_request(&request);
        assert_eq!(genai_req.messages.len(), 2);
    }

    #[test]
    fn system_prompt_included() {
        let request = ChatRequest::new(vec![Message::user("Hi")]).with_system_prompt("Be helpful");
        let genai_req = to_genai_request(&request);
        assert!(genai_req.system.is_some());
    }

    #[test]
    fn tool_definitions_convert() {
        let tools = vec![serde_json::json!({
            "name": "read_file",
            "description": "Read a file",
            "input_schema": {
                "type": "object",
                "properties": {
                    "path": {"type": "string"}
                }
            }
        })];
        let request = ChatRequest::new(vec![Message::user("Hi")]).with_tools(tools);
        let genai_req = to_genai_request(&request);
        assert!(genai_req.tools.is_some());
        assert_eq!(genai_req.tools.unwrap().len(), 1);
    }
}
