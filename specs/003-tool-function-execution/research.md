# Research: Tool & Function Execution

**Feature**: 003-tool-function-execution
**Date**: 2026-04-05

## Research Questions

### R1: File content search approach

**Decision**: Use `grep-regex` + `grep-searcher` crates (ripgrep's library components) for content search.

**Rationale**: These crates power ripgrep and provide fast, memory-efficient regex searching across files. They handle binary file detection, line numbering, and context lines natively. No need to shell out to `rg` or implement from scratch.

**Alternatives considered**:
- Shelling out to `rg`/`grep`: Adds external dependency, harder to test, output parsing fragile.
- Custom implementation with `regex` + `walkdir`: More code to maintain, missing binary detection and performance optimizations.

### R2: Glob matching approach

**Decision**: Use `globset` crate for file name pattern matching, combined with `walkdir` or `ignore` for directory traversal.

**Rationale**: `globset` is mature, supports multiple patterns efficiently, and is part of the ripgrep ecosystem. The `ignore` crate adds `.gitignore`-aware traversal which is essential for skipping `target/`, `node_modules/`, etc.

**Alternatives considered**:
- `glob` crate: Simpler but lacks `.gitignore` awareness and multi-pattern optimization.
- Custom implementation: Unnecessary given mature ecosystem.

### R3: Shell command execution and safety

**Decision**: Use `tokio::process::Command` for async shell execution with configurable timeout via `tokio::time::timeout`. Shell command AST validation via `tree-sitter-bash` before execution per constitution Principle V.

**Rationale**: Tokio's process API integrates naturally with the async runtime. The timeout wrapper ensures clean termination. Tree-sitter-bash provides structural command analysis to detect dangerous patterns (pipe-to-eval, rm -rf, credential file access) without fragile regex matching.

**Alternatives considered**:
- Synchronous `std::process::Command` in `spawn_blocking`: Works but loses streaming output capability.
- No AST validation (regex-only): Constitution requires tree-sitter; regex is brittle for shell syntax.

### R4: Output truncation strategy

**Decision**: Truncate at a configurable byte limit (default: 100KB) with a trailing marker: `\n... [output truncated at {limit} bytes, {total} bytes total]`.

**Rationale**: 100KB is large enough for most useful tool output but small enough to avoid overwhelming the model's context window. Byte-level truncation is simple and predictable. The marker tells the model (and user) that data was lost.

**Alternatives considered**:
- Line-based truncation: Harder to predict memory usage, could still be very large with long lines.
- No truncation (let model handle): Risks context overflow and increased costs.

### R5: Binary file detection for file-read tool

**Decision**: Check the first 8KB of the file for null bytes. If null bytes are found, classify as binary and return an error message instead of content.

**Rationale**: This is the same heuristic used by Git and ripgrep. It is fast, requires no external dependencies, and catches the vast majority of binary files.

**Alternatives considered**:
- MIME type detection via `infer` crate: More accurate but adds a dependency for marginal benefit.
- File extension checking: Unreliable; many binary formats lack standard extensions.

### R6: Tool registry design

**Decision**: `InMemoryToolRegistry` struct holding a `Vec<Box<dyn ToolPort>>`, implementing `ToolRegistryPort`. Tools are registered at startup via builder pattern. No runtime dynamic registration in v1.

**Rationale**: Simple, fast, and sufficient for built-in tools plus any tools wired at startup. Dynamic registration adds complexity (thread safety, tool lifecycle) without immediate need.

**Alternatives considered**:
- HashMap-based registry: Slightly faster lookup by name, but the tool count is small (<20) so Vec scan is negligible.
- Dynamic registration with `RwLock<HashMap>`: Overkill for v1; can be added later if plugin system needs it.
