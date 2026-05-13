/// Maximum response size in bytes (~10K tokens).
pub const MAX_RESPONSE_BYTES: usize = 40_000;

/// Truncate text if it exceeds [`MAX_RESPONSE_BYTES`], appending a clear notice.
pub fn truncate_if_needed(text: &str) -> String {
    if text.len() <= MAX_RESPONSE_BYTES {
        return text.to_string();
    }
    let truncated = &text[..MAX_RESPONSE_BYTES];
    format!(
        "{truncated}\n\n\
         [TRUNCATED: response exceeded 10K token limit. \
         Use limit/offset or more specific filters to narrow results.]"
    )
}
