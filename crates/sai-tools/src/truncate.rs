//! Output truncation utility.

/// Truncate `output` to at most `max_bytes`, appending a marker if truncated.
///
/// If the output fits within the limit it is returned unchanged.
/// Otherwise the output is cut at the last valid UTF-8 boundary before
/// `max_bytes` and a human-readable marker is appended.
pub fn truncate_output(output: &str, max_bytes: usize) -> String {
    if output.len() <= max_bytes {
        return output.to_string();
    }

    // Find the last valid char boundary at or before max_bytes.
    let mut end = max_bytes;
    while end > 0 && !output.is_char_boundary(end) {
        end -= 1;
    }

    let total = output.len();
    let truncated = &output[..end];
    format!("{truncated}\n... [output truncated at {end} bytes, {total} bytes total]")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn under_limit_unchanged() {
        let s = "hello";
        assert_eq!(truncate_output(s, 100), "hello");
    }

    #[test]
    fn at_limit_unchanged() {
        let s = "hello";
        assert_eq!(truncate_output(s, 5), "hello");
    }

    #[test]
    fn over_limit_truncated() {
        let s = "hello world";
        let result = truncate_output(s, 5);
        assert!(result.starts_with("hello"));
        assert!(result.contains("[output truncated at 5 bytes, 11 bytes total]"));
    }

    #[test]
    fn empty_input() {
        assert_eq!(truncate_output("", 100), "");
    }

    #[test]
    fn multibyte_boundary_respected() {
        // 'é' is 2 bytes in UTF-8
        let s = "café";
        // "caf" = 3 bytes, "é" = 2 bytes, total = 5 bytes
        // Truncating at 4 should not split the 'é'
        let result = truncate_output(s, 4);
        assert!(result.starts_with("caf"));
        assert!(result.contains("[output truncated"));
    }
}
