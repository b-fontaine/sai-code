# Tasks: Rich Terminal User Interface

**Input**: Design documents from `/specs/005-ratatui-tui/`
**Prerequisites**: plan.md, spec.md, research.md, data-model.md, contracts/tui-contract.md, quickstart.md

**Organization**: Tasks grouped by user story to enable independent implementation and testing.

## Format: `[ID] [P?] [Story] Description`

- **[P]**: Can run in parallel (different files, no dependencies)
- **[Story]**: User story label (US1–US6)
- Exact file paths included in every description

---

## Phase 1: Setup (Shared Infrastructure)

**Purpose**: Scaffold `sai-tui` workspace crate

- [x] T001 Add `crates/sai-tui` to workspace `Cargo.toml` members and add `sai-tui = { path = "crates/sai-tui" }` to `[workspace.dependencies]`
- [x] T002 Create `crates/sai-tui/Cargo.toml` — deps: sai-core (workspace), ratatui 0.29 (features = ["crossterm"]), crossterm 0.28 (features = ["event-stream"]), futures 0.3, tokio (workspace, full), async-trait (workspace), thiserror (workspace); dev-deps: insta 1
- [x] T003 Create `crates/sai-tui/src/lib.rs` — empty module declarations (app, event, terminal, tui, components, adapters) and re-exports of `TuiApp`, `TuiConfig`, `TuiError`, `TuiUiAdapter`, `TuiPermissionsAdapter`; verify `cargo build -p sai-tui` succeeds

---

## Phase 2: Foundational (Blocking Prerequisites)

**Purpose**: Core types and infrastructure shared across all user stories

**⚠️ CRITICAL**: No user story work can begin until this phase is complete

- [x] T004 Define `AgentStatus` enum (Idle, Thinking, Streaming, AwaitingPermission) and `TuiConfig` struct (frame_rate, tick_rate, min_width, min_height, model_name, working_dir) in `crates/sai-tui/src/app.rs`
- [x] T005 [P] Define `AppState` struct with all fields from data-model.md in `crates/sai-tui/src/app.rs` — conversation, active_response, tool_activity, pending_permission, input_buffer, status, model_name, working_dir, scroll_offset, auto_scroll, should_quit, error_message; implement `AppState::new(config: &TuiConfig) -> Self`
- [x] T006 [P] Define `Event` enum (Tick, Render, Key, Resize, Agent, Error) and `Action` enum (Quit, Render, SubmitInput, AppendInputChar, DeleteInputChar, ClearInput, ScrollUp, ScrollDown, ScrollToBottom, ToggleHelp, ClearConversation, ApprovePermission, DenyPermission, AgentEvent, Error) in `crates/sai-tui/src/event.rs`
- [x] T007 [P] Define `ConversationEntry` enum (User, Assistant), `ActiveResponse` struct, `ToolActivityEntry` struct, `ToolStatus` enum, and `PendingPermission` struct (with `response_tx: oneshot::Sender<PermissionDecision>`) in `crates/sai-tui/src/app.rs`
- [x] T008 [P] Define `TuiError` enum (Init, TerminalTooSmall, Render) with `thiserror` in `crates/sai-tui/src/lib.rs`
- [x] T009 Define `Component` trait in `crates/sai-tui/src/components/mod.rs` — methods: `register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()>`, `handle_events(&mut self, event: Option<Event>) -> Result<Option<Action>>`, `update(&mut self, action: Action) -> Result<Option<Action>>`, `draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()>`

**Checkpoint**: All shared types compile — user story phases can now proceed

---

## Phase 3: User Story 1 — Structured Terminal Layout on Launch (Priority: P1) 🎯 MVP

**Goal**: Launch shows a three-panel structured layout (conversation area, activity area, input area) with a persistent status bar, responsive to terminal resize.

**Independent Test**: `cargo run -p sai-cli` with an interactive terminal → verify structured layout appears, panels are visually separated, status bar shows model and working directory, and resizing the window reflows correctly.

