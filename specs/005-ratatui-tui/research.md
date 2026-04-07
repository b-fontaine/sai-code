# Research: Rich Terminal User Interface

**Feature**: 005-ratatui-tui
**Date**: 2026-04-07

## Research Questions

### R1: Async event loop design (tokio + ratatui)

**Decision**: Use a dedicated `tokio::spawn` task that owns a `crossterm::event::EventStream` (async) and two `tokio::time::interval` instances (tick and render), racing them with `tokio::select!`. The spawned task sends `Event` variants over a `tokio::sync::mpsc::UnboundedSender<Event>`. The main async task owns the `Terminal` (non-`Send`) and calls `terminal.draw()` when it receives `Event::Render`.

**Rationale**: `crossterm::EventStream` is already async; no `spawn_blocking` is needed. Decoupling render events (fired at 30 Hz) from input events prevents slow key handling from blocking redraws. `Terminal` is not `Send`, so it must remain on the main task.

**Pattern**:
```
tokio::spawn task:
  loop {
    select! {
      crossterm event → send Event::Key / Event::Resize
      tick interval   → send Event::Tick
      render interval → send Event::Render
    }
  }

main async loop:
  loop {
    event = rx.recv().await
    if Render → terminal.draw(render_all_components)
    else       → update app state
  }
```

**Alternatives considered**:
- `tokio::task::spawn_blocking` with `event::read()`: unnecessary — `EventStream` is already async.
- Dedicated OS thread for rendering: more complex without benefit since `Terminal` is single-threaded by design.
- Rendering on every token arrival: risks flicker and CPU waste; 30 Hz controlled rate is the idiomatic ratatui approach.

---

### R2: Component trait pattern

**Decision**: Each UI region (conversation panel, activity panel, input area, status bar, permission overlay) is a `Component` with three methods: `handle_events(event) → Option<Action>`, `update(action) → Option<Action>`, and `draw(frame, area)`. Components receive an `UnboundedSender<Action>` at init so they can schedule async actions (e.g., fire a background task then send `Action::TurnComplete`).

**Rationale**: This pattern matches the official ratatui component template and the architecture the constitution prescribes (action channels decoupled from rendering). The `Action` enum is the single source of truth for state transitions; the render loop only reads from computed state, never triggers side effects.

**Alternatives considered**:
- Direct mutation without action enum: simpler but conflates event handling with rendering; hard to test.
- Redux-style reducer (single `reduce(state, action) → state`): clean, but idiomatic ratatui apps use component-local state with action passing rather than a single global state tree.

---

### R3: Streaming token display

**Decision**: Store conversation content as `Vec<Line<'static>>` (owned strings). On each `AgentEvent::TextDelta(token)`, append spans to the last `Line` (splitting on `\n` to open new lines). Render with `Paragraph::new(text).scroll((scroll_offset, 0))`. Auto-scroll: after each append, set `scroll_offset = total_lines.saturating_sub(visible_height)` unless the user has manually scrolled.

**Rationale**: `Vec<Line<'static>>` is cheap to append without reallocating the full text. Ratatui's double-buffer diff means only changed cells are written to the terminal, so 30 Hz rendering with incremental content has no visible flicker. Tracking `visible_height` from the last draw call enables correct auto-scroll without over-engineering.

**Alternatives considered**:
- `String` accumulation re-converted each frame: allocation cost; also loses per-line formatting.
- `tui-scrollview` third-party crate: reduces boilerplate but adds a dependency for a well-understood pattern.
- Rendering on every token instead of at 30 Hz: produces flicker and burns CPU for fast models.

---

### R4: Permission prompt overlay

**Decision**: When `AppState::permission_prompt` is `Some(pending)`, render the main UI first, then compute a centered `Rect` (60% wide, 20% tall), render `Clear` into it to erase cells, then render the permission prompt `Block`+`Paragraph` on top. Use the `centered_rect` helper (the v0.29-compatible free function) rather than the v0.30 `Rect::centered()` method.

