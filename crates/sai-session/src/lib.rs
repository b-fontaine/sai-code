//! sai-session: Filesystem session persistence adapter.
//!
//! Implements the `SessionPort` trait from `sai-core` using the local filesystem.
//! Sessions are stored in the platform-appropriate data directory
//! (`~/.local/share/sai/sessions/` on Linux,
//! `~/Library/Application Support/sai/sessions/` on macOS).

pub mod adapter;
mod error;

pub use adapter::FilesystemSessionAdapter;