- [x] T010 [US1] Implement terminal init/restore helpers in `crates/sai-tui/src/terminal.rs` — `init_terminal() -> Result<Terminal<CrosstermBackend<Stderr>>, TuiError>` (enable raw mode, enter alternate screen, hide cursor) and `restore_terminal(terminal)` (reverse all); install panic hook that calls `restore_terminal` before printing error
- [x] T011 [US1] Implement `StatusBar` component in `crates/sai-tui/src/components/status_bar.rs` — stores `model_name: String`, `working_dir: PathBuf`, `status: AgentStatus`; `draw()` renders a single-line `Paragraph` showing model | directory | status; `update(Action::AgentEvent(_))` updates displayed status
- [x] T012 [US1] Implement `InputArea` component in `crates/sai-tui/src/components/input_area.rs` — stores `buffer: String`; `handle_events` maps printable key presses to `Action::AppendInputChar`, Backspace to `Action::DeleteInputChar`, Enter to `Action::SubmitInput`; `update(Action::ClearInput)` clears buffer; `draw()` renders `Paragraph::new(buffer)` with bordered block showing `/exit to quit` hint
- [x] T013 [US1] Implement `TuiApp` struct and `TuiApp::new(config: TuiConfig)` in `crates/sai-tui/src/tui.rs` — creates terminal via `init_terminal()`, validates min size → `TuiError::TerminalTooSmall` if too small, instantiates `Arc<Mutex<AppState>>`, creates `mpsc::unbounded_channel` pair, instantiates all components, registers action handler on each
- [x] T014 [US1] Implement the spawned event-loop task in `crates/sai-tui/src/tui.rs` inside `TuiApp::start_event_loop()` — `tokio::spawn` with `crossterm::event::EventStream`, tick interval (4 Hz), render interval (30 Hz); `select!` loops sending `Event::Key`, `Event::Resize`, `Event::Tick`, `Event::Render` to `UnboundedSender<Event>`
- [x] T015 [US1] Implement three-panel layout rendering in `crates/sai-tui/src/tui.rs` — `render_all(frame, &mut components)` splits `frame.area()` into: top status bar (1 line), middle area split horizontally (conversation 70% / activity 30%), bottom input area (3 lines); calls each component's `draw()`
- [x] T016 [US1] Implement `TuiApp::run()` in `crates/sai-tui/src/tui.rs` — main async loop: await `event_rx.recv()`, fan `Event` to all components via `handle_events`, drain `action_rx` via `try_recv`, fan each `Action` to all components via `update`, on `Action::Render` call `terminal.draw(render_all)`, on `Action::Quit` restore terminal and return `Ok(())`; implement `Drop` for `TuiApp` calling `restore_terminal`
- [x] T017 [US1] Handle `Event::Resize(w, h)` in `TuiApp::run()` in `crates/sai-tui/src/tui.rs` — if `w < min_width || h < min_height` write "Terminal too small" message to terminal; else re-render normally

**Checkpoint**: `cargo run -p sai-cli` (stub adapters OK) shows structured layout, no crash on resize, terminal restores on exit

---

## Phase 4: User Story 2 — Streaming AI Responses in the Conversation Area (Priority: P1)

**Goal**: Tokens stream progressively into the conversation panel; user/AI messages are visually distinct; "Thinking…" indicator shows before first token.

**Independent Test**: Submit a message — verify "Thinking…" appears, tokens stream one-by-one, turn completes with full response shown; user and AI messages have distinct labels.

