//! Model-to-provider routing and API key resolution.

use sai_core::error::LlmError;

/// Known provider families and their env var names.
const PROVIDERS: &[(&str, &str, Option<&str>)] = &[
    // (prefix, provider_name, env_var_name)
    ("claude", "anthropic", Some("ANTHROPIC_API_KEY")),
    ("gpt", "openai", Some("OPENAI_API_KEY")),
    ("o1", "openai", Some("OPENAI_API_KEY")),
    ("o3", "openai", Some("OPENAI_API_KEY")),
    ("gemini", "gemini", Some("GEMINI_API_KEY")),
    ("ollama::", "ollama", None),
    ("llama", "ollama", None),
    ("groq::", "groq", Some("GROQ_API_KEY")),
    ("deepseek", "deepseek", Some("DEEPSEEK_API_KEY")),
];

/// Derive the provider name from a model identifier.
pub(crate) fn provider_for_model(model: &str) -> Result<&'static str, LlmError> {
    for &(prefix, name, _) in PROVIDERS {
        if model.starts_with(prefix) {
            return Ok(name);
        }
    }
    Err(LlmError::Provider(format!(
        "unrecognized model '{}'; supported prefixes: {}",
        model,
        PROVIDERS
            .iter()
            .map(|(p, _, _)| format!("{p}*"))
            .collect::<Vec<_>>()
            .join(", ")
    )))
}

/// Check that the required API key is present for the given model.
///
/// Returns `Ok(())` if the key is set or if the provider doesn't
/// require one (e.g., Ollama). Returns an error naming the missing
/// environment variable.
pub(crate) fn check_api_key(model: &str) -> Result<(), LlmError> {
    for &(prefix, _, env_var) in PROVIDERS {
        if model.starts_with(prefix) {
            if let Some(var_name) = env_var {
                if std::env::var(var_name).is_err() {
                    return Err(LlmError::Connection(format!(
                        "{var_name} not set; required for model '{model}'"
                    )));
                }
            }
            return Ok(());
        }
    }
    // Unknown model — provider_for_model will produce a better error
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_model_detected() {
        assert_eq!(provider_for_model("claude-sonnet-4").unwrap(), "anthropic");
    }

    #[test]
    fn openai_model_detected() {
        assert_eq!(provider_for_model("gpt-4o").unwrap(), "openai");
        assert_eq!(provider_for_model("o1-preview").unwrap(), "openai");
    }

    #[test]
    fn gemini_model_detected() {
        assert_eq!(provider_for_model("gemini-2.0-flash").unwrap(), "gemini");
    }

    #[test]
    fn ollama_model_detected() {
        assert_eq!(provider_for_model("ollama::llama3").unwrap(), "ollama");
        assert_eq!(provider_for_model("llama3.2").unwrap(), "ollama");
    }

    #[test]
    fn unknown_model_returns_error() {
        let err = provider_for_model("xyz-unknown").unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("unrecognized model"));
        assert!(msg.contains("claude*"));
    }

    #[test]
    fn ollama_needs_no_key() {
        // Should not fail even without env var
        assert!(check_api_key("ollama::llama3").is_ok());
    }

    #[test]
    fn missing_anthropic_key_reports_var_name() {
        // Temporarily ensure the var is unset for this test
        let had_key = std::env::var("ANTHROPIC_API_KEY").ok();
        std::env::remove_var("ANTHROPIC_API_KEY");

        let result = check_api_key("claude-sonnet-4");
        assert!(result.is_err());
        let msg = result.unwrap_err().to_string();
        assert!(msg.contains("ANTHROPIC_API_KEY"));

        // Restore if it was set
        if let Some(key) = had_key {
            std::env::set_var("ANTHROPIC_API_KEY", key);
        }
    }
}
