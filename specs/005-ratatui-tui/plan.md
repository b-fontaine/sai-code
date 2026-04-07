# Implementation Plan: Rich Terminal User Interface

**Branch**: `005-ratatui-tui` | **Date**: 2026-04-07 | **Spec**: [spec.md](spec.md)
**Input**: Feature specification from `/specs/005-ratatui-tui/spec.md`

## Summary

Add the `sai-tui` library crate that implements `UiPort` and `PermissionPort` from `sai-core` using a full ratatui terminal UI. The TUI presents a three-panel layout (conversation history, tool activity, input area) with a persistent status bar. Streaming LLM tokens render progressively at 30 FPS via a controlled render loop decoupled from event handling. Tool execution events populate a dedicated activity panel in real time. Permission prompts appear as overlays resolved with a single keypress. `sai-cli` selects the TUI adapters when stdin is interactive and falls back to the existing plain-text adapters otherwise.

## Technical Context

**Language/Version**: Rust 2021 edition, MSRV 1.80.0
**Primary Dependencies**: ratatui 0.29.x (pinned — v0.30 requires MSRV 1.86, incompatible with our MSRV), crossterm 0.28 (with `event-stream` feature), futures 0.3, sai-core (workspace), tokio (workspace, full)
**Storage**: N/A (no persistence; all state is in-memory for the duration of the session)
**Testing**: cargo-nextest, ratatui `TestBackend` (unit tests), `insta` (snapshot regression tests)
**Target Platform**: macOS, Linux (POSIX terminals); Windows deferred
**Project Type**: Library crate (`sai-tui`) consumed by the application crate (`sai-cli`)
**Performance Goals**: ≤33ms per render frame (30 FPS); ≤500ms from `AgentEvent::TextDelta` receipt to pixel on screen (measuring only TUI overhead, not model latency)
**Constraints**: Minimum terminal size 80×24; full terminal restoration on clean exit and on panic; `Terminal<CrosstermBackend>` is not `Send` — must stay on the main async task
**Scale/Scope**: One new workspace crate (`sai-tui`), ~10 source modules, 5 UI components, 2 port adapter structs, minor changes to `sai-cli`

## Constitution Check

*GATE: Must pass before Phase 0 research. Re-check after Phase 1 design.*

| Principle | Status | Notes |
|-----------|--------|-------|
| I. Hexagonal Architecture | PASS | `sai-tui` is a new adapter crate. It implements `UiPort` and `PermissionPort` from `sai-core`. `sai-core` is unchanged. `sai-cli` (the application crate) selects which adapters to wire at startup. |
| II. Multi-Provider LLM Abstraction | PASS | No LLM provider code in this feature. |
| III. Test-First Development | PASS | Component state logic tested with unit tests; widget rendering tested with `TestBackend`; snapshot tests with `insta`. |
| IV. Type-Safe Domain Modeling | PASS | `AppState`, `Action`, `Event` are typed enums. `TuiError` uses `thiserror`. No `unwrap()` in library code. |
| V. Security by Default | PASS | `TuiPermissionsAdapter` is fail-closed: `is_read_only == false` in non-interactive mode → `Deny`. |

No violations. No complexity tracking needed.

**Post-design re-check**: All principles still pass after Phase 1 design. The ratatui MSRV tension (see research R6) is documented; v0.29.x is the correct pin. No constitutional amendments required.

## Project Structure

### Documentation (this feature)

```text
specs/005-ratatui-tui/
├── plan.md              # This file
├── research.md          # Phase 0 output
├── data-model.md        # Phase 1 output
├── quickstart.md        # Phase 1 output
├── contracts/
│   └── tui-contract.md  # Phase 1 output
└── tasks.md             # Phase 2 output (via /speckit.tasks)
```

### Source Code (repository root)

