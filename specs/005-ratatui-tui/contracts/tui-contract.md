# TUI Contract: sai-tui

**Feature**: 005-ratatui-tui
**Date**: 2026-04-07

## Public API Surface

The `sai-tui` crate exposes three public items to `sai-cli`:

---

### `TuiApp` — the TUI runner

Owns the terminal, the event loop, and the shared `AppState`. The application crate creates this, provides initial configuration, and calls `run()`.

```rust
pub struct TuiApp { /* opaque */ }

impl TuiApp {
    /// Create a new TUI app with the given configuration and initial status.
    pub fn new(config: TuiConfig) -> Result<Self, TuiError>;

    /// Run the TUI event loop until the user exits.
    /// Returns Ok(()) on clean exit, Err on unrecoverable error.
    pub async fn run(&mut self) -> Result<(), TuiError>;

    /// Return a `TuiUiAdapter` that routes AgentEvents into this TUI.
    /// The adapter is Send + Sync and can be passed to AgentLoop.
    pub fn ui_adapter(&self) -> TuiUiAdapter;

    /// Return a `TuiPermissionsAdapter` that shows permission prompts in this TUI.
    /// The adapter is Send + Sync and can be passed to AgentLoop.
    pub fn permissions_adapter(&self) -> TuiPermissionsAdapter;
}
```

**Lifetimes and ownership**: `TuiApp` owns the terminal handle and must be the only writer. Adapters hold only a channel sender + `Arc<Mutex<AppState>>`; they do not own the terminal.

---

### `TuiUiAdapter` — implements `UiPort`

Routes `AgentEvent` values from the agent loop into the TUI's shared state.

```rust
pub struct TuiUiAdapter { /* opaque, Clone, Send, Sync */ }

// Implements sai_core::ports::ui::UiPort:
// async fn emit_event(&self, event: AgentEvent)
```

**Contract**:
- `emit_event` MUST NOT block the agent loop for more than a few microseconds (channel send is non-blocking).
- `emit_event` MAY be called from any async task; the adapter is `Send + Sync`.
- If the TUI has been shut down (channel closed), `emit_event` silently discards the event.

**AgentEvent handling**:

| AgentEvent | TUI behavior |
|------------|--------------|
| `StreamStart` | Sets `status = Thinking`; pushes thinking indicator in conversation |
| `TextDelta(text)` | Sets `status = Streaming`; appends token to `active_response` |
| `ToolCallStart { name, call_id }` | Inserts `ToolActivityEntry { status: Running }` |
| `ToolCallComplete { call_id, success, summary }` | Updates `ToolActivityEntry.status` to Success or Failure |
| `TurnComplete` | Promotes `active_response` to `ConversationEntry::Assistant`; sets `status = Idle` |
| `Error(e)` | Sets `error_message`; clears `active_response`; sets `status = Idle` |
| `HistorySizeWarning { count }` | Appends a system notice to conversation |

---

### `TuiPermissionsAdapter` — implements `PermissionPort`

Shows an inline permission prompt in the TUI and waits for the user's y/n response.

```rust
pub struct TuiPermissionsAdapter { /* opaque, Clone, Send, Sync */ }

// Implements sai_core::ports::permissions::PermissionPort:
// async fn check(&self, request: &PermissionRequest) -> PermissionDecision
```

**Contract**:
- `check` MUST be called from within the agent's async context (it `await`s a oneshot channel).
- Read-only tools (`request.is_read_only == true`) return `PermissionDecision::Allow` immediately without showing a prompt.
- Non-interactive mode (`is_interactive == false`, set at construction time) returns `PermissionDecision::Deny("non-interactive")` immediately.
- The prompt remains visible until the user responds; `check` suspends the agent loop until then.
- Exactly one pending permission may exist at a time. If `check` is called while another prompt is active, the second call blocks until the first is resolved.

---

### `TuiConfig` — construction-time configuration

```rust
pub struct TuiConfig {
    pub frame_rate: f64,    // render Hz; default 30.0
    pub tick_rate: f64,     // tick Hz; default 4.0
    pub min_width: u16,     // minimum terminal width; default 80
    pub min_height: u16,    // minimum terminal height; default 24
    pub model_name: String, // shown in status bar
    pub working_dir: PathBuf, // shown in status bar
}
```

---

## Integration Protocol (`sai-cli` side)

```
1. Detect interactive mode: std::io::stdin().is_terminal()

2. If interactive:
   a. Create TuiConfig { model_name, working_dir, .. }
   b. let mut tui_app = TuiApp::new(config)?
   c. let ui = tui_app.ui_adapter()        // impl UiPort
   d. let perms = tui_app.permissions_adapter()  // impl PermissionPort
   e. let agent = AgentLoop::new(agent_config, &llm, &tools, &ui, &perms)
   f. tokio::spawn({ async move { repl::run_with_agent(agent).await } })
   g. tui_app.run().await                  // blocks until user exits
   h. Clean shutdown

3. If non-interactive:
   a. Use existing TerminalUi + TerminalPermissions (plain-text streaming)
```

The `sai-tui` crate MUST NOT depend on `sai-cli`. The integration is one-way: `sai-cli` depends on `sai-tui`.

---

## Error Handling

```rust
#[derive(Debug, thiserror::Error)]
pub enum TuiError {
    #[error("terminal initialization failed: {0}")]
    Init(#[from] std::io::Error),

    #[error("terminal is too small ({width}x{height}); minimum is {min_width}x{min_height}")]
    TerminalTooSmall {
        width: u16,
        height: u16,
        min_width: u16,
        min_height: u16,
    },

    #[error("render error: {0}")]
    Render(String),
}
```

**Terminal restoration guarantee**: `TuiApp` implements `Drop` to disable raw mode and leave the alternate screen, so a panic does not leave the terminal in a broken state. `color-eyre` hooks are installed by `sai-cli` before `TuiApp::new()` is called.
