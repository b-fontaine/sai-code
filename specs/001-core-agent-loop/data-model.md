# Data Model: Core Agent Loop

**Feature**: 001-core-agent-loop
**Date**: 2026-04-05

## Entities

### Message

The fundamental unit of conversation history.

```
Message (enum)
├── User
│   ├── content: String
│   └── timestamp: DateTime (optional)
├── Assistant
│   ├── content: Vec<ContentBlock>    # text and/or tool-use blocks
│   ├── stop_reason: StopReason
│   └── timestamp: DateTime (optional)
└── ToolResult
    ├── call_id: String               # references ToolCall.id
    ├── status: ToolResultStatus      # Success | Error
    ├── content: String
    └── timestamp: DateTime (optional)
```

**Validation rules**:
- `call_id` in ToolResult MUST match a ToolCall.id from a preceding
  Assistant message.
- Content MUST NOT be empty for User messages.
- Assistant messages MUST have at least one ContentBlock.

### ContentBlock

A single block within an Assistant message. The model response can
contain interleaved text and tool-use blocks.

```
ContentBlock (enum)
├── Text
│   └── text: String
└── ToolUse
    ├── id: String                    # unique call identifier
    ├── name: String                  # tool name
    └── input: JsonValue              # arguments as JSON
```

### StopReason

Why the model stopped generating.

```
StopReason (enum)
├── EndTurn          # model finished, no tool calls
├── ToolUse          # model wants tools executed
├── MaxTokens        # output truncated at token limit
└── Unknown(String)  # provider-specific reason
```

### ToolCall

A structured request extracted from a ContentBlock::ToolUse.
Used as input to the tool executor.

```
ToolCall
├── id: String           # unique call identifier
├── name: String         # tool name to look up in registry
└── input: JsonValue     # arguments as JSON
```

### ToolResult

The output from executing a tool.

```
ToolResult
├── call_id: String           # matches ToolCall.id
├── status: ToolResultStatus
└── content: String           # result text or error message

ToolResultStatus (enum)
├── Success
└── Error
```

### ConversationTurn

One complete cycle from user input to final model response.

```
ConversationTurn
├── user_message: Message::User
├── iterations: Vec<LoopIteration>
└── final_response: Option<String>    # None if turn was interrupted

LoopIteration
├── assistant_message: Message::Assistant
├── tool_calls: Vec<ToolCall>         # empty if text-only response
└── tool_results: Vec<ToolResult>     # empty if no tool calls
```

### AgentSession

The top-level stateful context for the agent.

```
AgentSession
├── id: Uuid
├── config: AgentConfig
├── messages: Vec<Message>            # full conversation history
├── current_turn: Option<ConversationTurn>
└── created_at: DateTime
```

### AgentConfig

Runtime configuration for the agent loop.

```
AgentConfig
├── system_prompt: String
├── model_name: String                # e.g., "claude-sonnet-4"
├── max_iterations_per_turn: usize    # default: 50
├── max_parallel_tool_calls: usize    # default: 10
└── max_retries_on_error: usize       # default: 3
```

### AgentEvent

Events emitted by the agent loop for the UI layer.

```
AgentEvent (enum)
├── StreamStart
├── TextDelta(String)
├── ToolCallStart { name: String, call_id: String }
├── ToolCallComplete { call_id: String, success: bool, summary: String }
├── TurnComplete
└── Error(AgentError)
```

## Relationships

```
AgentSession 1──* Message
AgentSession 1──1 AgentConfig
AgentSession 1──* ConversationTurn

Message::Assistant 1──* ContentBlock
ContentBlock::ToolUse 1──1 ToolCall (extracted)
ToolCall 1──1 ToolResult (after execution)
ToolResult ──> Message::ToolResult (appended to history)

ConversationTurn 1──* LoopIteration
LoopIteration 1──1 Message::Assistant
LoopIteration 1──* ToolCall
LoopIteration 1──* ToolResult
```

## State Transitions

### Agent Loop State Machine

```
                    ┌─────────────┐
                    │ WaitingInput │◄──────────────────────┐
                    └──────┬──────┘                        │
                           │ user submits message          │
                           ▼                               │
                    ┌─────────────┐                        │
                    │  Streaming   │                        │
                    └──────┬──────┘                        │
                           │ response complete             │
                           ▼                               │
                    ┌─────────────┐                        │
               ┌────│  Inspecting  │────┐                  │
               │    └─────────────┘    │                   │
               │ has tool calls        │ no tool calls     │
               ▼                       ▼                   │
        ┌──────────────┐       ┌────────────┐             │
        │ ExecutingTools │       │  EndOfTurn  │─────────────┘
        └──────┬───────┘       └────────────┘
               │ results collected
               ▼
        ┌──────────────┐
        │ iteration <  │──yes──► back to Streaming
        │ limit?       │
        └──────┬───────┘
               │ no
               ▼
        ┌──────────────┐
        │ LimitExceeded │──────► inform user, EndOfTurn
        └──────────────┘
```