- [x] T018 [US2] Implement `TuiUiAdapter` in `crates/sai-tui/src/adapters/ui.rs` — holds `event_tx: UnboundedSender<Event>`; `emit_event` sends `Event::Agent(agent_event)` via channel; derive `Clone`; implement `UiPort` with `#[async_trait]`; `TuiApp::ui_adapter()` returns a clone of the sender-holding struct
- [x] T019 [US2] Add `Event::Agent(AgentEvent)` branch to the event-loop `select!` in `crates/sai-tui/src/tui.rs` — `TuiApp` holds a second `UnboundedReceiver<Event>` for agent events, included in the select; route received agent events as `Event::Agent` into the main event channel
- [x] T020 [US2] Implement `ConversationPanel` component in `crates/sai-tui/src/components/conversation.rs` — stores `entries: Vec<ConversationEntry>`, `active: Option<ActiveResponse>`, `scroll_offset: u16`, `auto_scroll: bool`, `visible_height: u16`; `update` handles: `AgentEvent::StreamStart` → push "Thinking…" system line + set `auto_scroll = true`; `AgentEvent::TextDelta(t)` → append token to `active.lines` (split on `\n`), update `scroll_offset` if `auto_scroll`; `AgentEvent::TurnComplete` → promote `active` to `ConversationEntry::Assistant`; `AgentEvent::Error(e)` → push error line, clear `active`; `AgentEvent::HistorySizeWarning { count }` → push system notice
- [x] T021 [US2] Implement `ConversationPanel::draw()` in `crates/sai-tui/src/components/conversation.rs` — collect all rendered lines from `entries` + `active`; prepend user messages with "You: " label and AI messages with "AI: " label; render as `Paragraph::new(all_lines).scroll((scroll_offset, 0)).wrap(Wrap { trim: false })` inside a `Block::default().borders(Borders::ALL).title("Conversation")`; capture `area.height` into `visible_height` for auto-scroll math
- [x] T022 [US2] Add `SubmitInput` action handler in `crates/sai-tui/src/components/conversation.rs` — on `Action::SubmitInput`, push `ConversationEntry::User { text: input_buffer.clone() }` to `entries` and reset `auto_scroll = true` (the actual agent call is in `sai-cli`; the component only updates display)

**Checkpoint**: Submit a message via stub LLM, verify streaming tokens appear with correct labels and auto-scroll

---

## Phase 5: User Story 3 — Tool Execution Visible in a Dedicated Activity Area (Priority: P1)

**Goal**: Tool execution events appear in the activity panel in real time — tool name on start, success/failure icon on completion — visually separate from the conversation area.

**Independent Test**: Ask agent to read a file → verify tool name appears with running indicator, then a success (✓) or failure (✗) indicator after completion.

- [x] T023 [US3] Implement `ActivityPanel` component in `crates/sai-tui/src/components/activity.rs` — stores `entries: Vec<ToolActivityEntry>` (cap at 50 most recent); `update(Action::AgentEvent(ToolCallStart { name, call_id }))` → push `ToolActivityEntry { call_id, name, status: Running, summary: None }`; `update(Action::AgentEvent(ToolCallComplete { call_id, success, summary }))` → find entry by `call_id`, update `status` to Success/Failure and `summary`; clear entries on `AgentEvent::StreamStart` (new turn)
- [x] T024 [US3] Implement `ActivityPanel::draw()` in `crates/sai-tui/src/components/activity.rs` — render entries as a `List` inside `Block::default().borders(Borders::ALL).title("Tools")`; format each entry as `"⟳ {name}"` (Running), `"✓ {name}"` (Success), `"✗ {name}: {summary}"` (Failure); most recent at bottom; scroll to bottom automatically

**Checkpoint**: ActivityPanel shows correct entries with correct status icons for tool call sequences

---

## Phase 6: User Story 4 — Inline Permission Prompts Within the TUI (Priority: P1)

**Goal**: Permission prompts appear as a centered overlay in the TUI; y/Enter allows, n/Escape denies; resolved with a single keypress.

**Independent Test**: Trigger a write-tool action → verify permission overlay appears, pressing y allows the tool and overlay disappears, pressing n denies and overlay disappears.

