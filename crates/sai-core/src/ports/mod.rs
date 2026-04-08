//! Port traits: hexagonal architecture boundaries.
//!
//! These traits define the contracts that adapter crates must implement.
//! The agent loop depends only on these traits, never on concrete implementations.

pub mod llm;
pub mod permissions;
pub mod session;
pub mod tool;
pub mod ui;
