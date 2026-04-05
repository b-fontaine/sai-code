# Tool Contracts: Tool & Function Execution

**Feature**: 003-tool-function-execution
**Date**: 2026-04-05

These contracts define the behavioral expectations for each built-in tool.
All tools implement `ToolPort` from `sai-core`. The existing `ToolPort`,
`ToolRegistryPort`, and `PermissionPort` contracts (spec 001) apply and
are not repeated here.

## FileReadTool

**Name**: `file_read`
**Classification**: read-only, concurrency-safe

**Contract**:
- Accepts `FileReadInput` (path, optional offset, optional limit).
- Returns file contents as a string.
- Supports line-range filtering: offset is 1-based, limit is line count.
- If offset/limit are omitted, returns the entire file.
- Detects binary files (null bytes in first 8KB) and returns
  `ToolOutput::Error` with a descriptive message.
- Returns `ToolOutput::Error` if the file does not exist or is not readable.
- Output is truncated at `max_output_bytes` with a truncation marker.

## FileWriteTool

**Name**: `file_write`
**Classification**: mutating, NOT concurrency-safe

**Contract**:
- Accepts `FileWriteInput` (path, content).
- Creates the file if it does not exist (including parent directories).
- Overwrites the file if it exists.
- Returns `ToolOutput::Success` with a confirmation message including
  the file path and byte count written.
- Returns `ToolOutput::Error` if the write fails (permissions, disk full, etc.).

## FileEditTool

**Name**: `file_edit`
**Classification**: mutating, NOT concurrency-safe

**Contract**:
- Accepts `FileEditInput` (path, old_string, new_string).
- Reads the file, finds the first occurrence of `old_string`, replaces
  it with `new_string`, and writes the file back.
- Returns `ToolOutput::Error` if `old_string` is not found in the file.
  The file MUST NOT be modified in this case.
- Returns `ToolOutput::Error` if `old_string` appears more than once
  (ambiguous edit). The file MUST NOT be modified.
- Returns `ToolOutput::Success` with a confirmation message on success.

## GrepTool

**Name**: `grep`
**Classification**: read-only, concurrency-safe

**Contract**:
- Accepts `GrepInput` (pattern, optional path, optional glob filter,
  optional max_results, optional context_lines).
- Searches file contents for regex pattern matches.
- Returns matching lines with file path and line number in format:
  `filepath:line_number:matching_line`.
- Respects `.gitignore` rules during directory traversal.
- Returns an empty result (not an error) when no matches are found.
- Results are capped at `max_results` (default from config).
- Output is truncated at `max_output_bytes`.

## GlobTool

**Name**: `glob`
**Classification**: read-only, concurrency-safe

**Contract**:
- Accepts `GlobInput` (pattern, optional path).
- Returns file paths matching the glob pattern, one per line.
- Respects `.gitignore` rules during directory traversal.
- Returns an empty result (not an error) when no matches are found.
- Results are capped at `max_search_results` (default from config).

## ShellTool

**Name**: `shell`
**Classification**: mutating, NOT concurrency-safe

**Contract**:
- Accepts `ShellInput` (command, optional working_dir, optional timeout_ms).
- Before execution, validates the command via tree-sitter-bash AST
  analysis to detect dangerous patterns. If validation fails, returns
  `ToolOutput::Error` with the reason (the command is NOT executed).
- Spawns the command in a POSIX shell (`/bin/sh -c`).
- Captures stdout and stderr separately.
- Returns a structured result:
  ```
  Exit code: {code}
  --- stdout ---
  {stdout}
  --- stderr ---
  {stderr}
  ```
- If the command exceeds the timeout, the process is killed (SIGKILL)
  and the tool returns `ToolOutput::Error` with any partial output
  captured before termination.
- Output (stdout + stderr combined) is truncated at `max_output_bytes`.

## InMemoryToolRegistry

**Implements**: `ToolRegistryPort`

**Contract**:
- `get(name)`: Returns the tool with the given name, or `None`.
- `list()`: Returns all registered tools in registration order.
- `tool_definitions()`: Returns a JSON array where each element contains
  `name`, `description`, and `input_schema` for a registered tool.
- Tools are registered at construction time via a builder pattern.
  No runtime mutation after construction.

## Cross-Cutting Concerns

### Output Truncation
All tools that return potentially large output MUST check the output
size against `max_output_bytes` and truncate with a marker:
`\n... [output truncated at {limit} bytes, {total} bytes total]`

### Input Validation
All tools MUST validate their input against the declared JSON Schema
before executing. Invalid input returns `ToolOutput::Error` without
side effects.

### Permission Integration
Tools do NOT check permissions themselves. The `ToolExecutor` in
`sai-core` handles permission checks before calling `execute()`.
Tools can assume that if `execute()` is called, permission was granted.
