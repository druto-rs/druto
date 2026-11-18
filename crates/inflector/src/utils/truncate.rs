//! String truncation, word count limiting, and context excerpt extraction.

/// Options for configuring character truncation.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct TruncateOptions {
    /// Suffix appended when truncation occurs (default: `"..."`).
    pub suffix: String,
    /// Whether to avoid cutting words in half (default: `false`).
    pub complete_words: bool,
}

impl Default for TruncateOptions {
    fn default() -> Self {
        Self {
            suffix: "...".to_string(),
            complete_words: false,
        }
    }
}

impl TruncateOptions {
    /// Creates a default `TruncateOptions` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the truncation suffix.
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }

    /// Configures whole-word boundary preservation.
    pub fn with_complete_words(mut self, complete: bool) -> Self {
        self.complete_words = complete;
        self
    }
}

/// Truncates a string slice to a maximum character length.
///
/// Slices safely along Unicode scalar values without panicking.
pub fn truncate(input: &str, max_characters: usize, options: &TruncateOptions) -> String {
    let character_count = input.chars().count();
    if character_count <= max_characters {
        return input.to_string();
    }

    let suffix_len = options.suffix.chars().count();
    let target_character_count = max_characters.saturating_sub(suffix_len);

    let byte_limit = input
        .char_indices()
        .nth(target_character_count)
        .map_or(input.len(), |(byte_offset, _)| byte_offset);

    let candidate = &input[..byte_limit];

    if options.complete_words
        && let Some(last_space_byte) = candidate.rfind(char::is_whitespace)
    {
        let mut result = String::with_capacity(last_space_byte + options.suffix.len());
        result.push_str(&candidate[..last_space_byte]);
        result.push_str(&options.suffix);
        return result;
    }

    let mut result = String::with_capacity(candidate.len() + options.suffix.len());
    result.push_str(candidate);
    result.push_str(&options.suffix);
    result
}

/// Limits a string to a given number of words.
pub fn words(input: &str, max_words: usize, suffix: &str) -> String {
    if max_words == 0 || input.is_empty() {
        return String::new();
    }

    let word_list: Vec<&str> = input.split_whitespace().collect();
    if word_list.len() <= max_words {
        return input.to_string();
    }

    let selected = &word_list[..max_words];
    let mut result = selected.join(" ");
    result.push_str(suffix);
    result
}

/// Options for configuring context excerpt extraction.
#[derive(Debug, Clone)]
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct ExcerptOptions {
    /// Number of characters to include before and after the matched search term (default: 50).
    pub radius: usize,
    /// Suffix prepended/appended when content is clipped (default: `"..."`).
    pub suffix: String,
}

impl Default for ExcerptOptions {
    fn default() -> Self {
        Self {
            radius: 50,
            suffix: "...".to_string(),
        }
    }
}

impl ExcerptOptions {
    /// Creates a default `ExcerptOptions` instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the context character radius.
    pub fn with_radius(mut self, radius: usize) -> Self {
        self.radius = radius;
        self
    }

    /// Sets the clipping suffix.
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = suffix.into();
        self
    }
}

/// Extracts a surrounding context excerpt around the first case-insensitive match of a search term.
pub fn excerpt(input: &str, term: &str, options: &ExcerptOptions) -> String {
    if input.is_empty() || term.is_empty() {
        return String::new();
    }

    let term_chars: Vec<char> = term.chars().collect();
    let mut match_range: Option<(usize, usize)> = None;

    for (start_byte, _) in input.char_indices() {
        let candidate_slice = &input[start_byte..];
        let mut matched_chars = 0;
        let mut matched_bytes = 0;

        for (candidate_char, target_char) in candidate_slice.chars().zip(term_chars.iter()) {
            if candidate_char.to_lowercase().ne(target_char.to_lowercase()) {
                break;
            }
            matched_chars += 1;
            matched_bytes += candidate_char.len_utf8();
        }

        if matched_chars == term_chars.len() {
            match_range = Some((start_byte, start_byte + matched_bytes));
            break;
        }
    }

    let Some((match_byte_start, match_byte_end)) = match_range else {
        return String::new();
    };

    let chars_before: Vec<(usize, char)> = input[..match_byte_start].char_indices().collect();
    let chars_after: Vec<(usize, char)> = input[match_byte_end..].char_indices().collect();

    let has_prefix_omission = chars_before.len() > options.radius;
    let has_suffix_omission = chars_after.len() > options.radius;

    let excerpt_byte_start = if has_prefix_omission {
        let skip_count = chars_before.len() - options.radius;
        chars_before[skip_count].0
    } else {
        0
    };

    let excerpt_byte_end = if has_suffix_omission {
        let after_offset = chars_after[options.radius].0;
        match_byte_end + after_offset
    } else {
        input.len()
    };

    let body = &input[excerpt_byte_start..excerpt_byte_end];
    let mut result = String::with_capacity(body.len() + options.suffix.len() * 2);

    if has_prefix_omission {
        result.push_str(&options.suffix);
    }
    result.push_str(body);
    if has_suffix_omission {
        result.push_str(&options.suffix);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        let default_options = TruncateOptions::default();
        assert_eq!(
            truncate("The quick brown fox jumps", 15, &default_options),
            "The quick br..."
        );
        assert_eq!(truncate("Short", 10, &default_options), "Short");

        let word_options = TruncateOptions::new().with_complete_words(true);
        assert_eq!(
            truncate("The quick brown fox jumps", 15, &word_options),
            "The quick..."
        );
    }

    #[test]
    fn test_words() {
        assert_eq!(
            words("The quick brown fox jumps over the lazy dog", 4, "..."),
            "The quick brown fox..."
        );
        assert_eq!(words("Hello world", 5, "..."), "Hello world");
    }

    #[test]
    fn test_excerpt() {
        let text = "This is a long introductory text that leads to a critical keyword which continues for quite some time afterwards.";
        let options = ExcerptOptions::new().with_radius(15);
        let result = excerpt(text, "critical", &options);
        assert!(result.contains("critical"));
        assert!(result.starts_with("..."));
        assert!(result.ends_with("..."));
    }
}