```text
crates/sai-tui/
├── Cargo.toml
└── src/
    ├── lib.rs                    # Public API: TuiApp, TuiConfig, TuiError
    ├── app.rs                    # AppState, AgentStatus, TuiConfig (runtime)
    ├── event.rs                  # Event enum, Action enum
    ├── terminal.rs               # Terminal init/restore (raw mode, alternate screen)
    ├── tui.rs                    # TUI runner: event loop + render dispatch
    ├── components/
    │   ├── mod.rs                # Component trait definition
    │   ├── conversation.rs       # Conversation panel (streaming text, scroll)
    │   ├── activity.rs           # Tool activity panel (tool entries + status)
    │   ├── input_area.rs         # Fixed input text field at the bottom
    │   ├── status_bar.rs         # Persistent status line (model, dir, state)
    │   └── permission_prompt.rs  # Centered overlay for permission y/n
    └── adapters/
        ├── mod.rs
        ├── ui.rs                 # TuiUiAdapter: impl UiPort
        └── permissions.rs        # TuiPermissionsAdapter: impl PermissionPort

crates/sai-cli/src/
├── main.rs    # Updated: detect interactive mode → choose TUI vs plain adapters
└── repl.rs    # Updated: accept pre-constructed UiPort + PermissionPort (already DI'd)
```

**Structure Decision**: New `sai-tui` library crate following the constitution's hexagonal architecture. All TUI-specific code (ratatui, crossterm, event loops) lives in `sai-tui` — it never leaks into `sai-core` or `sai-cli`. `sai-cli` changes are minimal: mode detection + adapter selection at startup.

## Phase 2: Implementation Steps

### Step 1: Scaffold `sai-tui` crate

- Add `crates/sai-tui` to workspace `Cargo.toml` members
- Create `Cargo.toml` with deps: sai-core, ratatui 0.29, crossterm 0.28 (event-stream), futures, tokio, async-trait, thiserror
- Create `lib.rs` exporting `TuiApp`, `TuiConfig`, `TuiError`, `TuiUiAdapter`, `TuiPermissionsAdapter`
- Create `TuiError` in `lib.rs` with `Init`, `TerminalTooSmall`, `Render` variants

### Step 2: AppState and event/action types (P1 - foundational)

- Define `AppState` struct in `app.rs` (all fields from data model)
- Define `AgentStatus` enum in `app.rs`
- Define `Event` enum in `event.rs` (Tick, Render, Key, Resize, Agent, Error)
- Define `Action` enum in `event.rs` (full set from data model)
- Define `TuiConfig` in `app.rs`

### Step 3: Terminal initialization and restoration (P1 - US1)

- Implement `terminal.rs`: `init_terminal() → Result<Terminal<CrosstermBackend<Stderr>>, TuiError>`
  - enable raw mode, enter alternate screen, hide cursor, enable mouse capture (optional)
- Implement `restore_terminal(terminal)`: reverse all of the above
- Add `Drop` impl on `TuiApp` calling `restore_terminal`
- Install panic hook via `color-eyre` in `TuiApp::new()` that calls `restore_terminal` before printing the error

### Step 4: Component trait (P1 - foundational)

- Define `Component` trait in `components/mod.rs`:
  - `fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()>`
  - `fn handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>>`
  - `fn update(&mut self, action: Action) -> Result<Option<Action>>`
  - `fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()>`

### Step 5: TUI runner (P1 - US1)

- Implement `tui.rs`: `TuiApp` struct holding terminal + shared state + spawned event-loop task
- Spawn the input task: `tokio::spawn` with `EventStream` + tick interval (4 Hz) + render interval (30 Hz), sending to `UnboundedSender<Event>`
- Implement the main async loop in `TuiApp::run()`:
  1. Wait for next `Event` from receiver
  2. Fan `Event` to all components via `handle_events`
  3. Drain `action_rx`, fan actions to all components via `update`
  4. On `Action::Render` → `terminal.draw(render_all_components)`
  5. On `Action::Quit` → break, restore terminal, return `Ok(())`
- Implement terminal size check: if `< min_width × min_height` on `Event::Resize`, show warning

### Step 6: TuiUiAdapter (P1 - US2, US3)

- Implement `adapters/ui.rs`: `TuiUiAdapter` holding `Sender<Event>` (wraps `AgentEvent` as `Event::Agent`)
- `emit_event(&self, event: AgentEvent)`: sends `Event::Agent(event)` to the TUI event loop
- In the main event loop, `Event::Agent(ev)` is dispatched to the conversation component and activity component via the action channel

### Step 7: Status bar component (P1 - US1)

- Implement `components/status_bar.rs`: renders a single line at the top or bottom
- Displays: model name | working directory | current `AgentStatus`
- `update(Action)`: watches for `AgentEvent` actions that change status

### Step 8: Conversation panel component (P1 - US2)

