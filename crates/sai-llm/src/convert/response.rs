//! Convert genai stream events to sai-core stream events.

use genai::chat::ChatStreamEvent as GenaiEvent;
use sai_core::domain::message::StopReason;
use sai_core::ports::llm::ChatStreamEvent;

use super::tools::extract_tool_calls;

/// Convert a genai `ChatStreamEvent` into sai-core `ChatStreamEvent`(s).
///
/// Most events produce one output event, but `End` may produce multiple
/// (one per tool call, then the stream-end event).
pub(crate) fn convert_stream_event(event: GenaiEvent) -> Vec<ChatStreamEvent> {
    match event {
        GenaiEvent::Start => vec![ChatStreamEvent::StreamStart],

        GenaiEvent::Chunk(chunk) => {
            vec![ChatStreamEvent::TextDelta(chunk.content)]
        }

        GenaiEvent::ReasoningChunk(chunk) => {
            // Pass reasoning through as text delta for now (v1)
            vec![ChatStreamEvent::TextDelta(chunk.content)]
        }

        GenaiEvent::ThoughtSignatureChunk(_) => {
            // Ignore thought signatures in v1
            vec![]
        }

        GenaiEvent::ToolCallChunk(_) => {
            // Individual tool call chunks are accumulated by genai.
            // We emit the complete tool calls from the End event.
            vec![]
        }

        GenaiEvent::End(end) => {
            let mut events = Vec::new();

            // Emit assembled tool calls
            let tool_calls = extract_tool_calls(&end);
            let has_tools = !tool_calls.is_empty();
            for tc in tool_calls {
                events.push(ChatStreamEvent::ToolCallComplete(tc));
            }

            // Determine stop reason
            let stop_reason = if has_tools {
                StopReason::ToolUse
            } else {
                // If content was captured and there are no tool calls,
                // it's an end-of-turn. We can't distinguish MaxTokens
                // from the stream alone without provider-specific info,
                // so default to EndTurn.
                StopReason::EndTurn
            };

            events.push(ChatStreamEvent::StreamEnd { stop_reason });
            events
        }
    }
}