- [x] T025 [US4] Implement `TuiPermissionsAdapter` in `crates/sai-tui/src/adapters/permissions.rs` — holds `state: Arc<Mutex<AppState>>`, `is_interactive: bool`; implement `PermissionPort` with `#[async_trait]`; `check()`: if `!is_interactive` return `Deny` immediately; if `is_read_only` return `Allow` immediately; else create `oneshot::channel()`, lock state, set `state.pending_permission = Some(PendingPermission { tool_name, action_description, response_tx })`, set `state.status = AwaitingPermission`, unlock, await `rx`, return the received decision
- [x] T026 [US4] Implement `PermissionPrompt` component in `crates/sai-tui/src/components/permission_prompt.rs` — `draw()`: if `AppState.pending_permission.is_some()`, compute centered rect (60% wide, 30% tall) using `centered_rect` helper, render `Clear`, render `Block::default().title("Permission Required").borders(ALL)` with `Paragraph` showing tool name + "Allow? (y/n)"; `handle_events(Event::Key(k))`: if prompt active — 'y'/Enter → `Action::ApprovePermission`; 'n'/Escape → `Action::DenyPermission`
- [x] T027 [US4] Add `centered_rect(percent_x, percent_y, r: Rect) -> Rect` helper function in `crates/sai-tui/src/components/permission_prompt.rs` — implements the Layout-based centered rect calculation (v0.29-compatible, no `Rect::centered()`)
- [x] T028 [US4] Handle `Action::ApprovePermission` and `Action::DenyPermission` in `TuiApp::run()` in `crates/sai-tui/src/tui.rs` — lock `AppState`, take `pending_permission` (consuming the oneshot sender), set `status = AgentStatus::Streaming` (or Idle), unlock, send `PermissionDecision::Allow` or `Deny("user denied")` on the oneshot sender
- [x] T029 [US4] Implement `TuiApp::permissions_adapter(&self) -> TuiPermissionsAdapter` in `crates/sai-tui/src/tui.rs` — returns adapter holding `Arc::clone(&self.state)` and `is_interactive` detected via `std::io::stdin().is_terminal()`

**Checkpoint**: Full permission flow works end-to-end — adapter blocks until user responds, overlay appears and disappears correctly

---

## Phase 7: User Story 1 Integration — Wire into sai-cli (Priority: P1)

**Goal**: `sai-cli` detects interactive mode and uses TUI adapters; non-interactive mode uses existing plain-text adapters.

**Independent Test**: Run `cargo run -p sai-cli` in a terminal → TUI appears. Run `echo "hi" | cargo run -p sai-cli` → plain text output with no TUI.

- [x] T030 [US1] Update `crates/sai-cli/src/main.rs` — detect `std::io::stdin().is_terminal()`; if true: create `TuiConfig { model_name, working_dir, .. }`, call `TuiApp::new(config)?`, get `ui_adapter()` and `permissions_adapter()`, spawn agent REPL task via `tokio::spawn`, call `tui_app.run().await`; if false: use existing `TerminalUi` + `TerminalPermissions` path (unchanged)
- [x] T031 [US1] Refactor `crates/sai-cli/src/repl.rs` to accept `ui: &dyn UiPort` and `permissions: &dyn PermissionPort` as parameters instead of constructing them internally — extract `run_with_ports(cli: Cli, ui: &dyn UiPort, permissions: &dyn PermissionPort) -> Result<()>` so both TUI and plain-text paths share the same agent loop wiring

**Checkpoint**: Binary auto-selects TUI vs plain-text; `sai-cli` integration tests still pass

---

## Phase 8: User Story 5 — Scrollable Conversation History (Priority: P2)

**Goal**: User can scroll up through conversation history; scroll position persists during ongoing responses; jump-to-bottom key returns to latest.

**Independent Test**: After 20+ turns, press scroll-up key → older messages visible. Press scroll-down / End → latest message visible. Scroll position preserved while AI responds.

- [x] T032 [US5] Add scroll key bindings to `crates/sai-tui/src/components/input_area.rs` (or a global key handler in `tui.rs`) — `PageUp` / `k` → `Action::ScrollUp(5)`; `PageDown` / `j` → `Action::ScrollDown(5)`; `End` / `G` → `Action::ScrollToBottom`; these must NOT consume the key when the input area has focus (only scroll when modifier key or outside input context)
- [x] T033 [US5] Implement scroll action handlers in `crates/sai-tui/src/components/conversation.rs` — `Action::ScrollUp(n)`: set `auto_scroll = false`, decrease `scroll_offset` by n (floor 0); `Action::ScrollDown(n)`: increase `scroll_offset` by n (ceil at max); `Action::ScrollToBottom`: set `scroll_offset` to max, set `auto_scroll = true`