- Implement `components/conversation.rs`
- Internal state: `Vec<ConversationEntry>`, `Option<ActiveResponse>`, `scroll_offset: u16`, `auto_scroll: bool`, `visible_height: u16`
- `update(Action::AgentEvent(StreamStart))` → push thinking indicator, set status
- `update(Action::AgentEvent(TextDelta(t)))` → append to `active_response.lines`, auto-scroll
- `update(Action::AgentEvent(TurnComplete))` → promote `active_response` to `ConversationEntry::Assistant`
- `update(Action::ScrollUp(n))`, `ScrollDown(n)`, `ScrollToBottom` → update `scroll_offset`, toggle `auto_scroll`
- `draw()`: render `Paragraph::new(all_lines).scroll((scroll_offset, 0))`
- Test with `TestBackend`: verify message count, scroll behavior, token append

### Step 9: Tool activity panel component (P1 - US3)

- Implement `components/activity.rs`
- Internal state: `Vec<ToolActivityEntry>` (capped at last N entries to prevent overflow)
- `update(Action::AgentEvent(ToolCallStart { name, call_id }))` → push `ToolActivityEntry { status: Running }`
- `update(Action::AgentEvent(ToolCallComplete { call_id, success, summary }))` → find by `call_id`, update status
- `draw()`: render a `List` or `Paragraph` of entries with status icons
- Test with `TestBackend`: verify entry count, status updates

### Step 10: Input area component (P1 - US1)

- Implement `components/input_area.rs`
- Internal state: `String` (the current input buffer)
- `handle_events(Event::Key(k))`: append printable chars, backspace, enter → `Action::SubmitInput`
- `update(Action::ClearInput)` → clear buffer
- `draw()`: render `Paragraph::new(buffer)` with a visible cursor or border; display "/exit, /quit to exit"

### Step 11: TuiPermissionsAdapter (P1 - US4)

- Implement `adapters/permissions.rs`: `TuiPermissionsAdapter` holding `Arc<Mutex<AppState>>` + `is_interactive: bool`
- `check(&self, req)`:
  - If `!is_interactive` → return `Deny` immediately
  - If `req.is_read_only` → return `Allow` immediately
  - Create `oneshot::channel()`; lock `AppState`; set `pending_permission = Some(PendingPermission { ..., tx })`; unlock; await `rx`
- Test: mock `AppState`, simulate keypress resolving the oneshot, verify `Allow`/`Deny` returned

### Step 12: Permission prompt component (P1 - US4)

- Implement `components/permission_prompt.rs`
- `draw()`: if `AppState::pending_permission.is_some()`, compute centered rect, render `Clear`, render prompt block
- `handle_events(Event::Key(k))`: if prompt active — 'y'/Enter → `Action::ApprovePermission`; 'n'/Escape → `Action::DenyPermission`
- In main event loop: `Action::ApprovePermission/DenyPermission` → take `pending_permission.response_tx`, send decision, clear `pending_permission`

### Step 13: Wire into sai-cli (P1 - integration)

- In `sai-cli/src/main.rs`: `if stdin.is_terminal()` → create `TuiApp`, get `ui_adapter()` + `permissions_adapter()`, spawn agent task, call `tui_app.run().await`
- Else → use existing `TerminalUi` + `TerminalPermissions`
- The `repl::run()` function already accepts `impl UiPort + impl PermissionPort` via trait objects — no signature changes required

### Step 14: Scrollable history (P2 - US5)

- Add `ScrollUp`, `ScrollDown`, `ScrollToBottom` key bindings to `input_area.rs` (or a global key handler)
- Conversation component already handles scroll actions (from Step 8); wire the key → action mapping
- Test: verify scroll offset changes, auto-scroll disabled/re-enabled

### Step 15: Help overlay and keyboard shortcuts (P3 - US6)

- Add a `HelpOverlay` component or render directly in `tui.rs` when `AppState::show_help == true`
- Key `?` → `Action::ToggleHelp`; Escape → close help
- `draw()`: centered overlay listing all keybindings
- `Action::ClearConversation` → clear `conversation` entries (not `active_response`); bind to a key

### Step 16: Integration tests and snapshot tests

- Add `insta` dev-dependency to `sai-tui`
- Write snapshot tests for each component (status bar, conversation, activity, input area) using `TestBackend`
- Write state machine unit tests for `AppState` transitions (status, permission lifecycle)
- Verify clean exit and terminal restoration via `sai-cli` integration tests (existing tests should still pass)
