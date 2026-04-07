# CLI Contract: sai-code

**Feature**: 004-conversation-loop
**Date**: 2026-04-06

## Binary Interface

### Invocation

```
sai-code [OPTIONS] [MESSAGE]
```

### Arguments

| Argument | Required | Description |
|----------|----------|-------------|
| `MESSAGE` | No | Initial message to process before entering interactive mode |

### Options

| Option | Default | Description |
|--------|---------|-------------|
| `--model <MODEL>` | `$SAI_MODEL` or `claude-sonnet-4` | LLM model identifier |
| `--verbose` | false | Enable verbose logging |
| `--help` | - | Show help and exit |
| `--version` | - | Show version and exit |

### Environment Variables

| Variable | Required | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Yes (for Anthropic models) | API key for Anthropic provider |
| `OPENAI_API_KEY` | Yes (for OpenAI models) | API key for OpenAI provider |
| `SAI_MODEL` | No | Default model (overridden by `--model`) |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Normal exit (user quit, EOF, /exit, /quit) |
| 1 | Unrecoverable error (missing API key, invalid config) |

## Interactive Commands

| Command | Action |
|---------|--------|
| `/exit` | Exit the agent |
| `/quit` | Exit the agent (alias) |

## Keyboard Shortcuts

| Shortcut | Context | Action |
|----------|---------|--------|
| Ctrl-C | At input prompt | Exit the agent |
| Ctrl-C | During response | Cancel current turn, return to prompt |
| Ctrl-C (2x) | During response, within 1s | Force exit |
| Ctrl-D | At input prompt | Exit the agent (EOF) |

## Output Streams

| Stream | Content |
|--------|---------|
| stdout | Agent text responses (streamed token-by-token) |
| stderr | Startup banner, thinking indicator, tool activity, errors, permission prompts |

## Startup Banner Format

```
sai-code v{version}
Model: {model_name}
Directory: {working_directory}
```
