//! Built-in tool implementations for the sai-code agent.
//!
//! This crate provides concrete tools that implement the `ToolPort` trait
//! from `sai-core`. Each tool handles a specific capability (file read,
//! shell execution, etc.) and integrates with the permission system
//! via the `ToolExecutor`.

mod config;
mod file_edit;
mod file_read;
mod file_write;
mod glob;
mod grep;
mod registry;
mod shell;
mod shell_safety;
mod truncate;
mod validate;

pub use config::ToolConfig;
pub use file_edit::FileEditTool;
pub use file_read::FileReadTool;
pub use file_write::FileWriteTool;
pub use glob::GlobTool;
pub use grep::GrepTool;
pub use registry::InMemoryToolRegistry;
pub use shell::ShellTool;
