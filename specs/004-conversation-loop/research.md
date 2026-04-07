# Research: Interactive Conversation Loop

**Feature**: 004-conversation-loop
**Date**: 2026-04-06

## Research Questions

### R1: Async stdin reading approach

**Decision**: Use `tokio::io::BufReader` wrapping `tokio::io::stdin()` with `AsyncBufReadExt::read_line()` for async line reading. Detect interactive mode via `std::io::IsTerminal` trait on `std::io::stdin()`.

**Rationale**: Tokio's async stdin integrates with the existing async runtime without spawning a blocking thread manually. The `IsTerminal` check (stable since Rust 1.70) enables different behavior for piped vs interactive input. For v1, simple line-based reading is sufficient — no need for raw mode or complex line editing.

**Alternatives considered**:
- `rustyline` crate: Full readline support (history, keybindings, completion). Excellent UX but adds complexity and a synchronous API that requires `spawn_blocking`. Better suited for a follow-up enhancement.
- `crossterm` raw mode input: Gives character-level control for real-time editing. Overkill for v1 line-based input; would be needed if we add in-line editing later.
- `std::io::stdin().read_line()` in `spawn_blocking`: Works but less idiomatic and harder to cancel.

### R2: Signal handling strategy (Ctrl-C)

**Decision**: Use `tokio::signal::ctrl_c()` combined with `tokio::select!` in the REPL loop. During input: Ctrl-C exits. During an active turn: use a `CancellationToken` (from `tokio_util`) passed into the turn execution; first Ctrl-C triggers cancellation, second exits.

**Rationale**: `tokio::signal::ctrl_c()` is the idiomatic async signal handler in Tokio. `CancellationToken` provides cooperative cancellation — when triggered, the streaming future can be dropped cleanly. The double-Ctrl-C pattern is familiar from tools like Claude Code and prevents accidental process termination during long operations.

**Alternatives considered**:
- `ctrlc` crate: Simpler API but not async-aware; requires crossbeam channel or atomic flag for communication. Less composable with `tokio::select!`.
- Raw signal handlers via `libc`: Maximum control but complex, unsafe, and unnecessary for this use case.
- Single Ctrl-C always exits: Simpler but poor UX when the user just wants to cancel a long-running turn.

### R3: Application error handling

**Decision**: Use `color-eyre` for the application entry point (`main()`) and `thiserror` for typed errors within the domain (already in place). The `main()` function returns `Result<(), color_eyre::Report>` for rich error context and backtraces.

**Rationale**: `color-eyre` is the canonical choice for user-facing Rust CLI error reporting. It provides colored output, backtrace support, and `.wrap_err()` for adding context. It complements the existing `thiserror`-based domain errors without replacing them.

**Alternatives considered**:
- `anyhow`: Similar capability but less polished output for CLI users. `color-eyre` is preferred per constitution (Principle IV mentions it explicitly).
- Raw `eprintln!` for all errors: Loses structured error context and backtraces. Harder to debug in production.

### R4: CLI argument parsing

**Decision**: Use `clap` v4 with derive macros for argument parsing. Define a `Cli` struct with an optional positional `message` argument and a `--model` flag.

**Rationale**: `clap` is specified in the constitution's Technology Stack Constraints. Derive macros provide type-safe argument definitions with minimal boilerplate.

**Alternatives considered**:
- Manual `std::env::args()` parsing: Fragile, no --help generation.
- `argh`: Lighter weight but not constitution-specified.

### R5: Terminal output approach for streaming

**Decision**: Write `TextDelta` tokens directly to stdout via `std::io::Write` + explicit `flush()` after each token. Use stderr for non-response output (tool activity, errors, prompts).

**Rationale**: Direct stdout writes with flushing provide the lowest-latency streaming display. Separating response text (stdout) from metadata (stderr) follows Unix conventions and allows piping the agent's responses. No terminal UI framework needed for v1.

**Alternatives considered**:
- `ratatui` TUI framework: Constitution mentions it for TUI rendering, but it's a separate feature. For v1, simple stdout/stderr is sufficient and much simpler.
- Buffered writes with periodic flush: Adds latency for minimal benefit. Each token should be visible immediately.

### R6: Permission prompting approach

**Decision**: Implement `PermissionPort` as `TerminalPermissions` that writes prompts to stderr and reads y/n responses from `/dev/tty` (or stdin if interactive). For non-interactive mode, default to `Deny` for non-read-only tools.

**Rationale**: Reading from `/dev/tty` ensures permission prompts work even when stdin is piped (the user can still respond interactively if they have a terminal). Fail-closed default for non-interactive aligns with constitution Principle V.

**Alternatives considered**:
- Always read from stdin: Breaks when stdin is piped with a script.
- Auto-allow all tools: Violates constitution's fail-closed requirement.
- Flag-based allow modes (`--allow-write`, `--allow-shell`): Good future enhancement but out of scope for v1.

### R7: Cancellation of in-flight LLM requests

**Decision**: Use `tokio_util::sync::CancellationToken` to signal cancellation. The REPL loop creates a token per turn and passes it to the execution context. On Ctrl-C, the token is cancelled, which causes the `select!` branch to win, dropping the stream future. The partial response (if any) is preserved in session history.

**Rationale**: `CancellationToken` is the standard cooperative cancellation primitive in the Tokio ecosystem. Dropping the stream future closes the connection cleanly. Preserving partial history ensures the session state remains consistent.

**Alternatives considered**:
- Aborting the task via `JoinHandle::abort()`: Harder to ensure clean state preservation. Can leave partial messages in an inconsistent state.
- Ignoring cancellation (let the turn finish): Poor UX for long-running turns.