**Checkpoint**: Scroll up shows older messages; auto-scroll re-engages on jump-to-bottom; new responses don't force-jump scroll while user is viewing history

---

## Phase 9: User Story 6 — Keyboard Shortcuts and Help Overlay (Priority: P3)

**Goal**: `?` shows a help overlay listing all keybindings; Ctrl-Q/Ctrl-C exits; a clear-conversation key is available.

**Independent Test**: Press `?` → help overlay appears with key list. Press Escape → overlay closes. Press Ctrl-Q → agent exits cleanly with code 0.

- [x] T034 [US6] Add `show_help: bool` field to `AppState` in `crates/sai-tui/src/app.rs`; add `Action::ToggleHelp` handler to `TuiApp::run()` in `crates/sai-tui/src/tui.rs` — toggle `state.show_help`
- [x] T035 [US6] Implement help overlay rendering in `crates/sai-tui/src/tui.rs` (or a dedicated `HelpOverlay` component in `crates/sai-tui/src/components/mod.rs`) — if `state.show_help`, compute centered rect (70% × 60%), render `Clear`, render bordered block titled "Keyboard Shortcuts" listing all bindings
- [x] T036 [US6] Add global key handler in `TuiApp::run()` in `crates/sai-tui/src/tui.rs` before component dispatch — `?` → `Action::ToggleHelp`; Ctrl-Q → `Action::Quit`; Ctrl-C → `Action::Quit`; `Action::ClearConversation` → lock state, clear `state.conversation` and `state.active_response`, unlock

**Checkpoint**: Help overlay appears and dismisses; all documented exit methods work; conversation clear works

---

## Phase 10: Polish & Cross-Cutting Concerns

**Purpose**: Snapshot tests, cleanup, and final verification

- [x] T037 Write snapshot tests for `StatusBar::draw()` in `crates/sai-tui/src/components/status_bar.rs` using `ratatui::backend::TestBackend` and `insta::assert_snapshot!` — test idle state, thinking state, streaming state
- [x] T038 [P] Write snapshot tests for `ConversationPanel::draw()` in `crates/sai-tui/src/components/conversation.rs` — test empty state, one user message, one user + one AI message, "Thinking…" indicator active, long message with wrap
- [x] T039 [P] Write snapshot tests for `ActivityPanel::draw()` in `crates/sai-tui/src/components/activity.rs` — test empty, one running entry, mixed success/failure entries
- [x] T040 [P] Write snapshot tests for `PermissionPrompt::draw()` in `crates/sai-tui/src/components/permission_prompt.rs` — test no prompt active (no overlay), prompt active (overlay rendered correctly)
- [x] T041 [P] Write unit tests for `AppState` state transitions in `crates/sai-tui/src/app.rs` — test `AgentStatus` transitions, `pending_permission` lifecycle (set/clear), `scroll_offset` bounds
- [x] T042 [P] Write unit tests for `TuiPermissionsAdapter::check()` in `crates/sai-tui/src/adapters/permissions.rs` — test read-only → Allow immediately, non-interactive → Deny immediately, interactive prompt → simulate oneshot resolution
- [x] T043 Run `cargo clippy -p sai-tui -- -D warnings` and fix all warnings in `crates/sai-tui/src/`
- [x] T044 [P] Run `cargo fmt -p sai-tui -- --check` and fix formatting in `crates/sai-tui/src/`
- [x] T045 Run `cargo nextest run` (full workspace) — verify all existing `sai-cli` integration tests still pass and all new `sai-tui` tests pass; run `cargo insta review` to accept new snapshots

---

## Dependencies & Execution Order

### Phase Dependencies

