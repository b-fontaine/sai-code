# Data Model: Tool & Function Execution

**Feature**: 003-tool-function-execution
**Date**: 2026-04-05

## Entities

### FileReadInput

Input for the file-read tool.

```
FileReadInput
├── path: String              # absolute or project-relative file path
├── offset: Option<usize>     # start line (1-based, inclusive)
└── limit: Option<usize>      # number of lines to return
```

**Validation rules**:
- `path` MUST NOT be empty.
- `offset`, if provided, MUST be >= 1.
- `limit`, if provided, MUST be >= 1.

### FileWriteInput

Input for the file-write tool.

```
FileWriteInput
├── path: String              # absolute or project-relative file path
└── content: String           # full file content to write
```

**Validation rules**:
- `path` MUST NOT be empty.

### FileEditInput

Input for the file-edit tool.

```
FileEditInput
├── path: String              # absolute or project-relative file path
├── old_string: String        # exact string to find and replace
└── new_string: String        # replacement string
```

**Validation rules**:
- `path` MUST NOT be empty.
- `old_string` MUST NOT be empty.
- `old_string` and `new_string` MUST be different.

### GrepInput

Input for the content-search (grep) tool.

```
GrepInput
├── pattern: String           # regex pattern to search for
├── path: Option<String>      # directory or file to search in (default: project root)
├── glob: Option<String>      # file name filter (e.g., "*.rs")
├── max_results: Option<usize># cap on number of matches returned (default: 100)
└── context_lines: Option<usize> # lines of context before/after each match
```

**Validation rules**:
- `pattern` MUST NOT be empty.
- `pattern` MUST be a valid regex (validated before execution).
- `max_results`, if provided, MUST be >= 1.

### GlobInput

Input for the file-search (glob) tool.

```
GlobInput
├── pattern: String           # glob pattern (e.g., "**/*.rs")
└── path: Option<String>      # directory to search in (default: project root)
```

**Validation rules**:
- `pattern` MUST NOT be empty.
- `pattern` MUST be a valid glob expression.

### ShellInput

Input for the shell-execution tool.

```
ShellInput
├── command: String           # shell command to execute
├── working_dir: Option<String> # working directory (default: project root)
└── timeout_ms: Option<u64>   # timeout in milliseconds (default: from config)
```

**Validation rules**:
- `command` MUST NOT be empty.
- `timeout_ms`, if provided, MUST be > 0.

### ToolConfig

Shared configuration for tool behavior.

```
ToolConfig
├── project_root: PathBuf          # root directory of the project
├── max_output_bytes: usize        # output truncation limit (default: 102400)
├── shell_timeout_ms: u64          # default shell timeout (default: 120000)
└── max_search_results: usize      # default max grep/glob results (default: 100)
```

### InMemoryToolRegistry

Container for registered tools.

```
InMemoryToolRegistry
└── tools: Vec<Box<dyn ToolPort>>
```

**Behavior**:
- Lookup by name scans the vec (O(n), acceptable for <20 tools).
- `tool_definitions()` builds JSON objects from each tool's name, description, and schema.

## Relationships

```
ToolConfig 1──* Tool (each tool receives config at construction)
InMemoryToolRegistry 1──* Tool (owns all registered tools)
InMemoryToolRegistry ──implements──> ToolRegistryPort
Each Tool ──implements──> ToolPort

ShellInput ──validated-by──> tree-sitter-bash AST analysis
All Tools ──gated-by──> PermissionPort (via ToolExecutor)
```

## State Transitions

Tools are stateless. Each `execute()` call is independent. No state machine needed for individual tools.

The shell tool has an implicit lifecycle per execution:

```
    ┌──────────┐
    │ Validate  │  (input schema + AST safety check)
    └────┬─────┘
         │ valid
         ▼
    ┌──────────┐
    │ Spawn    │  (create child process)
    └────┬─────┘
         │
    ┌────▼─────┐
    │ Running  │◄─── collecting stdout/stderr
    └────┬─────┘
         │ exits or timeout
    ┌────▼──────────┐
    │ Complete      │  (return stdout, stderr, exit code)
    └───────────────┘
         or
    ┌───────────────┐
    │ Timed Out     │  (kill process, return partial output + timeout error)
    └───────────────┘
```
