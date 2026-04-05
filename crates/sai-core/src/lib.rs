//! sai-core: Domain layer for the sai-code CLI agent.
//!
//! Contains domain entities, port trait definitions, and service
//! orchestration for the agent loop. This crate has zero infrastructure
//! dependencies — all external concerns are accessed through port traits.

pub mod domain;
pub mod error;
pub mod ports;
pub mod services;
