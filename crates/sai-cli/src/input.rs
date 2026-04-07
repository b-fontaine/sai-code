//! Async stdin reader with exit detection and interactive mode support.

use color_eyre::Result;
use tokio::io::{AsyncBufReadExt, BufReader};

/// Result of reading one line of input.
#[derive(Debug, PartialEq, Eq)]
pub enum InputResult {
    /// User typed a non-empty message.
    Message(String),
    /// User requested exit (/exit, /quit, or EOF).
    Exit,
    /// User submitted empty or whitespace-only input.
    Empty,
}

/// Read one line of input from stdin, printing the prompt first.
///
/// Detects `/exit`, `/quit`, and EOF (Ctrl-D) as exit signals.
/// Trims whitespace and returns `Empty` for blank lines.
pub async fn read_input(prompt: &str) -> Result<InputResult> {
    use std::io::Write as _;

    // Print the prompt to stderr (non-blocking — stderr is unbuffered).
    let mut stderr = std::io::stderr();
    write!(stderr, "{prompt}")?;
    stderr.flush()?;

    let stdin = tokio::io::stdin();
    let mut reader = BufReader::new(stdin);
    let mut line = String::new();

    let n = reader.read_line(&mut line).await?;

    // EOF: read_line returns 0 bytes
    if n == 0 {
        return Ok(InputResult::Exit);
    }

    let trimmed = line.trim();

    if trimmed == "/exit" || trimmed == "/quit" {
        return Ok(InputResult::Exit);
    }

    if trimmed.is_empty() {
        return Ok(InputResult::Empty);
    }

    Ok(InputResult::Message(trimmed.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_result_equality() {
        assert_eq!(InputResult::Exit, InputResult::Exit);
        assert_eq!(InputResult::Empty, InputResult::Empty);
        assert_eq!(
            InputResult::Message("hi".into()),
            InputResult::Message("hi".into())
        );
        assert_ne!(InputResult::Exit, InputResult::Empty);
    }
}