- **Setup (Phase 1)**: No dependencies — start immediately
- **Foundational (Phase 2)**: Depends on Phase 1 — **blocks all user story phases**
- **US1 Layout (Phase 3)**: Depends on Phase 2 — no dependency on other stories
- **US2 Streaming (Phase 4)**: Depends on Phase 3 (layout must exist for conversation panel to render into)
- **US3 Activity (Phase 5)**: Depends on Phase 3 — parallelizable with US2 (different component file)
- **US4 Permissions (Phase 6)**: Depends on Phase 3 — parallelizable with US2/US3
- **US1 Integration (Phase 7)**: Depends on Phases 4, 5, and 6 — all P1 features must be complete before wiring into sai-cli
- **US5 Scrolling (Phase 8)**: Depends on Phase 4 (ConversationPanel must exist)
- **US6 Shortcuts (Phase 9)**: Depends on Phase 3 (TuiApp run loop must exist)
- **Polish (Phase 10)**: Depends on all desired phases complete

### User Story Dependencies

- **US1 Layout + Integration**: Foundational; all other stories build on it
- **US2, US3, US4**: All P1; US3 and US4 can be developed in parallel with US2 (different files)
- **US5, US6**: P2/P3; depend on P1 stories being complete

---

## Parallel Example: P1 User Stories (US2, US3, US4)

Once Phase 3 (US1 layout) is complete, these can proceed in parallel:

```
Developer A: US2 Streaming
  T018 TuiUiAdapter
  T019 Agent event routing
  T020 ConversationPanel update logic
  T021 ConversationPanel draw
  T022 SubmitInput handler

Developer B: US3 Tool Activity  
  T023 ActivityPanel update logic
  T024 ActivityPanel draw

Developer C: US4 Permissions
  T025 TuiPermissionsAdapter
  T026 PermissionPrompt component
  T027 centered_rect helper
  T028 Approve/Deny action handling
  T029 permissions_adapter() accessor
```

---

## Implementation Strategy

### MVP First (US1 Layout + US2 Streaming only)

1. Complete Phase 1: Setup
2. Complete Phase 2: Foundational types — **critical, blocks all stories**
3. Complete Phase 3: US1 — TUI layout, terminal init/restore, basic event loop
4. Complete Phase 4: US2 — streaming responses in conversation panel
5. **STOP and VALIDATE**: Manual test — terminal launches, tokens stream, exit restores terminal
6. Wire into sai-cli (Phase 7, T030–T031) with US2 adapters only

### Full P1 Delivery

1. Phases 1–3 (foundation + layout)
2. Phases 4, 5, 6 in parallel (streaming, activity, permissions)
3. Phase 7 (sai-cli integration) after all P1 stories pass
4. Ship and validate against all quickstart.md scenarios

### Incremental Delivery

1. Phases 1–3 → structured terminal layout (visual upgrade, no interaction yet)
2. Phase 4 → streaming responses (core interaction)
3. Phase 5 → tool activity display
4. Phase 6 → permission prompts
5. Phase 7 → full sai-cli integration
6. Phase 8 → scroll history (P2)
7. Phase 9 → keyboard shortcuts (P3)
8. Phase 10 → snapshot tests + cleanup

---

## Notes

- [P] tasks = different files or independent concerns, no sequential dependencies
- `Terminal<CrosstermBackend>` is NOT `Send` — keep on the main task; adapters hold only `Arc<Mutex<AppState>>` + channel senders
- `TuiPermissionsAdapter::check()` must await a `oneshot::Receiver` — this is safe in async code but blocks the agent turn until user responds
- Use `ratatui::backend::TestBackend` for all rendering tests — no real terminal required
- Pin ratatui to `"0.29"` (not `"0.30"`) to maintain MSRV 1.80.0 compatibility
- `TuiApp::Drop` must restore the terminal even on panic — install hook in `TuiApp::new()`
- Existing `sai-cli` plain-text adapters (`TerminalUi`, `TerminalPermissions`) are unchanged and remain active for non-interactive mode
