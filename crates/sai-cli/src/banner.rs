//! Startup banner display.

use std::io::Write as _;
use std::path::Path;

/// Print the startup banner to stderr.
///
/// Format:
/// ```text
/// sai-code v{version}
/// Model: {model}
/// Directory: {dir}
/// ```
pub fn display_banner(model: &str, dir: &Path) {
    let version = env!("CARGO_PKG_VERSION");
    let dir_display = dir.display();
    let mut stderr = std::io::stderr();
    let _ = writeln!(stderr, "sai-code v{version}");
    let _ = writeln!(stderr, "Model: {model}");
    let _ = writeln!(stderr, "Directory: {dir_display}");
    let _ = writeln!(stderr);
}
