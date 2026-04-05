//! In-memory tool registry implementing `ToolRegistryPort`.

use sai_core::ports::tool::{ToolPort, ToolRegistryPort};

/// A simple in-memory registry that owns a list of tools.
///
/// Tools are registered at construction time via the builder pattern.
/// The registry is immutable after construction.
pub struct InMemoryToolRegistry {
    tools: Vec<Box<dyn ToolPort>>,
}

impl InMemoryToolRegistry {
    /// Create an empty registry.
    pub fn new() -> Self {
        Self { tools: Vec::new() }
    }

    /// Register a tool and return self for chaining.
    #[must_use]
    pub fn with_tool(mut self, tool: impl ToolPort + 'static) -> Self {
        self.tools.push(Box::new(tool));
        self
    }

    /// Create a registry pre-loaded with all built-in tools.
    #[must_use]
    pub fn with_defaults(config: crate::config::ToolConfig) -> Self {
        Self::new()
            .with_tool(crate::FileReadTool::new(config.clone()))
            .with_tool(crate::FileWriteTool::new(config.clone()))
            .with_tool(crate::FileEditTool::new(config.clone()))
            .with_tool(crate::GrepTool::new(config.clone()))
            .with_tool(crate::GlobTool::new(config.clone()))
            .with_tool(crate::ShellTool::new(config))
    }
}

impl Default for InMemoryToolRegistry {
    fn default() -> Self {
        Self::new()
    }
}

impl ToolRegistryPort for InMemoryToolRegistry {
    fn get(&self, name: &str) -> Option<&dyn ToolPort> {
        self.tools
            .iter()
            .find(|t| t.name() == name)
            .map(AsRef::as_ref)
    }

    fn list(&self) -> Vec<&dyn ToolPort> {
        self.tools.iter().map(AsRef::as_ref).collect()
    }

    fn tool_definitions(&self) -> Vec<serde_json::Value> {
        self.tools
            .iter()
            .map(|t| {
                serde_json::json!({
                    "name": t.name(),
                    "description": t.description(),
                    "input_schema": t.input_schema(),
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use async_trait::async_trait;
    use sai_core::error::ToolError;
    use sai_core::ports::tool::ToolOutput;

    struct DummyTool {
        tool_name: &'static str,
    }

    #[async_trait]
    impl ToolPort for DummyTool {
        fn name(&self) -> &str {
            self.tool_name
        }
        fn description(&self) -> &str {
            "a dummy tool"
        }
        fn input_schema(&self) -> serde_json::Value {
            serde_json::json!({"type": "object"})
        }
        async fn execute(&self, _input: serde_json::Value) -> Result<ToolOutput, ToolError> {
            Ok(ToolOutput::Success("ok".into()))
        }
    }

    #[test]
    fn get_returns_registered_tool() {
        let reg = InMemoryToolRegistry::new().with_tool(DummyTool { tool_name: "alpha" });
        assert!(reg.get("alpha").is_some());
        assert_eq!(reg.get("alpha").unwrap().name(), "alpha");
    }

    #[test]
    fn get_returns_none_for_unknown() {
        let reg = InMemoryToolRegistry::new().with_tool(DummyTool { tool_name: "alpha" });
        assert!(reg.get("beta").is_none());
    }

    #[test]
    fn list_returns_all_tools() {
        let reg = InMemoryToolRegistry::new()
            .with_tool(DummyTool { tool_name: "alpha" })
            .with_tool(DummyTool { tool_name: "beta" });
        let names: Vec<&str> = reg.list().iter().map(|t| t.name()).collect();
        assert_eq!(names, vec!["alpha", "beta"]);
    }

    #[test]
    fn tool_definitions_has_correct_shape() {
        let reg = InMemoryToolRegistry::new().with_tool(DummyTool { tool_name: "alpha" });
        let defs = reg.tool_definitions();
        assert_eq!(defs.len(), 1);
        assert_eq!(defs[0]["name"], "alpha");
        assert_eq!(defs[0]["description"], "a dummy tool");
        assert!(defs[0]["input_schema"].is_object());
    }

    #[test]
    fn empty_registry_returns_empty() {
        let reg = InMemoryToolRegistry::new();
        assert!(reg.get("anything").is_none());
        assert!(reg.list().is_empty());
        assert!(reg.tool_definitions().is_empty());
    }
}
