# Data Model: Rich Terminal User Interface

**Feature**: 005-ratatui-tui
**Date**: 2026-04-07

## Entities

### AppState

The root shared state for the TUI. Wrapped in `Arc<tokio::sync::Mutex<AppState>>` so both the TUI event loop and the port adapters can read and write it.

```
AppState
├── conversation: Vec<ConversationEntry>     # ordered list of exchanges
├── active_response: Option<ActiveResponse>  # currently-streaming AI response
├── tool_activity: Vec<ToolActivityEntry>    # tool events in the current turn
├── pending_permission: Option<PendingPermission>  # active permission prompt (if any)
├── input_buffer: String                     # user's current typed text
├── status: AgentStatus                      # idle | thinking | streaming | awaiting-permission
├── model_name: String                       # displayed in the status bar
├── working_dir: PathBuf                     # displayed in the status bar
├── scroll_offset: u16                       # conversation panel scroll position
├── auto_scroll: bool                        # whether to auto-scroll to bottom
├── should_quit: bool                        # set to true to exit the TUI loop
└── error_message: Option<String>            # transient error to display
```

**Validation rules**:
- `scroll_offset` MUST NOT exceed `total_lines.saturating_sub(visible_height)`.
- When `pending_permission` is `Some`, `status` MUST be `AgentStatus::AwaitingPermission`.
- `active_response` MUST be `None` when `status` is `AgentStatus::Idle`.

---

### ConversationEntry

One complete turn visible in the conversation panel. Either a user message or a completed AI response.

```
ConversationEntry (enum)
├── User { text: String }
└── Assistant {
    lines: Vec<Line<'static>>,   # ratatui Lines for rendering
    raw_text: String,            # plain text (for copying/scrolling)
}
```

---

### ActiveResponse

The currently-streaming AI response being built token-by-token. Promoted to `ConversationEntry::Assistant` when `AgentEvent::TurnComplete` is received.

```
ActiveResponse
├── lines: Vec<Line<'static>>    # accumulated render lines
├── raw_text: String             # accumulated plain text
└── visible_height: u16          # last-known render area height (for auto-scroll)
```

---

### ToolActivityEntry

One tool execution event displayed in the activity panel.

```
ToolActivityEntry
├── call_id: String              # unique ID matching AgentEvent call_id
├── name: String                 # tool name
├── status: ToolStatus           # Running | Success | Failure
└── summary: Option<String>      # brief description (from ToolCallComplete)
```

```
ToolStatus (enum)
├── Running
├── Success
└── Failure
```

---

### PendingPermission

A permission request waiting for the user's y/n keypress. The `response_tx` is a `tokio::sync::oneshot::Sender<PermissionDecision>` placed here by `TuiPermissionsAdapter::check()`.

```
PendingPermission
├── tool_name: String            # name of the requesting tool
├── action_description: String   # what the tool will do (from ToolCall input)
├── response_tx: oneshot::Sender<PermissionDecision>  # resolved by the TUI event handler
```

---

### AgentStatus

Current agent loop phase, drives the status bar indicator.

```
AgentStatus (enum)
├── Idle               # no active turn; waiting for user input
├── Thinking           # model called; waiting for first token
├── Streaming          # tokens arriving
└── AwaitingPermission # permission prompt active
```

---

### Event

Raw events received by the TUI event loop from the spawned input task or from the agent adapter channel.

```
Event (enum)
├── Tick                         # periodic heartbeat (4 Hz)
├── Render                       # render trigger (30 Hz)
├── Key(crossterm::event::KeyEvent)
├── Resize(u16, u16)             # new (width, height) from crossterm
├── Agent(AgentEvent)            # from TuiUiAdapter via mpsc channel
└── Error                        # event stream error
```

---

### Action

Semantic actions derived from events. Components produce `Action` values; the app loop routes them back to all components.

```
Action (enum)
├── Quit
├── Render
├── SubmitInput
├── AppendInputChar(char)
├── DeleteInputChar
├── ClearInput
├── ScrollUp(u16)
├── ScrollDown(u16)
├── ScrollToBottom
├── ToggleHelp
├── ClearConversation
├── ApprovePermission
├── DenyPermission
├── AgentEvent(AgentEvent)       # forwarded from TUI event loop
└── Error(String)
```

---

### TuiConfig

Static configuration provided at TUI startup (not mutated after init).

```
TuiConfig
├── frame_rate: f64             # render Hz (default: 30.0)
├── tick_rate: f64              # tick Hz (default: 4.0)
├── min_width: u16              # minimum terminal width (default: 80)
├── min_height: u16             # minimum terminal height (default: 24)
└── help_key: KeyCode           # key to toggle help overlay (default: '?')
```

## Relationships

```
AppState ──read-by──> Components (at render time)
AppState ──written-by──> TuiUiAdapter (via AgentEvent channel)
AppState ──written-by──> TuiPermissionsAdapter (sets pending_permission)
AppState ──written-by──> App event loop (key events → state mutations)

TuiUiAdapter ──implements──> UiPort (from sai-core)
TuiPermissionsAdapter ──implements──> PermissionPort (from sai-core)

Event ──produced-by──> spawned input task (crossterm EventStream + intervals)
Event::Agent ──produced-by──> TuiUiAdapter.emit_event()
Action ──produced-by──> Components.handle_events()
Action ──consumed-by──> Components.update()

ConversationEntry ──composed-from──> multiple AgentEvent::TextDelta arrivals
ToolActivityEntry ──created-by──> AgentEvent::ToolCallStart
ToolActivityEntry ──updated-by──> AgentEvent::ToolCallComplete
PendingPermission ──created-by──> TuiPermissionsAdapter.check()
PendingPermission ──resolved-by──> Action::ApprovePermission | Action::DenyPermission
```

## State Transitions

### AgentStatus State Machine

```
        ┌─────────────────────┐
        │       Idle          │ ← Input submitted (user message sent to agent)
        └──────────┬──────────┘
                   │ AgentEvent::StreamStart
                   ▼
        ┌─────────────────────┐
        │     Thinking        │ ← Waiting for first token
        └──────────┬──────────┘
                   │ AgentEvent::TextDelta
                   ▼
        ┌─────────────────────┐
        │     Streaming       │ ← Tokens arriving
        └──────────┬──────────┘
                   │
          ┌────────┼────────┐
          │                 │
          │ ToolCallStart   │ AgentEvent::TurnComplete
          ▼                 ▼
  ┌──────────────┐   ┌──────────────┐
  │  Streaming   │   │    Idle      │
  │  (continues) │   │  (turn done) │
  └──────────────┘   └──────────────┘
          │
  Permission needed
          │
          ▼
  ┌──────────────────────┐
  │  AwaitingPermission  │ ← User must respond
  └──────────┬───────────┘
             │ y / n keypress
             ▼
  ┌──────────────────────┐
  │     Streaming        │ ← Continues after permission resolved
  └──────────────────────┘
```

### PendingPermission Lifecycle

```
TuiPermissionsAdapter.check(req):
  1. Create oneshot::channel() → (tx, rx)
  2. Lock AppState
  3. Set AppState.pending_permission = Some(PendingPermission { ..., tx })
  4. Set AppState.status = AwaitingPermission
  5. Unlock AppState
  6. Await rx  ← blocks until key press

TUI event loop (on 'y' or 'n' keypress, when status == AwaitingPermission):
  1. Lock AppState
  2. Take AppState.pending_permission (consuming the tx)
  3. Set AppState.status = Streaming (or Idle)
  4. Unlock AppState
  5. Send PermissionDecision on tx  ← unblocks TuiPermissionsAdapter.check()
```
