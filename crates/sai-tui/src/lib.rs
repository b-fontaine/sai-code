//! sai-tui: Rich terminal UI adapter for sai-code using ratatui.
// Allow dead code while crate is being scaffolded; stubs will be consumed
// by later implementation tasks.
#![allow(dead_code)]

pub mod adapters;
mod app;
pub mod components;
mod event;
mod terminal;
mod tui;

pub use adapters::permissions::TuiPermissionsAdapter;
pub use adapters::ui::TuiUiAdapter;
pub use app::TuiConfig;
pub use tui::{TuiApp, TuiError};
