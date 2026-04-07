# Data Model: Interactive Conversation Loop

**Feature**: 004-conversation-loop
**Date**: 2026-04-06

## Entities

### Cli (argument struct)

Command-line arguments parsed at startup.

```
Cli
├── message: Option<String>        # initial message to process before interactive mode
├── model: String                  # LLM model identifier (default: from env or "claude-sonnet-4")
└── verbose: bool                  # enable verbose logging output
```

**Validation rules**:
- `model` MUST NOT be empty.
- If `message` is provided, it MUST NOT be empty or whitespace-only.

### ReplConfig

Configuration for the conversation loop behavior.

```
ReplConfig
├── prompt_prefix: String          # text shown before user input (default: "> ")
├── farewell_message: String       # text shown on exit (default: "Goodbye!")
└── double_ctrl_c_window_ms: u64   # time window for double Ctrl-C detection (default: 1000)
```

### InputResult

Result of reading one input from the user.

```
InputResult (enum)
├── Message(String)                # user typed a message to send to the agent
├── Exit                           # user requested exit (/exit, /quit, Ctrl-D)
└── Empty                          # user submitted empty/whitespace-only input
```

### TerminalUi

Concrete implementation of `UiPort` that renders `AgentEvent` to the terminal.

```
TerminalUi
├── stdout: Stdout                 # handle for response output
└── stderr: Stderr                 # handle for metadata output (tools, errors, prompts)
```

**Behavior**:
- `TextDelta` → write to stdout, flush immediately
- `StreamStart` → write "Thinking..." to stderr
- `ToolCallStart` → write "[tool: {name}]" to stderr
- `ToolCallComplete` → write success/failure indicator to stderr
- `TurnComplete` → write newline to stdout
- `Error` → write formatted error to stderr
- `HistorySizeWarning` → write notice to stderr

### TerminalPermissions

Concrete implementation of `PermissionPort` for interactive approval.

```
TerminalPermissions
├── is_interactive: bool           # whether stdin is a terminal
└── tty: Option<File>              # /dev/tty handle for reading in piped mode
```

**Behavior**:
- Read-only tools → `Allow`
- Non-interactive mode → `Deny` for non-read-only tools
- Interactive mode → prompt "Allow {tool_name}? (y/n): " and read response
- "y" or "yes" → `Allow`; anything else → `Deny`

## Relationships

```
Cli ──creates──> AgentConfig + ReplConfig
ReplConfig ──configures──> REPL loop
TerminalUi ──implements──> UiPort (from sai-core)
TerminalPermissions ──implements──> PermissionPort (from sai-core)
InputResult ──produced-by──> input reader
InputResult ──consumed-by──> REPL loop

main.rs wires:
  GenaiLlmAdapter (sai-llm) ──as──> LlmPort
  InMemoryToolRegistry (sai-tools) ──as──> ToolRegistryPort
  TerminalUi ──as──> UiPort
  TerminalPermissions ──as──> PermissionPort
  All ports ──injected-into──> AgentLoop (sai-core)
```

## State Transitions

### REPL Loop State Machine

```
    ┌──────────────┐
    │ Startup      │  (parse args, wire DI, display banner)
    └──────┬───────┘
           │
           │ has initial message?
           ├── yes ──> ┌──────────────┐
           │           │ ProcessTurn  │ ◄─────────────────────────┐
           │           └──────┬───────┘                           │
           │                  │ turn complete                     │
           │                  ▼                                   │
           └── no ───> ┌──────────────┐                           │
                       │ AwaitInput   │                           │
                       └──────┬───────┘                           │
                              │                                   │
                    ┌─────────┼─────────────┐                     │
                    │         │             │                      │
               Empty      Message       Exit/EOF                  │
                 │           │             │                       │
                 │           ▼             ▼                       │
                 │     ┌──────────┐  ┌──────────┐                 │
                 └───> │ (ignore) │  │ Shutdown  │                 │
                       │ loop     │  │ farewell  │                 │
                       └──────────┘  │ exit(0)   │                 │
                              │      └──────────┘                 │
                              └───────────────────────────────────┘
```

### Signal Handling State Machine

```
    ┌────────────────┐
    │ AwaitInput     │ ── Ctrl-C ──> Exit
    └────────────────┘

    ┌────────────────┐                    ┌────────────────┐
    │ ProcessTurn    │ ── 1st Ctrl-C ──> │ CancelTurn     │
    └────────────────┘                    │ return to      │
                                          │ AwaitInput     │
                                          └────────────────┘

    ┌────────────────┐                    ┌────────────────┐
    │ ProcessTurn    │ ── 2nd Ctrl-C ──> │ ForceExit      │
    │ (within 1s of  │   (quick)         │ exit(0)        │
    │  1st Ctrl-C)   │                    └────────────────┘
    └────────────────┘
```
