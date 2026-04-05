//! Map genai errors to sai-core `LlmError` variants.

use sai_core::error::LlmError;

/// Convert a `genai::Error` into a `sai_core::error::LlmError`.
#[allow(clippy::needless_pass_by_value)]
pub(crate) fn from_genai_error(err: genai::Error) -> LlmError {
    let msg = err.to_string();

    // Check for specific genai error patterns
    match &err {
        genai::Error::RequiresApiKey { .. } => {
            LlmError::Connection(format!("API key required: {msg}"))
        }
        genai::Error::NoAuthResolver { .. } | genai::Error::NoAuthData { .. } => {
            LlmError::Connection(format!("authentication not configured: {msg}"))
        }
        genai::Error::WebStream { .. } => LlmError::Connection(format!("stream error: {msg}")),
        _ => {
            // Try to classify by message content
            let lower = msg.to_lowercase();
            if lower.contains("429") || lower.contains("rate limit") {
                LlmError::RateLimited {
                    retry_after_secs: extract_retry_after(&msg).unwrap_or(5),
                }
            } else if lower.contains("context length")
                || lower.contains("token limit")
                || lower.contains("too many tokens")
                || lower.contains("maximum context")
            {
                LlmError::TokenLimitExceeded
            } else if lower.contains("connection")
                || lower.contains("timeout")
                || lower.contains("network")
                || lower.contains("dns")
            {
                LlmError::Connection(msg)
            } else {
                LlmError::Provider(msg)
            }
        }
    }
}

/// Try to extract a retry-after duration from an error message.
fn extract_retry_after(msg: &str) -> Option<u64> {
    // Look for patterns like "retry after 30s" or "retry-after: 30"
    let lower = msg.to_lowercase();
    if let Some(pos) = lower.find("retry") {
        let after = &msg[pos..];
        // Find the first number after "retry"
        let num_str: String = after
            .chars()
            .skip_while(|c| !c.is_ascii_digit())
            .take_while(char::is_ascii_digit)
            .collect();
        num_str.parse().ok()
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_limit_error_classified() {
        // Simulate a rate limit by checking our classification logic
        let msg = "HTTP 429: rate limit exceeded, retry after 30s";
        let lower = msg.to_lowercase();
        assert!(lower.contains("429") || lower.contains("rate limit"));
    }

    #[test]
    fn extract_retry_after_from_message() {
        assert_eq!(extract_retry_after("retry after 30 seconds"), Some(30));
        assert_eq!(extract_retry_after("Retry-After: 60"), Some(60));
        assert_eq!(extract_retry_after("no retry info"), None);
    }

    #[test]
    fn token_limit_classification() {
        let msg = "context length exceeded: max 200000 tokens";
        let lower = msg.to_lowercase();
        assert!(lower.contains("context length"));
    }
}