**Rationale**: Ratatui uses a painter's model — later renders overwrite earlier ones. The `Clear` widget erases the background so the popup looks like a floating dialog, not a transparent overlay. This is the canonical ratatui popup pattern, well-established across versions.

**Alternatives considered**:
- Full-screen dimming before the popup: requires rendering a colored background layer; complex for marginal UX gain.
- Replacing the activity panel with the permission prompt: loses visual continuity; user can't see what tool triggered the prompt.

---

### R5: Adapter-loop communication

**Decision**:

*`TuiUiAdapter` (implements `UiPort`)*: Holds a `tokio::sync::mpsc::Sender<AgentEvent>`. `emit_event()` sends to the channel. The TUI event loop receives these as `Event::Agent(AgentEvent)` in its `select!`, updating `AppState`.

*`TuiPermissionsAdapter` (implements `PermissionPort`)*: Holds an `Arc<Mutex<Option<oneshot::Sender<PermissionDecision>>>>`. `check()`:
1. Creates a `oneshot::channel()` pair.
2. Locks the shared state, writes the `PermissionRequest` + sender into `AppState::pending_permission`.
3. Awaits the `oneshot::Receiver`.
4. When the user presses y/n, the TUI event handler writes to `AppState`, sends the decision on the oneshot, and clears `pending_permission`.

**Rationale**: This unblocks the agent loop (which is `async`) while the permission prompt waits for user input. The `oneshot` pattern is idiomatic Tokio for "request-response within async code".

**Alternatives considered**:
- Polling `AppState` in a loop: wastes CPU and adds latency.
- `std::sync::Condvar`: works but mixes sync/async primitives.
- `tokio::sync::Notify`: simpler but loses the typed response value.

---

### R6: ratatui version and MSRV compatibility

**Decision**: Use **ratatui v0.29.x** rather than v0.30.x.

**Rationale**: ratatui v0.30 bumps its MSRV to Rust 1.86.0. The sai-code workspace MSRV is 1.80.0 (constitution §Technology Stack Constraints). These are incompatible. ratatui v0.29.x is compatible with MSRV 1.80.0 and provides all required features (Component trait, TestBackend, async event stream, overlay pattern).

The constitution mentions "ratatui v0.30" in its technology constraints. This is a constitutional tension: the stated version conflicts with the stated MSRV. Resolution: **pin to v0.29.x** and document the conflict. A future amendment can raise the MSRV if v0.30-specific features are needed.

**v0.29 API notes** (differences from v0.30 to avoid):
- Use `centered_rect` helper function (not `Rect::centered()` method — v0.30 only).
- `Block::title()` accepts `&str` or `Span` — same in v0.29.
- `frame.area()` replaces `frame.size()` — available from v0.28+.
- `WidgetRef` blanket impl still active in v0.29.
- `Alignment` not yet renamed to `HorizontalAlignment` (v0.29).

**Alternatives considered**:
- Raise MSRV to 1.86: constitutional amendment required; impacts all other crates.
- Use v0.30 anyway, ignoring MSRV: creates false CI stability (builds on recent toolchains but fails the stated minimum).

---

### R7: Testing strategy

**Decision**:
- **Unit tests** for component state logic (no terminal): test `AppState` mutations directly.
- **Rendering unit tests**: `ratatui::backend::TestBackend` + `terminal.backend().assert_buffer(&expected)` for verifying widget layout and content.
- **Snapshot tests**: `insta` crate for regression detection of rendered output.
- **Manual testing**: document quickstart scenarios for interactive verification (signals, resize, streaming).

**Rationale**: `TestBackend` gives deterministic, in-memory rendering without a real terminal. Snapshot tests catch accidental layout regressions. Signal handling and streaming must be tested manually; automation would require a PTY harness (`ratatui-testlib`) which adds significant dependency overhead for this feature.

**Alternatives considered**:
- `ratatui-testlib` PTY integration tests: stronger coverage but complex CI setup; deferred to a follow-up.
- Only manual testing: insufficient for layout regressions.
