# Quickstart: Rich Terminal User Interface

**Feature**: 005-ratatui-tui
**Date**: 2026-04-07

## Prerequisites

- Rust toolchain >= 1.80.0 (MSRV)
- cargo-nextest: `cargo install cargo-nextest`
- Project on branch `005-ratatui-tui`
- An LLM provider API key (e.g., `ANTHROPIC_API_KEY`)
- A POSIX terminal at least 80×24 characters

## Workspace Setup

1. The new `sai-tui` crate must be added to `Cargo.toml` workspace members:
   ```toml
   members = ["crates/sai-core", "crates/sai-llm", "crates/sai-tools", "crates/sai-cli", "crates/sai-tui"]
   ```

2. Add `sai-tui` to workspace dependencies:
   ```toml
   sai-tui = { path = "crates/sai-tui" }
   ```

3. `crates/sai-tui/Cargo.toml` dependencies:
   - sai-core (workspace)
   - ratatui = { version = "0.29", features = ["crossterm"] }
   - crossterm = { version = "0.28", features = ["event-stream"] }
   - tokio (workspace, full)
   - async-trait (workspace)
   - thiserror (workspace)
   - futures = "0.3"
   - insta (dev-dependency, for snapshot tests)

4. Build the workspace:
   ```sh
   cargo build
   ```

## Running

```sh
# Interactive TUI mode (auto-detected when stdin is a terminal)
ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli

# Piped mode (falls back to plain text; TUI not activated)
echo "What is 2+2?" | ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli

# With a specific model
ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli -- --model claude-sonnet-4
```

## Running Tests

```sh
# sai-tui unit and snapshot tests
cargo nextest run -p sai-tui

# With captured output (useful for debugging TestBackend output)
cargo nextest run -p sai-tui --no-capture

# Full workspace regression check
cargo nextest run
```

## Quick Verification

### Build and lint
```sh
cargo build -p sai-tui
cargo clippy -p sai-tui -- -D warnings
cargo fmt -p sai-tui -- --check
```

### Automated tests
```sh
cargo nextest run -p sai-tui
# Expected: all unit tests pass; snapshot tests pass or new snapshots accepted
```

### Manual interactive test scenarios

**Scenario 1 — Basic launch and layout**
```sh
ANTHROPIC_API_KEY=sk-... cargo run -p sai-cli
```
Verify:
- Terminal clears and shows structured layout (conversation area, activity area, input area)
- Status bar shows model name and working directory
- Cursor is positioned in the input area

**Scenario 2 — Streaming response**
1. At the `>` prompt, type: `What is 2+2?` and press Enter
2. Verify:
   - "Thinking…" indicator appears before first token
   - Tokens stream one-by-one into the conversation area
   - After the response, input area is ready for the next message

**Scenario 3 — Tool execution with activity display**
1. Type: `List the files in src/` and press Enter
2. Verify:
   - Activity area shows the tool name with a running indicator
   - After tool completes, a success (✓) or failure (✗) indicator appears
   - The AI response in the conversation area is visually separate from tool activity

**Scenario 4 — Permission prompt**
1. Modify `TerminalPermissions` in `sai-cli` to always return `PermissionDecision::Ask` for all tools
2. Type: `Read src/main.rs` and press Enter
3. Verify:
   - A permission overlay appears with the tool name and action description
   - The rest of the layout is still visible behind the overlay
   - Pressing `y` allows and continues; pressing `n` denies

**Scenario 5 — Terminal resize**
1. While the agent is running, resize the terminal window
2. Verify:
   - Layout reflows to the new dimensions
   - No content is lost or corrupted

**Scenario 6 — Graceful exit**
1. Type `/exit` and press Enter, OR press Ctrl-C
2. Verify:
   - Terminal is restored to normal scrolling mode
   - No raw-mode artifacts remain
   - Shell prompt appears immediately after exit

**Scenario 7 — Small terminal**
1. Resize the terminal to below 80×24
2. Verify:
   - TUI displays a "terminal too small, please resize" message
   - Layout does not render in a corrupted state

## Implementation Order

Follow user story priorities from the spec:

1. **P1**: Scaffold `sai-tui` crate + `AppState` + `TuiApp` struct + terminal init/restore
2. **P1**: `TuiUiAdapter` implementing `UiPort` (channel → AppState updates)
3. **P1**: Basic layout: three-panel render (conversation, activity, input) with status bar
4. **P1**: Streaming token display in conversation panel (auto-scroll)
5. **P1**: Tool activity panel (Running/Success/Failure entries)
6. **P1**: `TuiPermissionsAdapter` implementing `PermissionPort` (oneshot + overlay)
7. **P1**: Integrate into `sai-cli` (detect interactive mode, use TUI adapters)
8. **P2**: Scrollable conversation history (scroll up/down, jump to bottom)
9. **P3**: Keyboard shortcuts + help overlay

## Key Design Notes

- `TuiApp::run()` owns the main async loop. It blocks until the user exits. `sai-cli` spawns the agent loop as a separate `tokio::spawn` task.
- `Terminal<CrosstermBackend>` is NOT `Send`. It must stay on the task that calls `tui_app.run()`.
- `TuiUiAdapter` and `TuiPermissionsAdapter` are `Clone + Send + Sync` — they can be freely passed to `AgentLoop::new()`.
- On exit (clean or panic), `TuiApp::drop()` calls `crossterm::terminal::disable_raw_mode()` and `crossterm::execute!(stdout, LeaveAlternateScreen)`.
