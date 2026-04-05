//! sai-llm: Multi-provider LLM adapter for sai-code.
//!
//! Implements `sai_core::ports::llm::LlmPort` using the `genai` crate
//! for multi-provider LLM support. Normalizes streaming responses,
//! tool calling, and error handling across all supported providers.
//!
//! The `genai` crate is a private implementation detail — no genai types
//! appear in this crate's public API.

mod adapter;
mod convert;
mod provider;

pub use adapter::GenaiLlmAdapter;
